# g512-control

Lighting control for the Logitech G512 RGB keyboard (USB `046d:c33c`) on a GNOME desktop.

It has four parts:

- **`g512d`**: a user service that owns the keyboard. It applies lighting over HID++ 2.0, saves the last setting, and re-applies it when the keyboard is plugged back in or the computer wakes from suspend. It offers a session D-Bus interface, `io.github.g512control.Lighting1`.
- **G512 Control** (`io.github.g512control.Control`): a GTK4/libadwaita app, packaged as a Flatpak. It talks to `g512d` over D-Bus and has no access to the keyboard itself.
- **Quick Settings tile** (`gnome-extension/`): an optional GNOME Shell 50 extension with the everyday controls. It also only talks to `g512d`.
- **`g512-hidpp`**: the protocol library the service is built on.

Effects: Off, Static, Breathe, Colour cycle, Wave, Ripple and Custom. Each has the colour and speed settings it supports, plus brightness. The keyboard has no brightness command of its own: brightness dims the colour, or for Cycle and Wave it sets their intensity.

Custom shows a key map: a colour for each key, set in the app's **Key Maps** page. A second map, Gaming, can be shown automatically while a game runs.

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

### 4. Quick Settings tile (optional)

A **Keyboard** tile in GNOME's Quick Settings menu:
- Clicking it turns the lights off and back on.
- Its menu has the effect list, a brightness slider, the gaming layout switch, and **Lighting Settings**, which opens the app.

It is a GNOME Shell extension for GNOME 50.

1. Turn on extensions if they are off: open the Extensions app and switch on **Use Extensions**, or run `gsettings set org.gnome.shell disable-user-extensions false`.
2. Install the tile:

   ```sh
   packaging/install-extension.sh
   ```

3. Log out and back in. GNOME Shell on Wayland only loads new extensions at login.
4. Enable it, in the Extensions app or with:

   ```sh
   gnome-extensions enable g512-control@g512control.github.io
   ```

The tile hides itself while `g512d` isn't running, and never starts it. After a GNOME upgrade beyond version 50, the Shell turns the tile off until the extension is updated.

## Lights on and off

**Lights** in the app, or clicking the tile, turns the keyboard dark without forgetting anything. Turning the lights back on shows your effect again, or the Gaming layout if a game is running.

- While the lights are off, starting a game doesn't light the keyboard.
- The keyboard stays dark after replugging, waking from suspend or logging in.
- Choosing an effect or changing brightness, in the app, the tile or with `SetState`, turns the lights back on.

## Key maps

**Key Maps** in the app opens a drawing of the keyboard. Choose **Custom** or **Gaming** at the top to pick which map you are editing.

- Click keys to select them, or drag across them. The buttons above the keyboard select groups: WASD, the arrows, the F-keys, the number row, the numpad or the modifiers.
- Pick a colour, then **Paint Selected**. **Turn Off** leaves the selected keys dark.
- Without a pointer: Tab moves to the drawn keyboard, the arrow keys move between keys, and Space selects one.

Choose **Custom** in the Effect list to show the Custom map. Brightness dims the whole map.

The Gaming map starts with W, A, S, D, Space, Left Shift, Left Ctrl, 1 to 5 and the arrows in orange-red, and every other key dim grey.

### Gaming layout while a game runs

Turn on **Switch to Gaming Layout While a Game Runs** in the app. While a game is registered with [GameMode](https://github.com/FeralInteractive/gamemode), the keyboard shows the Gaming map. When the last game closes, it goes back to your normal lighting. If you change the lighting during a game, your change stays until the next game starts.

A game is only registered if it uses GameMode. For Steam games, add this to the game's **Launch Options**:

```
gamemoderun %command%
```

The service never starts GameMode itself. It only listens for games starting and stopping, and it doesn't read your keystrokes.

The keyboard's own Game Mode key (Fn and the joystick key) works inside the keyboard and sends nothing to the computer, so it doesn't switch layouts.

### Evening out a bright key

If one key's LED is brighter than the rest, list it in `~/.config/g512-control/corrections.toml` with a percentage from 1 to 100. Name the key by its USB HID usage ID, written as two hex digits:

```toml
# Left Ctrl is about 15 % brighter than the other keys on this keyboard.
e0 = 85
```

Restart the service to load the file:

```sh
systemctl --user restart g512d
```

The service only reads this file and never rewrites it. The correction applies to Custom, Gaming and Static. When a correction exists, Static is sent as per-key colours so the correction can apply. Breathe, Cycle, Wave and Ripple run inside the keyboard, so they can't be corrected.

Common IDs: `e0` Left Ctrl, `e1` Left Shift, `e2` Left Alt, `e3` Left Super, `e4` Right Ctrl, `e5` Right Shift, `e6` Right Alt, `04` A to `1d` Z, `1e` 1 to `27` 0, `29` Esc, `2c` Space.

## Using it without the app

```sh
# Static orange at 60 % brightness (speed is ignored by Static)
busctl --user call io.github.g512control.Lighting1 /io/github/g512control/Lighting1 \
    io.github.g512control.Lighting1 SetState 's(yyy)yy' Static 255 128 0 50 60

# What the keyboard offers and what is set
busctl --user introspect io.github.g512control.Lighting1 /io/github/g512control/Lighting1
```

`SetState(effect, (r, g, b), speed, brightness)` takes an effect name from the `Effects` property, a speed from 1 (slowest) to 100 (fastest) and a brightness from 0 to 100. It returns `true` if the keyboard is not connected; in that case the state is saved and applied when the keyboard connects.

```sh
# Custom map: W A S D green, every other key off; then show it
busctl --user call io.github.g512control.Lighting1 /io/github/g512control/Lighting1 \
    io.github.g512control.Lighting1 SetKeys 'sa(yyyy)' custom 4 26 0 255 0 4 0 255 0 22 0 255 0 7 0 255 0
busctl --user call io.github.g512control.Lighting1 /io/github/g512control/Lighting1 \
    io.github.g512control.Lighting1 SetState 's(yyy)yy' Custom 0 0 0 50 100

# Show the Gaming map while GameMode reports a game
busctl --user call io.github.g512control.Lighting1 /io/github/g512control/Lighting1 \
    io.github.g512control.Lighting1 SetGameSwitch b true

# Lights off, keeping everything else; `b true` turns them back on
busctl --user call io.github.g512control.Lighting1 /io/github/g512control/Lighting1 \
    io.github.g512control.Lighting1 SetLightsOn b false
```

`SetKeys(map, [(key, r, g, b)...])` replaces the whole `custom` or `gaming` map. Keys not listed are off. The maps are in the `CustomKeys` and `GamingKeys` properties, the switch in `GameSwitch`. `GameActive` is true while the Gaming map is showing because a game is running. `LightsOn` is false while the keyboard is dark.

The saved state, both maps, the game switch and the lights setting are in `~/.config/g512-control/state.toml`.

## Uninstall

```sh
packaging/install-extension.sh --uninstall
flatpak uninstall --user io.github.g512control.Control
packaging/install-service.sh --uninstall
sudo rm /etc/udev/rules.d/70-g512-control.rules && sudo udevadm control --reload
```

Nothing is stored on the keyboard. It returns to its own default lighting the next time it is plugged in.

## Things to know

- **Don't run it alongside OpenRGB, g810-led or another lighting tool.** They write to the same keyboard and will overwrite each other.
- **Only these HID++ features can be called:** Root, FeatureSet, DeviceFwVersion, DeviceName, ColorLedEffects and PerKeyLighting. Of PerKeyLighting, only "set keys" and "commit" are used. The keyboard also has firmware-update, reset and hidden engineering features, which the code cannot address.
- **Lighting is written to the keyboard's RAM only,** never to its stored settings.
- **Speed ranges and command layouts** were confirmed on the keyboard by hand. The tests and their output are in `openspec/changes/archive/2026-10-22-add-g512-lighting-controller/evidence/`, for per-key lighting in `openspec/changes/archive/2026-10-23-add-per-key-lighting/evidence/`, and for the tile in `openspec/changes/add-quick-settings-tile/evidence/`.
- **Only the ANSI (US) layout** is drawn and addressed.

## Development

```sh
cargo test                                   # g512-hidpp and g512d, on the host
cargo test -p g512-hidpp -- --ignored        # hardware tests: Static green, then a WASD frame
```

The tile's logic tests need only `gjs`. The tile itself is tested in a headless nested GNOME Shell against a fake service, on a private session bus with throwaway settings. Neither touches your session or the keyboard. Both need GNOME Shell 50 and `mutter-devkit`, which a toolbox can provide:

```sh
toolbox run -c dev gjs -m gnome-extension/tests/logic-test.js
toolbox run -c dev sh gnome-extension/tests/nested.sh \
    openspec/changes/add-quick-settings-tile/evidence/tile_checks.sh /tmp/tile-out   # add --devkit to watch it
```

The app needs GTK and libadwaita development files. The host doesn't have them, so build it inside the GNOME SDK. One way is a build sandbox:

```sh
flatpak build-init --sdk-extension=org.freedesktop.Sdk.Extension.rust-stable --base-version=25.08 \
    /tmp/g512-sdk io.github.g512control.Control org.gnome.Sdk//50 org.gnome.Platform//50
flatpak build --share=network --filesystem="$PWD" --env=PATH=/usr/lib/sdk/rust-stable/bin:/usr/bin:/bin \
    --env=CARGO_TARGET_DIR="$PWD/target/flatpak" /tmp/g512-sdk cargo test --workspace
```
