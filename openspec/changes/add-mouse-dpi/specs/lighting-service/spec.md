## ADDED Requirements

### Requirement: Mouse profiles
When a G502 HERO is connected, the service SHALL read its onboard profiles and offer, for each: its number, report rate, default DPI step and DPI steps. It SHALL also offer which profile is active, and the sensor's DPI range.

- **Switching:** the service SHALL switch the active profile on request, then re-apply the mouse lighting.
- **Editing:** the service SHALL change a profile's report rate, DPI steps and default step on request, with these rules:
  - it SHALL reject:
    - fewer than 1 or more than 5 steps;
    - a step outside the sensor's range or off its step grid;
    - a default index outside the steps;
    - a report rate other than 1, 2, 4 or 8 ms;
    - a profile number outside the mouse's profiles;
  - before the first write it ever makes, it SHALL save the mouse's directory and user profile sectors to `mouse-profiles-backup.txt` in the configuration directory. It SHALL do this only if that file doesn't already exist, and SHALL never overwrite it;
  - it SHALL report a failed write to the caller and keep the previous profile values;
  - after editing the active profile, it SHALL re-apply the mouse lighting.
- **Restore:** `g512d --restore-mouse-profiles FILE` SHALL write each user sector from a backup file with a valid CRC back to the mouse, verifying each by reading it back.
- **The mouse's onboard mode** SHALL never be changed.

#### Scenario: Edit the active profile
- **WHEN** a front end sets profile 4 to steps 800, 1600, 3200 with default 1600 and a report rate of 1 ms
- **THEN** the mouse's DPI buttons cycle 800, 1600 and 3200, it starts at 1600 after a replug, and the service offers the new values

#### Scenario: Invalid edit
- **WHEN** a front end sets six DPI steps, or a step of 125
- **THEN** the call fails with an invalid-argument error and nothing is written

#### Scenario: Switch profile
- **WHEN** a front end makes profile 1 active
- **THEN** the mouse uses profile 1's DPI steps and report rate, and the mouse lighting is re-applied

#### Scenario: First write makes a backup
- **WHEN** the first profile edit ever is requested and no backup file exists
- **THEN** the service writes the backup before writing to the mouse, and later edits leave that file untouched

#### Scenario: Restore
- **WHEN** the user runs `g512d --restore-mouse-profiles` with the backup file
- **THEN** every user profile sector on the mouse matches the backup again

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
- receive a signal when the lighting state, a key map, the game switch, the game-running state, the lights-on setting, a mouse property or the device state changes.

Members that existed in v1 SHALL keep their names and signatures.

#### Scenario: Front end reads effects
- **WHEN** a front end asks for the selectable effects while the keyboard is connected
- **THEN** it receives Off, Static, Breathe, Cycle, Wave, Ripple and Custom with their colour and speed parameters

#### Scenario: Front end reads mouse effects
- **WHEN** a front end asks for the mouse's selectable effects while the mouse is connected
- **THEN** it receives Off, Static, Breathe and Cycle with their colour and speed parameters

#### Scenario: Front end reads mouse profiles
- **WHEN** a front end asks for the mouse's profiles while the mouse is connected
- **THEN** it receives five profiles with their report rates, default steps and DPI steps, and the active profile number

#### Scenario: State change signal
- **WHEN** one front end sets a new lighting state
- **THEN** every connected front end receives a state-changed signal with the new state

#### Scenario: Lights change signal
- **WHEN** one front end turns the lights off
- **THEN** every connected front end receives a signal with the new lights-on value

#### Scenario: v1 client still works
- **WHEN** a client written for v1 calls `SetState` with Static red
- **THEN** the call succeeds exactly as in v1
