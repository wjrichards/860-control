# Proposal

## Why

The user wants to remap the G502 HERO's buttons. They chose option A: each button in a profile can be set to a mouse button, a keyboard key or shortcut, a media key, a mouse function, or nothing. The setting is written into the onboard profiles, so the mouse keeps it without the service. Macros (option B) are the next change.

Spike 8 (`evidence/spike8.md`) showed:
- **The button table:** 11 four-byte entries from byte `0x20` of each profile, and which physical button each entry belongs to.
- **Keyboard entries work:** `80 02 <modifiers> <HID usage>` typed letters as soon as the profile was active.
- **A gap:** the mouse's own G9 button switches profiles without g512d noticing.

The user decided:
- shortcuts are set by pressing them;
- left click can never be remapped;
- the service checks the active profile every few seconds.

## What Changes

- **The profile module reads and edits the button table.** Each entry decodes to:
  - a mouse button;
  - a key with modifiers;
  - a media (consumer) key;
  - a mouse function (DPI up/down/cycle/shift, profile cycle, wheel tilt, G-shift);
  - disabled;
  - something left as it is (macros and unknown entries).

  Edits rewrite only the entries that change, plus the CRC, through the existing verified writer.
- **The service:**
  - offers each profile's buttons;
  - adds `SetMouseButton(profile, button, action)`, which refuses any change to left click;
  - re-reads the active profile every 2 s while the mouse is connected. When it changes (G9), front ends are told and the mouse lighting is re-applied.

  The once-only backup still comes first.
- **The app gets a Mouse Buttons group**, one row per button with its current action:
  - left click is shown but can't be changed;
  - every other button has a Change dialog: mouse button, keyboard shortcut (captured by pressing it), media key, mouse function, or disabled.

## Capabilities

### New Capabilities

None. Each piece extends an existing capability.

### Modified Capabilities

- `device-protocol`: adds the onboard-profile button table.
- `lighting-service`: adds mouse buttons and keeping the active profile current; changes the D-Bus interface requirement.
- `lighting-app`: adds the Mouse Buttons group.

## Impact

- **`g512-hidpp`:** `profiles` gets `ButtonAction`, `ProfileSector::buttons` and `with_button`. The fake mouse already holds real button tables.
- **`g512d`:**
  - D-Bus gets `MouseButtons` and `SetMouseButton`;
  - a 2 s active-profile watch is added.
- **`g512-control`:**
  - a Mouse Buttons group and a button dialog;
  - a GDK-key-to-HID-usage table, unit-tested.
- **Not in scope:**
  - macros (next change);
  - G-shift second functions;
  - per-application mappings;
  - restoring a single button to its factory default (the restore command covers whole profiles).
