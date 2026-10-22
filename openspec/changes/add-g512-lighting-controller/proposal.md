# Proposal

## Why

The Logitech G512 RGB keyboard (USB `046d:c33c`) on this Fedora Silverblue / GNOME 50 desktop has no lighting control: Linux binds only `hid-generic`, the HID++ interface (`/dev/hidraw1`) is root-only, and no lighting software is installed. Probing the device showed it exposes HID++ 2.0 lighting features (`0x8070` ColorLedEffects, `0x8080` PerKeyLighting), so a small native controller can drive it directly instead of depending on a large cross-vendor tool.

## What Changes

- New Rust user service `g512d` that owns the keyboard's HID++ interface, resolves feature slots at runtime, and applies lighting through `0x8070` ColorLedEffects only.
- A hard allowlist of HID++ features the service may call (Root, FeatureSet, DeviceFwVersion, DeviceName, ColorLedEffects, PerKeyLighting). Firmware update, reset and hidden/engineering features are never reachable.
- Lighting modes for v1: Off, Static colour, Breathe, Colour cycle, Colour wave, Ripple, with a brightness level applied by scaling the colours sent (the device has no brightness feature).
- The service saves the last applied lighting state and reapplies it when the keyboard reconnects and when the system resumes from suspend.
- A session D-Bus interface so front ends can read device info, list supported effects, and set or read the lighting state.
- A GTK4 / libadwaita app ("G512 Control") that talks to the service over D-Bus: effect picker, colour picker, speed and brightness controls.
- A udev rule granting the active local session access to the keyboard's hidraw nodes, so neither the service nor the app runs as root.

Out of scope for this change (follow-up changes): per-key colouring and a key-layout widget; a GNOME Shell Quick Settings tile; other Logitech devices.

## Capabilities

### New Capabilities
- `device-protocol`: HID++ 2.0 transport to the G512, feature discovery, the feature allowlist, effect discovery, and setting a zone effect.
- `lighting-service`: the `g512d` user service, its D-Bus interface, persisted lighting state, reapply on reconnect and resume, and hidraw access via udev.
- `lighting-app`: the libadwaita front end for choosing effects, colour, speed and brightness.

### Modified Capabilities
<!-- None: this is a new project with no existing specs. -->

## Impact

- New repository `g512-control` (Rust workspace): a protocol library crate, the `g512d` service binary, and the GTK app.
- Host changes on install: a udev rules file under `/etc/udev/rules.d/`, a systemd user unit, and a D-Bus session service file.
- Dependencies: `gtk4`/`libadwaita` Rust bindings, a D-Bus crate (e.g. `zbus`), and udev / logind access for hotplug and resume events.
- Device risk: the service sends write commands to keyboard firmware. The setZoneEffect byte layout comes from public projects and has not yet been tested on this unit. The first implementation task is a hands-on check of it.
