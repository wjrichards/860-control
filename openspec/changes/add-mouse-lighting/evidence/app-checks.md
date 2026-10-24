# App checks

The Flatpak was rebuilt from this branch and the app relaunched (22:32). The service from this branch has the mouse connected.

## 3.1 The Mouse section over AT-SPI

`app_mouse_a11y.py` walks the whole accessibility tree, including the Mouse group below the fold. It uses each node's VISIBLE state and scrolls nodes into view before acting. `app_mouse_check.sh` drives it, with the service's properties and recorded calls as the evidence.

**Matching (start):**
- the Mouse group shows the status row "G502 HERO Gaming Mouse" and the "Match Keyboard" switch;
- the switch's subtitle reads "Show the keyboard's lighting, or the nearest effect the mouse has";
- no own-setting rows are visible.

**The switch** (second run, matching off at the start):
- Each flip over AT-SPI sent exactly one call. The calls recorded were `SetMouseMatch true`, `SetMouseMatch false`, `SetMouseMatch true`.
- With matching off, Effect, Colour, Speed and Brightness were visible for the own setting, Breathe blue. With matching on, they were hidden.

**The own setting** (`app_mouse_check.out`, matching off): setting "Mouse Brightness" to 60, then 100, sent these calls, and `MouseLighting` followed:
- `SetMouseLighting "Breathe" 0 0 255 40 60`
- `SetMouseLighting "Breathe" 0 0 255 40 100`

**Change made outside the app:** `busctl … SetMouseMatch b true` made the app hide the own-setting rows by itself.

**Limits of the AT-SPI checks:**
- **Effect list:** GTK 4's combo row exposes no action and no selection interface over AT-SPI, so it can't be picked by script. Choosing an effect goes through the same `user_changed` → throttle → `SetMouseLighting` path as the slider. It's covered by the hands-on pass (4.3).
- **The checked state lags:** AT-SPI's checked state lagged behind the widget, so the service's properties and the recorded calls are the evidence. In the third run, toggles over AT-SPI stopped taking effect: `do_action` returned true and the switch didn't change. No popover or dialog was open, and the switch's states matched the Lights and Games switches. The second run's three working toggles stand. The switch is also in the hands-on pass.
- **Hidden group:** the first app instance (started 22:32:26, during the service reinstall) had the Mouse group hidden. A relaunch showed it. Restarting the service with the app open didn't reproduce it, so it's noted, not fixed.
- **Older service:** a service without mouse support hides the group. That follows from `optional("MouseState")` returning `None`, but it wasn't run against an old service.
