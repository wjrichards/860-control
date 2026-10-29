## MODIFIED Requirements

### Requirement: Mouse button controls
While the mouse is connected and the service offers its buttons, the app's Mouse › Buttons page SHALL show the active profile's buttons, as callouts beside or below the illustration ("Mouse button callouts"), headed by the active profile ("Profile 4 · stored on the mouse"). Each callout SHALL be named for the physical button and SHALL describe the button's current action. A button that plays a macro SHALL read "Macro: " followed by the macro's name, or "Macro: Macro in slot N" when the macro has no name.

The left-click callout SHALL have no way to change it. Every other callout SHALL open a dialog to choose:
- a mouse button;
- a keyboard shortcut, set by pressing it;
- a macro, from a picker listing the macros on the mouse, then "New Macro…" (shown only while the service offers macros);
- a media key;
- a mouse function;
- disabled.

Media keys and "disabled" SHALL be offered only once they have been confirmed on the mouse.

Choosing an action SHALL send `SetMouseButton`. A key the app can't map to a HID usage SHALL be refused in the dialog. "New Macro…" SHALL open the macro editor; after the new macro is saved, the dialog SHALL return with it chosen.

#### Scenario: Set a shortcut
- **WHEN** the user opens G9's dialog, chooses Keyboard Shortcut, and presses Ctrl+C
- **THEN** the app sends `SetMouseButton` for G9 as key `c` with Ctrl, and G9's callout then reads "Ctrl+C"

#### Scenario: Put a macro on G9
- **WHEN** the user opens G9's dialog, chooses Macro, picks "Copy line" and applies
- **THEN** the app sends `SetMouseButton` for G9 as kind `macro` with that macro's sector, and G9's callout then reads "Macro: Copy line"

#### Scenario: Left click fixed
- **WHEN** the user looks at the Left Click callout
- **THEN** it shows "Left click · fixed" and offers no way to change it

#### Scenario: Profile switched on the mouse
- **WHEN** the user presses G9 on the mouse while it is set to cycle profiles
- **THEN** within a few seconds the app shows the new active profile and its buttons

### Requirement: Device sidebar
The app SHALL organise its pages in a sidebar, in the style of GNOME Settings, with one section per device.

**Sections and pages:**
- **Keyboard** (headed "Keyboard · G512"): Lighting, Key Maps, Games.
- **Mouse** (headed "Mouse · G502 HERO"): Lighting, DPI & Profiles, Buttons, Macros. The Macros row SHALL be shown only while the service offers the mouse's macros.
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

#### Scenario: Open Macros
- **WHEN** the mouse is connected and the user clicks "Macros" under Mouse
- **THEN** the Macros page is shown beside the sidebar, and "Macros" is marked as selected

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

#### Scenario: Service without macros
- **WHEN** the app connects to a service that offers mouse buttons but no macros
- **THEN** the Mouse section shows Lighting, DPI & Profiles and Buttons, and no Macros row

## ADDED Requirements

### Requirement: Macros page
While the mouse is connected and the service offers its macros, the app SHALL provide a Mouse › Macros page, in one centred column at most 640 px wide.

**On the Mouse** (group title, with "They work on other computers too, even with this app closed" under it, and a **New Macro** button beside it, or under it when there is no room):
- one row per macro, opening the macro editor;
- each row's title SHALL be the macro's name, or "Macro in slot N" when it has none;
- each row's subtitle SHALL read: the number of steps ("5 steps"), then how it plays ("Plays once", "Repeats while held", "Repeats until pressed again"), then where it is used ("G9 in profile 4", "G7 in profiles 1 and 2", or "Not on a button"), and "Made on another computer" for a macro without a name, separated by " · ".

**Mouse Memory** (group title):
- "N of 10 macro slots used" with the free count ("6 free") or "Full", and a level bar;
- under it: "Each macro takes one slot. The editor shows how much of it a macro uses."
- When every slot is used, New Macro SHALL be insensitive and the note SHALL read "Delete a macro you don't use to make room for a new one."

**No macros:** the page SHALL show a status page with the macros icon, "No Macros Yet", "Record keys and pauses, then put the macro on a mouse button. Macros are stored on the mouse, so they work anywhere.", and a suggested **New Macro** button. The memory group SHALL NOT be shown.

A step SHALL count as one editor row: a key pressed and released is one step.

#### Scenario: Four macros
- **WHEN** the mouse holds Copy line, Reload, Sprint toggle and an unnamed macro in slot 9
- **THEN** the page lists all four, the last as "Macro in slot 9 · … · Made on another computer", and the meter reads "4 of 10 macro slots used · 6 free"

#### Scenario: Empty
- **WHEN** the mouse holds no macros
- **THEN** the page shows "No Macros Yet" and a New Macro button, and no meter

#### Scenario: Full
- **WHEN** all 10 slots are used
- **THEN** the meter reads "Full", New Macro can't be activated, and the note tells the user to delete a macro first

#### Scenario: Narrow window
- **WHEN** the window is 360 px wide
- **THEN** the page opens from the list with Back, New Macro sits under the group title, and long subtitles wrap

### Requirement: Macro editor
The app SHALL edit a macro in a dialog, opened from a Macros page row, from New Macro, or from "New Macro…" in a button's Change dialog.

**Header:** Cancel, the macro's name ("New Macro" until it is saved), and a suggested **Save**.

**Settings group:**
- **Name**, an entry row;
- **Plays**, a drop-down offering "Once", "Repeats while held" and "Repeats until pressed again", limited to the play modes the service supports;
- **Used by** (an existing macro only): where it is used, as on the Macros page.

**Steps group:**
- the title "Steps", with "N steps · uses P% of its slot · drag to reorder" under it, where P is how much of one sector the encoded macro takes, rounded;
- a **Record** button beside the title;
- one row per step, as the "Macro step row" component:
  - a key or mouse-button step: "Press and release" with "Key" or "Mouse button", and the key or button in a keycap;
  - a key or button held around other steps: a "Hold down" row and a "Let go" row, each with its keycap;
  - a pause: "Pause" with "Wait before the next step", and its length in a spin button (milliseconds);
  - each row SHALL have a drag handle and a delete button, and a step's keycap SHALL open the shortcut capture to change it;
- an **Add Step** row at the end, opening a menu: "Key…", "Mouse Button…", "Media Key…", "Pause" (50 ms).

**Recording:**
- Record SHALL become a red **Stop** with the elapsed time ("Stop · 0:04").
- While recording, every key pressed in the app window SHALL be added as a step, and the time between keys as a pause of that length. Mouse clicks SHALL NOT be recorded.
- A last row SHALL read "Waiting for the next key…" with the slot use so far ("uses 6% so far").
- Name, Plays, Save and the existing steps SHALL be insensitive until Stop. Cancel SHALL stop and discard.
- Recording on an existing macro SHALL add the new steps at the end.

**Delete Macro** (an existing macro only): a destructive button under the steps, opening an alert "Delete "<name>"?". The alert SHALL name every button and profile that plays the macro and say it will do nothing until something else is chosen ("G9 in profile 4 plays this macro. After it's deleted, G9 does nothing until you choose something else for it."), with Cancel and a destructive Delete.

**Problems, each shown as a banner under the header:**
- too long: "This macro needs P% of a slot. Remove some steps to save it." Save SHALL be insensitive until it fits;
- the save failed: "Couldn't save to the mouse. Nothing on it changed, and your steps are still here." with **Try Again**;
- the mouse is unplugged: "Mouse disconnected. Plug it back in to save." Save and Delete Macro SHALL be insensitive. The steps SHALL stay, and recording SHALL still work.

The step rows' icon-only controls SHALL have accessible names ("Move step", "Delete step").

#### Scenario: Record Copy line
- **WHEN** the user creates a new macro, names it "Copy line", presses Record, types Home, Shift+End and Ctrl+C, and presses Stop
- **THEN** the steps list shows the three keys with the pauses between them as typed, and Save stores it on the mouse

#### Scenario: Edit a pause
- **WHEN** the user changes the first pause of Copy line to 120 ms and saves
- **THEN** the macro on the mouse waits 120 ms between Home and Shift+End

#### Scenario: Add a mouse click
- **WHEN** the user chooses Add Step › Mouse Button… › Left click
- **THEN** a "Press and release · Mouse button · Left" step is added at the end

#### Scenario: Too long
- **WHEN** the steps need more than one slot
- **THEN** the editor shows the "needs P% of a slot" banner and Save can't be activated

#### Scenario: Delete in use
- **WHEN** the user deletes Copy line, which G9 plays in profile 4, and confirms
- **THEN** the app calls `DeleteMouseMacro`, the macro leaves the list, and G9's callout reads "Disabled"
