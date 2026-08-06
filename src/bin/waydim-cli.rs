use clap::{Parser, Subcommand};
use color_eyre::eyre::Context;
use tarpc::tokio_serde::formats::Bincode;
use tarpc::{client, context};
use tokio::net::UnixStream;
use waydim::rpc::waydim_api::WayDimAPIClient;

// use rpc

// use crate::rpc::common::InsightClient;
// use crate::{service::Bundle, tui};

#[derive(Parser, Debug)] // requires `derive` feature
struct CliArg {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Get the brightness state
    Get {},
    /// Set the brightness state
    Set { nit: f64 },
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> color_eyre::Result<()> {
    let args = CliArg::parse();
    const SOCKET_PATH: &str = "/tmp/waydim.sock";
    let conn = UnixStream::connect(SOCKET_PATH)
        .await
        .wrap_err(format!("While opening {SOCKET_PATH}"))?;

    let codec_builder = tarpc::tokio_util::codec::LengthDelimitedCodec::builder();
    let transport = tarpc::serde_transport::new(codec_builder.new_framed(conn), Bincode::default());
    let client = WayDimAPIClient::new(client::Config::default(), transport).spawn();

    // let res = client.get_brightness(context::current()).await?;

    match args.command {
        Commands::Get {} => {
            let res = client.get_brightness(context::current()).await?;
            dbg!(&res);
        }
        Commands::Set { nit } => {
            let res = client
                .set_brightness(
                    context::current(),
                    waydim::common::Nit::try_new(nit).unwrap(),
                )
                .await?;
            dbg!(&res);
        }
    };

    return Ok(());
}
