# Spec Delta

## Purpose

Defines the GNOME desktop app that lets the user choose the G512's lighting effect, colour, speed and brightness through the lighting service.

## ADDED Requirements

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
The app SHALL list the effects the service reports as selectable. For the selected effect, the app SHALL show a colour control only when the effect accepts a colour and a speed control only when it accepts a speed, limited to the reported range. A brightness control from 0 to 100 percent SHALL always be shown while connected.

#### Scenario: Cycle selected
- **WHEN** the user selects Cycle and the service reports that Cycle accepts a speed but no colour
- **THEN** the speed control is shown and the colour control is hidden

#### Scenario: Static selected
- **WHEN** the user selects Static
- **THEN** the colour control is shown and the speed control is hidden

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
