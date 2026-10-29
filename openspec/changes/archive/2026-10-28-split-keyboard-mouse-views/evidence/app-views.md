# Views and per-device state (tasks 2.1, 2.2)

**How it was checked, on 2026-10-28:**
- The worktree's debug build ran inside the installed Flatpak's sandbox (`run_app_fake.sh`).
- It ran against `gnome-extension/tests/fake_service.py` on the real session bus, under the test name `io.github.g512control.Lighting1Test`. The app follows that name through `G512_CONTROL_BUS_NAME`.
- g512d was not touched.
- The views were read over AT-SPI with `a11y_views.py`.

## 2.1 Two views

**Full fake: keyboard and mouse connected.**
- The switcher shows **Keyboard** and **Mouse**.
- **Keyboard view groups:** Keyboard (device), Lighting, Games. No mouse group.
- After pressing **Mouse:** Mouse ("G502 HERO Gaming Mouse", "Match Keyboard"), Mouse Profile, Mouse Buttons. No keyboard group.

## 2.2 Each view has its own state

**`--keyboard-missing`:**
- **Keyboard view:** the status page "Keyboard Not Connected".
- **Mouse view:** still Mouse, Mouse Profile and Mouse Buttons, with the mouse's name.
- The switcher stays usable.

**`--no-mouse`** (an older service, with no mouse members):
- No switcher.
- Only the keyboard groups: Keyboard, Lighting, Games.

The Keyboard view's permission-denied page uses `udev::install_command`, which unit tests show contains both rule lines (task 1.2).
