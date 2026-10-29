# Design

## Context

The app is GTK4 and libadwaita (crates `gtk4` with `v4_14`, `adw` 0.9 with `v1_6`), running on the GNOME 50 Flatpak runtime. It talks to `g512d` over D-Bus. This change is stacked on `split-keyboard-mouse-views` (`e67c081`), which built:
- the two views (`ViewStack`, `ViewSwitcher`, `ViewSwitcherBar` with a 550 sp breakpoint in `window.rs`);
- the drawings (`keyboard_art.rs`, `mouse_art.rs`, `art_area.rs`);
- the ink parser (`mouse_ink.rs`).

The approved boards, decisions and board ids are in `design-plan.md` §8–9. Motivation is in `proposal.md`.

Constraints found in the code:
- `ArtArea` keeps an aspect ratio with height following width. It has no way to fill a height.
- `mouse_ink.rs` sorts paths by CSS class: `cls-1` glow, `cls-2` outline, `cls-3` LEDs and logo. The new SVGs have no classes. Every path has an inline `fill="#0e0e10"` and sits in a group:
  - `Glow`, which is empty;
  - `Layer_5`, the outline;
  - `DPI_LEDs`;
  - `Logo`. In the side file, the logo group also holds the small side LEDs.
- `art::CARD` is a single constant, `#1D1D20`.
- The editor marks selected keys with `outline: 3px solid @accent_color`.

## Goals / Non-Goals

**Goals:**
- Use libadwaita's own adaptive widgets, so the sidebar, the collapse to a list, Back, and keyboard navigation behave as they do in GNOME Settings.
- One drawing widget that fills width, height, or both, so every page follows the "Fluid page layout" requirement.
- Keep each section's widgets (`mouse_section.rs`, `profile_section.rs`, `button_section.rs`, `editor.rs`) and move them onto pages, without rewriting their logic.

**Non-Goals:**
- No service, D-Bus or HID++ change.
- No new effects, button actions or profile features.
- No restyling of libadwaita's own widgets beyond the key selection outline and the drawing card.

## Decisions

### Navigation: `AdwNavigationSplitView` with a sidebar `GtkListBox`
- **Structure:**
  - The sidebar page holds a `GtkListBox` with the `.navigation-sidebar` style. Each device gets a section header and three rows.
  - The content side is an `AdwNavigationPage` per sidebar row, swapped in as the selection changes.
- **Collapse:** an `AdwBreakpoint` at `max-width: 550sp` sets `collapsed`, so the list becomes its own page and pages open with Back. This replaces the `ViewSwitcherBar`.
- **Alternatives:**
  - An `AdwViewStack` with a custom sidebar: rejected, because it means reimplementing collapse and Back.
  - `GtkStackSidebar`: rejected, because it has no section headers and no way to hide a device's rows as a group.
- **Narrow list drawings:** in collapsed mode only, each section header row gets the device's `ArtArea` below it, hidden when wide. The last one is set to `vexpand`.

- **Marking the open page (changed while building):** the lists use `SelectionMode::None`, and `Ui::open` gives the open page's row a `current` style class and the accessible `selected` state. With the list's own single selection, keyboard focus and pointer crossing moved the selection to another row after start-up, so a row other than the open page's looked selected.

### Sections follow device state
- **Section visibility:** the existing state handling in `window.rs` (`show_problem`, `show_controls`) changes to show or hide a section's rows.
  - `disconnected` hides the section.
  - `permission-denied` and `unsupported` keep it, and each of its pages shows the existing `StatusView` with today's `problem_text`, minus the two "Not Connected" texts, which are removed.
- **Moving away:** when the selected row's section disappears, the app selects the other section's first row and adds an `AdwToast` ("Mouse disconnected" / "Keyboard disconnected") to the existing `ToastOverlay`.
- **No devices:** with no section left, a new window-level stack child shows an `AdwStatusPage` titled "No Devices Connected", next to the existing service-missing page.

### Fluid drawings: `ArtArea` gains a fill-height mode
- **Fill mode:** `ArtArea::measure` reports the aspect-correct height as natural, and a minimum that is a fraction of it: 1.0 for the keyboard (it never shrinks; the page scrolls), 0.5 for the tall mouse drawing, 0.3 in the narrow sidebar list. The widget is `vexpand` inside a card box that is `vexpand` too. In `snapshot`, the draw function already gets the allocated width and height, and `keyboard_art::draw` already fits with `min(w/W, h/H)`. The mouse drawing does the same with its new area.
- **Page structure:** pages are a `GtkScrolledWindow` around a vertical box. The drawing card takes the spare height, and when there is none the box exceeds the viewport and scrolls. This is the board behaviour at 860 × 640, which the 860 × 780 default avoids.
- **Alternative:** a fixed-height drawing per breakpoint. Rejected: it leaves the empty bands the user objected to.

### Columns: `AdwBreakpoint` setters on a `GtkBox` orientation
- **Two columns at about 1000 sp:** each page puts its groups in a `GtkBox`, horizontal and homogeneous when wide, vertical when narrow. A second breakpoint, `min-width: 1000sp`, flips the orientation.
- **Mouse pages:** the same breakpoint moves the drawing card beside the groups (Lighting, DPI & Profiles) or beside the callouts (Buttons).
- **Callouts:** the existing callout `GtkGrid` is re-attached in 2 or 3 columns (`Callouts::arrange(columns, column_major)`), and the card box flips orientation, so `AdwMultiLayoutView` wasn't needed.
- **Width handling:** four breakpoints, `max-width: 550sp`, `min-width: 720sp`, `min-width: 1000sp` and `min-width: 1200sp`. Their apply and unapply handlers call `Ui::apply_width(Narrow | Compact | Medium | Wide | Full)`, which flips the box orientations, the callout columns, the margins and the narrow-only parts, then `Ui::update_fold`, which sets the split view's `collapsed` (so the narrow breakpoint has no setter).
- **Compact, 550 to 720 sp (added in the hands-on pass):** with the sidebar beside the page, three callout columns asked for 713 px, so the window clipped the page between 550 and 713 px. Compact uses two columns, 12 px page margins, and hides the note beside the Editing toggle, which wrapped onto four lines.

### Mouse art: classify by group
- **Parser (`mouse_ink.rs`):** reads each `<path>`'s enclosing `<g id>`:
  - `Layer_5` gives the outline;
  - `DPI_LEDs` gives the LEDs;
  - `Logo` gives the logo, except in the side view, where `Logo` shapes narrower than 30 units are LEDs. This is the existing `LED_MAX_WIDTH` rule.
  - `Glow` and inline fills are ignored. The `glow` field and its drawing code go.
  - An unknown group is an error, as an unknown class is today.
- **Placement (`mouse_art.rs`):** stacked in a 290 × 420 unit area. The top view sits at (61.9, 8) and is 300 units tall; the side view sits at (10, 320) and is 270 units wide. `trace_g_top` and `trace_g_side` are removed (no G cut). The hotspot rectangles are re-derived for the new placement.

### Card colour follows the style
- **Card colour:** `art::CARD` becomes `art::card()`, reading `AdwStyleManager::is_dark()`: `#1D1D20` when dark, `#F6F6F7` when light.
- **Border:** the card gets a 1 px border at 7% black in the light style only, through a `.drawing-card` CSS class with `@media (prefers-color-scheme: light)`.
- **Contrast:** `drawing_colour(led, card)` already lifts a colour to 3:1 against whichever card it is given. It now also darkens a colour against the light card, by moving the lightness away from the card's.
- **Redraw:** the drawings redraw on the style manager's `notify::dark`.

### Key Maps page
- `editor.rs`'s `NavigationPage` becomes the Key Maps content page.
- The selection CSS becomes `outline: 3px solid white; box-shadow: inset 0 0 0 4px rgb(0 0 0 / 60%)`, which gives the dark inner ring.
- A selection count is added to the hint label.
- **Key Maps at 550 to 1200 sp (hands-on pass, boards `05-keymaps-688-*`, `05-keymaps-860-*`):** at 688 px the page had about 410 px for a 765 px keyboard. Of the four options drawn (shrink to fit, fold the sidebar, a section switcher, an overview with a zoomed grid), the user chose folding the sidebar plus shrinking the keys.
  - **Fold:** an `AdwOverlaySplitView` (`Ui::fold`, always collapsed) wraps the `AdwNavigationSplitView`. On Key Maps between 550 and 1200 sp, `Ui::update_fold` moves the sidebar's `ToolbarView` out of the split view's sidebar page into the overlay, collapses the split view with the page shown, and hides Back and turns off popping on the Key Maps page, since Back would lead to the empty sidebar page. A `sidebar-show-symbolic` toggle in the Key Maps header is bound to `show-sidebar`. Opening any other page, or a width outside that range, moves the content back. Rejected: a second copy of the sidebar list, which would need its rows, sections and marks kept in step.
  - **Fit:** `key_grid::KeyGrid` replaces the `GtkFixed`. It places the keys by the layout's quarter units at `scale = clamp(width / 765, floor, 1)`, reports `floor × 765` as its least width (0.6 from 550 sp up, 1.0 when narrow, so the narrow grid scrolls at full size), and scales the key labels through a `font-size` percentage loaded on idle after an allocation changes the scale. Drag selection converts points with `KeyGrid::to_quarters`.
  - The grid stays in a horizontal `GtkScrolledWindow`; the hint follows its adjustment and says "Scroll sideways to reach the rest of the keyboard." with the selection count while it scrolls.
- The Gaming map is selected through the existing `Editor::set_target(MapName)`, called by "Edit the Gaming Map" and the Games drawing.

### Games and DPI & Profiles pages
- **Games:** uses a second `KeyboardArt` instance fixed to the Gaming map, and `per_key.game_active` for the "Now" row.
- **DPI & Profiles drawing:** a `MouseArt` in badge-subset mode, showing badges 6–9. Its caption is built from `button_section`'s current action labels, so it follows remaps.

## Risks / Trade-offs

- **Two drawings of each device are live at once (page and narrow list).** Each `ArtArea` runs its own effect loop. → Only drawings that are mapped tick: the frame-clock ticker stops when unmapped. Confirm with the existing reduced-motion frame counter.
- **Breakpoint flicker near 550 and 1000 sp.** → The breakpoints use `sp` and have no overlap. Check by resizing in the hands-on pass.
- **`drawing_colour` darkening on the light card** may change how pale colours read. → Unit tests for white, yellow and pale green against `#F6F6F7`, asserting at least 3:1.
- **Hotspots must line up with the new art.** → A test checks that every hotspot lies inside the 290 × 420 area and on the drawing's outline bounds, like today's `badges_stay_on_the_card`.
- **`MouseArt` gains a badge-subset mode,** which adds API surface. → Keep it a single `badges: &[u8]` option.

## Migration Plan

- This is an app-only change, shipped through the same Flatpak.
- Window size: the app doesn't save one, so every launch opens at the new default, 860 × 780.
- Rollback: revert the branch. No settings or service data change.
