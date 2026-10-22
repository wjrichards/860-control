# App checks (task group 4), 2026-10-22

These checks drove the installed Flatpak, or the debug build run inside the same sandbox with `flatpak run --command=...`, through AT-SPI (`a11y_drive.py`, scratch). The bus was watched with `busctl --user monitor`.

## 4.1 Sandbox

- `flatpak info --show-permissions` lists only ipc, wayland, fallback-x11, dri, and `io.github.g512control.Lighting1=talk`. There is no `--device=all`.
- `flatpak run --command=sh ... -c 'ls /dev/hidraw*'` gives `No such file or directory`.
- The app starts and shows the connected keyboard: `G512 RGB MECHANICAL GAMING KEYBOARD`, `Firmware U1 19.01 build 0014`.

## Bug found and fixed: UI state dropped after the first refresh

In the first build, `run()` dropped the only strong `Rc<Ui>` after the initial refresh. GTK kept the window on screen, but every handler's weak upgrade failed. As a result, slider changes sent nothing and external changes were never shown. The UI state is now owned by the window until it is destroyed.

An interim theory, that GTK's AT-SPI value setter skips change signals, was wrong. It has been withdrawn.

## Verified after the fix

- **External change, not echoed back (4.4):** `busctl` SetState Ripple → the app shows Ripple with Colour and Speed rows and speed 40. The bus saw exactly one SetState, from busctl.
- **Per-effect controls (4.3):**
  - Cycle (set from outside): Speed shown, Colour hidden.
  - Static: Colour shown, Speed hidden.
  - Ripple: both shown.
- **App → service:** brightness set through the app's slider reached the service with the selected effect. This was checked 6 times after external changes and 3 times after a fresh start.
- **Status pages (4.2):**
  - **Service missing:** real `g512d` masked and stopped → "Lighting Service Not Running", with `systemctl --user start g512d` and Try Again.
  - **Disconnected and permission-denied:** shown through a stand-in service owning the bus name (`fake_service.py`, scratch). They read "Keyboard Not Connected", and "No Access to the Keyboard" with the full rule-install command.
  - **Unsupported:** not yet checked.
  - **Real states:** `permission-denied` and `disconnected` were checked at the service level in 3.4. The app pages were not yet checked against them.

## Unexplained, not reproduced

Once, the first brightness change after an external Ripple sent effect `Breathe` (`"Breathe" 0 255 255 40 40`). This could not be reproduced in 9 further attempts covering the same and similar sequences. The cause is unknown.

## Side effect during testing

While `g512d` was masked and the stand-in held its name, the user's `systemctl --user start g512d` failed. The service was unmasked and restarted, and the saved state was set back to Static white.
