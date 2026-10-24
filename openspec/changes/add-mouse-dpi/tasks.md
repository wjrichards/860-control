# Tasks

## 0. Spike

- [x] 0.1 Spike 7: one reversible write to inactive profile 5, plus profile switching (`evidence/spike7.md`, `spike7.out`). The write, read-back and byte-exact restore all worked; `setCurrentProfile 00 N` applies the profile's default DPI at once

## 1. Protocol

- [x] 1.1 Add AdjustableDpi and OnboardProfiles to `Feature`, with `Feature::allows(function)` enforced in `Device::request` (D1). PerKeyLighting is limited to fn3 and fn5. Verify with tests that every function outside the lists returns `FunctionNotAllowed` with nothing sent, and that the allowlist test matches the spec
- [x] 1.2 Add `profiles` (D2): `read_info`, `UserSector`, `ProfileSector` (parse with CRC check, `edited`), `read_sector`, `write_sector` with read-back, `current_profile`/`set_current_profile`, `dpi_range`. Give the fake mouse profile memory built from the spike 6 backup. Verify with tests:
  - parsing all five real sectors;
  - `edited` changing only bytes 0, 1, 2 (when needed), 3–12 and 254–255;
  - the CRC against the real sectors;
  - sector 0 and `0x0101` refused before sending;
  - a read-back mismatch failing;
  - an unknown format refused;
  - switching profiles

## 2. Service

- [x] 2.1 Hold the profiles in the mouse connection (D4), and add `set_mouse_active_profile` and `set_mouse_profile` with the D5 validation, a once-only backup (D3) and a lighting re-apply. Verify with fake-mouse tests:
  - each validation error;
  - the backup is written once and never overwritten;
  - a failed write keeps the cached values;
  - a switch re-applies the lighting;
  - a mouse without readable profiles still does lighting
- [x] 2.2 `g512d --restore-mouse-profiles FILE` (D3). Verify on the fake that it writes only CRC-valid user sectors from the file and checks each read-back
- [x] 2.3 D-Bus members (D6). Verify on a private bus with no devices that they read as empty or zero and that invalid edits are refused
- [ ] 2.4 Install the service (with the user's OK for the restart) and check on the mouse with the user:
  - the backup file appears and matches the spike 6 backup;
  - editing an inactive profile, then the active one: the DPI buttons cycle the new steps, and the default holds after a replug;
  - a report-rate change;
  - a profile switch;
  - the restore command bringing back the original profiles

## 3. App

- [ ] 3.1 Add the Profile, Report Rate and DPI step rows to the Mouse section, with the 2 s debounce (D7). Verify:
  - by AT-SPI, that the rows show the active profile, a spin change sends one `SetMouseProfile` about 2 s after the last change, and choosing a profile sends `SetMouseActiveProfile` (the combo rows need a person; see change 4);
  - unit tests for the debounce and edit model

## 4. Docs and checks

- [x] 4.1 README: DPI and profiles in the Mouse section, the backup file and the restore command, `busctl` examples, and Things to know (flash writes)
- [ ] 4.2 Workspace checks: clippy `-D warnings` and tests in the GNOME SDK, the `gjs` tests, and `openspec validate --strict` for the change and specs
- [ ] 4.3 Hands-on pass with the user in the app: switch profile, change a DPI step and the report rate, and add and remove a step
