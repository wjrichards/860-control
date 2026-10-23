# Design

## Context

- **What v1 does.** It sends every lighting state as a `0x8070` zone effect (archived change `2026-10-22-add-g512-lighting-controller`). `g512d`'s `LightingState.effect` is a `g512_hidpp::effects::EffectKind`, an enum tied to the device's effect IDs and to discovery. `Controller::apply` has one path: `effects::set_effect`.
- **What spike 5 found** (`evidence/per-key.md`):
  - `0x8080` function 3 takes 14 `key R G B` entries per 64-byte report, and function 5 commits them.
  - A full 104-key frame (8 reports and a commit) takes 28 ms.
  - A frame stops a running zone effect, and a zone effect replaces a frame.
  - Left Ctrl matches the other keys at 85 %.
- **GameMode on this machine:** `gamemode` 1.8.2 is activatable on the session bus as `com.feralinteractive.GameMode`. `gamemoded` contains `GameRegistered`, `GameUnregistered`, `ClientCount` and `ListGames`, and `gamemode-simulate-game` is installed. Steam is a Flatpak; its games reach GameMode through the portal.

## Goals / Non-Goals

**Goals:**
- One apply path that decides between a zone effect and a per-key frame. Everything that shows lighting goes through it: SetState, key-map edits, connect, replug, resume, and game start and end.
- v1 D-Bus clients and v1 state files keep working unchanged.

**Non-Goals:**
- Reading the keyboard's layout feature (`0x4540`) to detect ISO. It is not on the allowlist, and the user's keyboard is ANSI (`us` layout).
- Live editing of `corrections.toml`. It is read when the service starts.

## Decisions

### D1. Custom is a service-level effect, not an `EffectKind`

`g512d` gets `enum Effect { Zone(EffectKind), Custom }`, and `LightingState.effect` becomes `Effect`. `EffectKind` stays exactly what the device reports.

- On D-Bus, Custom is named `"Custom"` in the `Effects` list and in `State`, with colour false and speed false.
- Custom is offered only when the PerKeyLighting slot resolved.

*Alternative:* add `EffectKind::Custom` to the protocol crate. Rejected, because every `EffectKind` has a device ID and index, which Custom has neither of, and discovery would have to special-case it.

### D2. `g512_hidpp::perkey`: whole-keyboard frames only

- `KeyId(u8)` is built only from the ANSI 104 set (`KeyId::ALL`, `KeyId::new` returns `Err(InvalidKey)` for anything else).
- `Frame` is `[Rgb; 104]`, indexed in `KeyId::ALL` order.
- `send_frame(dev, &Frame)` sends 8 "set keys" requests in `KeyId::ALL` order, then a commit, waiting for each reply through the existing `Device::request`.
- Partial updates are not exposed. A full frame costs 28 ms, so always sending every key keeps what's on the keyboard equal to what the service believes, after a replug or an effect switch alike.
- Report `0x12` needs a `Device::request_long`. Today's `request` builds 20-byte `0x11` reports only. Replies already match on slot, function and software ID, whatever their report ID.

*Alternative:* send only changed keys. Rejected: it saves at most 7 small writes per edit, and gets the state wrong after any effect switch.

### D3. One "what is due" decision, then one apply

```
due = if game_override { Gaming map } else { saved state }
apply(due):
  Zone effect (except Static when corrections exist) → setZoneEffect, as in v1
  Custom → frame(custom map)
  Gaming → frame(gaming map)
  Static with corrections → frame(every key = static colour)
frame(map) = for each key: map.get(key).unwrap_or(off) × brightness × correction(key)
```

- Brightness 0 still sends zone Off, as in v1.
- `game_override` is set when GameMode's count goes from 0 to more than 0 while the switch is on.
- It is cleared when the count returns to 0, and on any `SetState`, so the user's change wins until the next game.
- Turning the switch off also clears it, and the saved state is re-applied.
- Rounding matches v1's `device_color`: `(c × pct + 50) / 100`, applied once for brightness and once for the correction. The spec scenario `#FF` × 85 % = `#D9` holds.

### D4. GameMode watcher: listen only, never activate

- A thread with its own blocking session connection, like the resume watcher.
- It subscribes to `GameRegistered` and `GameUnregistered` from `com.feralinteractive.GameMode`, and to `NameOwnerChanged` for that name.
- It reads `ClientCount` only while the name has an owner (`NameHasOwner` first). The proxy is built with property caching off, because zbus's default `GetAll` at proxy creation would activate GameMode.
- Each signal triggers a fresh `ClientCount` read, so missed or duplicated signals self-correct.
- When the owner disappears, the count is 0.
- It is verified with `gamemode-simulate-game`, which registers a fake game for a few seconds.

*Alternative:* call GameMode's `ListGames` on a timer. Rejected, because it would activate GameMode and wake up regularly for no reason.

### D5. D-Bus additions (v1 members unchanged)

| Member | Type | Notes |
|---|---|---|
| `CustomKeys`, `GamingKeys` | property `a(yyyy)` | (key, r, g, b), sorted by key, only keys in the map |
| `SetKeys(s map, a(yyyy) keys)` | method | `map` is `"custom"` or `"gaming"`; replaces the whole map; `InvalidArgs` for unknown keys or a duplicate key |
| `GameSwitch` | property `b` | |
| `SetGameSwitch(b)` | method | A method, matching `SetState`, rather than a writable property |
| `GameActive` | property `b` | True while `game_override` is set |

All changes go out through the existing `emit_all`.

### D6. Storage

- `state.toml` gains three things, all with serde defaults so v1 files load:
  - `game_switch = false`;
  - a `[custom]` table;
  - a `[gaming]` table, written only once the user has edited it, so an unedited Gaming map follows the built-in default.
- Map tables use two-digit hex key IDs as keys and `"#rrggbb"` as values, for example `e0 = "#ff2000"`.
- `corrections.toml` uses the same key format with integer percentages, for example `e0 = 85`. The service reads it once at startup and never writes it, so comments the user puts in it survive.
- **Rolling back to the v1 binary is safe.** v1 has no `deny_unknown_fields`, so it ignores the new fields. A saved effect of `"Custom"` makes v1 warn and leave the keyboard alone.

### D7. The editor is a grid of real buttons, not a drawn canvas

- `layout.rs` in the app holds the ANSI table: key ID, label, row, x and width, in quarter-key units.
- Each key is a `gtk::ToggleButton` placed in a `gtk::Fixed` (or a custom layout manager), with a small colour swatch child and the accessible label "W, red".
- Buttons give focus, Space to toggle and accessible names for free. Arrow-key focus moves through GTK's directional focus. If that doesn't move cleanly between rows, a key controller maps arrows to the nearest key in that direction.
- Drag-select is a `GestureDrag` on the container, using `pick()` hit-testing.
- Group buttons and the colour button sit above the keyboard.
- Painting sends `SetKeys` through the existing `Throttle`.
- The window becomes an `AdwNavigationView`: the main page, plus a "Key maps" page that holds the editor and the Custom/Gaming switch.

*Alternative:* a single `DrawingArea` with custom accessible children. Rejected: gtk-rs makes per-key `GtkAccessible` objects hard, and keyboard focus would have to be rebuilt by hand.

The editor is built directly, as the v1 app was, and checked on screen. A Penpot mockup comes first only if the user asks.

## Risks / Trade-offs

- **Games that don't use GameMode won't switch.** → The app's setting explains `gamemoderun %command%`, and so does the README.
- **GameMode's signals may not fire as expected for portal-registered (Flatpak) games.** → The count is re-read on every signal and owner change. The hands-on check uses a real Steam game as well as `gamemode-simulate-game`.
- **Static sent as a frame might look slightly different from zone Static** (for example in timing, or if the firmware dims frames). → A hardware check compares the two side by side. If they differ, corrections apply only to Custom and Gaming.
- **Keys the G512 doesn't have** (for example Menu `0x65` or Right Win) are accepted by the keyboard silently (spike 5: no errors). → They remain in the set; the editor draws only the G512's physical keys.
- **The app painting at 10 updates/s means 10 frames/s**, about 0.3 s of USB traffic per second. → Acceptable, as measured in spike 5.
- **Two writers:** OpenRGB or g810-led running at the same time would fight over frames, as in v1. → The README warning stays.

## Migration Plan

1. Install a new `g512d` with `packaging/install-service.sh` and rebuild the Flatpak. The saved v1 state keeps applying.
2. Add `corrections.toml` with `e0 = 85` for the user's keyboard, and restart `g512d`.
3. Rollback: reinstall the v1 build. New fields are ignored. If the effect was Custom, set any v1 effect once.

## Open Questions

- **The exact physical G512 key set** (whether Menu and Right Win are present) decides which keys the editor draws. It can be checked by eye while building the layout, and it doesn't change the service or the specs.
