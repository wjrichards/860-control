## ADDED Requirements

### Requirement: Service-only control from the Shell
The tile SHALL be a GNOME Shell 50 extension that controls the keyboard only through the service's session D-Bus interface. It SHALL NOT open hidraw devices, SHALL NOT start the service, and SHALL NOT store lighting settings of its own.

#### Scenario: Service not running
- **WHEN** `io.github.g512control.Lighting1` has no owner on the session bus
- **THEN** the tile is hidden and the service is not started

#### Scenario: Service starts later
- **WHEN** the service starts while the Shell is running
- **THEN** the tile appears and shows the service's current values

### Requirement: Lights toggle
The tile SHALL be a Quick Settings menu toggle titled "Keyboard" whose checked state is the service's `LightsOn` value. Clicking the toggle SHALL call `SetLightsOn` with the opposite value. While lit, its subtitle SHALL be the current effect name, or "Gaming Layout" while the service reports a game running. While the keyboard is disconnected or not permitted, the toggle SHALL be insensitive and its subtitle SHALL say so.

#### Scenario: Turn off and on
- **WHEN** the state is Ripple, the lights are on, and the user clicks the tile twice
- **THEN** the keyboard goes dark after the first click, Ripple returns after the second, and the tile's checked state follows

#### Scenario: Game running
- **WHEN** the lights are on and the Gaming map is shown because a game runs
- **THEN** the tile's subtitle is "Gaming Layout"

#### Scenario: Keyboard unplugged
- **WHEN** the service reports the device as disconnected
- **THEN** the tile is insensitive and its subtitle is "Disconnected"

### Requirement: Tile menu
The tile's menu SHALL have a header with "Keyboard Lighting" and the device name, then:
- the effects the service reports as selectable, with the saved effect marked;
- a brightness slider from 0 to 100;
- a "Gaming Layout During Games" switch bound to the service's game switch;
- a "Lighting Settings" item that opens G512 Control.

Choosing an effect SHALL call `SetState` with that effect, the saved colour, the saved speed clamped to the effect's range, and the saved brightness. Moving the slider SHALL send at most one `SetState` per 100 ms, and SHALL always send the final value. Values changed by another front end SHALL show in the menu without reopening it.

#### Scenario: Pick an effect
- **WHEN** the saved state is Static red at speed 30 and the user chooses Wave in the menu
- **THEN** `SetState` is called with Wave, red, speed clamped to Wave's range, and the saved brightness

#### Scenario: Drag the slider
- **WHEN** the user drags the brightness slider from 100 to 20 in one second
- **THEN** the service receives at most about ten `SetState` calls, and the last one has brightness 20

#### Scenario: Change from the app
- **WHEN** the menu is open and the user changes the effect in G512 Control
- **THEN** the menu's marked effect and the tile's subtitle update

#### Scenario: Open the app
- **WHEN** the user chooses "Lighting Settings"
- **THEN** G512 Control opens and the Quick Settings menu closes

### Requirement: Clean enable and disable
Disabling the extension SHALL remove the tile and release every signal connection, D-Bus proxy and timeout it created. Enabling it again SHALL rebuild the tile from the service's current values.

#### Scenario: Disable and enable
- **WHEN** the extension is disabled and enabled again with the Extensions app
- **THEN** the tile disappears, then reappears with current values, and no warnings about leftover sources appear in the Shell log
