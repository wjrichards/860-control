# device-protocol Specification

## Purpose
Defines how the controller talks to the Logitech G512 RGB keyboard over HID++ 2.0: finding the right interface, resolving features, limiting which features can ever be called, discovering effects, and applying a lighting effect.

## Requirements

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

### Requirement: Runtime feature resolution
The system SHALL resolve the slot of every HID++ feature it uses by asking the device (Root `getFeature`) each time the device is opened, and SHALL NOT rely on fixed slot numbers.

#### Scenario: Slot differs from the observed firmware
- **WHEN** a firmware version places ColorLedEffects (`0x8070`) in a slot other than `0x0d`
- **THEN** the system uses the slot the device reports and lighting still works

#### Scenario: Required feature missing
- **WHEN** the device reports that ColorLedEffects (`0x8070`) is not present
- **THEN** the system reports the device as unsupported and sends no lighting commands

### Requirement: Feature allowlist
The system SHALL send HID++ requests only to these features: Root (`0x0000`), FeatureSet (`0x0001`), DeviceFwVersion (`0x0003`), DeviceName (`0x0005`), ColorLedEffects (`0x8070`), PerKeyLighting (`0x8080`), AdjustableDpi (`0x2201`) and OnboardProfiles (`0x8100`). Three of these features SHALL be further limited to listed functions:
- PerKeyLighting: functions 3 and 5;
- AdjustableDpi: functions 0, 1 and 2, all reads;
- OnboardProfiles: functions 0, 2, 3, 4, 5, 6, 7, 8 and 11. Never function 1 (set onboard mode).

A request for any other feature or function SHALL be rejected before anything is written to the device. In particular, firmware update (`0x00c2`), device reset (`0x1802`), and features the device flags as hidden or engineering-only SHALL be unreachable.

#### Scenario: Disallowed feature requested
- **WHEN** any code path attempts a request to feature `0x00c2` (DfuControlSigned)
- **THEN** the request fails with an allowlist error and no bytes are written to the device

#### Scenario: Disallowed function requested
- **WHEN** any code path attempts OnboardProfiles function 1 (set onboard mode)
- **THEN** the request fails with an allowlist error and no bytes are written to the device

#### Scenario: Allowed feature requested
- **WHEN** the system requests ColorLedEffects `getInfo`
- **THEN** the request is sent and its reply is returned

### Requirement: Request and reply handling
Each request SHALL be matched to its reply by feature slot, function number and software ID. A HID++ error reply SHALL be reported as an error carrying the device's error code. If no matching reply arrives within 1 second, the request SHALL fail with a timeout error. Unrelated reports read in the meantime (e.g. key events or notifications) SHALL be skipped, not treated as replies.

#### Scenario: Device returns an error
- **WHEN** the device answers a request with a HID++ error report
- **THEN** the caller receives an error that includes the HID++ error code

#### Scenario: Device does not answer
- **WHEN** no matching reply arrives within 1 second
- **THEN** the caller receives a timeout error and the device handle remains usable

### Requirement: Effect discovery
The system SHALL discover the lighting zones and their effects from the device using ColorLedEffects getter functions only. Each discovered effect SHALL carry its effect ID, capability bits and default period as reported by the device. Only effects whose IDs map to a known effect (Off `0x0000`, Static `0x0001`, Cycle `0x0003`, Wave `0x0004`, Breathe `0x000a`, Ripple `0x000b`) SHALL be offered for selection; any other reported effect SHALL be kept as unrecognised and not offered.

#### Scenario: Effects on the observed firmware
- **WHEN** the device (firmware U1 19.01) reports one zone with effect IDs `0x0000, 0x0001, 0x000a, 0x0003, 0x0004, 0x0005, 0x000b`
- **THEN** the selectable effects are Off, Static, Breathe, Cycle, Wave and Ripple, and `0x0005` is listed as unrecognised

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

### Requirement: Per-key key set
The system SHALL address single keys by their USB HID keyboard usage ID. The keys it accepts SHALL be the ANSI 104-key set:
- letters, digits and `Enter` through `\` (`0x04`–`0x31`);
- `;` through `Caps Lock` (`0x33`–`0x39`);
- `F1`–`F12` (`0x3a`–`0x45`);
- `Print Screen` through `Up` (`0x46`–`0x52`);
- `Num Lock` through keypad `.` (`0x53`–`0x63`);
- `Menu` (`0x65`);
- the eight modifiers (`0xe0`–`0xe7`).

Any other key ID SHALL be rejected before anything is sent.

#### Scenario: Known key accepted
- **WHEN** a per-key frame includes Left Ctrl (`0xe0`)
- **THEN** the frame is accepted

#### Scenario: ISO-only key rejected
- **WHEN** a per-key frame includes the ISO key `0x64`
- **THEN** the frame is rejected with an invalid-key error and nothing is sent to the device

### Requirement: Per-key frames
The system SHALL set per-key colours with PerKeyLighting (`0x8080`) function 3 ("set keys"), followed by function 5 ("commit"). It SHALL NOT call any other PerKeyLighting function.

Each "set keys" request SHALL be a long report (`0x12`) carrying the header `00 01 00 0e` and up to 14 entries of key ID, red, green and blue. Unused entries SHALL be zero. A frame SHALL be applied only when every "set keys" request and the commit have been acknowledged.

#### Scenario: Full keyboard frame
- **WHEN** the system sends a frame with all 104 keys
- **THEN** it sends 8 "set keys" requests and one commit, and the whole keyboard shows the frame's colours

#### Scenario: Frame rejected by device
- **WHEN** the device answers any request of a frame with a HID++ error
- **THEN** the frame is reported as failed with that error code

### Requirement: Per-key frame replaces a running effect
Per-key frames and zone effects SHALL be treated as two exclusive lighting modes. A frame stops any running zone effect, and a zone effect replaces any frame. To show a frame, the system SHALL send it without first sending a zone effect.

#### Scenario: Frame over Wave
- **WHEN** Wave is running and the system sends a per-key frame
- **THEN** the Wave stops and the keyboard shows the frame

### Requirement: Onboard profile memory
The system SHALL read the G502 HERO's onboard profiles with OnboardProfiles (`0x8100`) `getInfo` and `memoryRead`. It SHALL handle profile format 2 only:
- report rate in milliseconds at byte 0;
- default DPI index at byte 1;
- up to five DPI values as little-endian 16-bit numbers at bytes 3–12;
- a CRC-CCITT (initial value `0xFFFF`) over all but the last two bytes, stored big-endian in the last two.

It SHALL write a profile only as a whole user sector, with `memoryAddrWrite`, then `memoryWrite` in 16-byte parts, then `memoryWriteEnd`, and only when:
- the sector is one of the user profiles reported by `getInfo` (sectors 1 to the profile count);
- never the profile directory (sector 0) or a ROM sector (`0x0100` and above);
- the new content has a correct CRC;
- only the report rate, default DPI index, DPI values, switched DPI index and CRC differ from what was read.

A write SHALL count as done only when reading the sector back returns exactly what was written.

#### Scenario: Edit a profile's DPI steps
- **WHEN** the system sets profile 5's DPI steps to 800, 1600 and 3200 with 1600 as default
- **THEN** sector 5 is rewritten with those values, a correct CRC and every other byte unchanged, and reading it back matches

#### Scenario: Directory or ROM sector
- **WHEN** any code path tries to write sector 0 or sector `0x0101`
- **THEN** it is refused before anything is sent to the device

#### Scenario: Read-back differs
- **WHEN** the sector read back after a write differs from what was written
- **THEN** the write is reported as failed

#### Scenario: Unknown profile format
- **WHEN** `getInfo` reports a profile format other than 2
- **THEN** profiles are reported as unsupported and nothing is written

### Requirement: Profile switching
The system SHALL switch the active onboard profile with OnboardProfiles `setCurrentProfile`, addressing profile N as `00 N` for N from 1 to the profile count, and read it back with `getCurrentProfile`. It SHALL NOT change the onboard mode.

#### Scenario: Switch to profile 1
- **WHEN** the system sets the current profile to 1 on the user's mouse
- **THEN** `getCurrentProfile` reads `00 01`, the DPI becomes profile 1's default step, and the mode stays onboard
