## ADDED Requirements

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
- receive a signal when the lighting state, a key map, the game switch, the game-running state, the lights-on setting or the device state changes.

Members that existed in v1 SHALL keep their names and signatures.

#### Scenario: Front end reads effects
- **WHEN** a front end asks for the selectable effects while the keyboard is connected
- **THEN** it receives Off, Static, Breathe, Cycle, Wave, Ripple and Custom with their colour and speed parameters

#### Scenario: State change signal
- **WHEN** one front end sets a new lighting state
- **THEN** every connected front end receives a state-changed signal with the new state

#### Scenario: Lights change signal
- **WHEN** one front end turns the lights off
- **THEN** every connected front end receives a signal with the new lights-on value

#### Scenario: v1 client still works
- **WHEN** a client written for v1 calls `SetState` with Static red
- **THEN** the call succeeds exactly as in v1

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
