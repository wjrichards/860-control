# Proposal

## Why

Changing the keyboard lighting means opening G512 Control. The user wants the everyday controls in GNOME's Quick Settings menu, next to Wi-Fi and Night Light: turn the lights off and back on, pick an effect, set the brightness, and turn the gaming layout on or off. This was planned as change 3 when v1 was scoped.

On GNOME Shell 50.6 (Wayland), a Shell extension is the only way to add a Quick Settings tile. User extensions are currently disabled on this account (`disable-user-extensions` is `true`), so the user turns them on as part of installing the tile. The user chose the full tile (option A) and asked for the nested development Shell (`mutter-devkit`, now in the `dev` toolbox) for testing, so only the final check needs a logout.

"Off, then back on" needs somewhere to remember what was showing. Today Off is just another effect, and setting it replaces the saved state. That memory belongs in the service, so the tile and the app always agree.

## What Changes

- **Lights on and off in the service.** A new `LightsOn` setting turns the keyboard dark without touching the saved state, key maps or game switch. Turning it back on shows whatever is due: the saved state, or the Gaming map while a game runs.
  - The setting is saved, so the keyboard stays dark after replug, resume and login.
  - While the lights are off, a starting game doesn't light the keyboard.
  - A `SetState` from any front end turns the lights back on, because a user who picks an effect wants to see it.
  - New D-Bus members: the `LightsOn` property and `SetLightsOn(b)`. Existing members are unchanged.
- **A Quick Settings tile (GNOME Shell extension).** A "Keyboard" tile in Quick Settings:
  - clicking it turns the lights off and on;
  - its subtitle shows the effect, "Gaming Layout" while a game runs, or "Disconnected";
  - its menu has the effect list, a brightness slider, a "Gaming Layout During Games" switch, and "Lighting Settings", which opens G512 Control;
  - it hides itself when the service isn't running.
  - The extension only talks to the service over D-Bus. It never opens the keyboard and keeps no settings of its own.
- **A Lights switch in the app**, so the app shows and changes the same on/off setting.
- **Fix: the service gives up at login.** If it starts before the login has granted access to the keyboard, it now keeps retrying for 15 s. This was found during the first real login of this change's testing: lighting wasn't restored until the service was restarted.
- **Install and docs.** `packaging/install-extension.sh` packs and installs the extension. The README explains turning on extensions, the one logout, and enabling the tile.

## Capabilities

### New Capabilities

- `quick-settings-tile`: the GNOME Shell Quick Settings tile and its menu.

### Modified Capabilities

- `lighting-service`: adds the lights on/off setting. It changes the D-Bus interface, persisted state, reapply and gaming-layout requirements so they respect it.
- `lighting-app`: adds the Lights switch.

## Impact

- **`g512d`:**
  - `controller.rs` gets the lights setting and its effect on what is due;
  - `store.rs` gets `lights_on`, still reading v1 and v2 files;
  - `dbus.rs` gets the new property and method.
- **`g512-control`:** a Lights switch row, read as an optional property so the app still works with an older service.
- **New `gnome-extension/` directory:**
  - `metadata.json`, `extension.js`, and a pure `logic.js` tested with `gjs`;
  - a test script that drives the nested Shell against a fake service.
- **Dependencies:** none for the Rust crates. The extension uses only GNOME Shell 50's own modules.
- **Not in scope:**
  - publishing on extensions.gnome.org;
  - Shell versions other than 50;
  - a key-map editor or colour picker in the tile, which stay in the app;
  - a panel icon (the tile appears only inside Quick Settings).
