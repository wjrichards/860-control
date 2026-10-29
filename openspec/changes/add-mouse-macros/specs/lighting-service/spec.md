## MODIFIED Requirements

### Requirement: D-Bus interface
The service SHALL expose a session D-Bus interface that lets a front end:
- read device state (`connected`, `disconnected`, `permission-denied` or `unsupported`) and, when connected, the device name and firmware version;
- list the selectable effects, and for each one whether it accepts a colour and whether it accepts a speed, plus its allowed speed range;
- read the current lighting state (effect, colour, speed, brightness);
- set a new lighting state;
- read and replace the Custom and Gaming key maps;
- read and set whether the game switch is on, and read whether the Gaming map is being shown because a game is running;
- read and set whether the lights are on;
- read the mouse's state and name, its selectable effects, whether it matches the keyboard, and its own setting, and set matching and the own setting;
- read the mouse's profiles, active profile and DPI range, and switch and edit profiles;
- read each profile's button actions, and change one button;
- read the macros stored on the mouse and how many macro sectors are used out of how many, and save and delete a macro;
- receive a signal when the lighting state, a key map, the game switch, the game-running state, the lights-on setting, a mouse property or the device state changes.

Members that existed in v1 SHALL keep their names and signatures. Members added later SHALL keep theirs too.

#### Scenario: Front end reads effects
- **WHEN** a front end asks for the selectable effects while the keyboard is connected
- **THEN** it receives Off, Static, Breathe, Cycle, Wave, Ripple and Custom with their colour and speed parameters

#### Scenario: Front end reads mouse effects
- **WHEN** a front end asks for the mouse's selectable effects while the mouse is connected
- **THEN** it receives Off, Static, Breathe and Cycle with their colour and speed parameters

#### Scenario: Front end reads mouse profiles
- **WHEN** a front end asks for the mouse's profiles while the mouse is connected
- **THEN** it receives five profiles with their report rates, default steps and DPI steps, and the active profile number

#### Scenario: Front end reads mouse buttons
- **WHEN** a front end asks for the mouse's buttons while the mouse is connected
- **THEN** it receives eleven actions for each of the five profiles

#### Scenario: Front end reads mouse macros
- **WHEN** a front end asks for the mouse's macros while the mouse is connected and holds two
- **THEN** it receives both, each with its sector, name, play mode and steps, and the sector count "2 of 10"

#### Scenario: State change signal
- **WHEN** one front end sets a new lighting state
- **THEN** every connected front end receives a state-changed signal with the new state

#### Scenario: Lights change signal
- **WHEN** one front end turns the lights off
- **THEN** every connected front end receives a signal with the new lights-on value

#### Scenario: v1 client still works
- **WHEN** a client written for v1 calls `SetState` with Static red
- **THEN** the call succeeds exactly as in v1

### Requirement: Mouse buttons
When the mouse's profiles are readable, the service SHALL offer, for each profile, every button's action as a kind (`mouse`, `key`, `media`, `special`, `macro`, `disabled` or `other`), a value and modifier bits. For `macro`, the value SHALL be the macro's sector.

It SHALL change one button of one profile on request, and SHALL reject:
- button 0 (left click), or a button number outside the mouse's buttons;
- a profile outside the mouse's profiles;
- an unknown kind, the kind `other`, or a value that isn't valid for the kind;
- the kind `macro` with a sector that doesn't hold a macro.

Before the first write, it SHALL make sure the profile backup exists, as for profile edits. It SHALL report a failed write to the caller and keep the previous value.

#### Scenario: Remap G9 to a shortcut
- **WHEN** a front end sets profile 4's button 8 to the key `c` with Ctrl
- **THEN** pressing G9 on the mouse with profile 4 active acts as Ctrl+C, and the service offers the new action

#### Scenario: G9 plays a macro
- **WHEN** a front end sets profile 4's button 8 to the macro in sector 6, which types Home, Shift+End and Ctrl+C
- **THEN** pressing G9 with profile 4 active selects the line and copies it, and the service offers button 8 as `macro` with value 6

#### Scenario: Left click refused
- **WHEN** a front end tries to change button 0
- **THEN** the call fails with an invalid-argument error and nothing is written

#### Scenario: Macro sector that is empty
- **WHEN** a front end sets a button to the macro in sector 9 while sector 9 is free
- **THEN** the call fails with an invalid-argument error and nothing is written

## ADDED Requirements

### Requirement: Mouse macros
When the mouse's profiles are readable, the service SHALL read its macro sectors and offer each macro it holds, with:
- its sector;
- its name, or an empty name when the service doesn't know it;
- its play mode (`once`, `release`, `held` or `toggle`);
- its steps, each as a kind (`key-down`, `key-up`, `button-down`, `button-up`, `media-down`, `media-up` or `pause`), a value (usage, mask or milliseconds) and modifier bits.

It SHALL also offer how many macro sectors are used and how many there are.

**Names:** the mouse doesn't store names. The service SHALL keep each macro's name in its state file, tied to the sector and to the macro's content. When the content no longer matches (the macro was changed on another computer), the name SHALL be dropped.

**Saving** (a new macro, or a new version of one):
- it SHALL reject:
  - steps or a play mode it can't encode;
  - a pause outside 1–65535 ms;
  - a macro too long for one sector, as a "too long" error;
  - a new macro when no sector is free, as a "memory full" error;
  - saving over a sector that doesn't hold a macro.
- Before the first macro write it ever makes, it SHALL save every macro sector to `mouse-macros-backup.txt` in the configuration directory, only if that file doesn't exist, and SHALL never overwrite it.
- It SHALL write the sector and read it back. If the read-back differs, it SHALL write the sector's previous bytes back and report a "write failed" error that says the mouse is unchanged.
- It SHALL return the sector the macro was saved in.

**Deleting:**
- It SHALL first set every button, in every profile, that plays the macro to disabled, through the verified profile write.
- Only then SHALL it clear the macro's sector (all `ff` with its CRC) and drop its name.
- If a profile write fails, it SHALL stop, report it, and leave the macro in place.

**Restore:** `g512d --restore-mouse-profiles FILE` SHALL also accept the macro backup, and write each macro sector in it back to the mouse.

Each of "too long", "memory full" and "write failed" SHALL be a distinct D-Bus error name, so a front end can tell them apart without reading error text.

#### Scenario: Save a new macro
- **WHEN** a front end saves "Copy line" (played once: Home, 50 ms, Shift+End, 50 ms, Ctrl+C) and sectors 6 and 7 are used
- **THEN** it is written to sector 8, the service returns 8, offers it named "Copy line", and reports 3 of 10 sectors used

#### Scenario: Memory full
- **WHEN** a front end saves a new macro while all 10 macro sectors are used
- **THEN** the call fails with the "memory full" error and nothing is written

#### Scenario: First macro write makes a backup
- **WHEN** the first macro save ever is requested and no macro backup file exists
- **THEN** the service writes the backup of sectors 6–f before writing to the mouse, and later saves leave that file untouched

#### Scenario: Delete a macro in use
- **WHEN** a front end deletes the macro in sector 6, which G9 plays in profile 4
- **THEN** profile 4's G9 becomes disabled, then sector 6 is cleared, and the service offers neither the macro nor its name

#### Scenario: Macro made on another computer
- **WHEN** the mouse holds a macro in sector 9 that this service never saved
- **THEN** the service offers it with an empty name, its play mode and its steps
