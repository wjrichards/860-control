# Design

## Context

- **Spike 6** (`evidence/mouse.md`), on the G502 HERO (`046d:c08b`, firmware U1 27.03):
  - HID++ is on interface 1, vendor page `0xFF00`, reports `0x10` and `0x11`.
  - `0x8070` v3 has 2 zones: zone 0 (location 1) is the logo, zone 1 (location 2) the DPI strip.
  - Both zones offer Off `0x0000`, Static `0x0001`, Breathe `0x000a` and Cycle `0x0003`, the same IDs as the G512. Breathe and Cycle default to period 992.
  - `setZoneEffect` with persistence 0 works while the mouse stays in onboard mode (mode 1). A DPI press doesn't reset it.
- **The protocol crate:**
  - `discovery::is_lighting_interface` accepts only `046d:c33c` with page `0xFF43` and reports `0x11` and `0x12`;
  - `effects::discover` returns all zones;
  - `g512d` uses `zones[0]` only;
  - the allowlist is Root, FeatureSet, DeviceFwVersion, DeviceName, ColorLedEffects and PerKeyLighting.
- **The service:** `Controller` owns one keyboard connection. `due_output()` decides what the keyboard shows (lights, game override, saved state). Hotplug, resume and the login retry all reconnect "the keyboard".

## Goals / Non-Goals

**Goals:**
- One decision of what the mouse shows, derived from the same inputs as the keyboard's: lights, game, saved state and the mouse settings.
- The keyboard and mouse connect, disconnect and fail independently. A missing mouse never affects the keyboard.
- No new HID++ features, and no change of onboard mode.

**Non-Goals:**
- Any `0x2201`, `0x8060` or `0x8100` calls (later changes).
- Separate settings for the two zones.

## Decisions

### D1. Supported models are a table in `discovery`

```rust
pub enum Model { G512, G502Hero }
// product id, HID++ usage page, report IDs the interface must declare
G512:     0xc33c, 0xFF43, [0x11, 0x12]
G502Hero: 0xc08b, 0xFF00, [0x11]
```

- `find_lighting_interface(sysfs, model)` scans for one model.
- `is_lighting_interface(uevent, descriptor) -> Option<Model>` replaces the bool.
- Every other node is ignored, as before.
- The `Device` type and the allowlist don't change: the mouse uses only long (`0x11`) requests.

### D2. Effects go to every zone

`effects::set_effect_all(dev, zones, kind, params)` sends `setZoneEffect` to each zone that offers the effect, in zone order. It succeeds only if every zone accepts.

The keyboard keeps calling it with its one zone, so nothing changes for the keyboard. The fake device takes a zone count, so tests can model the mouse.

### D3. The mouse lives inside `Controller`, as a second connection

```rust
struct MouseSide<O: Opener> { opener: O, conn: Option<Connection<..>>, status: DeviceStatus }
// in Settings:
pub struct MouseSettings { pub match_keyboard: bool, pub own: Option<LightingState> }
```

The mouse lives inside `Controller`, not in a second actor, because what it shows depends on the keyboard's state:
- `mouse_output()` reads `lights_on`, `game_override`, the saved state and the maps directly, under the same lock;
- every path that changes them calls `apply_mouse_logged()` after the keyboard. That covers `set_state`, `set_keys` when the shown map changes, `set_lights_on`, `set_game_switch` and `set_game_count`.

A mouse error is logged and marks the mouse disconnected. It never fails the keyboard call.

`mouse_output()`:

| Condition | Mouse shows |
|---|---|
| lights off, or no mouse settings and no saved state | Off, or nothing if there's neither |
| own setting (matching off) | that state, with brightness 0 meaning Off |
| matching, Gaming map shown | Static in the Gaming map's colour for W (`0x1a`), at the saved brightness; Off if W is off |
| matching, Custom | Static in the Custom map's colour for W, at the state's brightness; Off if W is off |
| matching, Wave | Cycle at the same speed and brightness |
| matching, Ripple | Static in Ripple's colour |
| matching, Off/Static/Breathe/Cycle | the same |

Corrections don't apply to the mouse. They are per-key keyboard data.

### D4. Mouse speeds use the keyboard's ranges, checked on hardware

`EffectKind::speed_range()` comes from the G512 spikes, where Breathe and Cycle period ranges were confirmed by eye. The mouse reports the same default period (992) and similar caps (`0xc005` against `0xc001`), so the same ranges are used. A hands-on task checks that slow and fast look right on the mouse. If they don't, the ranges become per-model.

### D5. D-Bus additions

| Member | Type | Meaning |
|---|---|---|
| `MouseState` | `s` | `connected`, `disconnected`, `permission-denied` or `unsupported` |
| `MouseName` | `s` | device name when connected, else empty |
| `MouseEffects` | `a(sbbyy)` | Off, Static, Breathe, Cycle, in the same shape as `Effects` |
| `MouseMatch` | `b` | following the keyboard |
| `MouseLighting` | `(s(yyy)yy)` | the mouse's own setting (shown when not matching) |
| `SetMouseMatch(b)` | | follow the keyboard, or go back to the own setting |
| `SetMouseLighting(s(yyy)yy)` | returns `b` (pending) | validate against `MouseEffects`, save, turn matching off and the lights on |

- Before any own setting exists, `MouseLighting` reads as Static at the keyboard's colour, speed and brightness. Turning matching off then changes nothing visible.
- Every existing member is unchanged. `emit_all` includes the new properties.

### D6. Events cover both devices

- **Hotplug:**
  - "add" on a hidraw node reconnects whichever device isn't connected, after the existing settle delay;
  - "remove" on the mouse's node disconnects only the mouse.
- **Resume:** re-applies both.
- **Login retry:** keeps retrying while either device is `permission-denied`, for the same 15 s. That covers the new udev rule line.

### D7. Storage

`state.toml` gets a `[mouse]` table, written only once the user has changed something:

```toml
[mouse]
match_keyboard = true
effect = "Static"   # with color, speed and brightness, only once an own setting exists
```

Files without `[mouse]` load as matching, with no own setting.

### D8. App: a Mouse group

The Mouse group appears below Games when the service reports `MouseState`. It contains:
- a status row: the mouse name, or "Not connected" / "No permission";
- a "Match Keyboard" switch;
- while matching is off, Effect (Off/Static/Breathe/Cycle), Colour (Static and Breathe), Speed (Breathe and Cycle) and Brightness rows.

These rows follow the existing ones, including the throttle. The group is hidden when the property is absent, for an older service.

## Risks / Trade-offs

- **The mouse's own profile lighting returns on replug** until g512d re-applies, within 3 s, like the keyboard's firmware default.
- **The mouse may react differently to Breathe and Cycle parameters** (D4). Hardware checks cover it.
- **Unplugging the keyboard leaves the mouse matching the last keyboard state.** This is intended: matching follows the saved state, not the physical keyboard.
- **A second udev line needs `sudo`.** Until it's installed, `MouseState` is `permission-denied` and the app says so.

## Migration Plan

- Reinstall the udev rule (`sudo install …`, `udevadm trigger`), then reinstall the service and app.
- Existing `state.toml` files load with matching on, so the mouse takes the keyboard's look as soon as the service can open it.
- Rollback: older services ignore `[mouse]`.

## Open Questions

None. The user chose the nearest-effect fallbacks and one setting for both zones.
