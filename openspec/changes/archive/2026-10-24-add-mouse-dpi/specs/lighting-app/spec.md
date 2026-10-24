## ADDED Requirements

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
