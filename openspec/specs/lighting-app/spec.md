# lighting-app Specification

## Purpose
Defines the GNOME desktop app that lets the user choose the G512's lighting effect, colour, speed and brightness through the lighting service.

## Requirements

### Requirement: Service-only device control
The app SHALL control the keyboard only through the lighting service's D-Bus interface and SHALL NOT open hidraw devices itself.

#### Scenario: App needs no device access
- **WHEN** the app runs without access to any `/dev/hidraw*` node
- **THEN** all lighting controls still work through the service

### Requirement: Device and service status
The app SHALL show whether the keyboard is connected and, when it is, its name and firmware version. When the service is not running, or reports `permission-denied`, `disconnected` or `unsupported`, the app SHALL show a status page explaining the state and, for `permission-denied`, the command to install the udev rule. Lighting controls SHALL be disabled while the keyboard is not connected.

#### Scenario: Service not running
- **WHEN** the app starts and the lighting service is not on the session bus
- **THEN** the app shows a status page saying the service is not running and how to start it

#### Scenario: Permission denied
- **WHEN** the service reports `permission-denied`
- **THEN** the app shows a status page with the udev rule install command

### Requirement: Effect controls
The app SHALL list the effects the service reports as selectable. For the selected effect, the app SHALL show a colour control only when the effect accepts a colour, and a speed control only when it accepts a speed, limited to the reported range. When Custom is selected, the app SHALL hide the colour and speed controls and open the key-map editor on the Custom map from the "Key maps" row. A brightness control from 0 to 100 percent SHALL always be shown while connected.

#### Scenario: Cycle selected
- **WHEN** the user selects Cycle and the service reports that Cycle accepts a speed but no colour
- **THEN** the speed control is shown and the colour control is hidden

#### Scenario: Static selected
- **WHEN** the user selects Static
- **THEN** the colour control is shown and the speed control is hidden

#### Scenario: Custom selected
- **WHEN** the user selects Custom
- **THEN** the colour and speed controls are hidden, brightness stays visible, and the "Key maps" row opens the editor on the Custom map

### Requirement: Live apply
Changes in the app SHALL be sent to the service without an explicit apply button. While a slider or colour control is being dragged, the app SHALL send at most one update every 100 ms, and the final value SHALL always be sent. If the service rejects an update, the app SHALL show the error in a toast and return the controls to the service's current state.

#### Scenario: Dragging brightness
- **WHEN** the user drags the brightness slider from 100 to 20 over 1 second
- **THEN** the service receives no more than about 10 updates and the final brightness is 20

#### Scenario: Update rejected
- **WHEN** the service rejects an update
- **THEN** a toast shows the error and the controls show the service's current state

### Requirement: External changes reflected
When the service signals a lighting or device state change made elsewhere (another front end, a reconnect), the app SHALL update its controls to match without sending that state back.

#### Scenario: Change from another client
- **WHEN** another D-Bus client sets the effect to Ripple
- **THEN** the app shows Ripple selected and does not send a new update

### Requirement: GNOME conventions
The app SHALL follow the system light/dark style and accent colour, be fully usable with the keyboard alone, and give every control an accessible name.

#### Scenario: Keyboard-only use
- **WHEN** the user operates the app using only Tab, arrow keys, Space and Enter
- **THEN** they can select any effect and change colour, speed and brightness

### Requirement: Key-map editor
The app SHALL provide a key-map editor on its own page, reachable from a "Key maps" row whatever effect is selected. The editor draws the ANSI 104-key layout, with each key in its current colour.

**Editing:**
- The user SHALL be able to select keys by clicking, by dragging across them, and with group buttons for WASD, arrows, F-keys, number row, numpad and modifiers.
- They SHALL be able to paint the selected keys with a chosen colour or turn them off.
- A switch SHALL choose whether the Custom or the Gaming map is being edited.
- Edits SHALL be sent to the service as whole-map replacements, at most one every 100 ms while painting, with the final map always sent.

**Keyboard and accessibility:**
- Arrow keys SHALL move focus between drawn keys, Space SHALL toggle the focused key's selection, and Tab SHALL leave the drawn keyboard.
- Every drawn key SHALL have an accessible name giving the key and its colour.

#### Scenario: Paint WASD
- **WHEN** the user is editing Custom, presses the WASD group button, picks green and paints
- **THEN** the service's Custom map has W, A, S and D green, and if Custom is the current effect, the keyboard shows them green

#### Scenario: Edit Gaming while not shown
- **WHEN** the current effect is Ripple and the user paints keys in the Gaming map
- **THEN** the Gaming map is saved and the keyboard keeps showing Ripple

#### Scenario: Keyboard-only painting
- **WHEN** the user moves through the drawn keys with the arrow keys and selects Q with Space
- **THEN** Q is selected and can be painted without using a pointer

### Requirement: Game switch setting
The app SHALL show a "Switch to Gaming layout while a game runs" setting bound to the service's game switch. It SHALL also indicate when the Gaming map is currently shown because a game is running, and SHALL explain that games must use GameMode, for example with Steam's `gamemoderun %command%` launch option.

#### Scenario: Turn the switch on
- **WHEN** the user turns the setting on
- **THEN** the service's game switch is on, and other front ends see the change

#### Scenario: Game running
- **WHEN** the service reports that the Gaming map is shown because a game is running
- **THEN** the app shows that the gaming layout is active

### Requirement: Lights switch
The app SHALL show a "Lights" switch bound to the service's `LightsOn` value, calling `SetLightsOn` when the user flips it. The other controls SHALL stay usable while the lights are off. When the service doesn't report `LightsOn`, the switch SHALL be hidden.

#### Scenario: Turn off from the app
- **WHEN** the user turns the Lights switch off
- **THEN** the keyboard goes dark and the effect, colour, speed and brightness shown in the app don't change

#### Scenario: Turned off elsewhere
- **WHEN** the lights are turned off from the Quick Settings tile while the app is open
- **THEN** the app's Lights switch turns off

#### Scenario: Older service
- **WHEN** the app connects to a service without the `LightsOn` property
- **THEN** the Lights switch is hidden and the rest of the app works as before

### Requirement: Mouse controls
When the service reports a mouse state, the app SHALL show a Mouse section containing:
- the mouse's status: its name when connected, otherwise that it isn't connected or not permitted;
- a "Match Keyboard" switch bound to `MouseMatch`;
- while matching is off, effect, colour, speed and brightness controls for the mouse's own setting. They SHALL show only what the chosen mouse effect accepts, and send `SetMouseLighting` with the same throttling as the keyboard controls.

When the service doesn't report a mouse state, the section SHALL be hidden.

#### Scenario: Matching
- **WHEN** the mouse is connected and matching the keyboard
- **THEN** the Mouse section shows the mouse's name and the switch on, and hides the effect controls

#### Scenario: Own colour
- **WHEN** the user turns Match Keyboard off and chooses Static blue
- **THEN** the app sends `SetMouseMatch(false)` and then `SetMouseLighting` with Static blue, and the mouse turns blue while the keyboard keeps its own lighting

#### Scenario: Mouse unplugged
- **WHEN** the mouse is unplugged while the app is open
- **THEN** the Mouse section shows that the mouse isn't connected, and the keyboard controls are unaffected

#### Scenario: Older service
- **WHEN** the app connects to a service without mouse properties
- **THEN** the Mouse section is hidden

### Requirement: Mouse profile controls
While the mouse is connected and the service offers its profiles, the app's Mouse section SHALL show:
- a Profile selector (Profile 1 to the profile count). Choosing one SHALL make it the mouse's active profile;
- a Report Rate selector offering 125, 250, 500 and 1000 Hz;
- one row per DPI step of the active profile, each with a number entry limited to the sensor's range and step, a way to mark it as the default step, and a way to remove it;
- a way to add a step, up to five.

The app SHALL send an edit of the active profile only once the user has made no further change for about 2 seconds, and SHALL then send it as a single request. It SHALL NOT replace the values the user is editing with the service's while an edit is waiting to be sent. With one step left, removing it SHALL NOT be possible.

#### Scenario: Change a DPI step
- **WHEN** the user changes the second DPI step from 2400 to 2000 and then stops
- **THEN** about 2 seconds later the app sends one `SetMouseProfile` for the active profile with 2000 as the second step

#### Scenario: Switch profile
- **WHEN** the user chooses Profile 1
- **THEN** the app sends `SetMouseActiveProfile(1)`, and shows profile 1's report rate and DPI steps

#### Scenario: No profiles offered
- **WHEN** the service offers no profiles (an older service, or a mouse whose profiles it can't read)
- **THEN** the profile, report-rate and DPI controls are hidden, and the mouse lighting controls work as before
