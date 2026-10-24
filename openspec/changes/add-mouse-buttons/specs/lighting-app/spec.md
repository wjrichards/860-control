## ADDED Requirements

### Requirement: Mouse button controls
While the mouse is connected and the service offers its buttons, the app SHALL show a Mouse Buttons group for the active profile, with one row per button. Each row SHALL be named for the physical button and SHALL describe the button's current action.

The left-click row SHALL have no way to change it. Every other row SHALL open a dialog to choose:
- a mouse button;
- a keyboard shortcut, set by pressing it;
- a media key;
- a mouse function;
- disabled.

Media keys and "disabled" SHALL be offered only once they have been confirmed on the mouse.

Choosing an action SHALL send `SetMouseButton`. A key the app can't map to a HID usage SHALL be refused in the dialog.

#### Scenario: Set a shortcut
- **WHEN** the user opens G9's dialog, chooses Keyboard Shortcut, and presses Ctrl+C
- **THEN** the app sends `SetMouseButton` for G9 as key `c` with Ctrl, and the G9 row then reads "Ctrl+C"

#### Scenario: Left click fixed
- **WHEN** the user looks at the Left Click row
- **THEN** it shows "Left click" and offers no way to change it

#### Scenario: Profile switched on the mouse
- **WHEN** the user presses G9 on the mouse while it is set to cycle profiles
- **THEN** within a few seconds the app shows the new active profile and its buttons
