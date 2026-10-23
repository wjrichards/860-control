# Per-key lighting on the G512 (spike 5), 2026-10-22

`spike5_per_key.py` was run three times; the user watched the third run. The script's output is in `spike5-out.txt`, `spike5-run2-out.txt` and `spike5-run3-out.txt`. After each run, `g512d`'s `SetState` restored the saved Ripple.

## Protocol (from g810-led's G512/G513 path, confirmed on the keyboard)

The source is `MatMoul/g810-led`, `src/classes/Keyboard.cpp` (`getKeyGroupAddress`, `setKeys`, `commit`), read on 2026-10-22. g810-led is GPL-3.0. Only the packet layout was taken, as protocol facts; none of its code was copied.

- **Set keys:** `12 ff <0x8080 slot> 3a 00 01 00 0e`, then 14 × `key R G B` in a 64-byte report. Key IDs are USB HID keyboard usages.
- **Commit:** `11 ff <slot> 5a`, which is function 5.
- Every request got an `ok` reply, in all three runs: 418 per run, with no HID++ errors and no timeouts.

## Results

| Step | Seen by the user (run 3) | Measured |
|---|---|---|
| 1 Left Ctrl red over Static white | Only Left Ctrl turned red | ok |
| 2 Effect Off, then Esc green | Esc lit green on the dark keyboard | ok |
| 3a All 104 keys blue | Not asked | 9 requests in 28 ms |
| 3b 40-frame fade | Not asked | 35.4–35.7 frames/s |
| 4a Wave over the blue frame | Wave replaced the blue | ok |
| 4b Left Ctrl red during Wave | The Wave stopped (a per-key write takes over from the running effect) | ok |
| 5a/b/c White; Left Ctrl at 100/85/70 % | Brighter at 100 %; **matched at 85 %** | ok |
| 6 Game Mode key while amber | Not reported | No HID++ notification and no other report on interface 1 in any run |

## What this settles

- **Per-key colours work** from any starting state, and the commit makes them take effect. Starting a built-in effect with `0x8070` replaces the per-key frame, and a per-key write stops a running effect. So "Custom" is a mode of its own, like the effects.
- **A full frame is fast:** 28 ms, about 35 frames/s. Animations driven from the computer are possible later.
- **The Left Ctrl LED is about 15 % too bright.** A per-key trim (×0.85) fixes it in Custom, and in Static if Static is sent as a per-key frame. Breathe, Cycle, Wave and Ripple run in the keyboard's firmware and can't be trimmed.
- **The Game Mode key can't be followed.** The keyboard sent nothing on the HID++ interface. Whether the key was pressed during a run is not confirmed. Switching to a gaming layout should instead use the GameMode daemon (`com.feralinteractive.GameMode`).
