# Service checks

## 1.1–1.2 Protocol

35 `g512-hidpp` tests pass.

**Discovery:**
- `selects_mouse_interface_1` uses the mouse's real descriptors, read from sysfs. `hidraw10` (page `0xff00`, reports `0x10` and `0x11`) is the G502 HERO, and `hidraw9`, the boot mouse, is ignored.
- `descriptor_must_fit_the_product`: a keyboard ID with the mouse's descriptor is rejected, and so is the reverse.
- `other_products_are_ignored` covers the Logitech webcam `046d:0894` and the MSI device `1462:7c92`.
- `finds_device_in_sysfs_tree` finds the keyboard on `hidraw1` and the mouse on `hidraw3`.

**Effects:**
- `mouse_zones_from_spike_6`: two zones (locations 1 and 2), each with Off, Static, Breathe and Cycle.
- `set_effect_all_sends_each_zone`: zone 0, then zone 1, with effect index 1, `00 ff 00`, and persistence 0.
- `set_effect_all_fails_if_any_zone_refuses`: zone 0 is applied, zone 1 refuses, and the call fails.
- `set_effect_all_needs_a_zone_offering_it`: Ripple is refused before anything is sent.
- `set_effect_all_on_the_keyboard_matches_set_effect`: identical bytes on the keyboard.

## 2.1 Controller

- **`mouse.rs`** (9 tests): every row of the design's D3 table.
  - Off, Static, Breathe and Cycle are unchanged.
  - Wave becomes Cycle at the same speed.
  - Ripple becomes Static in its colour.
  - Custom shows W's colour, or Off without W.
  - Gaming shows the Gaming map's W `#ff2000` at the saved brightness.
  - Lights off wins.
  - An own setting ignores the keyboard, and defaults to Static in the keyboard's colour.
  - With nothing saved, the mouse is left alone.
- **`controller.rs`** (8 tests, with fake G512 and G502 HERO):
  - `mouse_matches_the_keyboard`: Static red is sent to both zones; Breathe gives index 2; Wave gives Cycle (3); Ripple gives Static.
  - `mouse_follows_custom_and_gaming_maps`: covers Custom with and without W, a game start (`ff 20 00`), and a game end.
  - `mouse_lights_off_and_on`.
  - `mouse_own_setting`: lights off, then an own blue setting turns matching off and the lights on, so the keyboard shows Static red again. A later keyboard change leaves the mouse alone, and matching again follows the keyboard. Ripple is refused.
  - `mouse_errors_never_reach_the_keyboard`: a refusal keeps the keyboard call's result and the mouse connected. A lost link disconnects only the mouse.
  - `mouse_replug_and_permission`: denied at first, then connected and applied by `reapply_mouse`.
  - `no_mouse_changes_nothing_for_the_keyboard`.
  - `mouse_offers_its_four_effects`.
- All 71 service tests pass, and clippy `-D warnings` is clean.

## 2.2 Storage

- `mouse_settings_round_trip`: an own Breathe blue round-trips, as does "matching off, no own setting yet".
- `file_without_mouse_settings_matches_the_keyboard`: a file from change 3 loads with matching on.
- `round_trip` now also checks that default mouse settings write no `[mouse]` table.

## 2.3 Events

- `retry_continues_while_either_device_is_denied`: the login retry goes on while the keyboard or the mouse is denied, and stops when there's simply no mouse.
- `remove_disconnects_only_the_matching_device`: `hidraw8` is the keyboard and `hidraw10` the mouse. `hidraw9`, the mouse's boot node, is neither.
- Hotplug "add" reconnects whichever device is missing. The mouse retry is silent when no mouse is plugged in, so a keyboard replug doesn't log mouse failures. Resume re-applies both.

## 2.4 D-Bus

`mouse_dbus_check.sh` runs the new g512d in the `dev` toolbox, under `bwrap --dev /dev` (no hidraw nodes), on a private bus, with a temporary config (`mouse_dbus_check.out`):

- **At start:**
  - `MouseState` reads `disconnected`, `MouseName` is empty, and `MouseEffects` is empty;
  - `MouseMatch` is true;
  - `MouseLighting` is Static white, the default before any state.
- **After `SetState` Ripple `#3d3846`:** `MouseLighting` reads Static `61 56 70`, speed 52, brightness 100. That's the default own setting, in the keyboard's colour.
- **`SetMouseLighting` Static blue at 80:**
  - returns pending `true`, since there's no mouse;
  - `MouseMatch` becomes false and `LightsOn` stays true;
  - a `PropertiesChanged` signal carries `MouseMatch` = false;
  - `state.toml` gains a `[mouse]` table with `match_keyboard = false` and Static `#0000ff`, speed 50, brightness 80.
- **`SetMouseMatch true`:** `MouseMatch` is true, and `match_keyboard = true` is saved.
- **`SetMouseLighting` Ripple:** refused with "the mouse does not offer Ripple". The first run said "the keyboard", because the error type was written for the keyboard; the mouse call now has its own message.

## 2.5 udev rule

- **Install:** the user reinstalled the rule with sudo. The `sudo` log shows `install`, `udevadm control --reload` and `udevadm trigger` at 22:31:57–58.
- **Result:** `/etc/udev/rules.d/70-g512-control.rules` has the `c08b` line. `getfacl` shows `user:wrichards:rw-` on both mouse nodes (`hidraw9`, `hidraw10`).
- **A bug found here:**
  - `udevadm trigger` sends "change" events, not "add", and the service ignored them. The mouse therefore stayed `permission-denied` after its 15 s login retry had already given up at 22:30:57.
  - Fixed in `953c219`: "change" on a hidraw node now reconnects any missing device, like "add".
  - After reinstalling, the service started with "keyboard connected, mouse connected" (22:32:46).

## 2.6 On the hardware

`mouse_hardware_check.sh` (`mouse_hardware_check.out`), with the user watching. Every call succeeded, and the service logged no mouse errors.

| Step | Keyboard | Mouse (expected) |
|---|---|---|
| 1 | Static red | red |
| 2 | Breathe green | breathes green |
| 3 | Cycle, speed 10 | slow cycle at the keyboard's pace |
| 4 | Cycle, speed 100 | fast cycle at the keyboard's pace |
| 5 | Wave | cycles |
| 6 | Ripple cyan | static cyan |
| 7 | `gamemode-simulate-game` | orange-red during the game (`GameActive` true), cyan after |
| 8 | lights off, then on | dark, then cyan |
| 9 | stays cyan Ripple | own Breathe blue (`MouseMatch` false) |
| 10 | user's Ripple restored | matching again |

**The user confirmed "All as described".** This includes the Cycle pace matching the keyboard, which settles design D4: the keyboard's speed ranges work for the mouse.

They also noted the mouse "now is green". Matching the restored Ripple shows Static `#3d3846`, a very dim grey-purple. That reads as green on the mouse, probably because its green LED is the most efficient at low levels. That's an explanation, not a measurement. No change was made: the user can choose an own colour, or a brighter Ripple colour.

**DPI buttons:** the user reports they still work. They never reach the computer in onboard mode, so this can't be seen from the service.

**Replug:** the first reported replug left no trace (no log lines, device nodes unchanged). The second one, at 22:39:44, took out the keyboard and the mouse together, probably through a shared hub:

```
22:39:44.494  disconnected
22:39:44.592  mouse disconnected
22:39:48.266  re-applied lighting (attempt 1)
22:39:48.358  mouse connected: G502 HERO Gaming Mouse on /dev/hidraw3
22:39:48.366  re-applied mouse lighting (attempt 1)
```

Both devices came back within 4 s on new nodes, each re-applied on the first attempt.
