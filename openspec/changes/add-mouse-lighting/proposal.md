# Proposal

## Why

The user has a Logitech G502 HERO mouse next to the G512 and wants it controlled from the same app: its lighting first, then DPI and report rate, then button remapping. They chose option A: lighting first, in `g512d`, with the mouse left in its onboard-profile mode. DPI, report rate and button remapping come in later changes, written to onboard profiles behind a verified backup.

Spike 6 (`evidence/mouse.md`) showed:
- the mouse uses the same lighting feature as the keyboard (`0x8070`, the same effect IDs);
- it has two zones: zone 0 is the logo, zone 1 the DPI strip;
- it offers Off, Static, Breathe and Cycle;
- lighting set from the computer holds while the mouse stays in onboard mode, even after a DPI press.

It also saved a CRC-checked backup of the mouse's profile memory.

## What Changes

- **The protocol crate recognises the G502 HERO** (`046d:c08b`, the HID++ interface on vendor page `0xFF00` with report `0x11`) as well as the G512. Other devices are still ignored. The feature allowlist is unchanged: the mouse only needs the identity features and `0x8070`.
- **Mouse lighting in the service**, with one setting for both zones:
  - **Match the keyboard** (the default). The mouse shows the keyboard's effect, colour, speed and brightness. Where the mouse has no equivalent, it uses the nearest effect:
    - Wave becomes Cycle at the same speed;
    - Ripple becomes Static in Ripple's colour;
    - the Custom map and the Gaming map become Static in the map's colour for W, or Off if W is off.
  - **Its own setting:** an effect (Off, Static, Breathe or Cycle), a colour, a speed and a brightness. Choosing one turns matching off.
  - **Lights off** darkens the mouse too. Choosing mouse lighting turns the lights back on.
  - The mouse is re-applied when it's plugged in, when the computer resumes, and when access arrives at login, independently of the keyboard.
  - New D-Bus members: `MouseState`, `MouseName`, `MouseEffects`, `MouseMatch`, `MouseLighting`, `SetMouseMatch(b)` and `SetMouseLighting(s(yyy)yy)`.
- **A Mouse section in the app:** the mouse's status, a "Match Keyboard" switch, and, when it's off, the effect, colour, speed and brightness controls.
- **The udev rule** grants the active session access to the mouse's hidraw nodes as well. This needs one `sudo` reinstall.

## Capabilities

### New Capabilities

None. Each piece extends an existing capability.

### Modified Capabilities

- `device-protocol`: device identification covers the G502 HERO, and effects apply to every zone a device reports.
- `lighting-service`: adds mouse lighting, and changes the D-Bus interface and unprivileged-access requirements.
- `lighting-app`: adds the Mouse section.

## Impact

- **`g512-hidpp`:**
  - `discovery.rs` gets a table of supported models (product ID, usage page, report IDs);
  - `effects.rs` applies to every zone;
  - the fake device gets an optional second zone.
- **`g512d`:**
  - a mouse connection and mouse settings beside the keyboard's, in the controller;
  - `store.rs` saves the mouse settings, still reading older files;
  - `dbus.rs` gets the new members;
  - `events.rs`: hotplug, resume and the login retry cover the mouse.
- **`g512-control`:** the Mouse section, reading the new properties as optional.
- **`packaging/70-g512-control.rules`:** a line for `046d:c08b`.
- **Not in scope:**
  - DPI, report rate, button remapping and any onboard-profile writes (later changes);
  - separate logo and DPI-strip settings;
  - mouse controls in the Quick Settings tile, though its Lights toggle covers the mouse;
  - other mice.
