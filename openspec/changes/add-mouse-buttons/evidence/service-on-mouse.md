# 2.4 The service on the mouse

On 2026-10-27, with the user's OK, the new g512d was installed with `packaging/install-service.sh`; the service restarted and connected both devices. Its `MouseButtons` matched the backup for all five profiles, including Ctrl+C and Ctrl+V on profile 3.

## Step 1: G9 as Ctrl+C (`hardware_step1.sh`, `.out`)

`SetMouseButton 4 8 key 6 1` succeeded, and the service logged "button 8 is now key 0x0006 (modifiers 0x01)". The write was read back from the mouse inside `write_sector`. The backup already existed, so it wasn't touched.

The user selected text, pressed G9 and pasted: **G9 copied**. Afterwards the mouse still reported profile 4, so G9 no longer switched profiles.

## Step 2: following a G9 switch

**First try** (`hardware_step2.sh`, `.out`): profile 5 was made active and the signals were recorded. Over two presses the user saw the DPI lights flash and the pointer slow down, but the mouse still reported profile 5 and 6400 DPI, and the service reported no switch.
- That matches a DPI button rather than G9: DPI down flashes the lights and slows the pointer, and this mouse reports the profile's default DPI whichever step the DPI buttons have moved to (change 5).
- It looked at first as if `getCurrentProfile` missed G9 switches. Spike 10 was run to check that.

**Spike 10** (`spike10_listen.py`, `spike10.out`, `spike10_journal.out`) was a read-only listen of 60 seconds on the mouse's HID++ interface, alongside g512d, while the user pressed G9. It found:
- **The mouse reports every switch:** an OnboardProfiles event `11 ff 0c 00 00 NN` (event 0, profile `NN`) on each change. Eight events arrived: 5, 1, 2, 3, then 4, 5, 1, 2. The user reported two presses, so either G9 was pressed more often or one press can move more than one profile. The events are the evidence either way.
- **`getCurrentProfile` does follow G9.** The service's 2 s watch logged each switch soon after its event:

  | Event (local) | Service log |
  |---|---|
  | 14:10:23 → 5, 14:10:24 → 1 | 14:10:24.88 "switched from profile 4 to 1" |
  | 14:10:25 → 2, → 3 | 14:10:26.96 "from 1 to 3" |
  | 14:10:37 → 4 | 14:10:39.06 "from 3 to 4" |
  | 14:10:41 → 5, 14:10:42 → 1 | 14:10:43.25 "from 4 to 1" |
  | 14:10:43 → 2 | 14:10:45.33 "from 1 to 2" |
  | (after the listen) | 14:10:53.42 "from 2 to 4" |

  Switches closer together than one poll are seen as one, which is all a front end needs.
- **Each switch was handled:** the profile's sector was re-read (16 `memoryRead` replies) and the lighting re-applied (two ColorLedEffects writes, one per zone), as D3 says.

Listening for the event instead of polling would follow a switch at once. The 2 s poll meets the spec, so this change keeps it, and the event is noted for later.

## Step 3: restore (`hardware_step3.out`)

`g512d --restore-mouse-profiles` wrote back only sector 4, putting G9 back to cycling profiles. Then `SetMouseActiveProfile 4`. The mouse reads profile 4 at 3200 DPI, as before, and after spike 10 the service reports profile 4 again.
