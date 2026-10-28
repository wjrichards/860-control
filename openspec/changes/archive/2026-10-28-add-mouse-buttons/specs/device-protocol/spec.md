## ADDED Requirements

### Requirement: Onboard profile buttons
The system SHALL read a format-2 profile's button table: one 4-byte entry per button reported by `getInfo`, starting at byte `0x20`. Each entry SHALL decode as one of:
- a mouse-button bitmask (`80 01 hi lo`);
- a key with modifier bits and a HID usage (`80 02 mm kk`);
- a consumer usage (`80 03 hi lo`);
- a special function (`90 cc …`);
- disabled;
- any other entry, which SHALL be kept unchanged.

It SHALL change a button only by rewriting that entry and the CRC, through the verified whole-sector write. It SHALL refuse:
- any change to entry 0 (left click);
- writing an entry it decoded as "other".

#### Scenario: Decode the real profiles
- **WHEN** the system reads profile 3 from the user's mouse backup
- **THEN** entries 6 and 7 decode as Ctrl+C and Ctrl+V, and entry 5 as G-shift

#### Scenario: Remap one button
- **WHEN** the system sets entry 8 (G9) of profile 5 to the key F13 without modifiers
- **THEN** only bytes `0x40`–`0x43` and the CRC change, and the sector reads back as written

#### Scenario: Left click protected
- **WHEN** any code path tries to change entry 0
- **THEN** the change is refused before anything is sent to the mouse
