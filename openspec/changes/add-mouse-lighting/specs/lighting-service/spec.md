## ADDED Requirements

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

## MODIFIED Requirements

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
- receive a signal when the lighting state, a key map, the game switch, the game-running state, the lights-on setting, a mouse property or the device state changes.

Members that existed in v1 SHALL keep their names and signatures.

#### Scenario: Front end reads effects
- **WHEN** a front end asks for the selectable effects while the keyboard is connected
- **THEN** it receives Off, Static, Breathe, Cycle, Wave, Ripple and Custom with their colour and speed parameters

#### Scenario: Front end reads mouse effects
- **WHEN** a front end asks for the mouse's selectable effects while the mouse is connected
- **THEN** it receives Off, Static, Breathe and Cycle with their colour and speed parameters

#### Scenario: State change signal
- **WHEN** one front end sets a new lighting state
- **THEN** every connected front end receives a state-changed signal with the new state

#### Scenario: Lights change signal
- **WHEN** one front end turns the lights off
- **THEN** every connected front end receives a signal with the new lights-on value

#### Scenario: v1 client still works
- **WHEN** a client written for v1 calls `SetState` with Static red
- **THEN** the call succeeds exactly as in v1
