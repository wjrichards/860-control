## ADDED Requirements

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
