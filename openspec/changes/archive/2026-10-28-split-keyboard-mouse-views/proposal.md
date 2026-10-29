# Proposal

## Why

Today the app is one long page: keyboard settings at the top, and the mouse's lighting, profile and eleven button rows under them, mostly below the window's 640 px fold. Nothing shows what the devices look like, so button names like "G9" or "Sniper" mean little until you can see where they are. The user asked for "a separated section for mouse and keyboard, and drawing of mouse and keyboard layout". It was planned in Penpot first (`design-plan.md`, boards in the "G512 Control" file, exports in `evidence/design/`).

## What Changes

- **Keyboard and Mouse views.** A view switcher in the header (Keyboard | Mouse) replaces the single page. In a narrow window it moves to a bottom bar.
- **A device illustration at the top of each view,** in the style of GNOME Settings' illustrations: one colour, taken from the device's live lighting.
  - **Keyboard:** line art built from the real key layout. Lit keys are solid tiles and unlit keys are outlines. Clicking it opens Key Maps.
  - **Mouse:** the user's ink drawings of the G502 HERO (top and 3/4 side views). The DPI LEDs and G logo are lit, with a glow. Numbered badges sit on the buttons, matched by a legend of clickable callouts showing each button's action; clicking one opens that button's Change dialog. Left click is shown as fixed.
- **Motion:** the drawings preview the current effect (Ripple, Wave, Breathe, Cycle), and mouse buttons fill and pulse on hover, focus and Apply. All of it stops when GNOME's animations are turned off.
- **Each view has its own state.** The Mouse view works with no keyboard connected, and the other way round; a missing device shows a status page inside its own view. Only "service not running" still covers the whole window.
- **Fix:** the "No access" page's udev command currently writes only the last rule line (the mouse's) and overwrites the rules file, which would drop the keyboard's rule. It will write both lines.
- The keyboard's "Key maps" row stays, as "Edit Key Maps…", for keyboard and screen-reader users; so do the mouse's Buttons rows.

## Capabilities

### New Capabilities
<!-- None: all of this is the app's behaviour, which lighting-app covers. -->

### Modified Capabilities
- `lighting-app`:
  - adds the Keyboard and Mouse views, the device illustrations, their motion, and the mouse button callouts;
  - changes "Device and service status" (per-view states and the fixed udev command), "Key-map editor" (opened from the illustration too) and "Mouse controls" (its own view, available without the keyboard).

## Impact

- **App (`g512-control`):**
  - `window.rs` splits into two views;
  - `service.rs` reads the mouse properties whatever the keyboard's state;
  - new illustration widgets, drawn with Cairo or snapshot;
  - the mouse SVGs are added as app resources;
  - `mouse_section.rs`, `profile_section.rs` and `button_section.rs` move into the Mouse view.
- **No service or D-Bus change:** g512d already connects and drives the mouse on its own.
- **Depends on `add-mouse-buttons`** (PR #11) for the Buttons rows and the Change dialog. This change is applied after that one is merged.
- **Assets:** `g502-top-ink.svg` and `g502-side-ink.svg`, provided by the user, ship in the app.
- **Packaging:** the Flatpak manifest gains the resources. Nothing else changes.
