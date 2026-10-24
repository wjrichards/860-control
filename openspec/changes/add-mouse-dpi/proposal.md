# Proposal

## Why

The user wants to set the G502 HERO's DPI steps and report rate from the app, and to switch between its profiles. They chose option A: write these into the mouse's onboard profiles, so the mouse keeps them without the service and its own DPI buttons cycle through the user's steps. Live host-mode DPI was rejected because it fights the mouse's buttons.

Spike 7 (`evidence/spike7.md`) showed this works on the user's mouse:
- **Writing:** a profile sector can be read, edited, rewritten with a recomputed CRC, and verified by reading it back. It was restored byte for byte to the spike 6 backup.
- **Switching:** `setCurrentProfile` switches the active profile, and that profile's default DPI applies at once.

The user decided:
- edits are written about 2 s after they stop changing them;
- choosing a profile makes it active;
- each profile has 1–5 DPI steps, edited with spin buttons.

## What Changes

- **Two new HID++ features, each limited to listed functions** that the code enforces, not just by convention:
  - OnboardProfiles (`0x8100`): get info, get mode, get and set the current profile, memory read, memory write (address, data, end), and get the current DPI index. Never "set mode", so the mouse always stays in onboard mode.
  - AdjustableDpi (`0x2201`): reads only, to learn the sensor's DPI range.
- **A profile module in the protocol crate:**
  - parses profile format 2: report rate, default DPI index, and up to five DPI values;
  - edits those fields only, keeping every other byte;
  - writes whole user sectors with the CRC and a read-back check.

  It refuses the profile directory (sector 0), the ROM sectors (`0x01xx`), and any sector outside the user profiles that `getInfo` reports.
- **The service:**
  - reads all five profiles when the mouse connects;
  - offers them and the active profile on D-Bus, with `SetMouseActiveProfile` and `SetMouseProfile`;
  - validates edits: 1–5 steps, within the sensor's range in steps of 50, a default index inside the list, and a report rate of 1, 2, 4 or 8 ms;
  - re-applies the mouse lighting after a switch or a write.
- **A backup before the first write:** the service saves the mouse's user sectors to `~/.config/g512-control/mouse-profiles-backup.txt`, once, if that file doesn't exist. `g512d --restore-mouse-profiles FILE` writes them back.
- **The app's Mouse section gains:**
  - a Profile selector (1–5) that makes the chosen profile active;
  - a Report Rate selector (125, 250, 500 or 1000 Hz);
  - the DPI steps as spin-button rows, with the default step marked, and Add and Remove.

  Edits are sent about 2 s after the user stops changing them.

## Capabilities

### New Capabilities

None. Each piece extends an existing capability.

### Modified Capabilities

- `device-protocol`: the feature allowlist gains `0x8100` and `0x2201`, with enforced function lists; new requirements cover onboard-profile reads and writes, and profile switching.
- `lighting-service`: adds mouse profiles; changes the D-Bus interface requirement.
- `lighting-app`: adds the profile, report-rate and DPI controls to the Mouse section.

## Impact

- **`g512-hidpp`:**
  - `feature.rs` gets two features and per-feature function lists, enforced in `Device::request`;
  - a new `profiles` module;
  - the fake mouse gets profile memory.
- **`g512d`:**
  - the mouse connection holds its profiles;
  - D-Bus gets `MouseProfiles`, `MouseActiveProfile`, `MouseDpiRange`, `SetMouseActiveProfile(y)` and `SetMouseProfile(yyyaq)`;
  - the backup file is written, and the `--restore-mouse-profiles` option added.
- **`g512-control`:** the Mouse section gets the new rows, plus a 2 s debounce for profile edits.
- **Not in scope:**
  - button remapping (the next change; it reuses the profile writer);
  - the profiles' lighting and other fields, which are kept unchanged;
  - host mode;
  - DPI in the Quick Settings tile.
