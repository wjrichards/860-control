# lighting-service Specification

## Purpose
Defines the per-user background service that owns the G512, exposes lighting control to front ends over the session D-Bus, remembers the chosen lighting, and restores it after reconnects and suspend.

## Requirements

### Requirement: Unprivileged device access
The service SHALL run as the logged-in user without root. Installation SHALL provide a udev rule that grants the active local session read/write access to the hidraw nodes of the G512 (USB ID `046d:c33c`) and the G502 HERO (USB ID `046d:c08b`), and to no other hidraw device.

If permission is denied for either device when the service starts, the service SHALL keep trying to open it for at least 10 seconds, because at login the session's access can arrive just after the service starts.

#### Scenario: Active session user
- **WHEN** the udev rule is installed and the user is logged in at the local seat
- **THEN** the service opens the keyboard's lighting interface without elevated privileges

#### Scenario: Access arrives just after start
- **WHEN** the service starts at login before the session has been granted access to the keyboard, and access arrives a second later
- **THEN** the service opens the keyboard and applies what is due within a few seconds, without a replug

#### Scenario: Rule not installed
- **WHEN** the service cannot open the lighting interface because permission is denied
- **THEN** the service stays running, reports the device state as `permission-denied`, and front ends can show how to install the udev rule

#### Scenario: Mouse rule missing
- **WHEN** the installed udev rule covers only the keyboard
- **THEN** the keyboard works, and the mouse state is `permission-denied`

### Requirement: Single instance
Only one instance of the service SHALL control the keyboard per user session. A second instance SHALL exit without opening the device.

#### Scenario: Second start
- **WHEN** the service is already running and another instance is started
- **THEN** the new instance exits with a message naming the running instance

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

### Requirement: Lighting state validation
A lighting state SHALL consist of an effect, an RGB colour, a speed, and a brightness from 0 to 100 percent. The service SHALL reject a state whose effect is not selectable on the connected device, or whose speed or brightness is outside its range, with an invalid-argument error and without changing the keyboard.

#### Scenario: Out-of-range brightness
- **WHEN** a front end sets brightness 150
- **THEN** the call fails with an invalid-argument error and the keyboard is unchanged

#### Scenario: Unsupported effect
- **WHEN** a front end sets an effect that the connected device did not report
- **THEN** the call fails with an invalid-argument error and the keyboard is unchanged

### Requirement: Brightness without a brightness feature
The device has no brightness feature. For effects that take a colour, the service SHALL apply brightness by scaling each colour channel it sends by the brightness percentage. For effects without a colour (Cycle, Wave), it SHALL send the brightness percentage as the effect's intensity value. Brightness 0 SHALL turn the lighting off. Changing brightness SHALL NOT change the stored colour.

#### Scenario: Half brightness
- **WHEN** the state is Static, colour `#FF8040`, brightness 50
- **THEN** the colour sent to the keyboard is `#804020` (each channel scaled by 0.5 and rounded) and the stored colour remains `#FF8040`

#### Scenario: Dimmed colour cycle
- **WHEN** the state is Cycle with brightness 20
- **THEN** the cycle is sent with intensity 20 and is visibly dimmer than at brightness 100

### Requirement: Persisted lighting state
The service SHALL save the last successfully applied lighting state to the user's configuration directory, together with both key maps, the game switch setting and the lights-on setting. It SHALL apply what is due when the service starts and the keyboard is connected. With no saved state and the lights on, the service SHALL leave the keyboard as it is.

A state file written by v1 SHALL load with its lighting state intact, the default Gaming map, an empty Custom map, the game switch off and the lights on. A state file without the lights-on setting SHALL load with the lights on.

#### Scenario: Restore at login
- **WHEN** the user logs in with a saved Static blue state
- **THEN** the keyboard turns blue once the service has started

#### Scenario: Lights off at login
- **WHEN** the user logs in with the lights saved as off
- **THEN** the keyboard stays dark once the service has started

#### Scenario: Set while disconnected
- **WHEN** a front end sets a valid state while the keyboard is disconnected
- **THEN** the service saves the state, reports that it is pending, and applies it when the keyboard next connects

#### Scenario: v1 state file
- **WHEN** the service starts with a state file containing only v1's effect, colour, speed, brightness and pending fields
- **THEN** the saved lighting state is applied and the new fields take their defaults

### Requirement: Reapply on reconnect and resume
The service SHALL detect the keyboard being plugged in and the system resuming from suspend. Within 3 seconds of either event, it SHALL reapply the lighting that is currently due:
- Off if the lights are off;
- otherwise the Gaming map if a game is running and the game switch is on;
- otherwise the saved lighting state.

Unplugging SHALL change the device state to `disconnected` without stopping the service.

#### Scenario: Keyboard replugged
- **WHEN** the keyboard is unplugged and plugged back in
- **THEN** the saved lighting state is visible on the keyboard within 3 seconds

#### Scenario: Resume from suspend
- **WHEN** the system resumes from suspend
- **THEN** the saved lighting state is visible on the keyboard within 3 seconds

#### Scenario: Replug during a game
- **WHEN** the Gaming map is shown because a game is running and the keyboard is replugged
- **THEN** the Gaming map is visible again within 3 seconds

#### Scenario: Replug with the lights off
- **WHEN** the lights are off and the keyboard is replugged
- **THEN** the keyboard is dark again within 3 seconds, instead of showing its firmware default

### Requirement: Custom effect
When the connected device has PerKeyLighting (`0x8080`), the service SHALL offer a Custom effect next to the device's zone effects. Custom takes neither a colour nor a speed.

With Custom selected, the service SHALL send the saved Custom key map as one per-key frame covering all 104 keys:
- keys missing from the map are sent as off;
- every key's colour is scaled by the brightness percentage;
- brightness 0 turns the lighting off.

#### Scenario: Custom offered
- **WHEN** a front end reads the effects while a G512 with PerKeyLighting is connected
- **THEN** Custom is listed, with no colour and no speed

#### Scenario: Custom at half brightness
- **WHEN** the Custom map sets W to `#FF0000`, the other keys are not in the map, and the state is Custom at brightness 50
- **THEN** the keyboard shows W at `#800000` and every other key off

### Requirement: Key maps
The service SHALL keep two key maps, Custom and Gaming, each mapping key IDs from the per-key key set to RGB colours.

- Until the user changes it, the Gaming map SHALL contain:
  - W, A, S, D, Space, Left Shift, Left Ctrl, the number-row keys 1 to 5 and the four arrow keys at `#FF2000`;
  - every other key at `#202020`.
- A front end SHALL be able to replace either map as a whole.
- A map with an unknown key ID SHALL be rejected with an invalid-argument error, leaving the saved map unchanged.
- Replacing the map that is currently shown SHALL update the keyboard.

#### Scenario: Edit the shown map
- **WHEN** Custom is the current effect and a front end replaces the Custom map
- **THEN** the keyboard shows the new map and the map is saved

#### Scenario: Edit the map that is not shown
- **WHEN** Static is the current effect and a front end replaces the Gaming map
- **THEN** the map is saved and the keyboard is unchanged

### Requirement: Per-key brightness correction
The service SHALL read an optional correction table from `corrections.toml`, in the same directory as the state file, when it starts. It SHALL never write that file. Each entry maps a key ID to a percentage from 1 to 100.

- Whenever the service sends a per-key frame, it SHALL multiply that key's colour by its correction, after brightness.
- When the table is non-empty and the effect is Static, the service SHALL send Static as a per-key frame of the Static colour, so corrections apply.
- Breathe, Cycle, Wave and Ripple SHALL be sent as zone effects, without corrections.
- An entry with an unknown key or an out-of-range percentage SHALL be ignored with a warning.

#### Scenario: Left Ctrl evened out
- **WHEN** the correction table has Left Ctrl at 85 and the state is Static `#FFFFFF` at brightness 100
- **THEN** every key is sent as `#FFFFFF` except Left Ctrl, which is sent as `#D9D9D9`

#### Scenario: No corrections
- **WHEN** the correction table is empty and the state is Static
- **THEN** Static is sent as a zone effect, as in v1

### Requirement: Gaming layout while a game runs
When the game switch is on and the lights are on, the service SHALL show the Gaming map while the GameMode daemon (`com.feralinteractive.GameMode` on the session bus) reports at least one registered game. When the count drops to zero, it SHALL return to the saved lighting state.

- The service SHALL follow GameMode's `GameRegistered` and `GameUnregistered` signals and its `ClientCount` property.
- It SHALL NOT start GameMode itself, and SHALL treat GameMode's absence as no game running.
- An explicit `SetState` from a front end while a game runs SHALL be applied immediately and replace the Gaming map until the next game starts.
- The Gaming map SHALL use the saved brightness and the corrections.
- The game switch SHALL be off until the user turns it on.
- While the lights are off, the service SHALL keep tracking games but SHALL NOT show the Gaming map.

#### Scenario: Game starts and ends
- **WHEN** the switch is on, the state is Ripple, and a game registers with GameMode
- **THEN** the keyboard shows the Gaming map; after the game unregisters, the keyboard shows Ripple again

#### Scenario: Switch off
- **WHEN** the switch is off and a game registers with GameMode
- **THEN** the keyboard is unchanged

#### Scenario: GameMode not running
- **WHEN** the switch is on and GameMode is not on the session bus
- **THEN** the service does not start it, and the saved lighting state stays shown

#### Scenario: User change during a game
- **WHEN** the Gaming map is shown because a game is running and a front end sets Static blue
- **THEN** the keyboard turns blue and stays blue until the next game registers

#### Scenario: Game starts with the lights off
- **WHEN** the switch is on, the lights are off, and a game registers with GameMode
- **THEN** the keyboard stays dark

### Requirement: Lights on and off
The service SHALL keep a lights-on setting, separate from the lighting state, that is on by default.
- While it is off, the service SHALL show Off on the keyboard, and SHALL keep the saved lighting state, both key maps and the game switch unchanged.
- Turning it on SHALL show whatever is currently due: the Gaming map if a game is running and the game switch is on, otherwise the saved lighting state.
- A successful `SetState` SHALL turn the lights on.
- `SetKeys` and `SetGameSwitch` SHALL NOT change it.

#### Scenario: Off and back on
- **WHEN** the state is Ripple, the lights are turned off, and then turned on
- **THEN** the keyboard goes dark, Ripple returns, and the saved state is Ripple throughout

#### Scenario: Picking an effect turns the lights on
- **WHEN** the lights are off and a front end sets Static blue
- **THEN** the lights turn on and the keyboard turns blue

#### Scenario: Turned on during a game
- **WHEN** the game switch is on, a game is running, and the lights are turned on
- **THEN** the keyboard shows the Gaming map

### Requirement: Mouse lighting
When a G502 HERO is connected, the service SHALL light both of its zones (the logo and the DPI strip) with one setting. The mouse SHALL NOT be switched out of its onboard-profile mode, and SHALL NOT be sent any HID++ feature beyond those allowlisted for the keyboard.

The mouse SHALL show:
- **Off** while the lights are off;
- otherwise, while **matching** the keyboard (the default), the keyboard's current lighting translated to the mouse's effects:
  - Off, Static, Breathe and Cycle map unchanged;
  - Wave becomes Cycle at the same speed;
  - Ripple becomes Static in Ripple's colour;
  - the Custom map, and the Gaming map while it is shown for a game, become Static in that map's colour for W, or Off if W is off;
  - brightness follows the keyboard's;
- otherwise, the mouse's **own** setting: an effect among Off, Static, Breathe and Cycle, with a colour, speed and brightness.

Further rules:
- **`SetMouseLighting`** SHALL validate the setting against the mouse's effects, save it, turn matching off, and turn the lights on.
- **`SetMouseMatch`** SHALL turn matching on or off.
- **Saved settings:** the mouse settings SHALL be saved with the rest of the state. A state file without them SHALL load with matching on.
- **Reapply:** within 3 seconds of the mouse being plugged in, of resume, or of access arriving after start, the service SHALL apply what the mouse should show.
- **Independence:** a mouse that is missing, not permitted or failing SHALL NOT change the keyboard's state or the result of a keyboard call.

#### Scenario: Matching a supported effect
- **WHEN** the mouse is matching and a front end sets Breathe green on the keyboard
- **THEN** both mouse zones breathe green

#### Scenario: Matching Ripple
- **WHEN** the mouse is matching and the keyboard is set to Ripple in `#3D3846`
- **THEN** both mouse zones show Static `#3D3846`

#### Scenario: Matching the Gaming map
- **WHEN** the mouse is matching, the game switch is on, and a game starts with the default Gaming map
- **THEN** the mouse shows Static in the Gaming map's W colour `#FF2000`, and returns to matching the saved state when the game ends

#### Scenario: Own setting
- **WHEN** a front end calls `SetMouseLighting` with Static blue
- **THEN** matching turns off, the mouse turns blue, the keyboard is unchanged, and the setting is still there after the service restarts

#### Scenario: Lights off
- **WHEN** the lights are turned off
- **THEN** the mouse goes dark as well as the keyboard, and returns to what it showed before when they are turned on

#### Scenario: Mouse replugged
- **WHEN** the mouse is unplugged and plugged back in
- **THEN** within 3 seconds it shows what it showed before, and the keyboard is untouched

#### Scenario: No mouse
- **WHEN** no G502 HERO is connected
- **THEN** the keyboard works exactly as before and the mouse state is `disconnected`

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
