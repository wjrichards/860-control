# Tasks

Boards are named as in `design-plan.md` §9. Exports are in `evidence/design/`.

## 1. Mouse art

- [x] 1.1 Replace `data/g502-top-ink.svg` and `data/g502-side-ink.svg` with the user's new files from `~/Pictures/`, and confirm `cargo build` embeds them.
- [x] 1.2 Rewrite `mouse_ink.rs` to sort paths by group:
  - `Layer_5` gives the outline, `DPI_LEDs` the LEDs, and `Logo` the logo;
  - side-view `Logo` shapes narrower than 30 units are LEDs;
  - ignore `Glow` and inline fills;
  - an unknown group is an error.

  Drop the `glow` field. Update `both_files_parse_into_their_layers` to the new counts, replace `unknown_class_is_an_error` with an unknown-group test, and verify both pass with `cargo test mouse_ink`.
- [x] 1.3 In `mouse_art.rs`:
  - place the views stacked in a 290 × 420 area: top at (61.9, 8), 300 tall; side at (10, 320), 270 wide;
  - remove `trace_g_top`, `trace_g_side` and the glow drawing;
  - re-derive the hotspot rectangles.

  Verify with `badges_stay_on_the_card`, and a new test that every hotspot lies inside the area. Compare a screenshot with `02-g502-drawing.png`.
- [x] 1.4 Add a badge-subset option (`badges: &[u8]`) to `MouseArt`, and verify that a test drawing with `[6, 7, 8, 9]` hit-tests only those four buttons.

## 2. Drawing card and fluid drawings

- [x] 2.1 Give `ArtArea` a fill mode: the natural height follows the aspect, and the minimum is about 40% of it; it fits the drawing with `min(w/W, h/H)`. Verify a unit test of `measure` for a wide and a tall allocation.
- [x] 2.2 Replace `art::CARD` with `art::card()`, following `AdwStyleManager::is_dark()`: `#1D1D20` when dark, `#F6F6F7` when light. Add the light-only 1 px border at `rgba(0, 0, 6, 0.07)` through a `.drawing-card` class, and redraw on `notify::dark`. Compare by screenshot with `05-light-1280.png`.
- [x] 2.3 Extend `drawing_colour` to darken colours against the light card. Verify tests asserting at least 3:1 for white, yellow (#F6D32D) and green (#26A269) on `#F6F6F7`, while the existing dark-card tests still pass.

## 3. Sidebar navigation

- [x] 3.1 Replace the `ViewStack`, `ViewSwitcher` and `ViewSwitcherBar` in `window.rs` with an `AdwNavigationSplitView`.
  - **Sidebar:** a `GtkListBox` with `.navigation-sidebar`.
  - **Headers:** "Keyboard · G512" and "Mouse · G502 HERO".
  - **Rows:** Lighting, Key Maps and Games; Lighting, DPI & Profiles and Buttons. Each has its icon from the `Sidebar row · states` board.

  Verify that clicking each row shows its page and marks the row selected, using only the keyboard (Tab, arrows, Enter).
- [x] 3.2 Collapse at `max-width: 550sp`: the list becomes its own page, and pages open with Back. Add each device's drawing under its header in collapsed mode only, with the last one expanding. Verify at 360 px against `03-storyboard-narrow.png` and the `Settings-style · narrow · list` board.
- [x] 3.3 Show and hide sections from device state.
  - `disconnected` hides a section. `permission-denied` and `unsupported` keep it, and each of its pages shows the existing status page.
  - If the open page's section goes, select the other section's first row and toast "Mouse disconnected" / "Keyboard disconnected".
  - The app opens on Keyboard › Lighting, or on Mouse › Lighting when only the mouse is present.

  Verify by unplugging each device with the app open, against `05-unplugged-mouse-1280.png` and `05-unplugged-mouse-360.png`.
- [x] 3.4 With no section shown, hide the sidebar and show an `AdwStatusPage`:
  - title "No Devices Connected";
  - description "Plug in the Logitech G512 keyboard or the G502 HERO mouse. Each one appears in the sidebar when it connects, and its saved lighting is applied.".

  Remove the "Keyboard Not Connected" and "Mouse Not Connected" texts from `problem_text`. Verify with both devices unplugged, against `05-unplugged-none-1280.png`.
- [x] 3.5 Keep an older service (no mouse properties) working: the Mouse section is hidden. When profiles aren't offered, hide the DPI & Profiles and Buttons rows. Verify with the existing older-service test path or a mocked state.

## 4. Pages

- [x] 4.1 **Keyboard › Lighting:**
  - drawing card filling the spare height, with the hint "Click the keyboard to edit Key Maps";
  - the Lighting and Keyboard groups in two columns from 1000 sp, one below;
  - remove the "Edit Key Maps…" row.

  Verify at 1280 and 360 against the `Settings-style · Keyboard › Lighting · 1280` and `· narrow` boards.
- [x] 4.2 **Keyboard › Key Maps:** move `editor.rs`'s page into the content side.
  - Add "Shown while a game runs: Gaming" beside the Editing toggle.
  - Add the selection count to the hint ("Click keys to select them, or drag across several. 4 keys selected.").
  - Make the grid card fill the spare height.
  - In narrow windows, keep 34 px keys in a horizontal scroller, with the hint "Scroll sideways to reach the rest of the keyboard". (Hands-on pass: the hint now follows whether the grid scrolls, at any width, and keeps the count.)

  Verify against `05-keymaps-1280.png` and `05-keymaps-360.png`.
- [x] 4.3 **Key Maps selection outline:** change `.keygrid button.key:checked` to a 3 px white outline with a dark inner ring, replacing `@accent_color`. Verify by screenshot that a selected key painted red, the accent teal, and white each show the mark.
- [x] 4.4 Open Key Maps on Gaming from "Edit the Gaming Map" and the Games drawing (the existing `Editor::set_target`), and verify that it opens with Gaming selected.
- [x] 4.5 **Keyboard › Games:**
  - **Drawing:** the keyboard drawing fixed to the Gaming map, with the hint "Gaming map · click the keyboard to edit it in Key Maps" (narrow: "Gaming map · click it to edit in Key Maps").
  - **"Games" group:** the existing switch and note.
  - **"Now" group:** "Game running" / "The keyboard shows the Gaming map", or "No game running" / "The keyboard shows its normal lighting", from `game_active`; then "Edit the Gaming Map ›".
  - **Banner:** the existing banner, on this page.

  Verify against `05-games-1280.png` and `05-games-360.png`.
- [x] 4.6 **Mouse › Lighting:**
  - **Drawing:** the stacked drawing without badges, beside the groups from 1000 sp and above them below that.
  - **Groups:** "Lighting" (the existing rows) and "Device" (the mouse's name).

  Verify against `05-mouse-lighting-1280.png` and `05-mouse-lighting-360.png`.
- [x] 4.7 **Mouse › DPI & Profiles:**
  - **Drawing:** badges 6–9, with a caption built from the current actions, for example "6 next profile · 7 DPI up · 8 DPI down · 9 DPI shift". Narrow: a short card with the drawing beside a four-line legend.
  - **Groups:** split the profile group into "Profile" ("Stored on the mouse"; Profile, Report Rate) and "DPI Steps" ("The DPI buttons cycle through these steps. The selected one is the default."; the steps and Add Step).

  Verify that remapping G9 updates the caption, and compare with `05-dpi-profiles-1280.png` and `05-dpi-profiles-360.png`.
- [x] 4.8 **Mouse › Buttons:**
  - the drawing fills the height;
  - the callouts are headed "Profile N · stored on the mouse";
  - the callouts lay out in two columns beside the drawing from 1000 sp, three below it from 720 sp, and two below under 720 sp. (Hands-on pass: was 550 sp; three columns clipped the window between 550 and 713 px.)

  Verify against the `Settings-style · Mouse › Buttons · 1280` board and `03-storyboard-mouse.png`.

## 5. Window and checks

- [x] 5.1 Change the default size to 860 × 780 (`window.rs` `default_height`). Verify that Keyboard › Lighting opens without scrolling.
- [x] 5.2 Check that only mapped drawings run effect loops: with Breathe on, opening the narrow list and then a page leaves one keyboard loop ticking. Use the reduced-motion frame counter in `art_area.rs`.
- [x] 5.3 Run `cargo clippy -- -D warnings` and `cargo test` in the GNOME SDK, all passing (the project has no rustfmt config and isn't rustfmt-formatted, so `cargo fmt` isn't part of its gate). Build the Flatpak and install it.

  Done 2026-10-29: clippy clean and all tests pass (55 app tests). Installed from the main checkout at `cb979f4` with the README command; the app's origin is `control1-origin`, the main checkout's cache. The extension was reinstalled for its new listed name.
- [x] 5.4 Hands-on pass, recording screenshots in `evidence/app/`:
  - each page at 360, 860 × 780 and 1280, in dark and light;
  - unplugging and replugging each device;
  - turning animations off;
  - keyboard-only navigation through the sidebar and every page.

  Done 2026-10-29 with the user on the installed app and the debug build, with these results:
  - **Pages and sizes:** keyboard-only sidebar navigation (3.1); resizing from narrow to full width, which found and fixed the Buttons clipping and the Key Maps cut-off (5.6); screenshots in `evidence/app/`.
  - **Light and dark:** the cards switch at once and the drawings stay readable.
  - **Animations off:** the drawings stop and start again with Reduce Animation; the setting was back on afterwards.
  - **Unplugging and replugging: not done on hardware.** Two attempts left no trace in the g512d log or `/dev/hidraw*`, and the user chose to rely on the fake-service checks (section hidden, move to the other device with a toast, "No Devices Connected"; screenshots in `evidence/app/`) plus the earlier changes' hardware replug tests.
- [x] 5.5 Compare the screenshots with the boards (`penpot-design-to-code-review`), and fix any mismatches or list them in the PR.
- [x] 5.6 **Key Maps between 550 and 1200 sp** (hands-on pass; boards `05-keymaps-688-*`, `05-keymaps-860-b-plus-a.png`, `05-keymaps-860-sidebar-open.png`):
  - fold the sidebar into an `AdwOverlaySplitView` on Key Maps, with a sidebar button in its header; Escape, the dimmed page or a row closes it;
  - `KeyGrid` scales the keys to the width down to 60%, full size and scrolling when narrow.

  Verify at 552, 688, 860, 1199 and 1280 that no page asks for more width than the window, that the keyboard shows whole from 552 up, and the sidebar-open screenshot against `05-keymaps-860-sidebar-open.png`.

## 6. Public name

- [x] 6.1 Show the public name **860 Control** (user's choice, 2026-10-29, option b): the window title and sidebar header, the `.desktop` name, comment and keywords, the metainfo name, summary ("Gaming peripheral center for GNOME") and description, the extension's listed name ("860 Control Quick Settings"), the README title and intro with the mission, and `PRODUCT.md`. The metainfo homepage and the extension's `url` point at the public repo, https://github.com/wjrichards/860-control. The app id, D-Bus names, `g512d`, the extension uuid, the binary, config paths and the repo keep `g512-control`.
- [x] 6.2 After installing, check that the app grid, the window and the Extensions app show "860 Control", and that the tile's "Lighting Settings" still opens the app. Done 2026-10-29: the app grid, the window and the tile's Lighting Settings show 860 Control; the Extensions app picks up the new listed name at the next login.
