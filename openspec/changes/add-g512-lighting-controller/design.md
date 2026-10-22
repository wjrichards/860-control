# Design

## Context

See proposal.md for motivation. What we know about the device and the host comes from read-only probes in `evidence/`:

- **Device.** G512 RGB, firmware U1 19.01 build 0014, HID++ 4.2. Lighting goes through interface 1 (`/dev/hidraw1` today), long report `0x11` (20 bytes). ColorLedEffects `0x8070` v7 is in slot `0x0d` with one zone ("Primary", location `0x0001`) and 7 effects. PerKeyLighting `0x8080` v2 is in slot `0x0c`. There is no BrightnessControl `0x8040` and no OnboardProfiles `0x8100` (`evidence/feature-table.txt`, `evidence/effects-out.txt`).
- **Access.** Both hidraw nodes are `root:root 0600`, the kernel driver is `hid-generic`, and no lighting software is installed.
- **Host.** Fedora Silverblue 44, GNOME Shell 50.6, Wayland. Host `cargo`/`rustc` 1.96 is available, but the host has no `gtk4`, `libadwaita-1`, `libudev` or `dbus-1` development files. The `org.gnome.Sdk//50` Flatpak SDK, the Rust SDK extension and `org.flatpak.Builder` are installed, and a Fedora 44 toolbox (`dev`) exists.

## Goals / Non-Goals

**Goals:**
- The protocol code can be tested without a keyboard, using a fake transport that replays the recorded probe replies.
- No HID++ request outside the allowlist can be built, enforced in one place.
- The service builds with plain host `cargo`, with no C library dependencies.

**Non-Goals:**
- Using PerKeyLighting `0x8080` for anything beyond allowlisting it. It is reserved for the per-key follow-up change.
- Writing lighting to the keyboard's non-volatile memory.
- Supporting more than one G512 at a time. The first matching device wins.

## Decisions

### Workspace layout
A Cargo workspace with three crates:
- `g512-hidpp` (library): transport, the feature allowlist, discovery and effect commands.
- `g512d` (binary): the user service.
- `g512-control` (binary): the GTK4/libadwaita app.

The app depends only on the D-Bus interface, not on `g512-hidpp`.
*Alternative:* a single binary with an `--daemon` flag. Rejected, because it would link GTK into the service and tie the service's build to the GTK headers that the host lacks.

### Raw hidraw I/O, no hidapi
Open the hidraw node and `read`/`write` 20-byte reports, as the probes do. Find the device by scanning `/sys/class/hidraw/*/device` for the USB ID and the `0xFF43` usage page in `report_descriptor`.
*Alternative:* `hidapi`. Rejected, because it pulls in libudev/libusb C dependencies for no gain on Linux-only code.

### Allowlist enforced in the type system
`g512-hidpp` exposes a `Feature` enum with exactly the six allowed IDs. Its request builder takes only `Feature`, so other feature IDs are unrepresentable. The resolver maps a `Feature` to its runtime slot. A unit test asserts the enum's set of IDs against the spec list.
*Alternative:* a runtime check against a list. Kept as a second guard inside the transport (debug assertion), but not relied on alone.

### Lighting through ColorLedEffects only
All v1 modes, Static included, are sent with `0x8070` `setZoneEffect` (function 3) on zone 0. The per-key feature is not used.
- **Why:** the probe shows Static (`0x0001`) is offered as a zone effect, so a whole-keyboard colour needs no key map.
- **Command layout:** taken from Solaar and libratbag, and **not yet tested on this unit**. Requests carry zone index, effect index, a 10-byte parameter block (colour R,G,B, then period as big-endian u16 for effects that use it), and a persistence byte. The persistence byte is sent as 0 (RAM only).
- **Task 1.2 checks the layout first.** It is a hands-on test that sends Static red, then Breathe, by hand and records the byte layouts that actually work in `evidence/`. If the layout differs, it is fixed here before the protocol crate is written.

### Effect parameters
The service publishes per-effect parameters derived from the probe's capability bits, refined by task 1.2:

| Effect | ID | Colour | Speed | Default period |
|---|---|---|---|---|
| Off | `0x0000` | no | no | — |
| Static | `0x0001` | yes | no | — |
| Breathe | `0x000a` | yes | yes | 992 |
| Cycle | `0x0003` | no | yes | 992 |
| Wave | `0x0004` | no | yes | 30 |
| Ripple | `0x000b` | yes | yes | 30 |

The name "Wave" for `0x0004` and Ripple accepting a colour are inferences, confirmed or corrected in task 1.2. The speed range offered to users is a percentage mapped onto a period range that task 1.2 finds by testing. The raw period never reaches the UI.

### Service: zbus, async, user systemd unit
- **D-Bus:** pure-Rust `zbus` on a single async runtime. Well-known name `io.github.g512control.Lighting1`, object `/io/github/g512control/Lighting1`, interface `io.github.g512control.Lighting1`.
- **Single instance:** comes from requesting the well-known name with `DoNotQueue`. Failure means another instance owns it.
- **Process:** runs as a systemd user unit (`g512d.service`) with D-Bus activation as a fallback.

*Alternative:* system service with polkit. Rejected, because per-user state and session-scoped device access via `uaccess` fit a user service, and there is no need for root.

### Hotplug and resume without libudev
- **Hotplug:** a `NETLINK_KOBJECT_UEVENT` socket (pure Rust) listening for `hidraw` add/remove events. On add, the device is rescanned and the state reapplied after a 500 ms settle delay.
- **Resume:** `org.freedesktop.login1.Manager.PrepareForSleep(false)` on the system bus, followed by a reapply after a 1 s delay.
- Both paths share one reapply routine with up to 3 attempts, which keeps it within the 3 s spec limit.

### State file
`$XDG_CONFIG_HOME/g512-control/state.toml` holds effect, colour, speed and brightness. It is written atomically (temp file and rename) after each successful apply, plus the pending flag when set while disconnected.

### udev rule
`/etc/udev/rules.d/70-g512-control.rules`:
```
KERNEL=="hidraw*", SUBSYSTEM=="hidraw", ATTRS{idVendor}=="046d", ATTRS{idProduct}=="c33c", TAG+="uaccess"
```
This needs sudo once, because `/etc` is writable on Silverblue and doesn't need layering. The install script prints the command rather than running sudo itself.

### App: Flatpak on GNOME 50, D-Bus only
`g512-control` is built as a Flatpak (`org.gnome.Platform//50`, Rust SDK extension) with `--talk-name=io.github.g512control.Lighting1` and no `--device` permission. This works around the missing host GTK headers and matches the spec that the app never touches hidraw. App ID: `io.github.g512control.Control`.
- **UI:** an `AdwApplicationWindow` with an `AdwStatusPage` for non-connected states. The main page has a preferences-style list: an effects `AdwComboRow`, a colour row with `GtkColorDialogButton`, and speed and brightness `AdwSpinRow`/scale rows.
- **Updates:** throttled to 100 ms with trailing send.
- **Service install:** `cargo install --path g512d --root ~/.local` plus unit files copied to `~/.config/systemd/user/` and `~/.local/share/dbus-1/services/`.

## Risks / Trade-offs

- **[Wrong `setZoneEffect` layout puts the keyboard in an odd lighting state]** → Task 1.2 tests it by hand before any code depends on it. All writes are RAM-only, so unplugging the keyboard restores its default lighting.
- **[A firmware update changes slots or effect IDs]** → Slots are resolved at runtime, and effects come from the device's own list. Unrecognised IDs are hidden, not guessed.
- **[The service and another tool (OpenRGB, g810-led) both write to the keyboard]** → Out of scope to arbitrate. The README says to run only one.
- **[Resume event arrives before USB re-enumeration]** → The hotplug path also triggers a reapply. Retries cover the gap.
- **[The `io.github.g512control` namespace is a placeholder the user may not own]** → It is used consistently, so renaming before first release is a single search-and-replace.

## Migration Plan

New install, no migration. Rollback: `systemctl --user disable --now g512d`, remove the udev rule, then `flatpak uninstall io.github.g512control.Control`. The keyboard returns to its own default lighting on the next replug, because nothing is stored on it.

## Open Questions

- Does a persistent (`persistence=1`) `setZoneEffect` survive a replug on this firmware? If so, a later change could optionally store the chosen effect on the keyboard. It doesn't affect v1, which stays RAM-only.
- The final reverse-DNS namespace and app ID (see Risks).
