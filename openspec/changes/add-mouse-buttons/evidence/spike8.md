# Spike 8: the button table

Run on 2026-10-24 with the user's go-ahead, as the user (no sudo), against `/dev/hidraw3`, alongside g512d. Output: `spike8.out`.

## Layout

Profile format 2 keeps 11 button entries of 4 bytes from byte `0x20`. Entries seen in the spike 6 backup:

| Entry | Meaning (libratbag's decoding) | Seen in |
|---|---|---|
| `80 01 hi lo` | mouse button bitmask (`0001` left … `0010` forward) | every profile, entries 0–4 |
| `80 02 mm kk` | keyboard: modifier bitmask `mm`, HID usage `kk` (`01 06` = Ctrl+C) | profile 3, entries 6–7 |
| `90 cc xx xx` | special: `01` wheel left, `02` wheel right, `03` DPI up, `04` DPI down, `07` DPI shift (sniper), `0a` profile cycle, `0b` G-shift | every profile |

## Physical buttons

Inactive profile 5 matched the backup and was rewritten:
- entries 0–1 (left and right) kept;
- entries 2–10 set to `80 02 00 06` … `80 02 00 0e`, the letters c–k with no modifier;
- 31 bytes changed, all in entries 2–10 plus the CRC. Read back, the sector matched.

Profile 5 was then made active. The user pressed, in order: wheel click, back (front thumb), forward (rear thumb), sniper, DPI down, DPI up, G9, wheel tilt right, wheel tilt left. They reported "cdefghijk". So:

| Entry | Button |
|---|---|
| 0, 1 | left, right |
| 2 | wheel click |
| 3 | back (front thumb) |
| 4 | forward (rear thumb) |
| 5 | sniper (red thumb) |
| 6 | DPI down |
| 7 | DPI up |
| 8 | G9 (behind the wheel) |
| 9 | wheel tilt right |
| 10 | wheel tilt left |

Keyboard entries written this way work as soon as the profile is active.

## Undo

`g512d --restore-mouse-profiles` wrote only sector 5 back; sectors 1–4 were unchanged. Then `SetMouseActiveProfile 4`, at the user's request. The mouse reads 3200 DPI on profile 4, and `MouseProfiles` matches the backup.

## Finding: the active profile changes on the mouse itself

Before the spike, the mouse's active profile was **1**, while g512d's `MouseActiveProfile` still read 4. The G9 button cycles profiles in the firmware (`90 0a`), and g512d only reads the active profile on connect and after its own switches. This change should keep it current, for example by reading it again when a front end refreshes, or by watching for a profile-change notification if the mouse sends one.
