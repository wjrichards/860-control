# Tasks

## 0. Spikes

- [x] 0.1 Spike 8: entry order on the user's G502 HERO, and keyboard entries written by us (`evidence/spike8.md`). Entries 0–10 are left, right, wheel click, back, forward, sniper, DPI down, DPI up, G9, tilt right, tilt left; `80 02 00 kk` types the key
- [ ] 0.2 Spike 9 (D2): on inactive profile 5, set wheel click to `80 03 00 cd` (play/pause) and G9 to `80 01 00 00` (disabled candidate), make it active, and have the user check both. Restore. Record which of media keys and "disabled" to offer

## 1. Protocol

- [x] 1.1 Add `ButtonAction` and `Special` with `ProfileSector::buttons` and `with_button` (D1). Verify with tests on the real backup sectors:
  - profile 1 and 3 decode (Ctrl+C and Ctrl+V on profile 3, G-shift);
  - `with_button` changes only that entry and the CRC;
  - entry 0 and `Other` are refused;
  - unchanged `ff ff` tails survive;
  - encode and decode round-trip for every kind

## 2. Service

- [ ] 2.1 Offer `MouseButtons` and add `SetMouseButton` with the D3 validation, the backup and a verified write. Verify with fake-mouse tests: every refusal (button 0, out of range, bad kind or value, `other`), a successful change updating the cache, and a failed write keeping the old value
- [ ] 2.2 Add the active-profile watch (D3). Verify with a fake-mouse test that a profile changed behind the service's back is picked up, the sector is re-read, lighting is re-applied and a change is reported; and that a lost link disconnects the mouse
- [ ] 2.3 D-Bus check on a private bus with no devices: `MouseButtons` is empty and `SetMouseButton` is refused
- [ ] 2.4 Install (with the user's OK for the restart) and check on the mouse:
  - remap G9 on profile 4 to Ctrl+C, and the user copies with G9;
  - press G9 under its normal profile-cycle mapping on another profile, and see the app follow within 2 s;
  - restore

## 3. App

- [ ] 3.1 Add the GDK-key-to-HID table and the action descriptions ("Ctrl+C", "DPI up", "Middle click"). Verify with unit tests
- [ ] 3.2 Add the Mouse Buttons group and the Change dialog (D4), with rows built once. Verify the rows and the left-click row by AT-SPI where the widgets allow; otherwise by hand in 4.3

## 4. Docs and checks

- [ ] 4.1 README: Mouse Buttons, the actions, left click fixed, G9 and the 2 s watch, `busctl` examples
- [ ] 4.2 Workspace checks: clippy `-D warnings` and tests in the GNOME SDK, the `gjs` tests, and `openspec validate --strict` for the change and specs
- [ ] 4.3 Hands-on pass with the user and a call recording: set a shortcut by pressing it, set a mouse function, set a media key and "disabled" if offered, check that left click has no Change; then restore
