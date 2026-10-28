# App checks

## 3.2 The Mouse Buttons group over AT-SPI

With `app_buttons_a11y.py`, against the installed app and service:
- **The group:** "Mouse Buttons", described as "Profile N, stored on the mouse". It has 11 rows named for the physical buttons, each subtitled with its current action: Left click, Right click, Middle click, Back, Forward, DPI shift (sniper), DPI down, DPI up, Next profile, Scroll right, Scroll left.
- **Left Click** has no button. Every other row has a Change button.
- **Following the mouse:** while the user pressed G9, the group switched to "Profile 1" by itself. The service had seen the switch, and the app followed its signal.
- **The Change dialog** (opened on Wheel Click by running the button's action) is exposed whole:
  - groups Mouse Button, Keyboard (Shortcut with Set Shortcut), Media Key, Mouse Function and Other (Disabled);
  - the current action, Middle click, checked;
  - Cancel and Apply. Cancel closed it.
- **Not drivable by script:** the dialog's radio buttons and their rows expose no action, so options can't be picked over AT-SPI (like the combo rows in change 4 and the spin rows in change 5). The hands-on pass below covers choosing and applying.

Found and fixed here:
- **Apply was active as soon as the dialog opened.** Pressing it would have written the same action again. Apply now waits for a choice that differs from the current action (`22b2c44`), and the user saw it turn blue only after picking DPI cycle.
- **The buttons were named after their rows** ("Right Click"), because a row's activatable widget takes the row's title. Without that, the name comes from the button's own text, "Change", and the label set on it was never used. Each button now carries a description, "Change what Sniper does" (`0250695`). Rechecked after a rebuild: "Change — Change what Right Click does".

## 4.3 Hands-on pass (with a call recording)

On 2026-10-27, profile 4. Recorded calls (UTC; `hands_on_part1.out`, `hands_on_part2.out`):

```
18:18:15  SetState Ripple … brightness 60           (not part of the pass: the keyboard
18:18:16  SetState Ripple … brightness 50            brightness slider moved, likely from scrolling)
18:20:38  SetState Ripple … brightness 60           (put back at the user's request)
18:25:10  SetMouseButton 4 5 special 5 0            Sniper → DPI cycle
18:28:48  SetMouseButton 4 8 key 6 1                G9 → shortcut, pressed as Ctrl+C
18:33:15  SetMouseButton 4 2 media 233 0            Wheel Click → Volume up
18:35:12  SetMouseButton 4 3 disabled 0 0           Back → Disabled
```

- **The first attempt sent no button calls at all**, although "all 5 worked" was reported. The app's rows still showed the old actions, so the steps were redone one at a time, each checked against the recording and the rows before the next.
- **One call per Apply**, and each row's subtitle changed to the new action: DPI cycle, Ctrl+C, Volume up, Disabled.
- **On the mouse:**
  - pressing G9 copied the selected text;
  - the wheel click showed GNOME's volume popup;
  - the front thumb button did nothing.
- **Left Click** has no Change button.
- **Restore** (`hands_on_restore.out`): `g512d --restore-mouse-profiles` wrote back only sector 4. Profile 4 is active, and the service and the app show the original actions again.
