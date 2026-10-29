# Proposal

## Why

`split-keyboard-mouse-views` gave each device its own view, but the switcher sits in the header, wide windows leave empty space, and the mouse drawing still uses the first ink SVGs with a glow and a G cut. The user asked to "update the mouse to these icons, move the menu to the left side instead of the top, and do a more responsive or fluid layout". It was planned in Penpot first: `design-plan.md`, boards in the "G512 Control" file, exports and the audit in `evidence/design/`.

## What Changes

- **BREAKING (UI):** a sidebar like GNOME Settings replaces the Keyboard | Mouse switcher and its bottom bar.
  - Sections: Keyboard (Lighting, Key Maps, Games) and Mouse (Lighting, DPI & Profiles, Buttons).
  - A section appears only while its device is detected.
- **Narrow windows:** the sidebar list becomes its own page, with a live drawing under each device header. Choosing a row opens the page with Back. There's no bottom bar.
- **Fluid pages:** drawing cards take the height that's left. Settings sit in two columns from about 1000 sp, and the mouse button labels sit beside the drawing when wide. The default window grows from 860 × 640 to **860 × 780**.
- **New pages, all using existing behaviour:**
  - **Key Maps:** the per-key editor becomes a normal sidebar page, no longer one pushed on top. The "Edit Key Maps…" row goes.
  - **Games:** the Gaming map drawing; the game switch; a "Now" group showing whether a game is running; and "Edit the Gaming Map ›".
  - **Mouse › Lighting** and **Mouse › DPI & Profiles:** each has its own page with the drawing. DPI & Profiles splits into "Profile" and "DPI Steps", and marks buttons 6–9 on the drawing.
- **Mouse art:** the user's new SVGs (`g502-top-ink.svg`, `g502-side-ink.svg`).
  - The top view is stacked above the side view.
  - It's lit by layer: the outline, the DPI LEDs and the logo.
  - There's no glow and no G cut; the logo is solid.
- **Devices coming and going:**
  - An unplugged device's section leaves the sidebar, and a toast says so ("Mouse disconnected").
  - With no device at all, the window shows "No Devices Connected".
  - A device that is present but has no access, or is unsupported, keeps its section and shows today's status page.
- **Light style:** the drawing cards turn light, like GNOME Settings' illustrations.
- **Key Maps selection:** the outline becomes white with a dark inner ring, replacing the accent colour, so it shows on every key colour (audit finding 4).
- **Key Maps between 550 and 1200 sp (added in the hands-on pass):** the sidebar folds away behind a header button and the keys shrink to fit, so the whole keyboard is in view.
- **Public name (added 2026-10-29):** the app shows itself as **860 Control**, a gaming peripheral center for GNOME, in its window, app grid entry, metainfo, extension listing and README. Ids, D-Bus names, the service and paths keep `g512-control`.

## Capabilities

### New Capabilities
<!-- None: this is the app's behaviour, which lighting-app covers. -->

### Modified Capabilities
- `lighting-app`:
  - replaces "Keyboard and Mouse views" with a device sidebar, and adds the fluid page layout;
  - changes "Device illustrations", "Illustration motion", "Mouse button callouts", "Device and service status", "Effect controls", "Key-map editor", "Game switch setting", "Mouse controls", "Mouse profile controls" and "Mouse button controls".
- `quick-settings-tile`: "Tile menu" names the app it opens as 860 Control.

## Impact

- **App (`g512-control`):**
  - `window.rs`:
    - `AdwNavigationSplitView` and the sidebar list replace the `ViewStack`, `ViewSwitcher` and bottom bar;
    - per-page breakpoints;
    - the default size changes.
  - `editor.rs`: becomes a page; the selection outline changes.
  - `mouse_ink.rs`: reads the new SVGs by group name, with no glow.
  - `mouse_art.rs`: stacked placement and no G cut.
  - `art.rs`: the card colour follows the light/dark style.
  - `art_area.rs`: drawings fit both width and height.
  - `profile_section.rs`, `mouse_section.rs`, `button_section.rs` and `callouts.rs` move onto their pages.
- **No service or D-Bus change.** The Games "Now" row uses the existing game-active state.
- **Depends on `split-keyboard-mouse-views`.** This change is stacked on its branch (`e67c081`), and that change is archived first, because this delta modifies the requirements it adds.
- **Assets:** the two new SVGs replace the old ones in the app resources.
