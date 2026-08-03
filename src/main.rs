use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dbus::blocking::stdintf::org_freedesktop_dbus::Properties as _;
use dbus::blocking::Connection;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Clamp hardware brightness between these values (raw sysfs units).
const HW_MIN: u32 = 100;
const HW_MAX: u32 = 1000;

/// Minimum software factor to avoid invalid zero-gamma
const MIN_SOFTWARE_FACTOR: f32 = 0.3;

/// The gamma relay binary to control software gamma; this tool is required and not configurable.
const WL_GAMMA_RELAY: &str = "wl-gammarelay-rs";

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Hybrid hardware + software dimmer (Wayland-compatible)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Backlight path to use (overrides autodetect), e.g. /sys/class/backlight/intel_backlight
    #[arg(short, long)]
    backlight: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Set absolute brightness (percent). Accepts 0-100 or 0.0-1.0
    Set { value: String },
    /// Increase brightness by percent (e.g. 5 or 5%)
    Up { step: Option<String> },
    /// Decrease brightness by percent
    Down { step: Option<String> },
    /// Print current hardware & software state
    Show,
}

fn parse_percent(s: &str) -> Result<f32> {
    // accepts "50", "50%", "0.5"
    let s = s.trim();
    if let Some(s) = s.strip_suffix('%') {
        let v: f32 = s.parse()?;
        Ok((v.clamp(0.0, 100.0)) / 100.0)
    } else {
        let v: f32 = s.parse()?;
        if v > 1.0 {
            Ok((v.clamp(0.0, 100.0)) / 100.0)
        } else {
            Ok(v.clamp(0.0, 1.0))
        }
    }
}

fn find_backlight(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    let base = Path::new("/sys/class/backlight");
    let entries = fs::read_dir(base).context("reading /sys/class/backlight")?;
    for e in entries.flatten() {
        let path = e.path();
        if path.join("brightness").exists() && path.join("max_brightness").exists() {
            return Ok(path);
        }
    }
    anyhow::bail!("no backlight device found under /sys/class/backlight");
}

fn read_u32(path: &Path) -> Result<u32> {
    let mut s = String::new();
    fs::File::open(path)
        .with_context(|| format!("opening {}", path.display()))?
        .read_to_string(&mut s)?;
    let v: u32 = s.trim().parse()?;
    Ok(v)
}

fn write_u32(path: &Path, value: u32) -> Result<()> {
    let mut f = fs::File::create(path)?;
    write!(f, "{}", value)?;
    Ok(())
}

fn apply_software_gamma_delta(factor: f32) -> Result<()> {
    // clamp factor into [0.0, 1.0] and convert to DBus double
    let factor = (factor.clamp(0.0, 1.0)) as f64;

    // connect to the user session bus
    let conn = Connection::new_session().context("connecting to DBus session bus")?;
    const WL_GAMMA_SERVICE: &str = "rs.wl-gammarelay";
    const WL_GAMMA_INTERFACE: &str = "rs.wl.gammarelay";
    // short timeout for method call
    let proxy = conn.with_proxy(WL_GAMMA_SERVICE, "/", Duration::from_millis(5000));

    // Call Brighness(double)
    proxy
        .method_call::<(), _, _, _>(WL_GAMMA_INTERFACE, "UpdateBrightness", (factor,))
        .with_context(|| format!("calling {}.Brighness", WL_GAMMA_SERVICE))?;

    Ok(())
}

fn apply_software_brightness(factor: f32) -> Result<()> {
    let conn = Connection::new_session().context("connecting to DBus session bus")?;
    // clamp factor into [0.0, 1.0] and convert to DBus double
    let factor = (factor.clamp(MIN_SOFTWARE_FACTOR, 1.0)) as f64;
    // let factor = factor as f64;

    const WL_GAMMA_SERVICE: &str = "rs.wl-gammarelay";
    const WL_GAMMA_INTERFACE: &str = "rs.wl.gammarelay";

    let proxy = conn.with_proxy(WL_GAMMA_SERVICE, "/", Duration::from_millis(5000));

    // The Properties trait allows direct calling of .set()
    // Parameters: (interface_name, property_name, value)
    proxy
        .set(WL_GAMMA_INTERFACE, "Brightness", factor)
        .with_context(|| format!("setting Brightness property on {}", WL_GAMMA_SERVICE))?;

    Ok(())
}
fn show_state(backlight: &Path) -> Result<()> {
    let current = read_u32(&backlight.join("brightness"))?;
    let device_max = read_u32(&backlight.join("max_brightness"))?;
    // We don't store the software factor state; query none — best-effort show
    println!("backlight: {}", backlight.display());
    println!("device max_brightness: {}", device_max);
    println!("current hardware brightness: {}", current);
    // There is no easy portable way to read the current gamma factor from the relay; just tell user how to inspect
    println!(
        "software gamma: controlled by {} daemon; inspect the relay's logs or control interface.",
        WL_GAMMA_RELAY
    );
    println!("hardware clamp: [{}, {}] (raw units)", HW_MIN, HW_MAX);
    Ok(())
}

fn set_brightness(backlight: &Path, target_percent: f32) -> Result<()> {
    let device_max = read_u32(&backlight.join("max_brightness"))?;
    let hw_upper = device_max.min(HW_MAX);
    let hw_lower = HW_MIN;

    // desired value in raw units relative to hw_upper
    let desired_raw = (target_percent * (hw_upper as f32)).round() as i64;
    let desired_raw = desired_raw.max(0) as u32;

    if desired_raw == 0 {
        // User wants "off" — set hardware to hw_lower and software factor to near-zero
        write_u32(&backlight.join("brightness"), hw_lower)?;
        // set software gamma to near-zero
        apply_software_brightness(0.0)?;
        println!(
            "Requested 0: set hardware to {} and applied software gamma to 0 (via {}).",
            hw_lower, WL_GAMMA_RELAY
        );
        return Ok(());
    }

    if desired_raw < hw_lower {
        // Hardware set to hw_lower, use software factor to reach desired_raw/hw_lower
        apply_hardware_brightness(backlight, hw_lower)?;
        let factor = (desired_raw as f32) / (hw_lower as f32);
        apply_software_brightness(factor)?;
        println!(
            "hardware set to {} (clamped); applied software gamma factor {:.3}",
            hw_lower, factor
        );
    } else {
        // We can set hardware directly. If hw_upper < device_max, that just means we don't push full device brightness.
        let hw_value = desired_raw.min(hw_upper);
        apply_hardware_brightness(backlight, hw_lower)?;

        // Reset software gamma to 1.0 (no software dim)
        apply_software_brightness(1.0)?;
        println!("hardware set to {}; software gamma reset to 1.0", hw_value);
    }

    Ok(())
}

fn apply_hardware_brightness(
    backlight: &Path,
    hw_lower: u32,
) -> std::prelude::v1::Result<(), anyhow::Error> {
    write_u32(&backlight.join("brightness"), hw_lower)
}

fn set_backlight_brightness(brightness: u32) -> anyhow::Result<()> {
    // 1. systemd-logind resides on the System Bus, not the Session Bus
    let conn = Connection::new_system().context("connecting to DBus system bus")?;

    const LOGIND_SERVICE: &str = "org.freedesktop.login1";
    const LOGIND_PATH: &str = "/org/freedesktop/login1/session/auto";
    const LOGIND_INTERFACE: &str = "org.freedesktop.login1.Session";

    let proxy = conn.with_proxy(LOGIND_SERVICE, LOGIND_PATH, Duration::from_millis(5000));

    // 2. Map signature 'ssu' to Rust tuple (&str, &str, u32)
    proxy
        .method_call::<(), _, _, _>(
            LOGIND_INTERFACE,
            "SetBrightness",
            ("backlight", "intel_backlight", brightness),
        )
        .with_context(|| format!("failed to call SetBrightness on {}", LOGIND_SERVICE))?;

    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let backlight = find_backlight(cli.backlight.clone())?;
    match cli.command {
        Commands::Show => {
            show_state(&backlight)?;
        }
        Commands::Set { value } => {
            let p = parse_percent(&value).context("parsing percent")?;
            set_brightness(&backlight, p)?;
        }
        Commands::Up { step } => {
            let step = step.as_deref().unwrap_or("5").to_string();
            let step_p = parse_percent(&step)?;
            // read current approximate percent
            let device_max = read_u32(&backlight.join("max_brightness"))?;
            let hw_upper = device_max.min(HW_MAX);
            let current_hw = read_u32(&backlight.join("brightness"))?;
            let current_percent = (current_hw as f32) / (hw_upper as f32);
            let new = (current_percent + step_p).clamp(0.0, 1.0);
            set_brightness(&backlight, new)?;
        }
        Commands::Down { step } => {
            let step = step.as_deref().unwrap_or("5").to_string();
            let step_p = parse_percent(&step)?;
            let device_max = read_u32(&backlight.join("max_brightness"))?;
            let hw_upper = device_max.min(HW_MAX);
            let current_hw = read_u32(&backlight.join("brightness"))?;
            let current_percent = (current_hw as f32) / (hw_upper as f32);
            let new = (current_percent - step_p).clamp(0.0, 1.0);
            set_brightness(&backlight, new)?;
        }
    }
    Ok(())
}
#[cfg(test)]
mod test {
    use crate::*;

    #[test]
    fn test_software_brightness() {
        apply_software_brightness(0.8).unwrap();
        std::thread::sleep(Duration::from_millis(500));
        apply_software_brightness(1.0).unwrap();
    }

    #[test]
    fn test_backlight_brightness() {
        set_backlight_brightness(500).unwrap();
        std::thread::sleep(Duration::from_millis(500));
        set_backlight_brightness(400).unwrap();
    }

    #[test]
    fn test_brightness() {
        let backlight = Path::new("/sys/class/backlight/intel_backlight");
        set_brightness(backlight, 0.8).unwrap();
        std::thread::sleep(Duration::from_millis(500));
        set_brightness(backlight, 1.0).unwrap();
    }
}
