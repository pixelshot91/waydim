use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Clamp hardware brightness between these values (raw sysfs units).
const HW_MIN: u32 = 100;
const HW_MAX: u32 = 1000;

/// Minimum software factor to avoid invalid zero-gamma
const MIN_SOFTWARE_FACTOR: f32 = 0.01;

#[derive(Parser)]
#[command(author, version, about = "Hybrid hardware + software dimmer (Wayland-compatible)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Backlight path to use (overrides autodetect), e.g. /sys/class/backlight/intel_backlight
    #[arg(short, long)]
    backlight: Option<PathBuf>,
    /// Try to use this wl-gammactl binary name/path
    #[arg(short = 'g', long, default_value = "wl-gammactl")]
    wl_gammactl: String,
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

/// Apply software brightness via wl-gammactl -c <factor>
/// factor: 1.0 is no change, <1.0 is darker.
fn apply_software_gamma(wl_gammactl: &str, factor: f32) -> Result<()> {
    // clamp factor to a sane non-zero value
    let factor = factor.max(MIN_SOFTWARE_FACTOR);
    // Call wl-gammactl -c <factor>
    let status = Command::new(wl_gammactl)
        .arg("-c")
        .arg(format!("{}", factor))
        .status()
        .with_context(|| format!("failed to run {}", wl_gammactl))?;
    if !status.success() {
        anyhow::bail!("{} exited with {}", wl_gammactl, status);
    }
    Ok(())
}

fn show_state(backlight: &Path, wl_gammactl: &str) -> Result<()> {
    let current = read_u32(&backlight.join("brightness"))?;
    let device_max = read_u32(&backlight.join("max_brightness"))?;
    // We don't store the software factor state; query none — best-effort show
    println!("backlight: {}", backlight.display());
    println!("device max_brightness: {}", device_max);
    println!("current hardware brightness: {}", current);
    // There is no easy portable way to read the current wl-gammactl factor; just tell user how to inspect
    println!("software gamma: controlled externally (wl-gammactl). To inspect, call wl-gammactl --help or check compositor state.");
    println!("hardware clamp: [{}, {}] (raw units)", HW_MIN, HW_MAX);
    Ok(())
}

fn set_brightness(backlight: &Path, wl_gammactl: &str, target_percent: f32) -> Result<()> {
    let device_max = read_u32(&backlight.join("max_brightness"))?;
    let hw_upper = device_max.min(HW_MAX);
    let hw_lower = HW_MIN;

    // desired value in raw units relative to hw_upper
    let desired_raw = (target_percent * (hw_upper as f32)).round() as i64;
    let desired_raw = desired_raw.max(0) as u32;

    if desired_raw == 0 {
        // User wants "off" — set hardware to hw_lower and software factor to 0 (or MIN_SOFTWARE_FACTOR)
        write_u32(&backlight.join("brightness"), hw_lower)?;
        // set software gamma to near-zero
        apply_software_gamma(wl_gammactl, 0.0)?;
        println!(
            "Requested 0: set hardware to {} and applied software gamma to 0 (via {}).",
            hw_lower, wl_gammactl
        );
        return Ok(());
    }

    if desired_raw < hw_lower {
        // Hardware set to hw_lower, use software factor to reach desired_raw/hw_lower
        write_u32(&backlight.join("brightness"), hw_lower)?;
        let factor = (desired_raw as f32) / (hw_lower as f32);
        apply_software_gamma(wl_gammactl, factor)?;
        println!(
            "hardware set to {} (clamped); applied software gamma factor {:.3}",
            hw_lower, factor
        );
    } else {
        // We can set hardware directly. If hw_upper < device_max, that just means we don't push full device brightness.
        let hw_value = desired_raw.min(hw_upper);
        write_u32(&backlight.join("brightness"), hw_value)?;
        // Reset software gamma to 1.0 (no software dim)
        apply_software_gamma(wl_gammactl, 1.0)?;
        println!("hardware set to {}; software gamma reset to 1.0", hw_value);
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let backlight = find_backlight(cli.backlight.clone())?;
    match cli.command {
        Commands::Show => {
            show_state(&backlight, &cli.wl_gammactl)?;
        }
        Commands::Set { value } => {
            let p = parse_percent(&value).context("parsing percent")?;
            set_brightness(&backlight, &cli.wl_gammactl, p)?;
        }
        Commands::Up { step } => {
            let step = step
                .as_deref()
                .unwrap_or("5")
                .to_string();
            let step_p = parse_percent(&step)?;
            // read current approximate percent
            let device_max = read_u32(&backlight.join("max_brightness"))?;
            let hw_upper = device_max.min(HW_MAX);
            let current_hw = read_u32(&backlight.join("brightness"))?;
            let current_percent = (current_hw as f32) / (hw_upper as f32);
            let new = (current_percent + step_p).clamp(0.0, 1.0);
            set_brightness(&backlight, &cli.wl_gammactl, new)?;
        }
        Commands::Down { step } => {
            let step = step
                .as_deref()
                .unwrap_or("5")
                .to_string();
            let step_p = parse_percent(&step)?;
            let device_max = read_u32(&backlight.join("max_brightness"))?;
            let hw_upper = device_max.min(HW_MAX);
            let current_hw = read_u32(&backlight.join("brightness"))?;
            let current_percent = (current_hw as f32) / (hw_upper as f32);
            let new = (current_percent - step_p).clamp(0.0, 1.0);
            set_brightness(&backlight, &cli.wl_gammactl, new)?;
        }
    }
    Ok(())
}
