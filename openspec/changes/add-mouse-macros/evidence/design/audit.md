# Design audit: mouse macros

Run 2026-10-29 on the Penpot file "G512 Control", over every new board: the 14 screens and dialogs on 05 Screens (y 11400 to 14200) and the two component boards on 02 Components. The Buttons After board is a patched app capture and was checked by eye instead. The audit was a script run through the Penpot plugin: it walked each board, composited every text over its real background (card, dialog, banner, overlays), and listed fills, strokes, radii and spacing that aren't bound to a token.

## Accessibility (WCAG 2.1 AA)

| Check | Result |
|---|---|
| Text contrast, 409 texts | All pass. Lowest: 4.61:1, white "New Macro" on `color.accent-strong` (empty state). |
| Dimmed text (recording, Save off, Delete off while unplugged, Apply off in Before) | 23 texts at 50% opacity; disabled controls are exempt from 1.4.3. |
| Banner text (white on `color.bg.banner`) | About 10:1. |
| Delete button (white on `color.destructive`) | About 5.9:1. |
| Accent button text | Fixed during the build: `color.accent` gave about 3.8:1, so New Macro uses `color.accent-strong` (4.61:1). |

**Medium, not fixed: the focus ring is under 3:1 against a card.** `color.focus-ring` (accent at 50%) gives about 1.9:1 on `color.bg.card`, below the 3:1 that 1.4.11 asks for a focus indicator. It matches libadwaita's own ring, and GTK draws the real ring, so the design can't change what users see without app CSS. Options: leave it as GNOME ships it, or set the token (and app CSS) to opaque accent, which gives about 3.6:1.

Other checks:
- Every control has a visible text label. The icon-only controls (drag handle, delete) need accessible names in code: "Move step", "Delete step".
- Recording uses the window's key events only, so nothing is captured outside the app.
- State is never shown by colour alone: Recording also says "Stop · 0:04", the selected macro radio is also the picker's value, and errors are in words.

## Tokens

- **Colours:** no unbound fill or stroke in the new work. Exceptions:
  - the drag and popover shadows use the value of `color.shadow.drag`, because Penpot can't bind a shadow colour;
  - the Buttons After patch uses `#333336` to match the app capture it sits on;
  - the radio ring in the Change dialog boards uses `color.border.keycap`, the closest value.
- **Spacing and radius:** 134 raw values that already had a token were bound in this pass (gaps 8, 12 and 24, padding 12, 24 and 32, radii 6 and 12).
- **Left raw, no token exists:**
  - row padding 14 and 10;
  - label gap 2;
  - header button padding 6;
  - meter radius 3 (half its height).
  These match the existing approved boards (DPI & Profiles rows use 14 and 8 too). Adding `spacing.2`, `spacing.6`, `spacing.10` and `spacing.14` would cover them; that's a foundations change for later, not this feature.
- **Inherited from the v2 screens:** the sidebar and header bars cloned from the approved boards keep their own raw values (sidebar stroke `#1a1a1d`, gaps 2 and 6, padding 4 and 6). Not changed here.

## New tokens this feature added

`color.focus-ring`, `color.bg.keycap`, `color.border.keycap`, `color.shadow.drag`, in both adwaita-dark and adwaita-light (approved 2026-10-29).
