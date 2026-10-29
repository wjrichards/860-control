# Product

<!-- impeccable:product-schema 1 -->

## Platform

gnome

<!-- Native GNOME desktop app: GTK 4 and libadwaita, packaged as a Flatpak (GNOME 50 SDK). None of Impeccable's values (web, ios, android, adaptive) fits; it is not a web app. -->

## Users

The owner first: one person with a Logitech G512 RGB keyboard and a G502 HERO mouse on a Fedora Silverblue 44 PC running GNOME 50, who wants to set the devices' lighting, DPI and buttons without Windows software.

Later, other Linux users with Logitech HID++ devices, once it's shared (Gitea now; possibly Flathub). Install steps and wording should already work for someone who isn't the author.

## Product Purpose

860 Control is a gaming peripheral center for GNOME desktops; its mission is to bridge the gap between what the system supports and what the hardware can do. Today it sets the lighting of the G512 and the G502 HERO, the mouse's DPI, report rate and onboard profiles, and the mouse's button actions, from a GNOME app and a Quick Settings tile. Success: the devices look and behave the way the user set them, and stay that way across replugs, suspend and login, without a vendor app.

## Positioning

A background service (`g512d`) owns the devices and re-applies their state after replug and resume, so the app and tile are just D-Bus clients with no device access. The app shows each device as a drawing in its live lighting: the keyboard from its real key layout, the mouse from the owner's own ink drawings.

## Operating Context

- Used now and then on the desktop: change an effect or colour, edit a key map, remap a mouse button, switch a profile. The tile covers the everyday toggles.
- The Gaming key map turns on while a GameMode game runs.
- The mouse's G9 button cycles profiles in the firmware, so the app follows the mouse, not only its own state.
- One device can be missing while the other works; each view reports its own device.

## Capabilities and Constraints

- Keyboard: Off, Static, Breathe, Colour cycle, Wave, Ripple, Custom (per-key maps: Custom and Gaming), brightness by colour scaling.
- Mouse: logo and DPI strip lighting (match keyboard, or Off, Static, Breathe, Cycle); DPI steps, report rate and onboard profiles; button remapping. Macros are planned.
- Hardware today: G512 (`046d:c33c`) and G502 HERO (`046d:c08b`). Future work should leave room for other Logitech HID++ keyboards and mice rather than hard-coding only these two.
- The service only calls an allowlist of HID++ features; firmware update, reset and engineering features are never used. Mouse profile writes go through a backup first.
- The app has no device access; the udev rule grants the logged-in user access to the supported devices only.

## Brand Commitments

- A native GNOME app: follow the GNOME HIG and libadwaita (system dark and light styles, the accent colour, standard widgets, Reduce Animation). Its own touch is the device drawings and their motion, not a custom chrome.
- Name: 860 Control, the public name (chosen 2026-10-29). The code, packages, app id (`io.github.g512control.Control`), D-Bus names, service and extension uuid keep `g512-control`.
- The G502 HERO drawings are the owner's own ink SVGs (`~/Pictures/g502-top-ink.svg`, `g502-side-ink.svg`, copied into `g512-control/data/`).

## Evidence on Hand

- Hardware findings and spikes are recorded in each archived OpenSpec change's `evidence/` (`openspec/changes/archive/`).
- App screenshots and the Penpot board exports for the Keyboard and Mouse views: `openspec/changes/split-keyboard-mouse-views/evidence/`.
- The Penpot file "G512 Control" holds the brief, adwaita-dark tokens, components, storyboard, motion and screens.
- No users, reviews or download numbers exist; don't invent any.

## Product Principles

1. The service is the single owner of the devices; every UI is a client and works without the others.
2. Show the device as it is: drawings follow the live lighting, the active profile and the real button actions.
3. Never risk the hardware: allowlisted features only, backup before flash writes, and a way to restore.
4. Native first: GNOME conventions over custom chrome.
5. Design for the next device: names, layouts and code shouldn't assume there will only ever be one keyboard and one mouse.

## Accessibility & Inclusion

Honour GNOME's Reduce Animation (drawings stay still), keep the drawing colour at 3:1 contrast or better against its card, and give every drawing and hotspot an accessible name and keyboard activation.
