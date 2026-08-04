use anyhow::{Context, Result};
use dbus::blocking::stdintf::org_freedesktop_dbus::Properties as _;
use dbus::blocking::Connection;
use std::time::Duration;

pub fn set_software_brightness(factor: f32) -> Result<()> {
    let conn = Connection::new_session().context("connecting to DBus session bus")?;

    /// Minimum software factor to avoid invalid zero-gamma
    const MIN_SOFTWARE_FACTOR: f32 = 0.3;
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
pub fn set_hardware_brightness(brightness: u32) -> anyhow::Result<()> {
    set_backlight_brightness(brightness)
}
pub fn set_backlight_brightness(brightness: u32) -> anyhow::Result<()> {
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

/* fn read_u32(path: &Path) -> Result<u32> {
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
fn apply_hardware_brightness(
    backlight: &Path,
    hw_lower: u32,
) -> std::prelude::v1::Result<(), anyhow::Error> {
    write_u32(&backlight.join("brightness"), hw_lower)
}
 */

// fn show_state(backlight: &Path) -> Result<()> {
//     let current = read_u32(&backlight.join("brightness"))?;
//     let device_max = read_u32(&backlight.join("max_brightness"))?;
//     // We don't store the software factor state; query none — best-effort show
//     println!("backlight: {}", backlight.display());
//     println!("device max_brightness: {}", device_max);
//     println!("current hardware brightness: {}", current);
//     // There is no easy portable way to read the current gamma factor from the relay; just tell user how to inspect
//     println!(
//         "software gamma: controlled by {} daemon; inspect the relay's logs or control interface.",
//         WL_GAMMA_RELAY
//     );
//     println!("hardware clamp: [{}, {}] (raw units)", HW_MIN, device_max);
//     Ok(())
// }
#[cfg(test)]
mod test {
    use crate::brightness_modifier::*;

    #[test]
    #[ignore = "Side-effect: really modify the software brightness"]
    fn test_software_brightness() {
        set_software_brightness(0.8).unwrap();
        std::thread::sleep(Duration::from_millis(500));
        set_software_brightness(1.0).unwrap();
    }

    #[test]
    #[ignore = "Side-effect: really modify the backlight brightness"]
    fn test_backlight_brightness() {
        set_backlight_brightness(500).unwrap();
        std::thread::sleep(Duration::from_millis(500));
        set_backlight_brightness(400).unwrap();
    }

    // #[test]
    // #[ignore = "Side-effect: really modify the brightness"]
    // fn test_brightness() {
    //     let backlight = Path::new("/sys/class/backlight/intel_backlight");
    //     set_brightness(backlight, 0.8).unwrap();
    //     std::thread::sleep(Duration::from_millis(500));
    //     set_brightness(backlight, 1.0).unwrap();
    // }
}
// fn apply_software_gamma_delta(factor: f32) -> Result<()> {
//     // clamp factor into [0.0, 1.0] and convert to DBus double
//     let factor = (factor.clamp(0.0, 1.0)) as f64;

//     // connect to the user session bus
//     let conn = Connection::new_session().context("connecting to DBus session bus")?;
//     const WL_GAMMA_SERVICE: &str = "rs.wl-gammarelay";
//     const WL_GAMMA_INTERFACE: &str = "rs.wl.gammarelay";
//     // short timeout for method call
//     let proxy = conn.with_proxy(WL_GAMMA_SERVICE, "/", Duration::from_millis(5000));

//     // Call Brighness(double)
//     proxy
//         .method_call::<(), _, _, _>(WL_GAMMA_INTERFACE, "UpdateBrightness", (factor,))
//         .with_context(|| format!("calling {}.Brighness", WL_GAMMA_SERVICE))?;

//     Ok(())
// }
