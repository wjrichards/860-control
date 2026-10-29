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
The app SHALL show each device's status in its own view.

**When a device is connected,** its view SHALL show its name, and for the keyboard its firmware version.

**When a device is `disconnected`, `permission-denied` or `unsupported`:**
- its view SHALL show a status page explaining the state, in place of that device's illustration and controls;
- the other view SHALL keep working;
- for `permission-denied`, the status page SHALL show a command that installs the udev rule with both the keyboard's and the mouse's lines, so that following it never removes the other device's access.

**When the lighting service is not running,** the app SHALL show a status page over the whole window.

#### Scenario: Service not running
- **WHEN** the app starts and the lighting service is not on the session bus
- **THEN** the app shows a status page saying the service is not running and how to start it

#### Scenario: Permission denied
- **WHEN** the service reports `permission-denied` for the keyboard
- **THEN** the Keyboard view shows a status page with a udev command that writes the keyboard's and the mouse's rule lines

#### Scenario: Mouse without the keyboard
- **WHEN** the keyboard is unplugged and the mouse is connected
- **THEN** the Keyboard view shows that the keyboard isn't connected, and the Mouse view still shows the mouse with all its controls

### Requirement: Effect controls
The app SHALL list the effects the service reports as selectable.

**For the selected effect:**
- a colour control SHALL be shown only when the effect accepts a colour;
- a speed control SHALL be shown only when the effect accepts a speed, limited to the reported range;
- a brightness control from 0 to 100 percent SHALL always be shown while connected.

**When Custom is selected,** the app SHALL hide the colour and speed controls. The "Edit Key Maps…" row and the keyboard illustration SHALL open the key-map editor on the Custom map.

#### Scenario: Cycle selected
- **WHEN** the user selects Cycle and the service reports that Cycle accepts a speed but no colour
- **THEN** the speed control is shown and the colour control is hidden

#### Scenario: Static selected
- **WHEN** the user selects Static
- **THEN** the colour control is shown and the speed control is hidden

#### Scenario: Custom selected
- **WHEN** the user selects Custom
- **THEN** the colour and speed controls are hidden, brightness stays visible, and the "Edit Key Maps…" row opens the editor on the Custom map

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
The app SHALL provide a key-map editor on its own page, reachable whatever effect is selected from the "Edit Key Maps…" row and from the keyboard illustration. The editor draws the ANSI 104-key layout, with each key in its current colour.

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

#### Scenario: Open from the illustration
- **WHEN** the user clicks the keyboard illustration
- **THEN** the key-map editor opens on the map being shown, or on Custom when neither map is shown

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
When the service reports a mouse state, the app's Mouse view SHALL contain:
- the mouse's status: its name when connected, otherwise that it isn't connected or not permitted;
- a "Match Keyboard" switch bound to `MouseMatch`;
- while matching is off, effect, colour, speed and brightness controls for the mouse's own setting. They SHALL show only what the chosen mouse effect accepts, and send `SetMouseLighting` with the same throttling as the keyboard controls.

The Mouse view SHALL work whether or not the keyboard is connected. When the service doesn't report a mouse state, the Mouse view SHALL be hidden.

#### Scenario: Matching
- **WHEN** the mouse is connected and matching the keyboard
- **THEN** the Mouse view shows the mouse's name and the switch on, and hides the effect controls

#### Scenario: Own colour
- **WHEN** the user turns Match Keyboard off and chooses Static blue
- **THEN** the app sends `SetMouseMatch(false)` and then `SetMouseLighting` with Static blue, and the mouse turns blue while the keyboard keeps its own lighting

#### Scenario: Mouse unplugged
- **WHEN** the mouse is unplugged while the app is open
- **THEN** the Mouse view shows that the mouse isn't connected, and the Keyboard view is unaffected

#### Scenario: Older service
- **WHEN** the app connects to a service without mouse properties
- **THEN** the Mouse view is hidden

### Requirement: Mouse profile controls
While the mouse is connected and the service offers its profiles, the app's Mouse view SHALL show:
- a Profile selector (Profile 1 to the profile count). Choosing one SHALL make it the mouse's active profile;
- a Report Rate selector offering 125, 250, 500 and 1000 Hz;
- one row per DPI step of the active profile, each with a number entry limited to the sensor's range and step, a way to mark it as the default step, and a way to remove it;
- a way to add a step, up to five.

**Sending edits:**
- The app SHALL send an edit of the active profile only once the user has made no further change for about 2 seconds, and SHALL then send it as a single request.
- It SHALL NOT replace the values the user is editing with the service's while an edit is waiting to be sent.
- With one step left, removing it SHALL NOT be possible.

#### Scenario: Change a DPI step
- **WHEN** the user changes the second DPI step from 2400 to 2000 and then stops
- **THEN** about 2 seconds later the app sends one `SetMouseProfile` for the active profile with 2000 as the second step

#### Scenario: Switch profile
- **WHEN** the user chooses Profile 1
- **THEN** the app sends `SetMouseActiveProfile(1)`, and shows profile 1's report rate and DPI steps

#### Scenario: No profiles offered
- **WHEN** the service offers no profiles (an older service, or a mouse whose profiles it can't read)
- **THEN** the profile, report-rate and DPI controls are hidden, the mouse illustration shows no button numbers or callouts, and the mouse lighting controls work as before

### Requirement: Mouse button controls
While the mouse is connected and the service offers its buttons, the app's Mouse view SHALL show a Buttons group for the active profile, with one row per button. Each row SHALL be named for the physical button and SHALL describe the button's current action.

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

### Requirement: Keyboard and Mouse views
The app SHALL separate the keyboard and the mouse into two views, Keyboard and Mouse, chosen with a view switcher.

**The switcher:**
- In a wide window it SHALL sit in the header bar.
- In a narrow window (under about 550 px wide) it SHALL move to a bar at the bottom of the window.
- It SHALL be usable with the keyboard alone, and each view SHALL have an accessible name.

**What each view holds:**
- **Keyboard:** the keyboard illustration, the keyboard's device row, the lighting controls with an "Edit Key Maps…" row, and the game switch.
- **Mouse:** the mouse illustration with its button callouts, the mouse's device row, the mouse lighting controls, the profile controls, and the button rows.

**Rules:**
- The app SHALL open on the Keyboard view.
- The switcher SHALL stay usable whatever either device's state, unless the lighting service isn't running.
- When the service doesn't report a mouse state (an older service), the Mouse view and the switcher SHALL be hidden, and the app SHALL show the Keyboard view alone.

#### Scenario: Switch to the mouse
- **WHEN** the user clicks "Mouse" in the header
- **THEN** the Mouse view is shown with the mouse illustration first, and no keyboard controls are visible

#### Scenario: Narrow window
- **WHEN** the window is made narrower than the switcher can fit
- **THEN** the switcher moves to a bar at the bottom and both views stay reachable

#### Scenario: Older service
- **WHEN** the app connects to a service without mouse properties
- **THEN** no switcher is shown and the app shows the keyboard controls as one view

### Requirement: Device illustrations
Each view SHALL begin with an illustration of its device on a dark inset card, drawn in one colour taken from the device's current lighting.

**Keyboard illustration:**
- It SHALL draw the G512's ANSI 104-key layout, with the same key positions as the key-map editor.
- Each lit key SHALL be a solid rounded tile in that key's colour, and each unlit key a rounded outline.
- Fn has no light and SHALL always be an outline.
- The case outline SHALL take the main lighting colour.
- With the lights off, every key and the case SHALL be dim outlines.
- Clicking or activating the illustration SHALL open the key-map editor.

**Mouse illustration:**
- It SHALL show the G502 HERO from the top and in a 3/4 side view.
- The outlines SHALL be in the mouse's lighting colour, with the DPI LEDs and the G logo as solid lit areas and a soft glow around them.
- With the mouse's lights off, it SHALL use dim outlines with no glow.

**Colour and access:**
- When the lighting colour would give less than 3:1 contrast against the card, the illustration SHALL use a lighter shade of the same hue. The device itself SHALL keep the real colour.
- Each illustration SHALL have an accessible name describing what it shows, for example "G512 keyboard, Ripple, green".

#### Scenario: Ripple in green
- **WHEN** the keyboard shows Ripple in green
- **THEN** the keyboard illustration's case and lit keys are green

#### Scenario: Custom map
- **WHEN** the keyboard shows a Custom map with only W, A, S and D lit in green
- **THEN** those four keys are green tiles and every other key is a dim outline

#### Scenario: Open Key Maps from the drawing
- **WHEN** the user clicks the keyboard illustration, or focuses it and presses Enter
- **THEN** the key-map editor opens

#### Scenario: Very dark colour
- **WHEN** the keyboard's colour is near-black
- **THEN** the illustration draws it in a lighter shade of the same hue that reaches 3:1 against the card

### Requirement: Illustration motion
While the system's animations are enabled, the illustrations SHALL preview the current effect and respond to interaction.

**Effect loops:**
- **Ripple:** a ring of lit keys spreads from a key and fades, about 2.4 s per ring.
- **Wave:** a band of lit keys sweeps from left to right in about 3 s.
- **Breathe:** every lit key fades between about 15% and full brightness over about 4 s.
- **Cycle:** the colour rotates through the hues in about 8 s.
- **Static and Custom:** no motion.

**Following the device:**
- The mouse illustration's LEDs, logo and glow SHALL follow the same effect and timing as the mouse's own lighting.
- The effect's Speed setting SHALL scale the loop length, so that a higher speed gives a shorter loop.

**Interaction:**
- Hovering or focusing a mouse button SHALL fill it within about 250 ms and pulse a ring out from it once.
- Applying a change to a button SHALL show one confirm pulse on it, then fade back after about a second.

**Reduced motion:** when the system's animations are turned off, no loops or pulses SHALL run. The illustrations SHALL show a still frame of the current lighting, and hover fills SHALL appear without animation.

#### Scenario: Breathe preview
- **WHEN** the keyboard's effect is Breathe and animations are enabled
- **THEN** the keyboard illustration's tiles fade up and down in a loop of about 4 seconds

#### Scenario: Animations off
- **WHEN** the user has turned off animations in GNOME's settings
- **THEN** the illustrations are still, with no effect loops or hover pulses

### Requirement: Mouse button callouts
The mouse illustration SHALL mark each programmable button with a number, and SHALL list the buttons beside it as callouts that show the same number, the button's name and its current action. The buttons are:
- Left Click, Right Click, Wheel Click;
- Wheel Tilt Left and Right;
- G9, the button behind the wheel;
- DPI Up and DPI Down;
- Sniper;
- Back (the front thumb button) and Forward (the rear thumb button).

**Interaction:**
- Clicking a button on the illustration, or activating its callout, SHALL open that button's Change dialog.
- Left Click SHALL be shown as fixed, and SHALL offer no way to change it.
- The callouts SHALL show the active profile's actions, and SHALL update when the active profile or an action changes, including a profile switch made on the mouse.

**Size:** each callout SHALL be at least 24 px tall, so it can be used where the drawn button is too small to click comfortably.

#### Scenario: Remap from the drawing
- **WHEN** the user clicks G9 on the mouse illustration
- **THEN** the Change dialog for G9 opens, and after Apply, G9's callout shows the new action

#### Scenario: Left click fixed
- **WHEN** the user looks at the Left Click callout
- **THEN** it reads "Left click · fixed" and activating it opens nothing
