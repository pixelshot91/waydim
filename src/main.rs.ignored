mod brightness_mapper;
mod brightness_modifier;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};

use crate::brightness_mapper::{BrightnessMapper, SamplePoint};
use crate::brightness_modifier::{set_hardware_brightness, set_software_brightness};

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
    // /// Increase brightness by percent (e.g. 5 or 5%)
    // Up { step: Option<String> },
    // /// Decrease brightness by percent
    // Down { step: Option<String> },
    // /// Print current hardware & software state
    // Show,
}

/* fn parse_percent(s: &str) -> Result<f32> {
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
 */
fn parse_nit(s: &str) -> Result<Nit> {
    let s = s.trim();
    Nit::try_new(s.parse()?)
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

fn set_brightness(mapper: &BrightnessMapper, target_luminance: Nit) -> Result<()> {
    let settings = mapper.nits_to_setting(target_luminance);

    println!("Using {:?}", settings);

    set_hardware_brightness(settings.hw)?;
    set_software_brightness(settings.sw)?;

    Ok(())
}

// Luminance in nits (cd/m2)
struct Nit(f32);
impl Nit {
    fn try_new(v: f32) -> Result<Self> {
        if v <= 0.0 {
            return Err(anyhow::anyhow!("A luminance is always positive"));
        }
        Ok(Self(v))
    }
}

fn main() -> Result<()> {
    let calibration = vec![
        SamplePoint {
            sw: 0.5,
            hw: 1,
            nits: 1.0,
        },
        SamplePoint {
            sw: 1.0,
            hw: 1,
            nits: 2.0,
        }, // Derived min hardware point
        SamplePoint {
            sw: 1.0,
            hw: 10,
            nits: 8.5,
        },
        SamplePoint {
            sw: 1.0,
            hw: 100,
            nits: 45.0,
        },
        SamplePoint {
            sw: 1.0,
            hw: 1000,
            nits: 180.0,
        },
        SamplePoint {
            sw: 1.0,
            hw: 10000,
            nits: 400.0,
        },
    ];
    let mapper = BrightnessMapper::new(calibration);

    let cli = Cli::parse();
    // let backlight = find_backlight(cli.backlight.clone())?;
    match cli.command {
        // Commands::Show => {
        //     // show_state(&backlight)?;
        // }
        Commands::Set { value } => {
            // let p = parse_percent(&value).context("parsing percent")?;
            let l = parse_nit(&value).context("parsing nit")?;
            set_brightness(&mapper, l)?;
        } // Commands::Up { step } => {
          //     let step = step.as_deref().unwrap_or("5").to_string();
          //     let step_p = parse_percent(&step)?;
          //     // read current approximate percent
          //     let hw_max = read_u32(&backlight.join("max_brightness"))?;
          //     let current_hw = read_u32(&backlight.join("brightness"))?;
          //     let current_percent = (current_hw as f32) / (hw_max as f32);
          //     let new = (current_percent + step_p).clamp(0.0, 1.0);
          //     set_brightness(&backlight, new)?;
          // }
          // Commands::Down { step } => {
          //     let step = step.as_deref().unwrap_or("5").to_string();
          //     let step_p = parse_percent(&step)?;
          //     let hw_max = read_u32(&backlight.join("max_brightness"))?;
          //     let current_hw = read_u32(&backlight.join("brightness"))?;
          //     let current_percent = (current_hw as f32) / (hw_max as f32);
          //     let new = (current_percent - step_p).clamp(0.0, 1.0);
          //     set_brightness(&backlight, new)?;
          // }
    }
    Ok(())
}
