use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::brightness_mapper::BrightnessMapper;
use crate::driver;

// use crate::brightness_mapper::{BrightnessMapper, SamplePoint};
// use crate::brightness_modifier::{set_hardware_brightness, set_software_brightness};

#[derive(Serialize, Deserialize, Debug, Error)]
pub enum WayDimAPIError {
    #[error("Internal daemon error: {0}")]
    Internal(String),
}

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

    driver::set_hardware_brightness(settings.hw)?;
    driver::set_software_brightness(settings.sw)?;

    Ok(())
}

// Luminance in nits (cd/m2)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Nit(pub f32);
impl Nit {
    pub fn try_new(v: f32) -> Result<Self> {
        if v <= 0.0 {
            return Err(anyhow::anyhow!("A luminance is always positive"));
        }
        Ok(Self(v))
    }
}
