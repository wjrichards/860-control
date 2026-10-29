## MODIFIED Requirements

### Requirement: Tile menu
The tile's menu SHALL have a header with "Keyboard Lighting" and the device name, then:
- the effects the service reports as selectable, with the saved effect marked;
- a brightness slider from 0 to 100;
- a "Gaming Layout During Games" switch bound to the service's game switch;
- a "Lighting Settings" item that opens 860 Control.

Choosing an effect SHALL call `SetState` with that effect, the saved colour, the saved speed clamped to the effect's range, and the saved brightness. Moving the slider SHALL send at most one `SetState` per 100 ms, and SHALL always send the final value. Values changed by another front end SHALL show in the menu without reopening it.

#### Scenario: Pick an effect
- **WHEN** the saved state is Static red at speed 30 and the user chooses Wave in the menu
- **THEN** `SetState` is called with Wave, red, speed clamped to Wave's range, and the saved brightness

#### Scenario: Drag the slider
- **WHEN** the user drags the brightness slider from 100 to 20 in one second
- **THEN** the service receives at most about ten `SetState` calls, and the last one has brightness 20

#### Scenario: Change from the app
- **WHEN** the menu is open and the user changes the effect in 860 Control
- **THEN** the menu's marked effect and the tile's subtitle update

#### Scenario: Open the app
- **WHEN** the user chooses "Lighting Settings"
- **THEN** 860 Control opens and the Quick Settings menu closes
