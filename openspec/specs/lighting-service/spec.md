# lighting-service Specification

## Purpose
Defines the per-user background service that owns the G512, exposes lighting control to front ends over the session D-Bus, remembers the chosen lighting, and restores it after reconnects and suspend.

## Requirements

### Requirement: Unprivileged device access
The service SHALL run as the logged-in user without root. Installation SHALL provide a udev rule that grants the active local session read/write access to the G512's hidraw nodes (USB ID `046d:c33c`) and to no other hidraw device.

#### Scenario: Active session user
- **WHEN** the udev rule is installed and the user is logged in at the local seat
- **THEN** the service opens the keyboard's lighting interface without elevated privileges

#### Scenario: Rule not installed
- **WHEN** the service cannot open the lighting interface because permission is denied
- **THEN** the service stays running, reports the device state as `permission-denied`, and front ends can show how to install the udev rule

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
- receive a signal when the lighting state, a key map, the game switch, the game-running state or the device state changes.

Members that existed in v1 SHALL keep their names and signatures.

#### Scenario: Front end reads effects
- **WHEN** a front end asks for the selectable effects while the keyboard is connected
- **THEN** it receives Off, Static, Breathe, Cycle, Wave, Ripple and Custom with their colour and speed parameters

#### Scenario: State change signal
- **WHEN** one front end sets a new lighting state
- **THEN** every connected front end receives a state-changed signal with the new state

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
The service SHALL save the last successfully applied lighting state to the user's configuration directory, together with both key maps and the game switch setting. It SHALL apply the saved state when the service starts and the keyboard is connected. With no saved state, the service SHALL leave the keyboard as it is.

A state file written by v1 SHALL load with its lighting state intact, the default Gaming map, an empty Custom map and the game switch off.

#### Scenario: Restore at login
- **WHEN** the user logs in with a saved Static blue state
- **THEN** the keyboard turns blue once the service has started

#### Scenario: Set while disconnected
- **WHEN** a front end sets a valid state while the keyboard is disconnected
- **THEN** the service saves the state, reports that it is pending, and applies it when the keyboard next connects

#### Scenario: v1 state file
- **WHEN** the service starts with a state file containing only v1's effect, colour, speed, brightness and pending fields
- **THEN** the saved lighting state is applied and the new fields take their defaults

### Requirement: Reapply on reconnect and resume
The service SHALL detect the keyboard being plugged in and the system resuming from suspend. Within 3 seconds of either event, it SHALL reapply the lighting that is currently due: the Gaming map if a game is running and the game switch is on, otherwise the saved lighting state. Unplugging SHALL change the device state to `disconnected` without stopping the service.

#### Scenario: Keyboard replugged
- **WHEN** the keyboard is unplugged and plugged back in
- **THEN** the saved lighting state is visible on the keyboard within 3 seconds

#### Scenario: Resume from suspend
- **WHEN** the system resumes from suspend
- **THEN** the saved lighting state is visible on the keyboard within 3 seconds

#### Scenario: Replug during a game
- **WHEN** the Gaming map is shown because a game is running and the keyboard is replugged
- **THEN** the Gaming map is visible again within 3 seconds

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
When the game switch is on, the service SHALL show the Gaming map while the GameMode daemon (`com.feralinteractive.GameMode` on the session bus) reports at least one registered game. When the count drops to zero, it SHALL return to the saved lighting state.

- The service SHALL follow GameMode's `GameRegistered` and `GameUnregistered` signals and its `ClientCount` property.
- It SHALL NOT start GameMode itself, and SHALL treat GameMode's absence as no game running.
- An explicit `SetState` from a front end while a game runs SHALL be applied immediately and replace the Gaming map until the next game starts.
- The Gaming map SHALL use the saved brightness and the corrections.
- The game switch SHALL be off until the user turns it on.

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
