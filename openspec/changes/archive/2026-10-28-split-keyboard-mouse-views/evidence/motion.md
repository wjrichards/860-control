# Motion (tasks 6.1, 6.2)

Checked on 2026-10-28, with the same setup as `app-views.md`: the worktree build ran in the installed Flatpak against `fake_service.py`, and g512d was not touched.

## 6.1 Effect functions: done

`motion.rs` has pure functions of time and speed. All six unit tests pass:
- **`ripple_matches_the_preview`, `wave_matches_the_preview`, `breathe_matches_the_preview`:** they check key levels 0.6 s, 1.5 s and 2 s into each effect against `design/motion-preview/render.py`, to within 0.001. Examples: F10 is 0.9884 at 0.6 s into Ripple, Keypad 4 is 0.6108 at 1.5 s, and Breathe is 0.3252, 0.8755 and 1.0 at the three times.
- **`cycle_turns_the_wheel_in_eight_seconds`.**
- **`speed_scales_the_loop`:** speed 100 halves the loop and speed 25 doubles it. Ripple at speed 100 and 0.3 s matches speed 50 at 0.6 s. Speed 0 doesn't divide by zero.
- **`still_effects_have_no_loop`:** Static, Custom and Off have none.

## 6.2 Ticks and interaction animations: implemented, two checks left for the hands-on pass

### What it does

- **`art_area::Ticker`** adds a frame-clock tick callback to an illustration only when all three hold:
  - the illustration is mapped;
  - the shown effect moves;
  - `gtk-enable-animations` is true.

  It watches map, unmap and that setting, and stops the callback, showing the still frame, as soon as any of them turns false. Every frame counts into `TICKS`, which the app prints every 2 s when `G512_CONTROL_TICK_LOG` is set.
- **The keyboard** draws each frame from its scene: Ripple's ring, Wave's band, Breathe's level and Cycle's hue, at the effect's speed.
- **The mouse** runs Breathe on its LEDs, logo and glow, and Cycle on its colour. A matching mouse runs at the keyboard's speed.
- **Hover:** pointing at a button on the drawing, or pointing at or focusing its callout, fills the button over 250 ms (an `adw::TimedAnimation`, ease-out cubic) and sends one ring out over 600 ms. The fill fades when the pointer leaves.
- **Confirm:** a successful Apply gives the button a confirm pulse: a 250 ms white flash and ring, the fill held to 1.6 s, then a 250 ms fade.
- **Reduced motion:** `adw::TimedAnimation` follows the system's animations setting, so with animations off the fill appears at once and no ring or pulse plays.

### Recorded

`app/motion-app.webm` (23 s, 30 fps) was rendered by the app itself (`G512_CONTROL_FRAMES`), so the user's screen wasn't recorded. It shows, in order:
1. the Keyboard view on Ripple;
2. the Keyboard view on Wave;
3. the Keyboard view on Breathe (green, speed 50);
4. the Mouse view with the keyboard on Breathe and the mouse matching.

Stills are in `app/motion-keyboard-frames.png` and `app/motion-mouse-frames.png`. Compared with `design/motion-preview.webm`:
- **Ripple:** the ring spreads from G and fades.
- **Wave:** a band sweeps left to right. The keys are in rainbow colours rather than the preview's green, because the G512's Wave ignores the colour.
- **Breathe:** every key fades between 15% and full, and the mouse's logo and LEDs follow it.

The tick log climbed at about the display's refresh rate: 438, 583 and 550 ticks during the three keyboard captures.

### Left for the hands-on pass (7.4)

- **The hover and the confirm pulse on G9:**
  - GTK 4 implements neither AT-SPI `grab_focus` nor actions on the Change dialog's check buttons, so a script can't focus a callout or pick a new action.
  - `drive_g9.py` can open G9's dialog, but it stops there.
  - These need a real pointer and Apply, recorded on screen.
- **Animations off:**
  - The first attempt ran GTK with portals off and a private GSettings keyfile, to avoid touching the desktop. The GNOME 50 runtime's GTK still reported `gtk-enable-animations` as true, and ticks kept running.
  - So the check needs the desktop's own setting.
  - `check_reduced_motion.sh` turns it off for a few seconds, runs the app on Breathe with the tick log, and restores the previous value. It's to be run with the user's OK.
  - Expected result: `ticks: 0` with animations off.
