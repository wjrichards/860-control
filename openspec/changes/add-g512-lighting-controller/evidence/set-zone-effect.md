# setZoneEffect (0x8070 fn3) on the G512, firmware U1 19.01 build 0014

Source: `spike_set_zone_effect.py`, run by the user on 2026-10-22. Device replies are in `spike-out.txt`. The user watched the keyboard and confirmed that every accepted step looked as intended.

## Request layout (confirmed)

```
11 ff <slot> <3<<4 | swid>  zone  effect_index  block[10]  persistence
```

- `effect_index` is the effect's position in the zone list, **not** its ID. Breathe is index 2 (ID `0x000a`) and Ripple is index 6 (ID `0x000b`); both behaved correctly when addressed by index.
- `persistence` was 0 (RAM-only) throughout.
- The success reply is all zeros. It does not echo the request.

## Parameter block per effect

| Effect | Index | Block bytes | Result |
|---|---|---|---|
| Off | 0 | all zero | dark ✔ |
| Static | 1 | `R G B 00` | red, blue and white correct, so the byte order is R,G,B ✔ |
| Breathe | 2 | `R G B period_hi period_lo 00 64` | breathes. Period 4000 visibly slower than 992 ✔ |
| Cycle | 3 | `00 00 00 00 00 period_hi period_lo 64` | cycles. Period 8000 visibly slower than 992 ✔ |
| Wave | 4 | `00 00 00 00 00 00 1e 00 01 64` | wave moves ✔ (guess B) |
| Wave | 4 | `00 00 00 00 00 period_hi period_lo 64` | **HID++ error 0x02** (guess A) |
| Ripple | 6 | `R G B 00 period_hi period_lo` (period 30) | cyan ripples from pressed keys ✔ |

`64` (100) in the last used byte is the intensity value, sent as in Solaar. Other intensities were not tried.

## Reading the wave result

Guess A was rejected and guess B accepted. In guess A, block byte 8 was 0. In guess B, block byte 8 was 1. That fits block byte 8 being the wave **direction**, with 0 invalid. If bytes 6–7 hold a big-endian period (as in Breathe and Cycle), guess B sent `0x1e00` = 7680. This is an inference; `spike2_periods.py` checks it.

## Other observations

- The user reported that **Left Ctrl looks brighter** than the other keys during the test. This hasn't been investigated. It may be the LED itself. Per-key lighting (a follow-up change) could correct it.

## Spike 2 (`spike2_periods.py`, replies in `spike2-out.txt`)

The user confirmed that only the accepted steps lit up.

- **Wave:** a big-endian period in block bytes 6–7 was **rejected (0x02) for every value**: 1000, 4000, 12000 and 200. The only accepted wave so far (spike 1, guess B) had byte 6 = `0x1e` (30) and byte 7 = `0x00`. So the earlier inference was wrong. Byte 7 must be 0 (or is otherwise constrained), and byte 6 is probably a one-byte speed. The device's reported default period for wave is also 30. Byte 8 = direction (1 works) and byte 9 = intensity are unchanged.
- **Ripple:** periods 10, 100 and 1000 (big-endian in block bytes 4–5) were all accepted and lit. Whether they look different was not reported.
- **Breathe:** period 8000 was accepted and worked.

## Spike 3 (`spike3_wave_ripple.py`, replies in `spike3-out.txt`)

All steps were accepted. The user reported that **every wave moved, a bigger wave value is slower, and ripple at 1000 is slower than at 10**.

- **Wave:** block byte 6 is a **one-byte speed value** (bigger = slower). Byte 7 must be 0. Values 10, 30, 100 and 250 all work.
- **Ripple:** the big-endian period in block bytes 4–5 changes the speed (bigger = slower). Values 10, 100 and 1000 all work.

## Final layout used by `g512-hidpp`

| Effect | Index | Block (10 bytes, zero-padded) | Colour | Speed value | Tested range |
|---|---|---|---|---|---|
| Off | 0 | — | no | — | — |
| Static | 1 | `R G B 00` | yes | — | — |
| Breathe | 2 | `R G B p_hi p_lo 00 64` | yes | u16 period, bigger = slower | 992–8000 |
| Cycle | 3 | `00 00 00 00 00 p_hi p_lo 64` | no | u16 period, bigger = slower | 992–8000 |
| Wave | 4 | `00 00 00 00 00 00 s 00 01 64` | no | u8 speed, bigger = slower | 10–250 |
| Ripple | 6 | `R G B 00 p_hi p_lo` | yes | u16 period, bigger = slower | 10–1000 |

The ranges the service offers stay within these tested values: Breathe and Cycle 1000–8000, Wave 10–250, Ripple 10–1000. Wave direction is fixed at 1, the only value tested.
