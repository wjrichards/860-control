# g512-control

Lighting control for the Logitech G512 RGB keyboard (USB `046d:c33c`) on a GNOME desktop.

It has three parts:

- **`g512d`**: a user service that owns the keyboard. It applies lighting over HID++ 2.0, saves the last setting, and re-applies it when the keyboard is plugged back in or the computer wakes from suspend. It offers a session D-Bus interface, `io.github.g512control.Lighting1`.
- **G512 Control** (`io.github.g512control.Control`): a GTK4/libadwaita app, packaged as a Flatpak. It talks to `g512d` over D-Bus and has no access to the keyboard itself.
- **`g512-hidpp`**: the protocol library both of these are built on.

Effects: Off, Static, Breathe, Colour cycle, Wave and Ripple. Each has the colour and speed settings it supports, plus brightness. The keyboard has no brightness command of its own: brightness dims the colour, or for Cycle and Wave it sets their intensity.

Tested on firmware U1 19.01 build 0014, Fedora Silverblue 44 and GNOME 50.

## Install

You need Rust (`cargo`) on the host. The app also needs `org.flatpak.Builder`, `org.gnome.Sdk//50` and `org.freedesktop.Sdk.Extension.rust-stable//25.08` from Flathub.

### 1. udev rule (once, needs sudo)

This lets the logged-in user open the keyboard, and only this keyboard. Details are in [packaging/INSTALL-udev.md](packaging/INSTALL-udev.md).

```sh
sudo install -m 0644 packaging/70-g512-control.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger --subsystem-match=hidraw
```

Then unplug and replug the keyboard.

### 2. Service

```sh
packaging/install-service.sh
```

This builds `g512d`, installs it to `~/.local/bin`, adds a systemd user unit (`g512d.service`) and a D-Bus activation file, and starts the service. It starts again at every login. Check it with `systemctl --user status g512d`.

### 3. App

```sh
flatpak run org.flatpak.Builder --user --install --force-clean \
    --state-dir=.flatpak-builder build-dir packaging/flatpak/io.github.g512control.Control.json
```

Open **G512 Control** from the app grid.

## Using it without the app

```sh
# Static orange at 60 % brightness (speed is ignored by Static)
busctl --user call io.github.g512control.Lighting1 /io/github/g512control/Lighting1 \
    io.github.g512control.Lighting1 SetState 's(yyy)yy' Static 255 128 0 50 60

# What the keyboard offers and what is set
busctl --user introspect io.github.g512control.Lighting1 /io/github/g512control/Lighting1
```

`SetState(effect, (r, g, b), speed, brightness)` takes an effect name from the `Effects` property, a speed from 1 (slowest) to 100 (fastest) and a brightness from 0 to 100. It returns `true` if the keyboard is not connected; in that case the state is saved and applied when the keyboard connects.

The saved state is in `~/.config/g512-control/state.toml`.

## Uninstall

```sh
flatpak uninstall --user io.github.g512control.Control
packaging/install-service.sh --uninstall
sudo rm /etc/udev/rules.d/70-g512-control.rules && sudo udevadm control --reload
```

Nothing is stored on the keyboard. It returns to its own default lighting the next time it is plugged in.

## Things to know

- **Don't run it alongside OpenRGB, g810-led or another lighting tool.** They write to the same keyboard and will overwrite each other.
- **Only these HID++ features can be called:** Root, FeatureSet, DeviceFwVersion, DeviceName, ColorLedEffects and PerKeyLighting. The keyboard also has firmware-update, reset and hidden engineering features, which the code cannot address.
- **Lighting is written to the keyboard's RAM only,** never to its stored settings.
- **Speed ranges and command layouts** were confirmed on the keyboard by hand. The tests and their output are in `openspec/changes/archive/2026-10-22-add-g512-lighting-controller/evidence/`.

## Development

```sh
cargo test                                   # g512-hidpp and g512d, on the host
cargo test -p g512-hidpp -- --ignored        # hardware test: sets the keyboard green
```

The app needs GTK and libadwaita development files. The host doesn't have them, so build it inside the GNOME SDK. One way is a build sandbox:

```sh
flatpak build-init --sdk-extension=org.freedesktop.Sdk.Extension.rust-stable --base-version=25.08 \
    /tmp/g512-sdk io.github.g512control.Control org.gnome.Sdk//50 org.gnome.Platform//50
flatpak build --share=network --filesystem="$PWD" --env=PATH=/usr/lib/sdk/rust-stable/bin:/usr/bin:/bin \
    --env=CARGO_TARGET_DIR="$PWD/target/flatpak" /tmp/g512-sdk cargo test --workspace
```
