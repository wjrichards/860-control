# Protocol and service checks

## 1.1 Enforced function lists

`Feature::allows` is checked in `Device::request` before anything is sent.
- `function_lists_match_spec`:
  - PerKeyLighting allows 3 and 5;
  - AdjustableDpi allows 0–2;
  - OnboardProfiles allows 0, 2–8 and 11, never 1;
  - other features allow any function.
- `disallowed_functions_are_never_sent`: on the fake mouse, OnboardProfiles 1, 9, 10, 12 and 15, AdjustableDpi 3, and PerKeyLighting 0 all return `FunctionNotAllowed`, with the fake's sent-report count unchanged.
- `other_per_key_functions_are_never_sent`: the old per-key fn1 test now expects `FunctionNotAllowed`.
- `allowlist_matches_spec`: 8 features. `dangerous_features_absent` now also covers the mouse's hidden features (`0x1300`, `0x18a1`, `0x1e22`, `0x1806`, `0x18b1`).

## 1.2 Profiles

The fake mouse holds the real sectors 0–5 from spike 6 (`g512-hidpp/src/fixtures/g502_hero_profiles.txt`), with `0x8100` and `0x2201` answered as the mouse did. 47 `g512-hidpp` tests pass. The profile tests:

| Test | What it shows |
|---|---|
| `reads_the_real_profiles` | (1 ms, default 1, 1200/2400/3200/6400), (1, 2, 400/800/1600/2400), (2, 0, 2400), (1, 0, 3200), (1, 0, 6400) |
| `crc_matches_the_real_sectors` | CRC-CCITT check value `0x29b1`, and all six real sectors |
| `edited_changes_only_its_bytes` | only bytes 0–12 and 254–255 change, the new CRC is valid, unused DPI slots are 0, and a switched index past the new steps follows the default |
| `edited_rejects_bad_values` | rate 3 ms, 0 steps, 6 steps, a default past the steps, a 0 DPI value |
| `only_user_sectors_can_be_named` | profiles 0, 6, 16 and 255 refused; sector 0 and `0x01xx` can't be expressed as a `UserSector` |
| `write_then_read_back` | write, read-back, `profile_writes == [5]` |
| `read_back_mismatch_fails` | a corrupted commit gives `WriteNotVerified { sector: 2 }` |
| `unknown_format_is_refused` | format 3 gives `UnsupportedProfileFormat(3)` |
| `switching_profiles` | 4 → 1, and profile 6 refused without changing anything |
| `dpi_range_from_the_sensor` | 100–25600 step 50; 125, 25650 and 50 rejected |

## 2.1 Service

`MouseConnection` reads the profiles, the active profile and the DPI range on connect. If they can't be read, they're `None` and lighting carries on. 82 service tests pass, among them:

- `mouse_profiles_are_read_on_connect`: active 4, range 100–25600/50, the real profile values.
- `mouse_profile_edit_writes_after_a_backup`:
  - the backup is written before the first write and holds profile 5 from before it;
  - the cache updates;
  - a second edit leaves the backup untouched.
- `mouse_profile_edits_are_validated`: nine bad requests (profiles 0 and 6, rate 3, 0 or 6 steps, a default past the steps, 125, 25650, 50) are refused with nothing written, not even the backup.
- `failed_profile_write_keeps_the_old_values`: `WriteNotVerified` reaches the caller, the cached values are unchanged, and the mouse stays connected.
- `switching_profiles_reapplies_the_lighting`: both zones are re-applied with the matching red, and profile 6 is refused.
- `editing_the_active_profile_reapplies_the_lighting`: an inactive profile leaves the lighting alone; the active one re-applies it.
- `mouse_without_readable_profiles_still_does_lighting` and `profile_requests_without_a_mouse`.

## 2.2 Restore

`profile_backup` (3 tests):
- the spike 6 format parses;
- `save_once` writes the directory and five profiles, and never overwrites the file;
- `restore` skips sector 0 and `0x0101` (not user sectors) and profile 3 (CRC no longer valid), leaves profiles 1 and 4 alone as unchanged, and writes back profiles 2 and 5, which had been edited. The writes are `[2, 5]`, and both match the backup afterwards.

`g512d --restore-mouse-profiles FILE` uses it.

## 2.3 D-Bus

`profiles_dbus_check.sh` runs the new g512d on a private bus, under `bwrap --dev /dev` with no hidraw nodes, and a temporary config (`profiles_dbus_check.out`):

- `MouseProfiles` is an empty array, `MouseActiveProfile` is 0, and `MouseDpiRange` is `(0, 0, 0)`.
- `SetMouseProfile` and `SetMouseActiveProfile` are refused with "the mouse is not connected", and no backup file is written.
- `g512d --restore-mouse-profiles` says "no G502 HERO found" and exits with 1.

## 2.4 On the mouse (installed service)

The service and app from this branch were installed with the user's OK (g512d 07:17 and 07:19, app at 07:20). On connect, the service read all five profiles, matching spike 6. Profile 4 was active, and the range was 100–25600 in steps of 50.

**Step 1, an inactive profile** (`profiles_hardware_step1.out`): `SetMouseProfile 5` with 2 ms, default 1, [800, 1600, 3200].
- The service logged "saved the mouse's profiles to …/mouse-profiles-backup.txt", then the write.
- **All six backed-up sectors (0–5) equal the spike 6 backup.**
- `MouseProfiles` shows profile 5 changed. Profile 4 stayed active and unchanged.

**Step 2, the active profile** (`profiles_hardware_step2.out`): `SetMouseProfile 4` with 4 ms (250 Hz), default 0, [800, 1600, 3200].
- `read_dpi.py` showed the mouse at **800 DPI** straight away, so a write to the active profile applies without re-selecting it.
- The user pressed DPI-up twice and reported "Pointer got faster".
- AdjustableDpi `getSensorDpi` and OnboardProfiles `getCurrentDpiIndex` still read 800 and index 0 afterwards.

**Finding:** in onboard mode, those two reads report the profile's default step, not the step the DPI buttons have moved to. The app doesn't show the current step, so nothing depends on them. The user's observation is the evidence that the buttons cycle the new steps.

**Step 3, report rate:** 4 ms was written as part of step 2 and verified by the read-back. A rate change can't be seen by eye, and `0x8060` isn't allowlisted for reading it.

**Steps 4 and 5, switch and restore** (`profiles_hardware_step4.out`):
- `SetMouseActiveProfile 1`: the mouse reads 2400 DPI on profile 1. Back to 4, it reads 800, profile 4's new default.
- `g512d --restore-mouse-profiles ~/.config/g512-control/mouse-profiles-backup.txt`:
  - sector 0 was skipped (not a user sector), sectors 1–3 were unchanged, and **sectors 4 and 5 were written**;
  - it then reported "the running service has re-read the profiles";
  - `MouseProfiles` shows the original values again, and the mouse reads **3200 DPI on profile 4**, the user's setting before this change;
  - the backup file still equals spike 6.

**Found and fixed here:** a restore made by the separate `--restore-mouse-profiles` process left the running service showing the edited values. Now:
- `set_mouse_active_profile` re-reads every profile after switching (test `switching_rereads_profiles_written_elsewhere`);
- the restore command asks a running service to switch to the already-active profile, so it re-reads.
