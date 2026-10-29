# Design audit: sidebar-fluid-layout boards (2026-10-28)

Scope: the After v2 boards on `05 Screens` (Key Maps, Games, Mouse Lighting, DPI & Profiles, Unplugged, Light style), the storyboard and the brief.

## Accessibility (WCAG 2.2 AA)

Contrast ratios computed from the board colours.

| # | Severity | Finding | Ratio | Resolution |
|---|---|---|---|---|
| 1 | Medium | "Paint Selected": white text on the accent (#2190A4) | 3.76:1 (needs 4.5) | Platform: libadwaita draws suggested buttons from the system accent. Not changed in the app. |
| 2 | Medium | Light style: dimmed text (subtitles, hints) on white | 3.23:1 | Platform: Adwaita's `.dim-label`. Not changed. |
| 3 | Medium | Key Maps: an unlit key's edge against the card | 1.49:1 (needs 3) | The keys are GTK buttons with labels (5.09:1), so each key is identifiable by its label. Accepted as the platform button style. |
| 4 | Medium | Key Maps selection outline: the boards use white, the app uses `@accent_color` | white on a red key 4.83:1; the accent disappears on teal keys | Approved 2026-10-28. Task: change the `.keygrid button.key:checked` outline to white with a thin dark inner ring, so it shows on pale and dark keys alike. |
| 5 | Low | Slider track and linked-toggle selected state | 1.73:1, 1.65:1 | Adwaita defaults. |
| 6 | Low | Unlit keys in the Gaming map drawing (25% outline) | ~1.4:1 | The drawing is a picture; its accessible name already says which map is shown (`keyboard_art.rs`). |
| 7 | Low | DPI & Profiles narrow: badges on the small drawing are unreadable | — | The legend beside the drawing carries the same text. |
| 8 | Low | Light style: green LED (#26A269) on the light card | 3.01:1 | Passes 3:1 just; `drawing_colour` lifts colours against the light card. |

Passing: primary text everywhere; dimmed text in dark style (4.81–5.99:1); banner text (10.68:1); white badge numbers (4.61:1); the accent switch against rows (3.6:1).
Targets: sidebar rows 40 px, spin and delete buttons 34 px, keys 34 px narrow and 36 px wide; all at least 24 px.
Names: drawings keep their accessible labels; the DPI caption and the "Now" rows are plain text; toasts are announced by `AdwToast`.
Motion: effect animations keep the existing reduced-motion check (`art_area.rs`).

## Tokens

- Bound on the dark After v2 boards: 465 fills to `adwaita-dark` tokens (text primary and secondary, accent, window, sidebar, card, button, sidebar row selected, slider track, illustration). The look is unchanged; the composited colours match their translucent tokens.
- Left raw on purpose:
  - LED sample colours on the drawings and key swatches (they are data, not UI colour);
  - the light-style board (the `adwaita-light` set is inactive, so binding would show dark values).
- Gaps, Low:
  - Switch and slider knobs are white on the boards; the token `color.slider.knob` is #DEDDDA. Check against the libadwaita 1.6 stylesheet at build time.
  - Added (approved 2026-10-28): `color.bg.banner` (dark #224349, light #B9DAE1) and `color.toggle.checked` (dark #5A5A5E, light #CDCDCF), bound on the dark boards.
  - Spacing, radius and type are not bound on these boards. The app takes them from libadwaita, so these are mockup values only.
