# Spike 9: media keys and "disabled"

Run on 2026-10-27 with the user's go-ahead, as the user (no sudo), against `/dev/hidraw3`, alongside g512d. Script: `spike9_media_disabled.py`. Output: `spike9.out`.

## What was written

A read-only check first: profile 4 active, and profile 5 equal to the backup. Then inactive profile 5 was rewritten with two entries changed:

| Entry | Button | Was | Written |
|---|---|---|---|
| 2 | wheel click | `80 01 00 04` (middle) | `80 03 00 e9`, consumer Volume Up |
| 8 | G9 | `90 0a ff ff` (profile cycle) | `80 01 00 00`, the empty mouse-button mask |

8 bytes changed: the two entries and the CRC. Read back, the sector matched. Profile 5 was made active.

The design named Play/Pause (`00 cd`) for the media test. Volume Up was used instead because GNOME shows a popup for it, so it's visible without a player running. Both are consumer usages in the same entry type.

## Results

- **Wheel click:** the user saw GNOME's volume popup and the volume went up. Consumer entries (`80 03 hi lo`) work.
- **G9:** the user pressed it two or three times and saw nothing happen. Read afterwards, the mouse's active profile was still 5; under its normal mapping G9 would have cycled profiles. `80 01 00 00` disables the button.

So the app offers both media keys and Disabled.

## Undo

`g512d --restore-mouse-profiles` wrote only sector 5 back; sectors 1–4 were unchanged. Then `SetMouseActiveProfile 4`. The mouse reads profile 4 active and profile 5's buttons match the backup, and g512d's `MouseActiveProfile` is 4.
