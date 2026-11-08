# Hardware: mouse macros

## 1.1 Read-only dump (2026-10-29)

Script: `hardware/spike10_read.py`; full output with every sector in hex: `hardware/spike10-read.txt`. It called only Root `getFeature` and OnboardProfiles `getInfo` and `memoryRead`, as the user, with g512d running (its own software ID, so no clash).

**getInfo:** `01 02 01 05 05 0b 10 01 00 0a 01`
- profile format 2;
- macro format 1;
- 5 profiles;
- 11 buttons;
- 16 sectors of 256 bytes.

The same as spike 6.

| Sectors | Content | CRC |
|---|---|---|
| 0 | directory: profiles 1–5 listed (`00 0N 01 00`), then `ff ff 00 00` and `ff` padding | ok |
| 1–5 | the five profiles | ok |
| 6–f | **all 256 bytes `00`**, the same in every sector | stored `00 00`; the CRC of 254 zero bytes would be `59 2d`, so not valid |

**What this changes in the plan:**
- **Free sectors are zero-filled, not `ff`.** The spec already counts a sector with a wrong CRC as free, so 6–f all count as free. The design assumed "all `ff`"; tasks 2.1 and 3.1 must treat zeros with a bad CRC as free too. Deleting a macro still writes `ff` padding with a valid CRC.
- **The directory lists profiles only.** Macros aren't registered there, so saving a macro never touches sector 0, as planned.
- **No button plays a macro today.** Every entry is a mouse button (`80 01`), a key (`80 02`, profile 3's Ctrl+C and Ctrl+V) or a function (`90 xx`).
- **G9 is entry 8.** In every profile it is `90 0a` (next profile); it shows as callout 6 in the app.
- **Nothing has changed since spike 6:** all 16 sectors match the 2026-10-24 dump byte for byte.

## 1.2 Write test (2026-10-29)

Script: `hardware/spike10_write.py write`; output: `hardware/spike10-write.txt`. The backup is the 1.1 dump.

1. The mouse reported profile 1, although the user had tried to switch. The script refused to write. Profile 5 was then made active through g512d (`SetMouseActiveProfile 5`).
2. **Sector 6:** written as `43 00 04 44 00 04 43 00 05 44 00 05 ff` (`a` down, `a` up, `b` down, `b` up, end), then `ff` padding and the CRC. It read back matching.
3. **Profile 5, entry 8 (G9):** changed from `90 0a ff ff` to `00 06 00 00`, with a new CRC. It read back matching.
4. The user pressed G9 in a text box and **"ab" appeared**. The mouse played the macro straight away, with no reload or profile switch.

**Confirmed:**
- the macro button entry is `00 sector 00 offset`;
- key down/up `43`/`44` with a HID usage;
- end `ff`;
- `ff` padding;
- a CRC-CCITT over 254 bytes in a macro sector;
- writing the active profile takes effect immediately.

## 1.3 Play modes and pauses (2026-10-29)

For each try, only sector 6 was rewritten (`spike10_write.py macro …`), and each read back matching. Profile 5's G9 stayed at `00 06 00 00`.

| Sector 6 | What the user saw |
|---|---|
| `a`, `40 07 d0` (2000 ms), `b` | "a", then "b" about 2 seconds later |
| `a`, `01`, `b` | "a" on press, "b" when G9 was let go |
| `02`, `a`, `b` | "abab…" repeating while held, stopping on release |
| `03`, `a`, `b` | "abab…" kept going after release, and the second press stopped it |

**All four play modes and the pause command work as libratbag describes:**
- `01` waits for release wherever it sits;
- `02` and `03` work as the first command.

## 1.4 Restore (2026-10-29)

`spike10_write.py restore` wrote sector 6 first, using its original bytes: all zeros, with no valid CRC.
- The mouse answered `memoryWriteEnd` with HID++ error `0x04`, but **the sector then read back as all zeros, matching the backup**.
- The script stopped there. Sector 5 was then written from the backup on its own, and read back matching.
- A fresh dump (`hardware/spike10-after-restore.txt`) matches the 1.1 dump in all 16 sectors.
- The active profile was set back to 1 through g512d, which reports profile 5's G9 as "next profile" again.

**Learned:**
- The mouse refuses to finish a write whose CRC is wrong. Clearing a macro sector must write `ff` padding with a valid CRC, as the spec already says. Writing zeros back is not a way to free a sector.
- Even so, a refused write can leave the sector's content changed or reset. Here it came back as zeros, which is a free sector, so no harm. The service must still read a sector back after any failed write, and never assume a refused write left it untouched.
