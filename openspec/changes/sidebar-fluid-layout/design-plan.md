# Design plan: Settings-style sidebar, fluid pages and the new mouse art

Planned in Penpot before any code (file "G512 Control"). Follows `split-keyboard-mouse-views`, whose branch this one is stacked on.

## 1. Goal

In the user's words: "update the mouse to these icons, and update the menu to the left side instead of the top, and do a more responsive or fluid layout".

Decisions, all made 2026-10-28; the critique boards are on Penpot `06 Critique · Sidebar v2`, exports in `evidence/design/critique/`:
- **Navigation (option A, approved):** a sidebar like GNOME Settings, with one section per device. A section appears only once its device is detected.
  - Keyboard: Lighting, Key Maps, Games.
  - Mouse: Lighting, DPI & Profiles, Buttons.

  This replaces the earlier "two rows plus a bottom bar when narrow", which wasted space.
- **Narrow window:** the sidebar list becomes its own page, and choosing a row slides that page in with a Back button. There's no bottom bar any more. The list page shows a live drawing card under each section header.
- **Fluid pages:** a page's drawing card fills the height left in the window, so no window ends in an empty band. The drawing scales to fit both the width and the height. On wide windows the settings groups sit in two columns, and the mouse button labels sit beside the drawing.
- **Mouse art:** the user's new SVGs, lit by layer.
  - No glow and no G cut; the logo is solid.
  - The outlines use the drawing colour. The `DPI_LEDs` and `Logo` layers use the live LED colour, and go dark when the lights are off.
  - The top and side views are stacked, top above side, in a 290 × 420 unit area.

## 2. Who uses it, and on what

The owner on GNOME 50 (see `PRODUCT.md`), and later others. Window sizes to design for:
- narrow: 360–550 sp (half a laptop screen, phone-sized);
- default: 860×640;
- wide: maximised on 1920×1080, and half of a 2560 screen (1280).

The system dark style is the main one. Light gets one check board, because the app follows the system style.

## 3. Inventory

| # | Screen or state | New / existing | Built from |
|---|---|---|---|
| 1 | **Sidebar:** the app title, then device sections.<br>• Each section has a header (device name), one-line rows (icon + page name) and a divider.<br>• A section exists only while its device is detected. | New | `window.rs` |
| 2 | **Keyboard › Lighting:** the drawing card fills the spare height, with the hint "Click the keyboard to edit Key Maps".<br>• Wide: Lighting beside the Keyboard info group.<br>• Narrow: the info group sits under Lighting. | New (regroups) | `window.rs`, `keyboard_art.rs` |
| 3 | **Keyboard › Key Maps:** the per-key editor as a normal page, no longer a page pushed on top | Changed | `editor.rs` |
| 4 | **Keyboard › Games:** the gaming layout switch and its GameMode note | New page (regroups) | `window.rs` |
| 5 | **Mouse › Lighting:** the mouse drawing card, with Match Keyboard and the effect rows | New page (regroups) | `mouse_section.rs` |
| 6 | **Mouse › DPI & Profiles:** profile, report rate and DPI steps | New page (regroups) | `profile_section.rs` |
| 7 | **Mouse › Buttons:** the stacked drawing filling the height, with the 11 labels in two columns beside it | New layout | `mouse_art.rs`, `callouts.rs` |
| 8 | **Narrow list page:** the sections with drawing cards; the mouse card takes the rest of the height | New | `window.rs` |
| 9 | **Narrow page with Back** (any page) | New | — |
| 10 | **A device unplugged:** its section disappears. If the open page was in it, the app shows the other device's first page. If neither device is there, the content area shows the not-connected status page. | Changed | `window.rs` |
| 11 | **The new G502 art:** stacked, lit and Off, with and without badges | Changed | `mouse_ink.rs`, `mouse_art.rs` |
| 12 | Change dialog, and the service-missing page | Existing, unchanged | — |

## 4. Existing pages to match

The current app (`worktree-keyboard-mouse-views-7d1c` at `e67c081`):
- screenshots in `../split-keyboard-mouse-views/evidence/app/`;
- Penpot `05 Screens`: the "After" boards, which become this change's "Before".

## 5. Design system

**Tokens added (approved 2026-10-28):**
- `adwaita-dark`, sidebar surfaces:
  - `color.bg.sidebar` and `color.bg.sidebar-backdrop`;
  - `color.border.sidebar`.
- `adwaita-dark`, sidebar rows:
  - `color.sidebar.row-selected` and `color.sidebar.row-selected-ring`;
  - `color.sidebar.row-hover` and `color.sidebar.row-pressed`.
- A new `adwaita-light` set for the light check board, inactive by default.

**Tokens added (approved 2026-10-28, light check board):**
- `color.bg.illustration`: `#1D1D20` in `adwaita-dark` (today's `art::CARD`), `#F6F6F7` in `adwaita-light`.
- `color.border.illustration`: transparent in `adwaita-dark`, `rgba(0, 0, 6, 0.07)` in `adwaita-light`.

In light style the drawing cards are light, like the illustration cards in GNOME Settings › Mouse & Touchpad. In code, `art::CARD` follows the style, and `drawing_colour` lifts LED colours against whichever card is shown, so pale colours darken on the light card.

**Components on `02 Components`:**
- "Sidebar row" has the states Default, Hover, Pressed, Selected and Device missing. It changes to one line (icon + label) to match option A.
- New: "Sidebar section header".
- The G502 drawing is rebuilt from the new SVGs.
- The view switcher is retired.

## 6. Board plan

One checkpoint each, in this order:
1. `02 Components`:
   - the one-line Sidebar row and the section header;
   - the G502 drawing: stacked, lit, Off, and without badges.
2. `05 Screens`, After v2. Each board is built at 1280 and narrow; the critique boards for Keyboard › Lighting and Mouse › Buttons carry over:
   - Keyboard › Key Maps;
   - Keyboard › Games;
   - Mouse › Lighting;
   - Mouse › DPI & Profiles;
   - one device unplugged (its section gone), and neither device present;
   - one light-style board.
3. `03 Storyboard`: the same window at narrow, 860 and 1280, showing where the sidebar, columns and labels go at each size.
4. `00 Brief`: this change's section, written last from the approved boards.

There's no Architecture page. In code this is:
- `AdwNavigationSplitView`, which collapses to a navigation stack with Back when narrow;
- a sidebar `GtkListBox` with the `.navigation-sidebar` style and per-device sections;
- `AdwMultiLayoutView` for one or two columns;
- size-aware drawing areas that fit both width and height;
- the SVG reader sorting paths by group name.

## 7. Breakpoints (refined on the storyboard)

| Window width | Sidebar | Settings | Mouse button labels |
|---|---|---|---|
| < 550 sp | collapsed: the list is a page, other pages open with Back | one column | below the drawing, one or two columns |
| 550 – ~1000 sp | shown | one column | below the drawing, three columns |
| ≥ ~1000 sp | shown | two columns | beside the drawing, two columns |

## 8. Decisions from the boards (all 2026-10-28)

- **Default window size: 860 × 780** (was 860 × 640). At 640, Keyboard › Lighting needs about 760 px and scrolls; at 780 every page opens without scrolling.
- **Key Maps:** a normal sidebar page; white outline on selected keys; the grid card fills the spare height; narrow keeps 34 px keys and scrolls sideways.
- **Games:** the Gaming map drawing fills the spare height. Groups: "Games" (today's switch and GameMode note) and "Now" (game running or not, from `game_active`, plus "Edit the Gaming Map ›", which opens Key Maps on Gaming). The banner stays.
- **Mouse › Lighting:** the stacked drawing (no badges) in a full-height card beside the groups when wide, above them when narrow. Groups and copy unchanged.
- **Mouse › DPI & Profiles:** the profile group splits into "Profile" ("Stored on the mouse") and "DPI Steps" ("The DPI buttons cycle through these steps. The selected one is the default."). The drawing shows badges 6–9 only, with a caption built from each button's current action (factory: next profile, DPI up, DPI down, DPI shift). Narrow: a short card, drawing beside a four-line legend.
- **Unplugged:** the section leaves the sidebar; if its page was open, the app shows the other device's first page with a "Mouse disconnected" / "Keyboard disconnected" toast. With no devices, the sidebar hides and the window shows "No Devices Connected" ("Plug in the Logitech G512 keyboard or the G502 HERO mouse. Each one appears in the sidebar when it connects, and its saved lighting is applied."). Today's "Keyboard Not Connected" and "Mouse Not Connected" pages go. A device that is present but has no access or is unsupported keeps its section and shows today's status page there.
- **Light style:** drawing cards use `color.bg.illustration` / `color.border.illustration`, light in the light style like GNOME Settings' illustrations.

## 9. Boards

Penpot file "G512 Control". Exports in `evidence/design/`.

| Board | Page | Id | Export |
|---|---|---|---|
| Settings-style · Keyboard › Lighting · 1280 | 06 Critique · Sidebar v2 | `3a3cfffb-d449-803a-8008-dc4ec4aea434` | `critique/` |
| Settings-style · Mouse › Buttons · 1280 | 06 Critique · Sidebar v2 | `3a3cfffb-d449-803a-8008-dc4ee7c0c5b2` | `critique/` |
| Settings-style · narrow · list | 06 Critique · Sidebar v2 | `3a3cfffb-d449-803a-8008-dc4f1487fce1` | `critique/` |
| Settings-style · narrow · Keyboard › Lighting | 06 Critique · Sidebar v2 | `3a3cfffb-d449-803a-8008-dc4f2b1a20e2` | `critique/` |
| Sidebar row · states | 02 Components | `3a3cfffb-d449-803a-8008-dc4c4d4e4978` | `02-sidebar-row.png` |
| G502 drawing | 02 Components | `3a3cfffb-d449-803a-8008-dc53dc4ec2c8` | `02-g502-drawing.png` |
| Keyboard › Key Maps · 1280 / 360 | 05 Screens | `…dc54bba8f0f8` / `…dc55514da7bb` | `05-keymaps-1280.png`, `05-keymaps-360.png` |
| Keyboard › Games · 1280 / 360 | 05 Screens | `…dc7b4218bf6e` / `…dc7c31801974` | `05-games-1280.png`, `05-games-360.png` |
| Mouse › Lighting · 1280 / 360 | 05 Screens | `…dc89fa807f17` / `…dc8a14e552b0` | `05-mouse-lighting-1280.png`, `05-mouse-lighting-360.png` |
| Mouse › DPI & Profiles · 1280 / 360 | 05 Screens | `…dc8b58225448` / `…dc8b9a7be516` | `05-dpi-profiles-1280.png`, `05-dpi-profiles-360.png` |
| Unplugged · mouse gone 1280 / no devices 1280 / narrow list | 05 Screens | `…dc8d76324c80` / `…dc8d446e0b71` / `…dc8d46747b60` | `05-unplugged-mouse-1280.png`, `05-unplugged-none-1280.png`, `05-unplugged-mouse-360.png` |
| Light style · Keyboard › Lighting · 1280 | 05 Screens | `…dc8e733f7256` | `05-light-1280.png` |
| Storyboard rows 1–3 | 03 Storyboard | `…dc90b281b04a` / `…dc90ddaa38c6` / `…dc9110b769b6` | `03-storyboard-narrow.png`, `03-storyboard-keyboard.png`, `03-storyboard-mouse.png` |
| Key Maps at 688 × 795 · options (hands-on pass): Current / A shrink / B sidebar folds / C sections / D overview / B + A | 05 Screens, y = 10340 | `…dcdf388a0161` / `…dce0513587be` / `…dce12a8ccec1` / `…dce1f663e103` / `…dce2446865b8` / `…dce18c7acd58` | `05-keymaps-688-current.png`, `-a-shrink`, `-b-sidebar-folds`, `-c-sections`, `-d-overview`, `-b-plus-a` |
| Key Maps · 860 · B + A (chosen 2026-10-29) | 05 Screens, y = 10340 | `…dce402a34f42` | `05-keymaps-860-b-plus-a.png` |
| Key Maps · 860 · B + A · Sidebar open | 05 Screens, y = 10340 | `…dce4b6f45f11` | `05-keymaps-860-sidebar-open.png` |

Ids starting `…` share the prefix `3a3cfffb-d449-803a-8008-`.
