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
