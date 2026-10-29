# Design plan: Keyboard and Mouse views

Planned in Penpot before any code (approved 2026-10-27).

## 1. Goal

In the user's words: "a separated section for mouse and keyboard, and drawing of mouse and keyboard layout".

## 2. Who uses it, and on what

The user, in the GNOME app on the desktop: the default window is 860×640, with the dark style and teal accent. The window should still work narrow, where libadwaita moves the view switcher to a bar at the bottom.

## 3. Inventory

| # | Screen or state | New / existing | Built from |
|---|---|---|---|
| 1 | **Keyboard tab:** `AdwViewSwitcher` (Keyboard \| Mouse) in the header, then the keyboard drawing in its live colours (clicking it opens Key Maps), then Device, Lighting, the Key Maps row and Games | New (regroups existing) | `window.rs`, `layout.rs` |
| 2 | **Mouse tab:** a drawing of the G502 HERO from the top and the left side, with the logo and DPI strip in their colours and a callout per button showing its action (clicking a button opens its Change dialog), then Lighting, Profile and Buttons | New (regroups existing) | `mouse_section.rs`, `profile_section.rs`, `button_section.rs` |
| 3 | **Mouse tab, mouse missing:** not connected, no access, or profiles unreadable (lighting only) | New | — |
| 4 | **Keyboard tab, keyboard missing:** not connected or no access, while the Mouse tab keeps working | New | — |
| 5 | **Narrow window:** the switcher as a bottom bar | New | — |
| 6 | Key Maps editor page | Existing, unchanged | `editor.rs` |
| 7 | Change dialog | Existing, unchanged | `button_section.rs` |
| 8 | Service missing and no session bus status pages | Existing | `window.rs` |

## 4. Existing pages to match

The current window is one long `AdwPreferencesPage` with the groups Keyboard, Lighting, Games, Mouse, Mouse Profile and Mouse Buttons. It is rebuilt faithfully as a "Before" board, from the source and a full screenshot of the running app, before any "After" board.

## 5. Design system

Checked in Penpot first (`high_level_overview`, token overview). Likely new tokens:
- Adwaita dark surfaces: window, card, row separators;
- the teal accent;
- the 12 px card radius;
- text styles for the row title and subtitle.

Each new token is proposed for approval before it's used.

## 6. Board plan

One checkpoint each, in this order:
1. `00 Brief`: goal, inventory and open questions.
2. `01 Foundations`: token changes, once approved.
3. `02 Components`, one at a time:
   - the keyboard drawing in live colours;
   - the G502 drawing (a clean, stylised outline, top and left side) with a button callout component;
   - the view switcher.
4. `03 Storyboard`:
   - switch tabs;
   - click a key, which opens Key Maps;
   - click a mouse button, which opens Change;
   - press G9 on the mouse, and the Mouse tab follows the profile.
5. `05 Screens`, Before | After:
   - the Keyboard tab;
   - the Mouse tab;
   - each device missing;
   - the narrow window.

There's no Architecture page: this is a regrouping of existing widgets, plus the service change in section 7.

## 7. Decisions

- **Structure:** a view switcher in the header with Keyboard and Mouse tabs. Macros (change 7) can become a section of the Mouse tab later.
- **Drawings:** live and clickable. The keyboard shows its current colours, and clicking it opens Key Maps. The mouse shows the logo and DPI-strip colours and each button's action, and clicking a button opens its Change dialog. The rows stay below for keyboard and screen-reader use.
- **One device connected:** each tab shows its own state. The Mouse tab works without the keyboard, and the other way round. This needs a service change: today `MouseState` and the other mouse properties are only read when the keyboard is connected, and a missing keyboard replaces the whole window with a status page.
- **Mouse drawing:** a clean, stylised vector outline rather than a traced photo.

Decided while drawing the boards (2026-10-27):
- **Keyboard drawing:** shown full size (662×214), like the Key Maps editor (option a). The 640 px fold then falls at the Colour row.
- **Mouse drawing:**
  - redrawn from the user's two product photos: faceted body, DPI panel with three LEDs, wheel-mode toggle and G9 behind the wheel, thumb wing, right fin;
  - the Mouse tab uses the **compact** component (760×340): the top view at 75% and the side view at 60%, numbered teal badges 1–11 on the buttons, and a two-column legend of clickable callout chips. The full drawing with leader lines stays as a component for larger uses.
- **Key Maps row:** kept as "Edit Key Maps…" for keyboard users, alongside the clickable drawing.
- **Profiles unreadable:** the Mouse tab shows the drawing (lights only, no badges or chips), Device and Lighting. Profile and Buttons stay hidden, as today.

Art direction (2026-10-27/28):
- **Reference:** GNOME Settings' Scroll Direction illustration, a one-colour drawing on a dark inset card with motion that explains. Here the colour is the device's live LED colour (Off = dim white).
- **Keyboard illustration:** generated from `layout.rs`, with lit keys as solid rounded tiles, unlit keys as rounded outlines, and a 3 px rounded case.
- **Mouse illustration:** the user's own ink drawings, `g502-top-ink.svg` and `g502-side-ink.svg` (copied to `evidence/design/mouse-art/`). Their layers are used as they are:
  - `Layer_5` outlines take the LED colour;
  - `DPI_LEDs` and `Logo` are solid LED colour;
  - `Glow` is a halo at 20% that the motion pulses.

  A G mark is cut into the top-view logo disc and the side logo.
- **Motion:** the `04 Motion` board (spec plus keyframe strips) and `evidence/design/motion-preview/motion-preview.webm`, made by `render.py` from the same key table and mouse SVG.
- **Contrast rule:** a LED colour under 3:1 on the card is lightened, keeping its hue, for the drawing only (`evidence/design/checks.md`).

## 8. Open questions

- The legend chips and the Buttons rows show the same actions. The rows stay for keyboard and screen-reader use; collapsing them into an "All buttons" expander is a possible later change.

## 8a. Findings

- **The No Access page breaks the keyboard rule.** `window.rs:439` takes only the last rule line of `70-g512-control.rules`, which is the mouse line (c08b), and pipes it into `sudo tee`, which overwrites the file. Following the keyboard's "No Access" instructions would drop the keyboard rule. The status pages in this change show a command that writes both lines (board "After · Mouse tab, no access"); the fix belongs in this change.

## Boards (Penpot file "G512 Control")

| Page | Board | Export (`evidence/design/`) |
|---|---|---|
| 00 Brief | Brief | — |
| 01 Foundations | Tokens · adwaita-dark (26 tokens) | — |
| 02 Components | Keyboard illustration (Lit, Custom map, Off), Mouse illustration (Lit, Off; user's ink SVGs), Keyboard card (Default, Hover), Button callout (Default, Hover, Fixed), Mouse drawing compact (Effect, Lights off), Keyboard drawing narrow, Mouse drawing narrow, View switcher (Wide/Narrow × Keyboard/Mouse). The earlier keycap and faceted drawings were removed on 2026-10-28. | 02-keyboard-illustration.png, 02-mouse-illustration.png |
| 04 Motion | Motion spec and keyframe strips (Ripple, Wave, Breathe, G9 hover, confirm) | 04-motion.png, motion-preview/motion-preview.webm |
| 03 Storyboard | Storyboard: four strips | 03-storyboard.png |
| 05 Screens | Before · today; After · Keyboard tab; After · Mouse tab; keyboard missing; mouse missing; mouse no access; narrow Keyboard tab; narrow Mouse tab | 05-before.png, 05-after-keyboard.png, 05-after-mouse.png, 05-keyboard-missing.png, 05-mouse-missing.png, 05-mouse-no-access.png, 05-narrow-keyboard.png, 05-narrow-mouse.png |

## 9. Depends on

- `add-mouse-buttons` (PR #11): the Mouse tab includes its Buttons group. This change is built on main after that merge.
