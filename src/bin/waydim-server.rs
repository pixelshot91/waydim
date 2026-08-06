use std::{
    future::Future,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

use fs_err::PathExt;
use opentelemetry::KeyValue;
use tokio::{net::UnixListener, process::Command, task::JoinSet, time::timeout};

use waydim::rpc::{server::WayDimServer, waydim_api::WayDimAPI};

use clap::Parser;
use futures::prelude::*;

use color_eyre::{
    eyre::{eyre, Context as _},
    Result,
};

use tarpc::{
    server::{BaseChannel, Channel},
    tokio_serde::formats::Bincode,
    tokio_util::codec::LengthDelimitedCodec,
};
use tracing::{debug, error, field, info, warn, Instrument as _};

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Hybrid hardware + software dimmer (Wayland-compatible)"
)]
struct Cli {
    // #[command(subcommand)]
    // command: Commands,
    // /// Backlight path to use (overrides autodetect), e.g. /sys/class/backlight/intel_backlight
    // #[arg(short, long)]
    // backlight: Option<PathBuf>,
}

// opentelemetry metric provider need multi_thread runtime
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // let config: Config = Config::load_from_file(&cli.config).wrap_err(format!(
    //     "Failed to load config file {}",
    //     cli.config.display()
    // ))?;

    waydim::telemetry::init_telemetry()?;

    // setup_waydim_cli_server().unwrap();

    // let meter = opentelemetry::global::meter("waydim");

    // let service_hysteresis_state_instrument = meter
    //     .f64_gauge("waydim_service_hysteresis_state")
    //     .with_description("Like service_up, but more detailed. It aggregates the result the last function_return value.
    //     It can take intermediate values between 0 and 1 for a failed service raising, or a successful service failing")
    //     .build();
    // }

    // fn setup_waydim_cli_server() -> Result<()> {
    async fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
        debug!("spawning");
        tokio::spawn(fut);
    }

    let pid_path = "/tmp/waydim.pid";
    match fs_err::read_to_string(pid_path) {
        Ok(stored_pid) => {
            let stored_pid = stored_pid.trim();
            let another_bw_is_running =
                std::path::Path::new(&format!("/proc/{stored_pid}")).fs_err_try_exists()?;
            if another_bw_is_running {
                error!("Another waydim with PID {stored_pid} is already running");
                std::process::exit(1);
            } else {
                fs_err::write(pid_path, format!("{}\n", std::process::id()))?;
            }
        }
        Err(e) => {
            if let std::io::ErrorKind::NotFound = e.kind() {
                fs_err::write(pid_path, format!("{}\n", std::process::id()))?;
            } else {
                return Err(e).context(format!(
                    "Trying to know if another waydim is running by looking at PID file `{pid_path}`",
                ));
            }
        }
    }

    let socket_path = "/tmp/waydim.sock";
    match std::fs::remove_file(socket_path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e).context(format!("Cannot remove file {socket_path}")),
    }

    let listener = UnixListener::bind(Path::new(socket_path)).unwrap();

    let codec_builder = LengthDelimitedCodec::builder();

    loop {
        let (conn, _addr) = listener.accept().await.unwrap();
        let framed = codec_builder.new_framed(conn);
        let transport = tarpc::serde_transport::new(framed, Bincode::default());

        let server = WayDimServer {};
        let fut = BaseChannel::with_defaults(transport)
            .execute(server.serve())
            .for_each(spawn);
        tokio::spawn(fut);
    }
    Ok(())
}
