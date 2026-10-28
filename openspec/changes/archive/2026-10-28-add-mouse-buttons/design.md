# Design

## Context

- **Profile format 2:** 11 button entries of 4 bytes from byte `0x20` (`getInfo` reports a button count of 11).
- **Spike 8**, on the user's G502 HERO:
  - **Entry order:** left, right, wheel click, back (front thumb), forward (rear thumb), sniper, DPI down, DPI up, G9, tilt right, tilt left.
  - **Keyboard entries:** `80 02 mm kk` written by us typed as expected.
- **Seen in the backup** (decoded the way libratbag does):

  | Entry | Meaning |
  |---|---|
  | `80 01 hi lo` | mouse-button bitmask |
  | `80 02 mm kk` | key `kk` with modifiers `mm` |
  | `90 cc xx xx` | special `cc` |

  Specials seen: `01` wheel left, `02` wheel right, `03` DPI up, `04` DPI down, `07` DPI shift, `0a` profile cycle, `0b` G-shift. The trailing bytes are `00 00` in profile 1 and `ff ff` elsewhere.
- **Not seen yet**, so checked on the hardware before use: consumer keys (`80 03 hi lo` in libratbag) and a "disabled" entry.
- **Change 5 left:**
  - `profiles::write_sector` with read-back;
  - the once-only backup;
  - `set_mouse_profile`;
  - re-reading every profile on `SetMouseActiveProfile`.
- **The active profile goes stale:** g512d reads it only on connect and after its own switches, but G9 switches in the firmware.

## Goals / Non-Goals

**Goals:**
- Left click can't be changed through any path.
- Entries this change doesn't understand (macros, unknown types) are shown and kept byte for byte.
- One button change is one sector write, verified.

**Non-Goals:**
- Writing macros, or editing macro memory.
- G-shift second-layer bindings.

## Decisions

### D1. `ButtonAction` in `g512_hidpp::profiles`

```rust
pub enum ButtonAction {
    Mouse(u16),                      // 80 01 hi lo: button bitmask (1 left, 2 right, 4 middle, 8 back, 16 forward)
    Key { modifiers: u8, usage: u8 }, // 80 02 mm kk: HID modifier bits (Ctrl 1, Shift 2, Alt 4, Super 8) and usage
    Media(u16),                      // 80 03 hi lo: consumer usage (checked on hardware first)
    Special(Special),                // 90 cc 00 00
    Disabled,                        // checked on hardware first; see D2
    Other([u8; 4]),                  // anything else: shown, never rewritten
}
pub enum Special { WheelLeft = 1, WheelRight = 2, DpiUp = 3, DpiDown = 4, DpiCycle = 5, DpiShift = 7, ProfileCycle = 10, GShift = 11 }
```

- `ProfileSector::buttons(count)` decodes the entries.
- `ProfileSector::with_button(i, action)` rewrites entry `i` and the CRC. It refuses entry 0 (left click) and `Other`.
- Unchanged entries keep their exact bytes, `ff ff` tails included.

### D2. Media keys and "disabled" are confirmed before they're offered

Spike 9 runs before the app offers either. On inactive profile 5, it writes:
- wheel click as `80 03 00 cd` (play/pause);
- G9 as the disabled candidate `80 01 00 00`, an empty mouse-button mask.

The user presses both, then it restores. Whatever doesn't work isn't offered; a disabled candidate that fails is dropped from the menu. The result decides the task list's Media and Disabled items.

### D3. Service

- **`MouseProfiles` gains buttons per profile.** Its signature can't change without breaking the app shipped in change 5, so there's a separate property, `MouseButtons` `a(ya(sqy))`:
  - per profile: number and entries;
  - each entry is (kind, value, modifiers);
  - kind is `mouse`, `key`, `media`, `special`, `disabled` or `other`;
  - value is the mask, usage or special code.
- **`SetMouseButton(y profile, y button, s kind, q value, y modifiers)`:**
  - validates the profile, `button` in 1..count (never 0), the kind, and the value (the special codes in D1, mask 1–16, key usage `0x04`–`0xe7`);
  - writes the backup if it's missing;
  - writes with `with_button`, verified;
  - updates the cache.

  No debounce: one choice is one write.
- **Active-profile watch:** a thread reads `getCurrentProfile` every 2 s while the mouse is connected and has profiles. On a change it:
  - updates `active`;
  - re-reads that profile's sector (a G9 switch may come after a write made elsewhere);
  - re-applies the mouse lighting;
  - emits.

  If the read fails with a lost link, the mouse is disconnected, as elsewhere.

### D4. App: the Mouse Buttons group

- **Rows:** one `AdwActionRow` per button, titled with the physical name (D1 order) and subtitled with the current action, such as "Ctrl+C", "DPI up" or "Middle click". Left click's row has no Change button.
- **The Change dialog** (`AdwDialog`) offers:
  - **Mouse button:** left, right, middle, back, forward;
  - **Keyboard shortcut:** "Set Shortcut" listens with `EventControllerKey` and shows the combination; Escape cancels;
  - **Media key:** play/pause, next, previous, volume up, volume down, mute (if D2 confirms);
  - **Mouse function:** the specials in D1;
  - **Disabled** (if D2 confirms).

  Apply calls `SetMouseButton`.
- **Key mapping:** a pure table from GDK keyval to HID usage covers letters, digits, F1–F12, Enter, Escape, Backspace, Tab, Space, punctuation, arrows, Insert/Delete/Home/End/Page Up/Down, and the modifier bits. Keys outside the table are refused in the dialog.
- **Rows are built once** and only updated, because rows added later don't reach the accessibility tree (change 5).

## Risks / Trade-offs

- **Right click can be remapped.** Left click still works, so the app stays usable, and the restore command undoes it.
- **Flash wear:** one write per choice.
- **The 2 s watch** is one 20-byte request every 2 s while the mouse is connected: negligible, but it never stops while the service runs.
- **Media and disabled entries** are decoded from libratbag's documentation until spike 9 confirms them.

## Migration Plan

Nothing stored changes. Button edits live on the mouse, behind the existing backup and restore.

## Open Questions

None. The user chose shortcut capture, a fixed left click, and the 2 s watch.
