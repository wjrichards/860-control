## ADDED Requirements

### Requirement: Mouse buttons
When the mouse's profiles are readable, the service SHALL offer, for each profile, every button's action as a kind (`mouse`, `key`, `media`, `special`, `disabled` or `other`), a value and modifier bits.

It SHALL change one button of one profile on request, and SHALL reject:
- button 0 (left click), or a button number outside the mouse's buttons;
- a profile outside the mouse's profiles;
- an unknown kind, the kind `other`, or a value that isn't valid for the kind.

Before the first write, it SHALL make sure the profile backup exists, as for profile edits. It SHALL report a failed write to the caller and keep the previous value.

#### Scenario: Remap G9 to a shortcut
- **WHEN** a front end sets profile 4's button 8 to the key `c` with Ctrl
- **THEN** pressing G9 on the mouse with profile 4 active acts as Ctrl+C, and the service offers the new action

#### Scenario: Left click refused
- **WHEN** a front end tries to change button 0
- **THEN** the call fails with an invalid-argument error and nothing is written

### Requirement: Active profile kept current
While the mouse is connected and its profiles are readable, the service SHALL read the active profile at least every 2 seconds. When it differs from what the service holds, which happens when the mouse's own button switches profiles, the service SHALL:
- update it;
- re-read that profile;
- re-apply the mouse lighting;
- signal front ends.

#### Scenario: G9 switches profiles
- **WHEN** the user presses the mouse's profile-cycle button and the mouse moves from profile 4 to profile 5
- **THEN** within 2 seconds the service reports profile 5 as active, and the mouse shows the lighting it should

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

#### Scenario: State change signal
- **WHEN** one front end sets a new lighting state
- **THEN** every connected front end receives a state-changed signal with the new state

#### Scenario: Lights change signal
- **WHEN** one front end turns the lights off
- **THEN** every connected front end receives a signal with the new lights-on value

#### Scenario: v1 client still works
- **WHEN** a client written for v1 calls `SetState` with Static red
- **THEN** the call succeeds exactly as in v1
