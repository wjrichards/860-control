# g512-control

Lighting control for the Logitech G512 RGB keyboard (USB `046d:c33c`) and G502 HERO mouse (`046d:c08b`) on a GNOME desktop.

It has four parts:

- **`g512d`**: a user service that owns the keyboard and the mouse. It applies lighting over HID++ 2.0, saves the last setting, and re-applies it when a device is plugged back in or the computer wakes from suspend. It offers a session D-Bus interface, `io.github.g512control.Lighting1`.
- **G512 Control** (`io.github.g512control.Control`): a GTK4/libadwaita app, packaged as a Flatpak. It talks to `g512d` over D-Bus and has no access to the devices itself.
- **Quick Settings tile** (`gnome-extension/`): an optional GNOME Shell 50 extension with the everyday controls. It also only talks to `g512d`.
- **`g512-hidpp`**: the protocol library the service is built on.

Effects: Off, Static, Breathe, Colour cycle, Wave, Ripple and Custom. Each has the colour and speed settings it supports, plus brightness. The keyboard has no brightness command of its own: brightness dims the colour, or for Cycle and Wave it sets their intensity.

Custom shows a key map: a colour for each key, set in the app's **Key Maps** page. A second map, Gaming, can be shown automatically while a game runs.

The mouse's logo and DPI strip can match the keyboard, or have their own Off, Static, Breathe or Cycle.

Tested on firmware U1 19.01 build 0014, Fedora Silverblue 44 and GNOME 50.

## Install

You need Rust (`cargo`) on the host. The app also needs `org.flatpak.Builder`, `org.gnome.Sdk//50` and `org.freedesktop.Sdk.Extension.rust-stable//25.08` from Flathub.

### 1. udev rule (once, needs sudo)

This lets the logged-in user open the keyboard and the mouse, and no other device. Details are in [packaging/INSTALL-udev.md](packaging/INSTALL-udev.md). If you installed it before mouse support, run these commands again; the rule gained a line for the mouse.

```sh
sudo install -m 0644 packaging/70-g512-control.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger --subsystem-match=hidraw
```

`udevadm trigger` applies it to devices that are already plugged in.

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

## The app

The app has two views, **Keyboard** and **Mouse**, switched at the top of the window, or at the bottom when the window is narrow. The Mouse view appears when the service supports a G502 HERO. Each view shows its own device's state: if the keyboard is unplugged, the Keyboard view says so and the Mouse view keeps working, and the other way round.

Each view opens with a drawing of its device in its current lighting:
- **The keyboard** shows lit keys as tiles in their colour and unlit keys as outlines: the effect's colour, or the key map being shown. Click it, or press Enter on it, to open **Key Maps**; **Edit Key Maps…** below does the same.
- **The mouse** is drawn from above and from the side, in its lighting colour, with the G logo and DPI LEDs lit. A number marks each programmable button, and the list beside the drawing (below it in a narrow window) shows what each one does in the active profile. Click a button on the drawing, or its entry in the list, to change it. Left click is shown as fixed.

Colours too dark to see against the drawing's background are lightened, keeping their hue. With the lights off, the drawings are dim outlines.

While GNOME's animations are on, the drawings play the current effect: Ripple's ring, Wave's band, Breathe's fade and Cycle's colour, at the effect's speed. Pointing at a mouse button fills it and sends a ring out, and applying a change pulses it once. With **Reduce Animation** on (Settings → Accessibility), the drawings stay still and nothing pulses.

The mouse drawings are the project's own ink drawings of the G502 HERO, in `g512-control/data/g502-top-ink.svg` and `g502-side-ink.svg`. The app reads their paths at start-up and colours them itself.

## Lights on and off

**Lights** in the app, or clicking the tile, turns the keyboard dark without forgetting anything. Turning the lights back on shows your effect again, or the Gaming layout if a game is running.

- While the lights are off, starting a game doesn't light the keyboard.
- The keyboard stays dark after replugging, waking from suspend or logging in.
- Choosing an effect or changing brightness, in the app, the tile or with `SetState`, turns the lights back on.

## Mouse

The app's **Mouse** view appears when the service supports a G502 HERO. With **Match Keyboard** on, which is the default, the logo and DPI strip follow the keyboard's lighting and brightness. The mouse has only Off, Static, Breathe and Cycle, so it shows the nearest one:
- **Wave** becomes Cycle at the same speed.
- **Ripple** becomes Static in Ripple's colour.
- **The Custom map, or the Gaming map during a game,** becomes Static in the colour that map gives W, or Off if W is off.

With **Match Keyboard** off, choose the mouse's own effect, colour, speed and brightness. **Lights** off darkens the mouse too.

The mouse stays in its own onboard-profile mode, so its DPI buttons keep working. Lighting set here isn't written to the mouse. Unplugged, it shows its own lighting until the service re-applies it.

### DPI, report rate and profiles

The mouse stores five profiles, each with a report rate and up to five DPI steps. Its DPI buttons cycle through the active profile's steps. The Mouse view's **Profile** group shows the active profile:
- **Profile:** choosing one makes it active. The mouse switches to its DPI and report rate straight away. G9, the button behind the wheel, also switches profiles on the mouse; the app follows within about 2 seconds.
- **Report Rate:** 125, 250, 500 or 1000 Hz.
- **DPI steps:** 1 to 5 steps from 100 to 25,600 DPI, in steps of 50. The radio button marks the step the mouse starts on; the remove button deletes a step; **Add Step** adds one.

Changes are **written to the mouse** about 2 seconds after you stop editing, as one write. Because they live on the mouse, they also work on other computers.

Before its first write, the service saves your profiles as they were to `~/.config/g512-control/mouse-profiles-backup.txt`, and never overwrites that file. To put them back:

```sh
g512d --restore-mouse-profiles ~/.config/g512-control/mouse-profiles-backup.txt
```

Only the report rate, DPI steps and default step are changed here. Button assignments, macros and the profiles' stored lighting are kept as they are.

### Buttons

The **Buttons** group lists what each button does in the active profile: Right Click, Wheel Click, Back and Forward (the thumb buttons), Sniper, DPI Down, DPI Up, G9, and Wheel Tilt Right and Left. The same list sits beside the mouse drawing, numbered to match it. **Change**, a button in that list, or the button itself on the drawing opens a dialog with:
- **Mouse Button:** left, right or middle click, back or forward;
- **Keyboard Shortcut:** choose **Set Shortcut**, then press the keys, for example Ctrl+C. Esc cancels. Letters, digits, F1 to F12, Enter, Tab, Space, Backspace, punctuation, the arrows and Insert, Delete, Home, End, Page Up and Page Down work; the dialog says when a key can't be used;
- **Media Key:** play/pause, next and previous track, volume up and down, mute;
- **Mouse Function:** DPI up, down, cycle and shift (sniper), next profile, scroll left and right, G-shift;
- **Disabled:** the button does nothing.

**Left click** can't be changed, so the mouse can always click. Each change is written to the mouse straight away, behind the same backup as the profiles, and works on other computers too. A button set by another program, such as a macro, shows as "Set elsewhere" and is kept until you change it.

If G9 no longer switches profiles, the app's **Profile** row still does.

## Key maps

Clicking the keyboard drawing, or **Edit Key Maps…**, opens **Key Maps**, a drawing of the keyboard to edit. Choose **Custom** or **Gaming** at the top to pick which map you are editing.

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

# The mouse breathing blue on its own, then matching the keyboard again
busctl --user call io.github.g512control.Lighting1 /io/github/g512control/Lighting1 \
    io.github.g512control.Lighting1 SetMouseLighting 's(yyy)yy' Breathe 0 0 255 40 100
busctl --user call io.github.g512control.Lighting1 /io/github/g512control/Lighting1 \
    io.github.g512control.Lighting1 SetMouseMatch b true

# Mouse profile 2: 1000 Hz (1 ms), steps 800, 1600, 3200, starting on 1600; then make it active
busctl --user call io.github.g512control.Lighting1 /io/github/g512control/Lighting1 \
    io.github.g512control.Lighting1 SetMouseProfile 'yyyaq' 2 1 1 3 800 1600 3200
busctl --user call io.github.g512control.Lighting1 /io/github/g512control/Lighting1 \
    io.github.g512control.Lighting1 SetMouseActiveProfile y 2
```

`SetMouseProfile(profile, report period in ms, default step index, DPI steps)` writes a profile. The period is 1, 2, 4 or 8 ms, and the index counts from 0. `MouseProfiles`, `MouseActiveProfile` and `MouseDpiRange` describe what's on the mouse. The service reads the active profile every 2 seconds, so a switch made with G9 shows up.

```sh
# Profile 4's G9 (button 8) as Ctrl+C: kind "key", HID usage 0x06 (c), modifier bits 1 (Ctrl)
busctl --user call io.github.g512control.Lighting1 /io/github/g512control/Lighting1 \
    io.github.g512control.Lighting1 SetMouseButton 'yysqy' 4 8 key 6 1
# ...and back to cycling profiles (mouse function 0x0a)
busctl --user call io.github.g512control.Lighting1 /io/github/g512control/Lighting1 \
    io.github.g512control.Lighting1 SetMouseButton 'yysqy' 4 8 special 10 0
```

`SetMouseButton(profile, button, kind, value, modifiers)` changes one button. Buttons count from 0 in the order above, with left click 0 and refused. The kinds are:
- `mouse`: 1 left, 2 right, 4 middle, 8 back, 16 forward;
- `key`: a HID keyboard usage, with modifier bits Ctrl 1, Shift 2, Alt 4, Super 8;
- `media`: a HID consumer usage, such as `0xcd` play/pause or `0xe9` volume up;
- `special`: 1 scroll left, 2 scroll right, 3 DPI up, 4 DPI down, 5 DPI cycle, 7 DPI shift, 10 next profile, 11 G-shift;
- `disabled`, with value 0.

Modifiers go only with `key`. `MouseButtons` lists every profile's buttons the same way; `other` marks an entry the service doesn't write, such as a macro.

`SetKeys(map, [(key, r, g, b)...])` replaces the whole `custom` or `gaming` map. Keys not listed are off. The maps are in the `CustomKeys` and `GamingKeys` properties, the switch in `GameSwitch`. `GameActive` is true while the Gaming map is showing because a game is running. `LightsOn` is false while the keyboard is dark.

`MouseState`, `MouseName` and `MouseEffects` describe the mouse. `MouseMatch` is true while it follows the keyboard, and `MouseLighting` is its own setting. `SetMouseLighting` takes the same arguments as `SetState`. It turns matching off and the lights on.

The saved state, both maps, the game switch, the lights setting and the mouse settings are in `~/.config/g512-control/state.toml`.

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
- **Only these HID++ features can be called:** Root, FeatureSet, DeviceFwVersion, DeviceName, ColorLedEffects, PerKeyLighting, AdjustableDpi and OnboardProfiles. The code enforces a function list for three of them:
  - PerKeyLighting: only "set keys" and "commit";
  - AdjustableDpi: reads only;
  - OnboardProfiles: never "set mode", so the mouse always stays in onboard mode.

  Both devices also have firmware-update, reset and hidden engineering features, which the code cannot address.
- **Profile writes go to the mouse's flash memory.** Only the five user profile sectors can be written, never the profile directory or the factory profiles. Every write is read back and checked. Flash wears with writes, so edits are grouped into one write per pause; scripts calling `SetMouseProfile` in a loop are not limited.
- **Lighting is written to the devices' RAM only,** never to their stored settings.
- **Speed ranges and command layouts** were confirmed on the keyboard by hand. The tests and their output are in `openspec/changes/archive/2026-10-22-add-g512-lighting-controller/evidence/`, for per-key lighting in `openspec/changes/archive/2026-10-23-add-per-key-lighting/evidence/`, for the tile in `openspec/changes/archive/2026-10-23-add-quick-settings-tile/evidence/`, and for the mouse in `openspec/changes/archive/2026-10-24-add-mouse-lighting/evidence/` (which also holds a backup of the mouse's profile memory), and for mouse profiles in `openspec/changes/archive/2026-10-24-add-mouse-dpi/evidence/`.
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
    openspec/changes/archive/2026-10-23-add-quick-settings-tile/evidence/tile_checks.sh /tmp/tile-out   # add --devkit to watch it
```

The app needs GTK and libadwaita development files. The host doesn't have them, so build it inside the GNOME SDK. One way is a build sandbox:

```sh
flatpak build-init --sdk-extension=org.freedesktop.Sdk.Extension.rust-stable --base-version=25.08 \
    /tmp/g512-sdk io.github.g512control.Control org.gnome.Sdk//50 org.gnome.Platform//50
flatpak build --share=network --filesystem="$PWD" --env=PATH=/usr/lib/sdk/rust-stable/bin:/usr/bin:/bin \
    --env=CARGO_TARGET_DIR="$PWD/target/flatpak" /tmp/g512-sdk cargo test --workspace
```
