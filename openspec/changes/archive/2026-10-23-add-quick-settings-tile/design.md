# Design

## Context

- **What the service does now.** It works out what is due in one place, `Controller::due_output`:
  - the Gaming map while a game overrides the state;
  - otherwise the saved state, as a zone effect or a per-key frame.
  - Connect, replug, resume and game start and end all go through it.
  - Off is an ordinary effect (`EffectKind::Off`), and `SetState` with Off replaces the saved state.
- **This desktop.** GNOME Shell 50.6 on Wayland (Fedora Silverblue 44).
  - `disable-user-extensions` is `true`, and no extensions are enabled.
  - The Extensions app (`org.gnome.Extensions`) is installed.
  - On Wayland the Shell loads a newly installed extension only at the next login, and it loads changed extension code the same way.
- **Shell API, read from the 50.6 sources** (`libshell-18.so` resources):
  - `ui/quickSettings.js` exports `QuickToggle`, `QuickMenuToggle` (with `menu.setHeader` and `addHeaderSuffix`), `QuickSlider` and `SystemIndicator`.
  - `ui/panel.js` has `Main.panel.statusArea.quickSettings.addExternalIndicator(indicator, colSpan)`.
- **Test Shell.** The `dev` toolbox (Fedora 44) now has `gnome-shell`, `mutter` and `mutter-devkit` 50.6, the same version as the host, so `gnome-shell --devkit` can run a nested Shell in a window. It was installed through `podman exec --user root`, because `sudo` in the toolbox asks for a password.

## Goals / Non-Goals

**Goals:**
- One on/off setting that the service owns. The tile, the app and `busctl` all see the same value.
- The tile only calls the service. All validation, saving and device access stay in `g512d`.
- Most of the tile can be tested without logging out, in the nested Shell.

**Non-Goals:**
- Supporting Shell versions other than 50, or publishing on extensions.gnome.org.
- Editing colours or key maps in the tile.

## Decisions

### D1. `LightsOn` masks what is due; it doesn't replace the state

`Controller` gets `lights_on: bool`, saved in `state.toml` and defaulting to true. `due_output()` returns zone Off when it is false, and otherwise what it returns today. Nothing else changes:
- The saved state, both key maps, the game switch and game tracking all keep running underneath.
- Turning the lights back on during a game shows the Gaming map, because game tracking never stopped.

Rules:
- `SetLightsOn(false)` sends Off and saves. `SetLightsOn(true)` applies whatever is due and saves.
- `SetState` sets `lights_on = true` before applying. Picking an effect in any front end means the user wants to see it. v1 clients that call `SetState` keep working and simply turn the lights on.
- `SetKeys` and `SetGameSwitch` don't change `lights_on`. An edit to a hidden map, or to any map while the lights are off, is saved but not shown.
- With the lights off, a game starting doesn't light the keyboard. Off wins over the Gaming map.
- Connect, replug and resume apply `due_output()`, so a dark keyboard is sent Off again. Without that, the firmware would show its own default lighting after a replug.

**Alternative considered:** have the tile remember the last state and call `SetState(Off)`. Rejected because the memory would live in one front end. Turning the lights off from the tile and back on from the app would lose the effect, and the app would show Off as the chosen effect.

### D2. D-Bus additions

- Property `LightsOn` (`b`), emitted on change like the other properties.
- Method `SetLightsOn(b on)`. It never fails because of the keyboard: while disconnected, the value is saved and applied on connect, like a pending state.
- Everything else is unchanged. `State` keeps reporting the saved state while the lights are off, so front ends can show what will come back.

### D3. Extension layout

```
gnome-extension/
  metadata.json   uuid g512-control@g512control.github.io, shell-version ["50"]
  extension.js    Extension subclass: builds the indicator, wires D-Bus, cleans up in disable()
  logic.js        pure functions: subtitle, menu model, speed clamping, throttle timing
  tests/logic-test.js   run with gjs -m, no Shell needed
```

- `logic.js` imports nothing from the Shell, so `gjs -m tests/logic-test.js` tests it directly.
- `extension.js` keeps the Shell glue thin: build widgets, connect signals, call the proxy.

### D4. The tile

- **Indicator.** A `SystemIndicator` holding one `QuickMenuToggle`: title "Keyboard", icon `input-keyboard-symbolic`, `checked` = `LightsOn`.
- **Click.** Calls `SetLightsOn(!checked)`.
- **Subtitle**, decided by `logic.js`:
  - the effect name while lit (Custom shows "Custom");
  - "Gaming Layout" while `GameActive`;
  - none while off, matching other GNOME toggles;
  - "Disconnected" or "No Permission" when `DeviceState` says so, with the toggle insensitive.
- **Menu.**
  - Header: the icon, "Keyboard Lighting", and the device name as subtitle.
  - The effects from `Effects`, the current one marked with a check ornament. Choosing one calls `SetState` with that effect, the saved colour, the saved speed clamped to the effect's range, and the saved brightness.
  - A brightness slider, 0–100, with the same 100 ms throttle as the app. The final value is always sent.
  - A "Gaming Layout During Games" switch item, calling `SetGameSwitch`.
  - "Lighting Settings", which opens `io.github.g512control.Control` and closes the menu.
- **Visibility.** Hidden while `io.github.g512control.Lighting1` has no owner on the session bus.
- **D-Bus.** A `Gio.DBusProxy` created with `DO_NOT_AUTO_START`, so the Shell never starts the service. It follows `g-properties-changed` and owner changes. Errors from calls are logged with `console.warn`, and the menu re-reads the properties.
- **Cleanup.** `disable()` destroys the indicator, disconnects every signal, drops the proxy and removes timeouts, as the review guidelines for extensions require.

### D5. Testing without logging out

`tests/nested.sh` runs, in the `dev` toolbox:
- `dbus-run-session`, so the nested Shell gets its own session bus and never touches the real Shell's bus names;
- `GSETTINGS_BACKEND=memory`, so enabling the extension in the nested Shell doesn't write the user's real dconf;
- a temporary `XDG_DATA_HOME` with the extension installed into it;
- `tests/fake_service.py`, a small Gio D-Bus service with the `Lighting1` interface. It logs every call and can change its properties on command.
- `gnome-shell --devkit --wayland`, with the extension enabled through `gnome-extensions enable` on that private bus.

The real `g512d` is not used in the nested Shell. It would open the same keyboard as the running service.

The nested test checks the tile's states and its calls through the fake service's log, plus screenshots of the devkit window. The real-session check comes once, after the user logs out and in with extensions enabled.

### D6. The app's Lights switch

An `AdwSwitchRow` "Lights" at the top of the lighting group, bound to `LightsOn` and calling `SetLightsOn`. It is read as an optional property, like the per-key ones, so the app still runs against an older service; the row is hidden then. The other controls stay usable while the lights are off, and changing one turns the lights on (D1).

## Risks / Trade-offs

- **Extensions break across Shell releases.** `shell-version` is `["50"]`, so a Shell upgrade disables the tile until the extension is updated. That beats having it misbehave silently.
- **Two logouts in the worst case.** One to load the extension. A second only if the real-session check finds a bug, because changed code also needs a login. The nested Shell is there to make that unlikely.
- **The devkit may not render inside the toolbox.** It needs the host's Wayland socket and GPU. If it doesn't start, fall back to the `gjs` tests and the fake service, and do the visual checks in the real session.
- **`SetState` turning the lights on** changes behaviour for a client that sets state while the lights are off. That is intended (D1), and no such client exists yet.

## Migration Plan

- `state.toml` files without `lights_on` load as lights on. Nothing else changes for existing installs.
- To install the tile: turn on extensions, run `packaging/install-extension.sh`, log out and in, then enable it with `gnome-extensions enable g512-control@g512control.github.io` or the Extensions app.
- Rollback: `gnome-extensions uninstall g512-control@g512control.github.io`. The service change is backward compatible.

## Open Questions

None. The user chose the full tile (option A) and the devkit setup.
