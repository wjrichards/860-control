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
The app SHALL show each device's status within its own sidebar section.

**When a device is connected,** its Lighting page SHALL show its name, and for the keyboard its firmware version.

**When a device is `permission-denied` or `unsupported`:**
- its section SHALL stay in the sidebar, and each of its pages SHALL show a status page explaining the state;
- the other device's pages SHALL keep working;
- for `permission-denied`, the status page SHALL show a command that installs the udev rule with both the keyboard's and the mouse's lines, so that following it never removes the other device's access.

**When a device becomes `disconnected`:**
- its section SHALL leave the sidebar;
- if one of its pages was open, the app SHALL show the other device's first page;
- a toast SHALL say "Mouse disconnected" or "Keyboard disconnected".

**When no device is in the sidebar,** the window SHALL show a status page titled "No Devices Connected" with the text "Plug in the Logitech G512 keyboard or the G502 HERO mouse. Each one appears in the sidebar when it connects, and its saved lighting is applied."

**When the lighting service is not running,** the app SHALL show a status page over the whole window.

#### Scenario: Service not running
- **WHEN** the app starts and the lighting service is not on the session bus
- **THEN** the app shows a status page saying the service is not running and how to start it

#### Scenario: Permission denied
- **WHEN** the service reports `permission-denied` for the keyboard
- **THEN** the Keyboard section stays in the sidebar, and its pages show a status page with a udev command that writes the keyboard's and the mouse's rule lines

#### Scenario: Mouse without the keyboard
- **WHEN** the keyboard is unplugged and the mouse is connected
- **THEN** only the Mouse section is in the sidebar, and its pages show the mouse with all its controls

#### Scenario: Mouse unplugged while open
- **WHEN** Mouse › Buttons is open and the mouse is unplugged
- **THEN** the Mouse section leaves the sidebar, Keyboard › Lighting is shown, and a toast says "Mouse disconnected"

#### Scenario: No devices
- **WHEN** both devices are unplugged
- **THEN** the sidebar is hidden and the window shows "No Devices Connected"

### Requirement: Effect controls
The app SHALL list the effects the service reports as selectable.

**For the selected effect:**
- a colour control SHALL be shown only when the effect accepts a colour;
- a speed control SHALL be shown only when the effect accepts a speed, limited to the reported range;
- a brightness control from 0 to 100 percent SHALL always be shown while connected.

**When Custom is selected,** the app SHALL hide the colour and speed controls. The Key Maps page and the keyboard illustration SHALL open the editor on the Custom map.

#### Scenario: Cycle selected
- **WHEN** the user selects Cycle and the service reports that Cycle accepts a speed but no colour
- **THEN** the speed control is shown and the colour control is hidden

#### Scenario: Static selected
- **WHEN** the user selects Static
- **THEN** the colour control is shown and the speed control is hidden

#### Scenario: Custom selected
- **WHEN** the user selects Custom and then opens Key Maps
- **THEN** the colour and speed controls are hidden, brightness stays visible, and Key Maps opens on the Custom map

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
The app SHALL provide a key-map editor as the Keyboard › Key Maps page, reachable whatever effect is selected from the sidebar, from the keyboard illustration, and from "Edit the Gaming Map" on the Games page. The editor draws the ANSI 104-key layout, with each key in its current colour.

**Editing:**
- The user SHALL be able to select keys by clicking, by dragging across them, and with group buttons for WASD, arrows, F-keys, number row, numpad and modifiers.
- They SHALL be able to paint the selected keys with a chosen colour or turn them off.
- A switch SHALL choose whether the Custom or the Gaming map is being edited.
- The page SHALL say which map is shown while a game runs ("Shown while a game runs: Gaming").
- Edits SHALL be sent to the service as whole-map replacements, at most one every 100 ms while painting, with the final map always sent.

**Showing the selection:**
- Selected keys SHALL be marked with a white outline and a thin dark inner ring, so that the mark shows on every key colour.
- The hint below the keys SHALL say how many keys are selected.

**Layout:** the key grid's card SHALL take the page's free height.
- From 550 sp up, the keys SHALL shrink to fit the card's width, down to 60% of their full size; below that the grid SHALL scroll sideways. Key labels SHALL shrink with the keys.
- In a narrow window (under 550 sp) the keys SHALL keep their full size and the grid SHALL scroll sideways.
- While the grid scrolls, the hint SHALL say so.

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
- **THEN** Key Maps opens on the map being shown, or on Custom when neither map is shown

#### Scenario: Selection on a teal key
- **WHEN** a key painted in the accent colour is selected
- **THEN** its white outline with a dark inner ring is clearly visible

### Requirement: Game switch setting
The app SHALL provide a Keyboard › Games page.

**The page:**
- **Drawing:** the keyboard illustration showing the Gaming map, with the hint "Gaming map · click the keyboard to edit it in Key Maps".
- **"Games" group:** a "Switch to Gaming Layout While a Game Runs" switch bound to the service's game switch, with the note "Games must use GameMode, for example with the Steam launch option gamemoderun %command%".
- **"Now" group:**
  - a row stating "Game running" with "The keyboard shows the Gaming map", or "No game running" with "The keyboard shows its normal lighting", following the service's game-active state;
  - an "Edit the Gaming Map" row that opens Key Maps on the Gaming map.

**While the Gaming map is shown because a game is running,** the app SHALL show the banner "Gaming layout active while a game runs".

#### Scenario: Turn the switch on
- **WHEN** the user turns the setting on
- **THEN** the service's game switch is on, and other front ends see the change

#### Scenario: Game running
- **WHEN** the service reports that the Gaming map is shown because a game is running
- **THEN** the app shows the banner, and the Now row reads "Game running"

#### Scenario: Edit the Gaming map
- **WHEN** the user activates "Edit the Gaming Map"
- **THEN** Key Maps opens with Gaming selected

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
When the service reports a mouse state, the app's Mouse › Lighting page SHALL contain:
- the mouse illustration;
- a "Lighting" group:
  - a "Match Keyboard" switch bound to `MouseMatch`;
  - while matching is off, effect, colour, speed and brightness controls for the mouse's own setting. They SHALL show only what the chosen mouse effect accepts, and send `SetMouseLighting` with the same throttling as the keyboard controls;
- a "Device" group with the mouse's name.

The Mouse pages SHALL work whether or not the keyboard is connected. When the service doesn't report a mouse state, the Mouse section SHALL be hidden.

#### Scenario: Matching
- **WHEN** the mouse is connected and matching the keyboard
- **THEN** Mouse › Lighting shows the mouse's name and the switch on, and hides the effect controls

#### Scenario: Own colour
- **WHEN** the user turns Match Keyboard off and chooses Static blue
- **THEN** the app sends `SetMouseMatch(false)` and then `SetMouseLighting` with Static blue, and the mouse turns blue while the keyboard keeps its own lighting

#### Scenario: Mouse unplugged
- **WHEN** the mouse is unplugged while the app is open
- **THEN** the Mouse section leaves the sidebar, and the Keyboard pages are unaffected

#### Scenario: Older service
- **WHEN** the app connects to a service without mouse properties
- **THEN** the Mouse section is hidden

### Requirement: Mouse profile controls
While the mouse is connected and the service offers its profiles, the app's Mouse › DPI & Profiles page SHALL show:
- **the mouse illustration,** with only buttons 6 to 9 numbered, and a caption giving each one's current action (by default "6 next profile · 7 DPI up · 8 DPI down · 9 DPI shift");
- **a "Profile" group, described as "Stored on the mouse":**
  - a Profile selector (Profile 1 to the profile count). Choosing one SHALL make it the mouse's active profile;
  - a Report Rate selector offering 125, 250, 500 and 1000 Hz;
- **a "DPI Steps" group, described as "The DPI buttons cycle through these steps. The selected one is the default.":**
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

#### Scenario: Remapped button in the caption
- **WHEN** G9 (button 6) is set to Ctrl+C
- **THEN** the DPI & Profiles caption reads "6 Ctrl+C" in place of "6 next profile"

#### Scenario: No profiles offered
- **WHEN** the service offers no profiles (an older service, or a mouse whose profiles it can't read)
- **THEN** the DPI & Profiles and Buttons rows are hidden from the sidebar, and Mouse › Lighting works as before

### Requirement: Mouse button controls
While the mouse is connected and the service offers its buttons, the app's Mouse › Buttons page SHALL show the active profile's buttons, as callouts beside or below the illustration ("Mouse button callouts"), headed by the active profile ("Profile 4 · stored on the mouse"). Each callout SHALL be named for the physical button and SHALL describe the button's current action.

The left-click callout SHALL have no way to change it. Every other callout SHALL open a dialog to choose:
- a mouse button;
- a keyboard shortcut, set by pressing it;
- a media key;
- a mouse function;
- disabled.

Media keys and "disabled" SHALL be offered only once they have been confirmed on the mouse.

Choosing an action SHALL send `SetMouseButton`. A key the app can't map to a HID usage SHALL be refused in the dialog.

#### Scenario: Set a shortcut
- **WHEN** the user opens G9's dialog, chooses Keyboard Shortcut, and presses Ctrl+C
- **THEN** the app sends `SetMouseButton` for G9 as key `c` with Ctrl, and G9's callout then reads "Ctrl+C"

#### Scenario: Left click fixed
- **WHEN** the user looks at the Left Click callout
- **THEN** it shows "Left click · fixed" and offers no way to change it

#### Scenario: Profile switched on the mouse
- **WHEN** the user presses G9 on the mouse while it is set to cycle profiles
- **THEN** within a few seconds the app shows the new active profile and its buttons

### Requirement: Device illustrations
Pages SHALL show their device as an illustration on an inset card, drawn in the device's current lighting.

**The card:**
- In the dark style the card SHALL be dark (#1D1D20).
- In the light style it SHALL be light (#F6F6F7) with a faint border, like the illustrations in GNOME Settings.

**Keyboard illustration** (shown on Keyboard › Lighting, Keyboard › Games and the narrow list):
- It SHALL draw the G512's ANSI 104-key layout, with the same key positions as the key-map editor.
- Each lit key SHALL be a solid rounded tile in that key's colour, and each unlit key a rounded outline.
- Fn has no light and SHALL always be an outline.
- The case outline SHALL take the main lighting colour, or a map's most common colour.
- With the lights off, every key and the case SHALL be dim outlines.
- On the Games page it SHALL show the Gaming map, whatever the keyboard is showing.
- Clicking or activating it SHALL open Key Maps.

**Mouse illustration** (shown on every Mouse page and the narrow list):
- It SHALL use the user's G502 HERO ink drawings, with the top view stacked above the side view.
- It SHALL be lit by layer:
  - the outlines in the mouse's lighting colour;
  - the DPI LEDs and the logo as solid areas in the live LED colour.
- There SHALL be no glow and no cut-out in the logo.
- With the mouse's lights off, it SHALL use dim outlines, and the LEDs and logo SHALL be dark.

**Colour and access:**
- When a lighting colour would give less than 3:1 contrast against the card, the illustration SHALL use a shade of the same hue that reaches 3:1: lighter on the dark card, darker on the light card. The device itself SHALL keep the real colour.
- Each illustration SHALL have an accessible name describing what it shows, for example "G512 keyboard, Ripple, green".

#### Scenario: Ripple in green
- **WHEN** the keyboard shows Ripple in green
- **THEN** the keyboard illustration's case and lit keys are green

#### Scenario: Custom map
- **WHEN** the keyboard shows a Custom map with only W, A, S and D lit in green
- **THEN** those four keys are green tiles and every other key is a dim outline

#### Scenario: Open Key Maps from the drawing
- **WHEN** the user clicks the keyboard illustration, or focuses it and presses Enter
- **THEN** the Key Maps page opens

#### Scenario: Very dark colour
- **WHEN** the keyboard's colour is near-black in the dark style
- **THEN** the illustration draws it in a lighter shade of the same hue that reaches 3:1 against the card

#### Scenario: Light style
- **WHEN** the system uses the light style and the mouse is white
- **THEN** the mouse card is light, and the outlines, LEDs and logo are drawn in a darker grey that reaches 3:1 against it

#### Scenario: Mouse lit by layer
- **WHEN** the mouse shows Static blue
- **THEN** the mouse illustration's outlines, DPI LEDs and logo are blue, with no glow

### Requirement: Illustration motion
While the system's animations are enabled, the illustrations SHALL preview the current effect and respond to interaction.

**Effect loops:**
- **Ripple:** a ring of lit keys spreads from a key and fades, about 2.4 s per ring.
- **Wave:** a band of lit keys sweeps from left to right in about 3 s.
- **Breathe:** every lit key fades between about 15% and full brightness over about 4 s.
- **Cycle:** the colour rotates through the hues in about 8 s.
- **Static and Custom:** no motion.

**Following the device:**
- The mouse illustration's LEDs and logo SHALL follow the same effect and timing as the mouse's own lighting.
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
On the Buttons page, the mouse illustration SHALL mark each programmable button with a number, and SHALL list the buttons as callouts that show the same number, the button's name and its current action. The buttons are:
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
- The callouts SHALL be laid out as "Fluid page layout" describes for the Buttons page.

**Size:** each callout SHALL be at least 24 px tall, so it can be used where the drawn button is too small to click comfortably.

#### Scenario: Remap from the drawing
- **WHEN** the user clicks G9 on the mouse illustration
- **THEN** the Change dialog for G9 opens, and after Apply, G9's callout shows the new action

#### Scenario: Left click fixed
- **WHEN** the user looks at the Left Click callout
- **THEN** it reads "Left click · fixed" and activating it opens nothing

### Requirement: Device sidebar
The app SHALL organise its pages in a sidebar, in the style of GNOME Settings, with one section per device.

**Sections and pages:**
- **Keyboard** (headed "Keyboard · G512"): Lighting, Key Maps, Games.
- **Mouse** (headed "Mouse · G502 HERO"): Lighting, DPI & Profiles, Buttons.
- Each row SHALL show an icon and the page name, and SHALL be reachable with the keyboard alone.
- The selected row SHALL be marked as selected.

**Which sections are shown:**
- A device's section SHALL be shown only while the service reports that device as connected, permission-denied or unsupported. It SHALL NOT be shown while the device is disconnected.
- The Mouse section SHALL be hidden when the service doesn't report a mouse state (an older service).
- With no section left, the sidebar SHALL be hidden and the window SHALL show "No Devices Connected" (see "Device and service status").

**Wide and narrow windows:**
- From 550 sp wide, the sidebar SHALL sit beside the page.
- Under 550 sp, the sidebar list SHALL become its own page.
  - Choosing a row SHALL open that page with a Back button that returns to the list.
  - There SHALL be no bottom bar.
- In the narrow list, each device header SHALL be followed by that device's drawing, showing its live lighting. The last section's drawing SHALL take the remaining height.

**Key Maps folds the sidebar away:** on Key Maps, from 550 sp to 1200 sp, the sidebar SHALL fold away so the keyboard gets the window's width.
- A sidebar button at the start of the Key Maps header SHALL slide the sidebar in over the page, which dims behind it. Key Maps SHALL stay marked as the open row.
- Choosing a row SHALL open that page and close the sidebar. Clicking the dimmed page or pressing Escape SHALL close it and stay on Key Maps.
- When the sidebar slides in, keyboard focus SHALL move to the open page's row; when it closes, focus SHALL return to the sidebar button.
- Every other page SHALL keep the sidebar beside it from 550 sp up.

**Start-up:** the app SHALL open on the first page of the first section shown: Keyboard › Lighting, or Mouse › Lighting when only the mouse is present.

#### Scenario: Open a mouse page
- **WHEN** both devices are connected and the user clicks "Buttons" under Mouse in the sidebar
- **THEN** the Buttons page is shown beside the sidebar, and "Buttons" is marked as selected

#### Scenario: Narrow window
- **WHEN** the window is narrower than 550 sp
- **THEN** the sidebar list fills the window with a drawing under each device header, and choosing "Games" opens the Games page with a Back button

#### Scenario: Key Maps in an 860 px window
- **WHEN** the window is 860 px wide and Key Maps is open
- **THEN** the sidebar is folded away, the whole keyboard shows at full key size, and the sidebar button slides the sidebar in over the dimmed page

#### Scenario: Only the mouse
- **WHEN** the app starts with the mouse connected and the keyboard unplugged
- **THEN** the sidebar shows only the Mouse section and the app opens on Mouse › Lighting

#### Scenario: Older service
- **WHEN** the app connects to a service without mouse properties
- **THEN** the sidebar shows only the Keyboard section

### Requirement: Fluid page layout
Pages SHALL use the window's space at every size.

**Drawings:**
- A page's drawing card SHALL take the height its settings leave free.
- The drawing SHALL scale to fit both the card's width and its height, keeping its proportions.
- The keyboard drawing SHALL NOT shrink below its natural height for the width. The taller mouse drawing MAY shrink to half its natural height, and in the narrow sidebar list further, so the rows below it stay in view. Beyond that, the page SHALL scroll.

**Columns:**
- From about 1000 sp wide, a page's settings groups SHALL sit in two columns. Below that they SHALL sit in one column.
- On the mouse pages, the drawing card SHALL sit beside the groups when wide, and above them when narrow.

**Mouse button labels (Buttons page):**
- At 1000 sp or wider: beside the drawing, in two columns.
- From 720 sp to 1000 sp: below the drawing, in three columns.
- Under 720 sp: below the drawing, in two columns.

**Fitting:** from 550 sp up, no page SHALL ask for more width than the window has.

**Window:** the default window size SHALL be 860 × 780.

#### Scenario: Wide Keyboard › Lighting
- **WHEN** the window is 1280 × 860 and Keyboard › Lighting is shown
- **THEN** the Lighting and Keyboard groups sit side by side and the keyboard drawing fills the height above them, with no empty band below the groups

#### Scenario: Default size
- **WHEN** the app opens at its default size of 860 × 780
- **THEN** Keyboard › Lighting is shown in full without scrolling

#### Scenario: Buttons at the default size
- **WHEN** Mouse › Buttons is shown in an 860 px wide window
- **THEN** the eleven button labels sit below the drawing in three columns
