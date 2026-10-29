# Hands-on pass (tasks 6.2 and 7.4)

**Date:** 2026-10-28.

**Build:** the worktree build installed as the user Flatpak (`flatpak info`: commit `0566758046c9`, built at 14:49 UTC), running against the real g512d with the G512 and the G502 HERO plugged in.

## What the user did

All steps worked:
1. Switched between Keyboard and Mouse, then narrowed the window. The switcher moved to the bottom bar, and the callouts dropped below the drawing.
2. Unplugged the keyboard while on the Mouse view. The Keyboard view showed "not connected", and the Mouse view kept working. Plugged it back in.
3. Pointed at G9 on the drawing. It filled and sent a ring out. Clicked it, chose an action and pressed Apply. G9 pulsed once, and its callout updated. Then set G9 back to Next profile.
4. Pressed G9 on the mouse. The callouts followed the new profile.
5. Turned Reduce Animation on. The drawings stood still, and hover filled with no ring. Turned it back off.

**What the user reported:** "done the hands-on pass, all steps worked".

## Read back afterwards

- **The service** (`MouseActiveProfile`, `MouseButtons` over D-Bus): profile 4 is active, and G9 on all five profiles is `('special', 10, 0)`, i.e. Next profile. So the remap in step 3 was put back.
- **The desktop:** `gsettings get org.gnome.desktop.interface enable-animations` gives `true`. So step 5 was undone.
- **A real pointer, earlier:** `app/mouse-groups.png` happened to catch the hover fill on Right Click, from the user's own pointer.

## Not recorded

The pass wasn't screen-recorded. The three screencasts from 09:34 to 09:37 in `~/Videos/Screencasts` are an unrelated product demo, so they aren't evidence. The effect loops are covered by the app's own recording, `app/motion-app.webm` (see `motion.md`). The hover, the confirm pulse and Reduce Animation rest on the user's report above.
