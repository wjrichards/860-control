# Spike 6: the G502 HERO

Run by the user with sudo on 2026-10-23, against `/dev/hidraw10`, the mouse's HID++ interface (interface 1, reports `0x10` and `0x11`). The `sudo` log shows `spike6_mouse_read.py` at 18:19:59 and `spike6_mouse_light.py` at 18:20:15. Outputs: `spike6-read.txt` and `spike6-light.txt`.

## Features (`hidpp_probe.py`, 18:11)

Logitech G502 HERO, `046d:c08b`, firmware U1 27.03 build 0010, HID++ 4.2.

| Slot | ID | Name | Use |
|---|---|---|---|
| 0x02 | 0x8070 v3 | ColorLedEffects | lighting (this change) |
| 0x03 | 0x0003 | DeviceFwVersion | info |
| 0x04 | 0x0005 | DeviceName | info |
| 0x0a | 0x2201 v1 | AdjustableDpi | later change |
| 0x0b | 0x8060 | AdjustableReportRate | later change |
| 0x0c | 0x8100 | OnboardProfiles | later change (DPI steps, report rate, buttons) |
| 0x0d | 0x8110 | MouseButtonSpy | not used |
| 0x05, 0x06–0x09, 0x0e–0x12 | 0x1300, 0x18a1, 0x1e00, 0x1e22, 0x1eb0, 0x00c2, 0x1801, 0x1802, 0x1806, 0x18b1 | LED test, hidden features, SPI and TDE access, DFU, reset, properties | **never called**: not allowlisted |

There is no `0x1B04` ReprogControls, so button remapping has to go through onboard profiles.

## Lighting (`0x8070`)

- **Zones:** 2.
  - Zone 0 is at location `0x0001`. The user saw it turn red in step 1: it's **the logo**.
  - Zone 1 is at location `0x0002`, which makes it **the DPI strip**.
- **Effects:** each zone has 4, at the same indexes in both zones:

  | Index | ID | Effect | Caps | Default period |
  |---|---|---|---|---|
  | 0 | `0x0000` | Off | | |
  | 1 | `0x0001` | Static | `0x0005` | |
  | 2 | `0x000a` | Breathe | `0xc005` | 992 |
  | 3 | `0x0003` | Cycle | `0xc005` | 992 |

  The effect IDs are the same as on the G512. Wave and Ripple aren't offered.
- **`setZoneEffect`** (fn3; zone, index, Static block `R G B` and zeros, persistence 0) was accepted for all 8 writes.
- **Onboard mode holds.** `getMode` read `1` (onboard) at the start, at every step and at the end. The tool never changes the mode.
- **A DPI press doesn't reset the lighting.** With both zones white, the user pressed DPI-up, and both stayed white for the next 20 s ("Stayed white"). So lighting set from the computer coexists with the stored profile, and the mouse's own DPI buttons keep working.

## DPI, report rate, profiles (read only)

- **AdjustableDpi:** 1 sensor. The list `00 64 e0 32 64 00` means 100 to 25600 in steps of 50. It was at 3200, with a default of 1000.
- **AdjustableReportRate:** periods of 1, 2, 4 and 8 ms were offered, and it was at 1 ms (1000 Hz).
- **OnboardProfiles `getInfo`:**
  - memory model 1, profile format 2, macro format 1;
  - 5 profiles and 5 ROM profiles, 11 buttons;
  - 16 sectors of 256 bytes, mechanical layout `0x0a`.
- **State:** mode 1 (onboard), current profile 4, current DPI index 0.

## Memory backup

All 16 user sectors and the 5 ROM sectors are in `spike6-read.txt`, one hex line per sector. Checksums are CRC-CCITT (init `0xFFFF`, polynomial `0x1021`) over bytes 0–253, stored big-endian in 254–255.

| Sectors | Contents |
|---|---|
| `0x0000` | profile directory, CRC ok. Entries `00 0N 01 00` point to sectors 1–5, enabled. |
| `0x0001`–`0x0005` | the 5 user profiles, CRC ok. Each starts with the report rate, DPI indexes and DPI list in little-endian; profile 1 has 1200, 2400, 3200, 6400. |
| `0x0006`–`0x000f` | all zero, unused. Their "CRC mismatch" just means no CRC was written. |
| `0x0100`–`0x0104` | ROM (factory) directory and profiles, in a different layout. Read only. They don't match the user-sector CRC rule, which doesn't matter because they're never written. |

This backup is what a later change restores from before any profile write.

## Conclusions for this change

- **Mouse lighting fits the existing `0x8070` code.** The G512's `effects::discover` and `set_effect` cover the same IDs. The mouse needs a second zone, and only the effects it reports.
- **No host mode is needed.** Lighting works and stays put while the mouse remains in onboard mode.
- **To match the keyboard, map effects:**
  - Off, Static, Breathe and Cycle map one to one;
  - Wave, Ripple and Custom have no mouse equivalent, and need a fallback.
- **A udev rule for `046d:c08b`** (uaccess, as for the keyboard) is needed for g512d to open the mouse without root.
