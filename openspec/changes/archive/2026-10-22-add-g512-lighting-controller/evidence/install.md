# Fresh install from the README (task 5.1), 2026-10-22

Done with the user's OK, from a clean `git clone` of the branch at `4f10483` (into `target/fresh`). Every file the install needs had to be tracked in git, and each command was run exactly as the README gives it.

## Uninstall (README "Uninstall", 13:40 UTC)

- `flatpak uninstall --user io.github.g512control.Control` removed the app and its Debug extension.
- `packaging/install-service.sh --uninstall` reported that saved lighting is kept in `~/.config/g512-control/`.
- Afterwards:
  - `g512d` was `inactive` and `not-found`;
  - `~/.local/bin/g512d`, the user unit, and the D-Bus activation file were gone;
  - the bus name had no owner, and nothing re-activated it;
  - `state.toml` was still there.
- The udev-rule removal line was **not** run. It needs sudo, and it would cut the service off from the keyboard. Installing the rule (README step 1) was already done on this machine earlier in the change.

## Reinstall (README "Install", steps 2 and 3)

- **`packaging/install-service.sh`:**
  - built `g512d` from the clone in 27 s;
  - enabled the unit under `default.target.wants` and started it;
  - in the journal at 09:41:06.81 local: `connected: G512 RGB MECHANICAL GAMING KEYBOARD on /dev/hidraw1, firmware U1 19.01 build 0014`;
  - `DeviceState` was `connected`, and `State` was `"Ripple" 61 56 70 52 100`, the saved state.
- **The README's `flatpak run org.flatpak.Builder ...` command:**
  - built and installed the app in 68 s, exit 0 (`--locked` works because `Cargo.lock` is tracked);
  - permissions are unchanged: ipc, wayland, fallback-x11, dri, and `io.github.g512control.Lighting1=talk`;
  - the desktop file and the scalable icon are exported to `~/.local/share/flatpak/exports/share/`.
- **Launch:** started the way the app grid does, with `gio launch` on the exported desktop file.
  - The app called `GetAll` on `io.github.g512control.Lighting1` through the Flatpak bus proxy.
  - Its window, read through AT-SPI, showed:
    - `G512 RGB MECHANICAL GAMING KEYBOARD`, `Firmware U1 19.01 build 0014`;
    - Effect `Ripple`;
    - Colour `Red 24%, Green 22%, Blue 27%`;
    - Speed 52, Brightness 100.
