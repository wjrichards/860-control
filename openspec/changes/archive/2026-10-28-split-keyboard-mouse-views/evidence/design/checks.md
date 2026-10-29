# Design checks (2026-10-27)

## Contrast (WCAG 2.2 AA)

Alpha colours are flattened onto their real background before measuring (`target/contrast.py`, `target/contrast2.py` in the worktree).

| Pair | Ratio | Needs | Result |
|---|---|---|---|
| Body text, white on card | 13.51 | 4.5 | pass |
| Secondary 55% on card (row subtitles) | 5.24 | 4.5 | pass |
| Secondary 55% on window (hints, descriptions) | 5.78 | 4.5 | pass |
| Secondary 55% on view (drawing hint) | 5.91 | 4.5 | pass |
| Badge number, white on accent #2190A4 | 3.76 | 4.5 | **fail** |
| Apply button text, white on accent | 3.76 | 4.5 | **fail** (libadwaita's own suggested-action style) |
| Fixed chip title (chip at 60% opacity) | 6.05 | 4.5 | pass |
| Fixed chip action (55% text in a 60% chip) | 2.84 | 4.5 | **fail** |
| Lit key legend, LED green on its cap | 4.17 | 4.5 | **fail** (and worse for dark LED colours) |
| Unlit key legend, white 35% | 3.14 | 4.5 | **fail** |
| Switch on vs card (non-text) | 3.60 | 3.0 | pass |
| Selected tab, white 10% over header (non-text) | 1.37 | 3.0 | **fail** (libadwaita's own view-switcher style) |
| Focus ring, accent on window (non-text) | 4.22 | 3.0 | pass |
| Badge disc vs mouse body (non-text) | 4.17 | 3.0 | pass |

## Target size (2.5.8)

Buttons on the scaled drawings are smaller than 24×24 px (DPI Up is about 10×18 px in the compact drawing). Each has an equivalent control of at least 24 px: its legend chip (44 px tall) and its row's Change button. That meets 2.5.8 under the equivalent-control exception. The badges are labels, not separate targets.

## Tokens

On `05 Screens`, 141 hard-coded fills matched a token exactly and were bound (no visual change):
- 129 texts → `color.text.primary`;
- 12 switch, slider and button fills → `color.accent`.

Left unbound on purpose:
- `#26A269`: the live LED colour (data, not a token).
- `#E5A50A`, `#C0C0C8`: board annotations, not UI.

Waiting for new tokens (proposed, not created):

| Proposed token | Value | Used by |
|---|---|---|
| `color.separator` | rgba(255, 255, 255, 0.07) | 42 row separators |
| `color.bg.button` | rgba(255, 255, 255, 0.10) | 23 Change and colour buttons; today they only match `color.border.subtle` by value |
| `color.slider.track` | rgba(255, 255, 255, 0.15) | 8 slider tracks and off switches |
| `color.slider.knob` | #DEDDDA | 6 slider knobs |
| `color.accent.strong` | #1D8092 (white text 4.61:1) | badges |

## Applied (approved 2026-10-27)

- **Tokens added** to `adwaita-dark`: `color.separator`, `color.bg.button`, `color.slider.track`, `color.slider.knob` and `color.accent-strong`. "accent-strong" uses a hyphen, because a token name can't extend another token's name (`color.accent`). On `05 Screens` they're bound to 42 separators, 23 buttons, 8 tracks and off switches, and 6 knobs, each checked one by one.
- **Badges:** all 33 badge discs (compact and narrow mouse drawings) use `color.accent-strong`; white numbers are at 4.61:1.
- **Fixed chip:** the main is fully opaque with a dashed border, and the 5 Left Click instances read "Left click · fixed". The action text is at 5.24:1.
- **Key legends:** in the three Keyboard drawing variants, 107 lit keys use white at 85% (10.21:1) and 205 unlit keys white at 50% (4.86:1).
- The screens updated through their instances. `05-after-keyboard.png`, `05-after-mouse.png` and `05-narrow-mouse.png` were exported again.

Still open by choice: the Apply button and the selected view-switcher tab keep libadwaita's styles.

## Illustrations (art direction, 2026-10-28)

| Pair (non-text, needs 3:1) | Ratio | Result |
|---|---|---|
| Lit outlines and tiles, LED green #26A269 on the card | 5.17 | pass |
| Off outlines, white 35% on the card | 3.21 | pass |
| A darker LED colour, deep blue #1C71D8 | 3.53 | pass |
| Unlit key outlines in the effect animations (green at about 25%) | 1.44 | decorative, shown only between lit frames |

**Rule for the build:** the drawings take the live LED colour, which can be any colour, including near-black. When it gives less than 3:1 on the card, the app lightens it toward white, keeping the hue, until it reaches 3:1. The device keeps the real colour; only the drawing changes.

## Proposed fixes (as approved)

1. **Badges** fill with `color.accent.strong` (#1D8092): white numbers reach 4.61:1.
2. **Fixed chip:** keep it fully opaque, and show "fixed" with a dashed border and the action text "Left click · fixed". The action text is then 5.24:1.
3. **Key legends:** always light, whatever the LED colour. Lit keys use white at 85% on the tinted cap (10.21:1); unlit keys use white at 50% (4.86:1). The cap tint and outline still show the LED colour, which also keeps dark colours such as deep blue readable.
4. **Accepted as system styling, not changed:** the Apply button and the selected view-switcher tab use libadwaita's own styles, which the app inherits. They follow the user's accent and contrast settings (GNOME's High Contrast mode raises both).
