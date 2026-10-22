# Spec Delta

## Purpose

Defines how the controller talks to the Logitech G512 RGB keyboard over HID++ 2.0: finding the right interface, resolving features, limiting which features can ever be called, discovering effects, and applying a lighting effect.

## ADDED Requirements

### Requirement: Device identification
The system SHALL treat a hidraw node as a G512 lighting interface only when its USB ID is `046d:c33c` and its report descriptor declares the Logitech vendor usage page `0xFF43` with report IDs `0x11` and `0x12`. Other hidraw nodes, including the keyboard's boot-keyboard interface and other Logitech devices, SHALL be ignored.

#### Scenario: Lighting interface selected
- **WHEN** the G512 is connected and exposes two hidraw nodes, one with only a boot-keyboard descriptor and one with the `0xFF43` vendor page
- **THEN** the system opens only the node with the `0xFF43` vendor page

#### Scenario: Other Logitech device ignored
- **WHEN** a Logitech mouse that also exposes HID++ reports is connected
- **THEN** the system does not open or write to the mouse's hidraw nodes

### Requirement: Runtime feature resolution
The system SHALL resolve the slot of every HID++ feature it uses by asking the device (Root `getFeature`) each time the device is opened, and SHALL NOT rely on fixed slot numbers.

#### Scenario: Slot differs from the observed firmware
- **WHEN** a firmware version places ColorLedEffects (`0x8070`) in a slot other than `0x0d`
- **THEN** the system uses the slot the device reports and lighting still works

#### Scenario: Required feature missing
- **WHEN** the device reports that ColorLedEffects (`0x8070`) is not present
- **THEN** the system reports the device as unsupported and sends no lighting commands

### Requirement: Feature allowlist
The system SHALL send HID++ requests only to these features: Root (`0x0000`), FeatureSet (`0x0001`), DeviceFwVersion (`0x0003`), DeviceName (`0x0005`), ColorLedEffects (`0x8070`) and PerKeyLighting (`0x8080`). A request for any other feature SHALL be rejected before anything is written to the device. In particular, firmware update (`0x00c2`), device reset (`0x1802`), and features the device flags as hidden or engineering-only SHALL be unreachable.

#### Scenario: Disallowed feature requested
- **WHEN** any code path attempts a request to feature `0x00c2` (DfuControlSigned)
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
The system SHALL apply a lighting effect by sending ColorLedEffects `setZoneEffect` for the effect's zone and index, with the effect's colour and period where that effect accepts them. Changes SHALL be sent as non-persistent (not written to the keyboard's non-volatile memory). A successful reply SHALL be required before the effect is considered applied.

#### Scenario: Static colour applied
- **WHEN** the system applies Static with colour `#FF0000`
- **THEN** the whole keyboard lights red and the call reports success

#### Scenario: Apply rejected by device
- **WHEN** the device answers `setZoneEffect` with a HID++ error
- **THEN** the call reports the error and the effect is not recorded as applied
