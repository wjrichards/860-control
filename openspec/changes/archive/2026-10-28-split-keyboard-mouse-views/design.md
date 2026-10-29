# Design

## Context

- **The boards:** everything visible here is designed in Penpot ("G512 Control" file). `design-plan.md` lists the boards and exports, `evidence/design/04-motion.png` and `motion-preview/` show the motion, and `evidence/design/checks.md` has the contrast results and the colour rule.
- **The window today:** `window.rs` builds one `AdwPreferencesPage`.
  - The keyboard groups are followed by `MouseSection`, `ProfileSection` and (from `add-mouse-buttons`) `ButtonSection`.
  - `service::snapshot` reads the mouse only inside the keyboard's `connected` branch. A missing keyboard therefore hides the mouse too, even though g512d connects and drives the mouse on its own (`main.rs` connects both devices separately, and the service spec's Independence rule).
- **The key-map editor** (`editor.rs`) draws keys as buttons in a `gtk::Fixed` from `layout::KEYS`. The illustration reuses `KEYS` but not the editor's widgets.
- **Toolkit:** gtk4 0.11 (feature `v4_14`), libadwaita 0.9 (`v1_6`), GNOME 50 runtime.
  - The runtime ships librsvg but no gdk-pixbuf SVG loader.
  - GTK 4.14's `gsk::Path::parse` reads SVG path data directly.
- **The mouse art** is the user's: `g502-top-ink.svg` and `g502-side-ink.svg`.
  - Layer groups: `Glow`, `Layer_5` (outlines as filled shapes), `DPI_LEDs` and `Logo`.
  - Classes: `cls-1` glow, `cls-2` outline, `cls-3` LED or logo.
  - The paths use only M, L, C and Z.

## Goals / Non-Goals

**Goals:**
- The app matches the `05 Screens` boards in structure and the `04 Motion` board in timing.
- The illustrations are vectors: sharp at any scale factor, recoloured live, and cheap to animate.
- Losing either device affects only its own view.

**Non-Goals:**
- Changes to g512d or the D-Bus interface.
- Animating the key-map editor itself.
- A light-style version of the illustrations. The card stays dark in both styles, like GNOME Settings' illustrations.

## Decisions

### D1. Views: `AdwViewStack`, with the switcher in the header and a bottom bar when narrow

- **Structure:**
  - two `AdwViewStackPage`s ("Keyboard" with `input-keyboard-symbolic`, "Mouse" with `input-mouse-symbolic`), each holding a scrolled column of preference groups. The column is clamped to the board's width (694 px for the keyboard, 760 px for the mouse, so the callouts fit beside the drawing) instead of `AdwPreferencesPage`'s fixed 600 px clamp;
  - an `AdwViewSwitcher` as the header bar's title widget;
  - an `AdwViewSwitcherBar` in the toolbar's bottom bar;
  - an `AdwBreakpoint` (`max-width: 550sp`) that hides the header switcher and reveals the bar.
- **Group titles:** the view names the device, so its groups are titled by what they hold, as on the boards: Device, Lighting and Games on the Keyboard view; Device, Lighting, Profile and Buttons on the Mouse view. The mouse's status row and its lighting controls, once one "Mouse" group, are split into Device and Lighting.
- **Whole-window state:** the existing status page stays for "service not running" and "no session bus" only, at the window level.
- **Alternatives rejected:**
  - a sidebar (`AdwNavigationSplitView`), turned down by the user;
  - an overview page with cards, which adds a click to every task.

### D2. Device state per view

- **The snapshot:** `service::snapshot` returns `{ keyboard: KeyboardState, mouse: Option<Mouse> }`.
  - It reads the mouse properties whatever `DeviceState` says.
  - `KeyboardState` keeps today's variants: `Connected { … }`, `Disconnected`, `PermissionDenied` and `Unsupported`.
- **Status pages:** each view swaps its content for an `AdwStatusPage` when its device isn't connected, using an inner `GtkStack` per view.
- **Older services:** `mouse == None` (no `MouseState` property) hides the Mouse page and the switchers (D1).
- **The udev command** is built from every non-comment line of the rules file, as `printf '%s\n' '<line1>' '<line2>' | sudo tee …`, so it never drops a line. A unit test parses the generated command back into the rules.

### D3. Keyboard illustration: an aspect-keeping area drawn with Cairo

- **Drawing:** `ArtArea` (a small `gtk::Widget` subclass, height-for-width from an aspect ratio) hands a Cairo context, from `Snapshot::append_cairo`, to a draw function. That function draws `layout::KEYS` to the board's proportions (694 × 246, 7 px per quarter key): outlines for unlit keys, filled rounded rects for lit ones, and a rounded case.
- **Colours:** a pure function, `key_colours(state, maps, frame) -> Vec<Option<Rgb>>`, so it can be unit-tested. It covers:
  - the effect colour;
  - the Custom or Gaming map;
  - lights off;
  - the animation frame (D5).
- **Click:** the area sits inside a flat `gtk::Button`, which gives focus, Enter, the Button role and a spoken label (`"G512 keyboard, Ripple, green. Open Key Maps"`). App CSS draws the accent ring on hover and focus, as on the board.
- **Narrow width:** the same widget scales down. Below 400 px it hides nothing; there are no legends at any size.
- **Alternative rejected:** reusing `editor.rs`'s button grid. That means 104 widgets, CSS recolouring per frame, and no tile/outline look.

### D4. Mouse illustration: the user's SVG paths, drawn with Cairo

- **Assets:** `g512-control/data/g502-top-ink.svg` and `g502-side-ink.svg` are embedded with `include_str!`.
- **Parsing:** a small parser takes each `<path d="…" class="cls-N">` and its layer into `Layer { glow, outline, led }`. The `Logo` group sorts by class, so the side view's LED dashes inside it come out right. Each `d` is parsed once at start-up into Cairo path commands. The files use only M, L, C and Z, so the parser is small, and it rejects anything else.
- **The G mark:** the top-view G is a fixed path cut out in the card colour (the same arc and bar as the boards, `build-variants.py`).
- **Drawing:** layers are drawn in order:
  1. glow, at 20% × the LED level;
  2. outlines, in the drawing colour (D6);
  3. LEDs and logo, at the LED level;
  4. the G cut.
- **Hotspots:** a table, `HOTSPOTS: [(Button, Rect in SVG units, view)]`, gives each of the 11 buttons a centre and a hit rect. The table comes from the Penpot badge positions: G9 is (211.7, 346.8, 41.6, 52.2) on the top view, and the sniper, Back and Forward are on the side view. It drives:
  - the badge positions;
  - the hit testing (a click opens that button's Change dialog);
  - the hover fill, a rounded rect over the hit rect.
- **Layouts:**
  - **Compact (Mouse view):** the top view at 300 px tall, the side view at 270 px wide, and the legend to the right.
  - **Narrow:** both views on top and the legend in two columns below (the `Mouse drawing narrow` board).
- **Alternatives rejected:**
  - rasterising the SVG with librsvg: a new crate, a recolour means a re-render, and a G9 overlay is still needed;
  - `gsk::Path::parse` with snapshot fills: it works too, but Cairo keeps one drawing path for both illustrations;
  - hand-converting the art to Rust code: it loses the user's file as the source.

### D5. Motion: frame-clock ticks only while visible and animations are on

- **When it runs:** each illustration adds a `tick_callback` while it's mapped, the effect animates, and `gtk::Settings::is_gtk_enable_animations()` is true.
- **What a tick does:** it computes the phase from the frame time and the effect's speed (the loop length scales with `50 / speed`), then queues a redraw.
- **Loops:** the effect functions (Ripple ring, Wave band, Breathe level, Cycle hue) are pure, take `(t, speed)`, and are unit-tested against the `render.py` keyframes.
- **Interactions:** hover fill, ring pulse and confirm pulse use `adw::TimedAnimation` (250 ms ease-out-cubic, and 600 ms). It already obeys the animations setting, so with Reduce Animation it jumps to the end state.
- **Following the device:** the mouse's LED level follows the mouse's own effect, the one `MouseLighting` reports, or the matched keyboard effect, translated as the service does (Wave → Cycle, Ripple → Static).

### D6. The drawing colour keeps 3:1 contrast

`drawing_colour(led, card) -> Rgb` returns the LED colour unchanged when its contrast with the card is at least 3:1. Otherwise it mixes in white, in steps, until it reaches 3:1, keeping the hue. Off uses white at 35%. It's unit-tested with green #26A269 (unchanged), deep blue #1C71D8 (unchanged) and near-black #101010 (lightened, ≥ 3:1).

### D7. Callouts

- **The legend:** the 11 callouts are `gtk::Button`s in a two-column `GtkGrid`: down the columns (1–6, 7–11) beside the drawing (compact), or across in pairs below it (narrow). Each shows the number, the button name and the action. Each is at least 44 px tall, which meets the 24 px target, styled as on the `Button callout` board.
- **Change dialog:** they open the `ChangeDialog` from `button_section.rs`, moved to its own module so the rows and the callouts share it.
- **Left Click:** insensitive, labelled "Left click · fixed", with a dashed border.
- **Badges:** drawn by the illustration (D4) in `color.accent-strong` (#1D8092, white numbers 4.61:1).

### D8. Tokens and colours in code

The boards' tokens map to libadwaita named colours (`@window_bg_color`, `@card_bg_color`, `@accent_bg_color`) wherever one exists. App CSS defines only:
- the illustration card colour (#1D1D20);
- `accent-strong` for the badges;
- the dashed Fixed border.

## Risks / Trade-offs

- **[A path from the SVG fails to parse]** → Parse at start-up with unit tests over every path in both files; a failure fails the tests, not the user.
- **[Redrawing about 140 paths every frame on slow GPUs]** → Ticks run only while the view is mapped and the effect animates; Static, Custom and Off never tick. The outlines can be cached in a Cairo recording surface and reused between frames.
- **[Hotspot rects only roughly follow the button shapes]** → Hovering highlights a rect, not the exact button outline. It's acceptable at this size, and the callouts give a precise target.
- **[AT-SPI can't drive custom-drawn areas]** → The illustration has a single accessible role and label. Every action it offers is also reachable through real buttons: "Edit Key Maps…", the callouts and the rows.
- **[`add-mouse-buttons` isn't merged yet]** → This change applies on top of it. Tasks start by rebasing onto main after that merge.
- **[The user-provided art]** → It's shipped as given; the proposal records its source.

## Migration Plan

Nothing stored changes. Rolling back means installing the previous app build; the service is untouched.

## Open Questions

- The exact breakpoint width (550sp) can be tuned during the hands-on pass without changing the specs.
