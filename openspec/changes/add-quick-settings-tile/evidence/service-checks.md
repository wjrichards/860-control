# Service checks

## 1.1 Controller (fake keyboard tests)

New tests in `g512d/src/controller.rs`. All 47 service tests pass.

| Test | What it shows |
|---|---|
| `lights_off_then_on_restores_the_state` | Off sends zone effect 0 (Off) and keeps Ripple saved. On sends Ripple (6) again. |
| `set_state_turns_the_lights_on` | `SetState` while dark turns the lights on and shows the new state. |
| `rejected_state_leaves_the_lights_off` | An invalid state (brightness 150) doesn't turn the lights on. |
| `game_with_the_lights_off_stays_dark` | A game starting while dark writes nothing to the keyboard and doesn't set `GameActive`. Turning on mid-game shows the Gaming map. |
| `replug_with_the_lights_off_sends_off` | The reopened keyboard gets exactly one write, Off, rather than Ripple or its firmware default. |
| `lights_off_while_disconnected_applies_on_connect` | A pending Ripple plus lights off gives Off on connect. |
| `map_edits_and_game_switch_keep_the_lights_off` | A Custom edit while dark is saved but not sent. `SetGameSwitch` leaves the lights off. |

## 1.2 Storage

`store.rs`:
- `round_trip` now saves and reloads `lights_on = false`.
- `v1_file_loads_with_defaults` loads a v1 file with the lights on.
- `file_without_lights_setting_loads_lit` loads a file from change 2, which has the game switch and a Custom map but no `lights_on`, with the lights on.

`Settings::default()` is now written by hand, so the default is lights on.

## 1.3 D-Bus

`lights_dbus_check.sh` runs the new `g512d` inside the `dev` toolbox:
- under `bwrap --dev /dev`, so no hidraw nodes are visible;
- on a private `dbus-run-session` bus;
- with a temporary `XDG_CONFIG_HOME`.

The installed service and the keyboard aren't touched. Output, from `lights_dbus_check.out`:

- `LightsOn` starts `true`.
- `SetLightsOn false` makes it `false`, and a `PropertiesChanged` signal carries `LightsOn` = `false`.
- `state.toml` gets `lights_on = false`.
- `SetState` Static blue (pending, since no keyboard is visible) turns `LightsOn` back to `true` and saves `lights_on = true`.

The "no resume events" warning in the log appears because the toolbox has no system bus. It doesn't happen in the installed service.

## 1.4 On the keyboard (installed service)

`packaging/install-service.sh` from this branch, user OK, 07:08:05. The existing `state.toml` (from change 2, with no `lights_on`) loaded as lights on, Ripple and game switch on.

`lights_hardware_check.sh` (`lights_hardware_check.out`), with the user watching the keyboard:
- `SetLightsOn false`: `LightsOn` false, keyboard dark.
- `gamemode-simulate-game` while dark: `GameActive` stayed false, the keyboard stayed dark, and the log reads "game started; the lights are off" and "game ended; the lights are off".
- `SetLightsOn true`: Ripple back.
- Dark again, then `SetState` Ripple: `LightsOn` true, Ripple back.

The user confirmed: "it went dark and came back".

**Replug while dark.** The first attempt pulled the G512's USB pass-through plug: no kernel or service event, and the device nodes were unchanged. On the second attempt, with the keyboard plug, the service logged:

```
07:10:27.293 disconnected
07:10:31.424 connected: G512 RGB MECHANICAL GAMING KEYBOARD on /dev/hidraw8, firmware U1 19.01 build 0014
07:10:31.428 re-applied lighting (attempt 1)
```

The user confirmed: "replugged the keyboard plug, it stayed dark". The keyboard showed Off, not its firmware default.

## 2.1 App Lights switch (AT-SPI)

The Flatpak was rebuilt and reinstalled from this branch, and the app relaunched. `app_lights_check.sh` (`app_lights_check.out`):
- The service started with the lights off, and the app's "Lights" switch was unchecked.
- Flipping the switch over AT-SPI sent exactly one `SetLightsOn` call, with `true`. The service reported `LightsOn` true and the row became checked.
- `busctl SetLightsOn false` unchecked the row, and `true` checked it again.
