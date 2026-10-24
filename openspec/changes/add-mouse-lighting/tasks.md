# Tasks

## 0. Spike

- [x] 0.1 Spike 6 on the G502 HERO: a read-only survey with a CRC-checked backup of the profile memory, and a RAM-only lighting test in onboard mode (`evidence/mouse.md`, `spike6-read.txt`, `spike6-light.txt`). Zone 0 is the logo and zone 1 the DPI strip. The effects are Off, Static, Breathe and Cycle. Lighting holds in onboard mode and after a DPI press

## 1. Protocol: the mouse model and every zone

- [x] 1.1 Add `Model` (G512, G502Hero) to `discovery` (D1): the product ID, usage page and required report IDs; `is_lighting_interface` returns the model; `find_lighting_interface` takes one. Verify with unit tests on the real descriptors (the keyboard's, and the mouse's `hidraw9` and `hidraw10` from sysfs): each matches only its own lighting node, and a boot node or another product is ignored
- [x] 1.2 Add `set_effect_all` (D2), and give the fake device a zone count. Verify with tests that a two-zone fake gets one `setZoneEffect` per zone with the right indexes, that a rejection from either zone fails the call, and that the keyboard's one-zone path sends the same bytes as before

## 2. Service: mouse lighting

- [x] 2.1 Add `MouseSide` and `MouseSettings` to `Controller`, `mouse_output()` with the D3 table, and `apply_mouse_logged()` after every keyboard-affecting path (D3). Verify with fake-device tests:
  - every mapping (Off, Static, Breathe, Cycle; Wave to Cycle, Ripple to Static, Custom and Gaming to W's colour, Off when W is off);
  - lights off, then on again;
  - a game starting and ending;
  - an own setting turns matching off and the lights on;
  - a mouse error leaves the keyboard call's result unchanged;
  - no mouse leaves the keyboard tests unchanged
- [x] 2.2 Save `[mouse]` in `state.toml` (D7). Verify with store tests: a change-3 file loads with matching on and no own setting; the own setting round-trips; `[mouse]` isn't written until something changes
- [x] 2.3 Hotplug, resume and the login retry cover the mouse (D6). Verify with unit tests of the retry loop over two devices, and of "remove" disconnecting only the matching device
- [x] 2.4 D-Bus members (D5), emitted with the others. Verify on a private bus with no devices visible (`bwrap --dev /dev`, as in change 3) that the properties read and `SetMouseLighting` and `SetMouseMatch` round-trip and save
- [x] 2.5 Add the `046d:c08b` line to `packaging/70-g512-control.rules`, and the reinstall steps to the README. Verify that the user installs it with sudo, and that the mouse's hidraw nodes get the session ACL (`getfacl`)
- [x] 2.6 Install the service (with the user's OK for the restart) and check on the hardware, with the user watching:
  - matching Static, Breathe, Cycle, Ripple and Gaming (via `gamemode-simulate-game`);
  - an own setting;
  - lights off and on;
  - a mouse replug;
  - slow and fast Breathe and Cycle on the mouse looking like the keyboard's (D4);
  - the DPI buttons still cycling

## 3. App: Mouse section

- [x] 3.1 Add the Mouse group (D8). Verify by AT-SPI: the status row, the switch reaching `SetMouseMatch`, the own-setting rows appearing only with matching off and sending `SetMouseLighting`, and the group hidden against an older service

## 4. Docs and checks

- [x] 4.1 README: the mouse in the intro and in Things to know, the udev reinstall, mouse `busctl` examples, and the evidence path
- [x] 4.2 Workspace checks: clippy `-D warnings` and tests in the GNOME SDK, the `gjs` tests, and `openspec validate --strict` for the change and specs
- [ ] 4.3 Hands-on pass with the user in the app: match on and off, an own colour, and the Mouse section in light and dark style
