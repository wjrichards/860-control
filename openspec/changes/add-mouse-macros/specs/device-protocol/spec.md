## MODIFIED Requirements

### Requirement: Onboard profile memory
The system SHALL read the G502 HERO's onboard profiles with OnboardProfiles (`0x8100`) `getInfo` and `memoryRead`. It SHALL handle profile format 2 only:
- report rate in milliseconds at byte 0;
- default DPI index at byte 1;
- up to five DPI values as little-endian 16-bit numbers at bytes 3–12;
- a CRC-CCITT (initial value `0xFFFF`) over all but the last two bytes, stored big-endian in the last two.

It SHALL write only whole sectors, with `memoryAddrWrite`, then `memoryWrite` in 16-byte parts, then `memoryWriteEnd`, and only to:
- **a user profile sector** (sectors 1 to the profile count reported by `getInfo`), when the new content has a correct CRC and only the report rate, default DPI index, DPI values, switched DPI index, button entries and CRC differ from what was read;
- **a macro sector**: a sector above the profile count and below the sector count reported by `getInfo` (sectors 6 to `0x0f` on the G502 HERO). The new content SHALL be one macro, as "Onboard macros" describes, padded with `ff` and ending in a correct CRC.

It SHALL never write the profile directory (sector 0) or a ROM sector (`0x0100` and above).

A write SHALL count as done only when reading the sector back returns exactly what was written.

#### Scenario: Edit a profile's DPI steps
- **WHEN** the system sets profile 5's DPI steps to 800, 1600 and 3200 with 1600 as default
- **THEN** sector 5 is rewritten with those values, a correct CRC and every other byte unchanged, and reading it back matches

#### Scenario: Write a macro sector
- **WHEN** the system writes a two-key macro to sector 6 of the user's mouse
- **THEN** sector 6 holds the macro, `ff` padding and a correct CRC, and reading it back matches

#### Scenario: Directory or ROM sector
- **WHEN** any code path tries to write sector 0 or sector `0x0101`
- **THEN** it is refused before anything is sent to the device

#### Scenario: Sector outside both ranges
- **WHEN** any code path tries to write sector `0x10` on a mouse that reports 16 sectors
- **THEN** it is refused before anything is sent to the device

#### Scenario: Read-back differs
- **WHEN** the sector read back after a write differs from what was written
- **THEN** the write is reported as failed

#### Scenario: Unknown profile format
- **WHEN** `getInfo` reports a profile format other than 2
- **THEN** profiles are reported as unsupported and nothing is written

### Requirement: Onboard profile buttons
The system SHALL read a format-2 profile's button table: one 4-byte entry per button reported by `getInfo`, starting at byte `0x20`. Each entry SHALL decode as one of:
- a mouse-button bitmask (`80 01 hi lo`);
- a key with modifier bits and a HID usage (`80 02 mm kk`);
- a consumer usage (`80 03 hi lo`);
- a special function (`90 cc …`);
- a macro, as the sector and byte offset it starts at (`00 ss 00 oo`);
- disabled;
- any other entry, which SHALL be kept unchanged.

It SHALL change a button only by rewriting that entry and the CRC, through the verified whole-sector write. It SHALL refuse:
- any change to entry 0 (left click);
- writing an entry it decoded as "other";
- a macro entry that doesn't point at offset 0 of a macro sector.

#### Scenario: Decode the real profiles
- **WHEN** the system reads profile 3 from the user's mouse backup
- **THEN** entries 6 and 7 decode as Ctrl+C and Ctrl+V, and entry 5 as G-shift

#### Scenario: Remap one button
- **WHEN** the system sets entry 8 (G9) of profile 5 to the key F13 without modifiers
- **THEN** only bytes `0x40`–`0x43` and the CRC change, and the sector reads back as written

#### Scenario: Point a button at a macro
- **WHEN** the system sets entry 8 (G9) of profile 4 to the macro in sector 6
- **THEN** bytes `0x40`–`0x43` read `00 06 00 00`, only they and the CRC change, and the sector reads back as written

#### Scenario: Left click protected
- **WHEN** any code path tries to change entry 0
- **THEN** the change is refused before anything is sent to the mouse

## ADDED Requirements

### Requirement: Onboard macros
The system SHALL read and write macros in the format the mouse reports as macro format 1, one macro per macro sector, starting at byte 0.

**A macro SHALL be, in order:**
- at most one play-mode command: none (play once), `01` (press and release parts: the steps after it play when the button is released), `02` (repeat while held) or `03` (repeat until pressed again);
- its steps, each three bytes:
  - `43 mm kk` / `44 mm kk`: key down / up, with modifier bits and a HID keyboard usage;
  - `41 hi lo` / `42 hi lo`: mouse button down / up, as a button bitmask;
  - `45 hi lo` / `46 hi lo`: media key down / up, as a consumer usage;
  - `40 hi lo`: pause, in milliseconds, big-endian, from 1 to 65535;
- the end command `ff`.

**Limits:**
- the system SHALL refuse a macro that, with its end command, doesn't fit in the sector size less the two CRC bytes (254 bytes on the G502 HERO);
- it SHALL NOT write the jump command (`60`), so no macro continues into another sector;
- a play mode SHALL be offered only once it has been confirmed on the user's mouse.

**Reading:** a sector whose CRC is correct and whose bytes decode as above SHALL count as holding a macro. A sector that is all `ff`, or whose CRC is wrong, SHALL count as free. A sector with a correct CRC but commands the system doesn't know SHALL count as used and SHALL be kept unchanged.

#### Scenario: Encode Copy line
- **WHEN** the system encodes Home, a 50 ms pause, Shift+End, a 50 ms pause and Ctrl+C, played once
- **THEN** the sector starts `43 00 4a 44 00 4a 40 00 32 43 02 4d 44 02 4d 40 00 32 43 01 06 44 01 06 ff`, and the rest is `ff` padding and the CRC

#### Scenario: Too long
- **WHEN** a macro's commands and end command take more than 254 bytes
- **THEN** it is refused before anything is sent to the mouse

#### Scenario: Unknown commands kept
- **WHEN** a macro sector with a correct CRC contains a command byte the system doesn't know
- **THEN** the sector is reported as used by a macro it can't show, and it is never rewritten except by deleting that macro
