# App checks

## 3.1 The Mouse Profile group

**Over AT-SPI**, with `app_profile_a11y.py` and `app_profile_check.sh`:
- The group shows "Mouse Profile", with its description, the Profile row on "Profile 4", the Report Rate row on "1000 Hz", and "Add Step".
- The step rows are visible on screen: the user confirmed a "Step 1" row with 3200, twice, in both builds. But they never appear in the accessibility tree. A search of the whole app found no spin buttons except the keyboard's sliders.
  - The first build added rows after the window was built, so it was refactored to build all five rows at the start and only show or hide them (`eaab3a2`). The rows still didn't appear, so libadwaita's `AdwSpinRow` itself isn't exposed here. That would also affect screen-reader users; noted, not fixed.
- Running the "Add Step" button row's action changed nothing and sent no call. The combo rows expose no action, as in change 4.
- So nothing in this group can be driven by script, and the hands-on recording below is the evidence for 3.1 and 4.3.

**Unit tests:** `profile_edit` (5 tests) covers:
- snapping DPI values to the range and its step;
- adding steps (doubling, within the range, at most 5);
- removing them, keeping one, with the default following its step or falling back to the one before;
- setting the default, and report-rate positions.

The tests found one bug: removing the default step left the default on the next step. It was fixed before the hands-on pass.

## 4.3 Hands-on pass (with a call recording)

The user worked through six steps on profile 4. Recorded calls (UTC; the user is UTC−4):

```
12:05:57  SetMouseProfile 4, 1 ms, default 0, [3000]         step 1: Step 1 3200 → 3000
12:06:02  SetMouseProfile 4, 1 ms, default 0, [3000 6000]    step 2: Add Step
12:06:12  SetMouseProfile 4, 1 ms, default 1, [3000 6000]    step 3: Step 2 as default
12:06:28  SetMouseProfile 4, 2 ms, default 1, [3000 6000]    step 4: 500 Hz
12:06:37  SetMouseProfile 4, 2 ms, default 1, [3000 5900]    step 5: "minus" on Step 2
12:06:55  SetMouseActiveProfile 1                            step 6
12:07:05  SetMouseActiveProfile 4
```

- **One write per change.** Changing 3200 to 3000 takes several keystrokes or four arrow clicks, but sent only one write, so the debounce held each edit together.
- **The profile switch** sent one call each way.
- **Found here:** step 5 lowered Step 2 by 50 instead of removing it. The remove button used a minus icon right next to the spin row's own − button. It now uses a trash icon (`user-trash-symbolic`); see the commit "Use a trash icon for removing a DPI step". Removal is checked again below.
- **Restore:** afterwards, `g512d --restore-mouse-profiles` wrote back only sector 4. The service re-read the profiles, and the mouse is back to 3200 DPI on profile 4.

**Removing a step with the trash button** was checked after PR #9 merged, with the app rebuilt with the trash icon. Two earlier recordings caught nothing; a third, on 2026-10-24, recorded (UTC):

```
13:26:27  SetMouseProfile 4, 1 ms, default 0, [3200 6400]   Add Step
13:26:41  SetMouseProfile 4, 1 ms, default 0, [3200]        trash on Step 2
13:26:43  SetMouseProfile 4, 1 ms, default 0, [3200 6400]   Add Step
13:26:46  SetMouseProfile 4, 1 ms, default 0, [3200]        trash on Step 2
```

- Each removal wrote a single-step profile.
- `g512d --restore-mouse-profiles` then reported every user sector "Unchanged": profile 4 is byte-for-byte the user's original again.

## 4.2 Workspace checks

- `cargo clippy --workspace --all-targets -- -D warnings` in the GNOME 50 SDK: clean.
- `cargo test --workspace` in the SDK: 144 passed (14 + 47 + 83), plus 2 hardware tests skipped as usual.
- `gjs -m gnome-extension/tests/logic-test.js`: all passed.
- `openspec validate add-mouse-dpi --strict` passes, and `openspec validate --specs --strict` gives 4 passed.
