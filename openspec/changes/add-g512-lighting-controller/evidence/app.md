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

## 4.4 Live apply on a real drag (user dragging, 2026-10-22 06:56 UTC)

The capture used `stdbuf -oL busctl --user monitor --match "type='method_call',member='SetState'"`, stopped with SIGINT. Two earlier captures lost lines: busctl block-buffers output to a file, and `timeout`'s default SIGTERM dropped the buffer.

The user dragged **Speed** (same throttle as Brightness):

- **Main drag:** 66 → 32 sent as **14 calls over 1456 ms**. Gaps in ms: 209.7, 99.5, 100.0, 100.1 ×8, 147.0, 99.4. The sub-100 values are bus-timestamp noise; the app's 100 ms timer never fires early.
- **Further small moves:** 9 more calls in pairs or threes, also about 100 ms apart.
- **Final value:** 52. The service `State` is `"Ripple" 154 153 150 52 100`, and the app's Speed slider shows 52.0.

Still open for 4.4 at this point: the toast and rollback when the service rejects an update (closed in stand-in run 2 below).

## Stand-in run 2, with the user's OK (2026-10-22)

`g512d` was masked (`--runtime`) and stopped for about a minute, then unmasked and restarted. Afterwards it was `connected` and had re-applied the user's saved state (`"Ripple" 61 56 70 52 82`).

- **Unsupported (4.2):** the stand-in reported `unsupported` → "Keyboard Not Supported" page.
- **Rejected update (4.4):** the stand-in reported a connected "Fake G512" with brightness 77 and rejected every SetState with InvalidArgs. Setting Brightness 30 through the app produced:
  - a toast labelled `rejected by the test service`;
  - the slider back at **77.0**, the stand-in's state.

So 4.4 is fully covered: throttled live apply on a real drag, external changes shown without being echoed back, and toast plus rollback on rejection.

For 4.2, the disconnected and permission-denied pages were shown through the stand-in reporting those states. They were not shown by removing the udev rule or unplugging the keyboard while the app was open. The real service reports those states correctly (3.4); the app reads the same property.

## 4.3 Hands-on effect pass (user, 2026-10-22, after the fresh install)

The user selected every effect in the Effect list of the freshly installed app: Off, Static, Breathe, Colour cycle, Wave and Ripple. The keyboard followed each choice, and the Colour and Speed rows showed and hid correctly for each effect, including the spec's Cycle and Static scenarios. This adds a check by eye to the AT-SPI checks above.

## 4.5 GNOME conventions (2026-10-22)

- **Accessible names:** the AT-SPI dump of the installed app (see `install.md`) names every control: combo box `Effect`, button `Colour` (with its colour described as `Red 24%, Green 22%, Blue 27%`), slider `Speed`, slider `Brightness`, and button `Close`. Each slider also exposes its value.
- **Keyboard only (user):** the whole app works with Tab, the arrow keys, Space and Enter.
- **System style (user):** switching Light/Dark and changing the accent colour in Settings changed the running app straight away, with no restart.
