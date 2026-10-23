## ADDED Requirements

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
