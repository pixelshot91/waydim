# waydim

Hybrid hardware + software brightness manager (Wayland-compatible).

Goals
- Only adjust display brightness (no ambient webcam usage, no keyboard backlight).
- Clamp hardware brightness to a safe range: raw units [100, 1000].
- If user requests dimmer than hardware minimum, use software gamma (via `wl-gammactl`) to go darker.
- No automatic brightness changes.

Install
1. Install Rust and build:
   cargo build --release

2. Ensure `wl-gammactl` is installed (used for software gamma):
   - On NixOS, add a package that provides `wl-gammactl` or build it manually.
   - Alternatively, replace calls to `wl-gammactl` in `src/main.rs` with a custom Wayland gamma client.

3. Permissions: writing to `/sys/class/backlight/*/brightness` often requires elevated privileges.
   - Option 1: run as root (not recommended for daily use).
   - Option 2: use a systemd user unit with appropriate permissions or a small helper with setuid (be careful).
   - Option 3: create udev rules that allow your user to write to the backlight devices (NixOS-specific).

Usage
- Show state:
  ./target/release/waydim show

- Set absolute brightness:
  ./target/release/waydim set 30%   # sets to 30%
  ./target/release/waydim set 0.3   # sets to 30%

- Increase/decrease:
  ./target/release/waydim up 5%     # +5%
  ./target/release/waydim down 5%   # -5%

Sway / DE keybinding (example)
Add to your Sway config (~/.config/sway/config):
bindsym XF86MonBrightnessUp exec /home/you/.local/bin/waydim up 5%
bindsym XF86MonBrightnessDown exec /home/you/.local/bin/waydim down 5%

Systemd user unit (example)
See `systemd/waydim.service` for a sample systemd user service for a persistent helper (if you want one).

Credits
Created by @pixelshot91 — feel free to open issues or PRs.
