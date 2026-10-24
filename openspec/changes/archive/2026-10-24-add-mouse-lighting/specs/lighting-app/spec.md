## ADDED Requirements

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
