# Spec Delta

## Purpose

Defines the per-user background service that owns the G512, exposes lighting control to front ends over the session D-Bus, remembers the chosen lighting, and restores it after reconnects and suspend.

## ADDED Requirements

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
- receive a signal when the lighting state or device state changes.

#### Scenario: Front end reads effects
- **WHEN** a front end asks for the selectable effects while the keyboard is connected
- **THEN** it receives Off, Static, Breathe, Cycle, Wave and Ripple with their colour and speed parameters

#### Scenario: State change signal
- **WHEN** one front end sets a new lighting state
- **THEN** every connected front end receives a state-changed signal with the new state

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
The service SHALL save the last successfully applied lighting state to the user's configuration directory and SHALL apply it when the service starts and the keyboard is connected. With no saved state, the service SHALL leave the keyboard as it is.

#### Scenario: Restore at login
- **WHEN** the user logs in with a saved Static blue state
- **THEN** the keyboard turns blue once the service has started

#### Scenario: Set while disconnected
- **WHEN** a front end sets a valid state while the keyboard is disconnected
- **THEN** the service saves the state, reports that it is pending, and applies it when the keyboard next connects

### Requirement: Reapply on reconnect and resume
The service SHALL detect the keyboard being plugged in and the system resuming from suspend, and SHALL reapply the saved lighting state within 3 seconds of either event. Unplugging SHALL change the device state to `disconnected` without stopping the service.

#### Scenario: Keyboard replugged
- **WHEN** the keyboard is unplugged and plugged back in
- **THEN** the saved lighting state is visible on the keyboard within 3 seconds

#### Scenario: Resume from suspend
- **WHEN** the system resumes from suspend
- **THEN** the saved lighting state is visible on the keyboard within 3 seconds
