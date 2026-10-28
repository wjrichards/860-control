# Tasks

## 0. Base

- [x] 0.1 After `add-mouse-buttons` (PR #11) is merged and archived, rebase this branch onto main. Verify `openspec validate split-keyboard-mouse-views --strict` still passes against the updated main specs, and that the workspace builds in the GNOME SDK.
- [x] 0.2 Add `g512-control/data/g502-top-ink.svg` and `g502-side-ink.svg` (from `evidence/design/mouse-art/`). Verify they're byte-identical to the user's files (`cmp`) and embedded with `include_str!`.

## 1. Device state per view

- [x] 1.1 Change `service::snapshot` to return the keyboard state and `Option<Mouse>` separately, reading the mouse properties whatever `DeviceState` says (D2). Verify with unit tests on property maps:
  - keyboard disconnected and mouse connected gives a mouse;
  - no `MouseState` property gives `None`.
- [x] 1.2 Build the "No access" udev command from every non-comment line of `70-g512-control.rules` (D2). Verify with a unit test that the command contains both the c33c and c08b lines, and that it parses back into exactly the rules file's lines.

## 2. Views

- [x] 2.1 Replace the single page with an `AdwViewStack`:
  - Keyboard and Mouse pages with their icons;
  - the header `AdwViewSwitcher`;
  - an `AdwViewSwitcherBar`;
  - an `AdwBreakpoint` at 550sp (D1).

  Move the keyboard groups and `MouseSection`/`ProfileSection`/`ButtonSection` into their pages, and rename the Key Maps row to "Edit Key Maps…". Verify over AT-SPI that both pages exist with their names and that each group sits on the right page.
- [x] 2.2 Give each page an inner stack with an `AdwStatusPage` for disconnected, permission-denied and unsupported; keep the window-level status page for the service missing (D2). Hide the Mouse page and switchers when the service reports no mouse. Verify on a private bus with `tests/fake_service.py` (extended with a keyboard-missing mode) that:
  - with the keyboard missing, the Keyboard page shows its status page and the Mouse page shows its controls;
  - with no mouse properties, no switcher is shown.

## 3. Keyboard illustration

- [x] 3.1 Add `key_colours(state, maps, frame)` for effect colour, Custom and Gaming maps, lights off and Fn (D3). Add `drawing_colour(led, card)` (D6). Verify with unit tests:
  - WASD-only Custom lights exactly 4 keys;
  - lights off lights none;
  - Fn is never lit;
  - #26A269 and #1C71D8 are unchanged, and #101010 is lightened to ≥ 3:1.
- [x] 3.2 Add the keyboard illustration widget, drawn with `gtk::Snapshot` from `layout::KEYS`: lit tiles, unlit outlines, the case, and the hover ring (D3). Give it an accessible role and label, and make a click or Enter open Key Maps. Verify:
  - a screenshot of the running app beside `evidence/design/05-after-keyboard.png`;
  - over AT-SPI, the widget's name and that activating it opens Key Maps.

## 4. Mouse illustration

- [x] 4.1 Add the SVG layer parser, loading each path as Cairo path commands, plus the G cut path (D4). Verify with unit tests:
  - every `d` in both files parses;
  - the layer counts are as expected: top 52 outline, 3 LED, 1 logo, 2 glow; side 41 outline, 3 LED, 1 logo, 2 glow (counted from the files: `cls-2`, `cls-3`, `cls-1`);
  - an unknown class is an error.
- [x] 4.2 Add the mouse illustration widget: glow, outlines, LEDs and logo in the drawing colour; Off; the compact and narrow layouts; the hotspot table and badges (D4). Verify:
  - a hit-test unit test maps each badge centre to its button;
  - a screenshot beside `05-after-mouse.png` and `05-narrow-mouse.png`.

## 5. Callouts

- [x] 5.1 Move `ChangeDialog` to its own module so the rows and the callouts share it. Add the callout legend: 11 buttons, Left Click fixed and insensitive (D7). Make clicking a hotspot on the illustration open the same dialog. Verify:
  - over AT-SPI, 11 callout buttons with names, and Left Click insensitive;
  - opening G9's dialog from its callout and from the drawing gives the same dialog.
- [x] 5.2 Make the callouts follow the active profile and action changes, including a G9 switch on the mouse. Verify with the fake service changing `MouseActiveProfile` and `MouseButtons` that the callouts update without a restart.

## 6. Motion

- [ ] 6.1 Add the pure effect functions (Ripple ring, Wave band, Breathe level, Cycle hue, scaled by speed). Verify with unit tests against `render.py`'s keyframes at 0.6 s, 1.5 s and 2 s.
- [ ] 6.2 Add tick callbacks that run only while mapped, while animating, and while `gtk-enable-animations` is true. Add the hover, ring and confirm `adw::TimedAnimation`s (D5). Verify:
  - a screen recording of Ripple, Wave and Breathe on the Keyboard page, and of G9's hover and confirm, compared with `motion-preview.webm`;
  - with animations turned off (`gsettings set org.gnome.desktop.interface enable-animations false`), the illustrations are still and the tick count stays 0 (debug log).

## 7. Docs and checks

- [ ] 7.1 README: the two views, the illustrations, motion and Reduce Animation, and the art's source. Verify the README renders, and that its udev instructions match the app's command.
- [ ] 7.2 Workspace checks: clippy `-D warnings` and tests in the GNOME SDK, `openspec validate --strict` for the change and specs, and the Flatpak build with the SVGs in place. Verify that all pass.
- [ ] 7.3 Design review: compare the built screens with the Penpot boards (`penpot-design-to-code-review`), and list or fix the differences. Verify with a written comparison in `evidence/design-review.md`.
- [ ] 7.4 Hands-on pass with the user:
  - switch views;
  - narrow the window;
  - unplug the keyboard while on the Mouse page;
  - remap a button from the drawing;
  - press G9 on the mouse and watch the callouts follow;
  - turn animations off.

  Verify with a call recording and screenshots in `evidence/`.
