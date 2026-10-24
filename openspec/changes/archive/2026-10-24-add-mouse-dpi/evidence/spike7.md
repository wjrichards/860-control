# Spike 7: onboard-profile write and profile switching

Run on 2026-10-24 with the user's go-ahead, as the user and without sudo (the udev rule from change 4 gives the session access), against `/dev/hidraw3`. g512d was running alongside. Output: `spike7.out`.

## Write

The target was user profile sector 5, profile 5, which wasn't active (the active profile was 4). It was read first, and passed every guard: valid CRC, and identical to the spike 6 backup (`archive/2026-10-24-add-mouse-lighting/evidence/spike6-read.txt`).

**Edit:** the first DPI step went from 6400 to 6000. That changed bytes 3–4 (little-endian DPI) and 254–255 (the CRC: CRC-CCITT, init `0xFFFF`, over bytes 0–253, stored big-endian).

**Write sequence (`0x8100`):**
1. fn6 `memoryAddrWrite(sector 00 05, offset 00 00, length 01 00)`, which replies with zeros;
2. sixteen fn7 `memoryWrite` calls of 16 bytes each;
3. fn8 `memoryWriteEnd`, which replies with zeros.

The whole sector took about 0.3 s.

**Read back:** identical to the edit, CRC valid, first DPI step 6000.

**Restore:** the same sequence with the original bytes. Read back, sector 5 equals the spike 6 backup byte for byte, with a valid CRC.

## Profile switching

- fn3 `setCurrentProfile(00 01)`: `getCurrentProfile` reads `00 01`, and AdjustableDpi reads 2400, profile 1's default step (1200, **2400**, 3200, 6400).
- fn3 `setCurrentProfile(00 04)`: back to `00 04` and 3200 DPI.
- The mode stayed 1 (onboard) throughout.
- Afterwards `SetMouseMatch true` re-applied the matching lighting, in case the switch had brought back the profile's stored lighting. g512d logged no errors.

## Conclusions for the change

- **Writes:** a whole-sector read, edit and write, with a recomputed CRC and a read-back check, works. The sequence is fn6, then fn7 ×16, then fn8.
- **Profiles:** directory entries map profile N to sector N, `getCurrentProfile` and `setCurrentProfile` use `00 N`, and the profile's default DPI takes effect at once.
- **Lighting after a switch:** g512d should re-apply the mouse lighting after switching profiles. The user didn't watch the lighting during this spike, so whether a switch resets it is unconfirmed.
