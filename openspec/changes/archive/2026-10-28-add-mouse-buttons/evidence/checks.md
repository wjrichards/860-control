# Checks

## 4.2 Workspace checks

Run on 2026-10-27 at `824f87c`:
- `cargo clippy --workspace --all-targets -- -D warnings` in the GNOME 50 SDK: clean.
- `cargo test --workspace` in the SDK: 160 passed (18 app + 51 protocol + 91 service), plus the 2 hardware tests skipped as usual. New in this change:
  - protocol: decoding the real profiles' buttons, `with_button` changing only its entry and the CRC, refusing left click and `Other`, and the round trip of every action kind;
  - service: the bus kinds and their refusals, `MouseButtons` on connect, a change written after the backup, every refusal writing nothing, a failed write keeping the old action, the profile watch following a G9 switch (re-reading the sector and re-applying the lighting), and a lost link disconnecting the mouse;
  - app: action descriptions, the key table, modifier bits and button names.
- `gjs -m gnome-extension/tests/logic-test.js`: all passed.
- `openspec validate add-mouse-buttons --strict` passes, and `openspec validate --specs --strict` gives 4 passed.

## 2.3 D-Bus check

`buttons_dbus_check.sh` on a private bus with no hidraw nodes (`buttons_dbus_check.out`):
- `MouseButtons` is an empty `a(ya(sqy))`;
- `SetMouseButton` is refused with "the mouse is not connected", and the kind `other` with "\"other\" entries can't be written" before the mouse is even looked at;
- no backup file was written, and the service was still running after a profile-watch period.
