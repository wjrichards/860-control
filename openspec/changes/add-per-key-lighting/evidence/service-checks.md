# Service checks on the keyboard, 2026-10-22

With the user's OK, the new `g512d` (commit `328805c`) was installed over v1 with `packaging/install-service.sh`, at 20:03:24 local. The user's v1 `state.toml` was backed up first.

## Install over v1, and GameMode left alone (task 3.2, part of 5.2)

- The journal shows `connected: G512 RGB MECHANICAL GAMING KEYBOARD on /dev/hidraw3`. The node number has changed since v1; discovery found it anyway.
- `State` read `"Ripple" 61 56 70 52 100`, the user's v1 state.
- `busctl --user introspect` lists the six new members:
  - `CustomKeys` (empty);
  - `GamingKeys` (104 keys, default map);
  - `SetKeys sa(yyyy)`;
  - `GameSwitch` (false);
  - `SetGameSwitch b`;
  - `GameActive` (false).

  `Effects` has seven entries, ending with `Custom`.
- At 20:04:03, 39 s after the start, `busctl --user list` still showed `com.feralinteractive.GameMode` as `(activatable)` with no PID, and `pgrep gamemoded` found nothing. The service had not started GameMode.

## Watched run (tasks 1.5, 2.6, 2.7, 3.3)

The script is `service_checks.sh` and its output is `service-checks-out.txt`. The user watched the keyboard and confirmed every step.

| Step | Call | The user saw |
|---|---|---|
| S1 | `SetKeys custom` WASD green while Ripple shows | Ripple stayed |
| S2 | `SetState Custom` | W A S D green, the rest dark |
| S3 | `SetKeys custom` WASD + arrows | The arrows joined |
| S4 | v1-style `SetState Static 255 0 0 50 100` | All red |
| S5 | `SetKeys custom` while Static shows | Stayed red |
| S6 → S7 | Static white as a zone effect, then `corrections.toml` `e0 = 85` and a restart, so Static white is sent as a frame | Left Ctrl brighter in S6 and **matched in S7**; nothing else changed |
| S8 | `cargo test -p g512-hidpp --test hardware -- --ignored sets_wasd_frame` | W A S D red on dim white; the test passed |
| S9 | Switch on, `gamemode-simulate-game` (10 s) | The orange Gaming layout for the game's duration, then Ripple back |

S9 timings:

| Time | Event |
|---|---|
| 20:12:57.760 | Game starts (script) |
| 20:12:57.831 | `game started; showing the Gaming map` (+71 ms) |
| 20:13:07.858 | `last game ended; restoring the saved state` |
| 20:13:07.859 | Game process exits (script) |

So the Static frame with the correction looks the same as zone Static, apart from Left Ctrl matching the other keys. The correction can stay on Static, as designed.

At the end, `State` read `"Ripple" 61 56 70 52 100`, with `GameSwitch` false and `GameActive` false. The user's `corrections.toml` (`e0 = 85`) stays in place.
