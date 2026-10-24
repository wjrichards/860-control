## ADDED Requirements

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

## MODIFIED Requirements

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
