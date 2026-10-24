## MODIFIED Requirements

### Requirement: Device identification
The system SHALL treat a hidraw node as a lighting interface only when it matches a supported model:
- the G512 keyboard: USB ID `046d:c33c`, with a report descriptor declaring the Logitech vendor usage page `0xFF43` with report IDs `0x11` and `0x12`;
- the G502 HERO mouse: USB ID `046d:c08b`, with a report descriptor declaring the vendor usage page `0xFF00` with report ID `0x11`.

Other hidraw nodes, including each device's boot interface and other Logitech devices, SHALL be ignored.

#### Scenario: Lighting interface selected
- **WHEN** the G512 is connected and exposes two hidraw nodes, one with only a boot-keyboard descriptor and one with the `0xFF43` vendor page
- **THEN** the system opens only the node with the `0xFF43` vendor page

#### Scenario: Mouse interface selected
- **WHEN** the G502 HERO is connected and exposes two hidraw nodes, one with only a boot-mouse descriptor and one with the `0xFF00` vendor page declaring report `0x11`
- **THEN** the system opens only the node with the vendor page, as the mouse

#### Scenario: Other Logitech device ignored
- **WHEN** a Logitech device other than the G512 and the G502 HERO that also exposes HID++ reports is connected
- **THEN** the system does not open or write to that device's hidraw nodes

### Requirement: Apply zone effect
The system SHALL apply a lighting effect by sending ColorLedEffects `setZoneEffect` to every zone the device reports that offers the effect, using that zone's index for it and the effect's colour and period where that effect accepts them. Changes SHALL be sent as non-persistent (not written to the device's non-volatile memory). A successful reply from every zone SHALL be required before the effect is considered applied.

#### Scenario: Static colour applied
- **WHEN** the system applies Static with colour `#FF0000`
- **THEN** the whole keyboard lights red and the call reports success

#### Scenario: Both mouse zones
- **WHEN** the system applies Static `#00FF00` to the G502 HERO
- **THEN** `setZoneEffect` is sent to zone 0 (the logo) and zone 1 (the DPI strip), and both light green

#### Scenario: Apply rejected by device
- **WHEN** the device answers `setZoneEffect` with a HID++ error
- **THEN** the call reports the error and the effect is not recorded as applied
