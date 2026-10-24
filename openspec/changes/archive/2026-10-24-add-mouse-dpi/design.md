# Design

## Context

- **Spike 6** (archived with change 4) read and backed up the G502 HERO's memory:
  - 16 sectors of 256 bytes;
  - sector 0 is the profile directory, with entries `00 0N 01 00` (profile N lives in sector N, enabled);
  - sectors 1–5 are the user profiles, all with valid CRCs;
  - `0x0100`+ are the ROM profiles.
- **Profile format 2** (`getInfo` byte 1), first bytes:

  | Bytes | Field |
  |---|---|
  | 0 | report rate in ms |
  | 1 | default DPI index |
  | 2 | switched DPI index |
  | 3–12 | five DPI values, little-endian, 0 = unused |
  | 254–255 | CRC-CCITT (init `0xFFFF`) over bytes 0–253, big-endian |

  The bytes in between hold other settings (buttons and lighting) and are never interpreted here.
- **Spike 7** wrote sector 5 with fn6 `memoryAddrWrite(sector, 0, 256)`, then 16 × fn7 `memoryWrite`, then fn8 `memoryWriteEnd`.
  - The read-back matched, and the restore was byte-exact.
  - fn3 `setCurrentProfile(00 N)` switched profiles, and the new profile's default DPI applied at once.
- **AdjustableDpi `0x2201`:** fn1 returned `00 64 e0 32 64 00`, meaning 100 to 25600 in steps of 50.
- **The allowlist today:** `Feature` covers six features. The PerKeyLighting function limit (fn3 and fn5 only) is a convention in `perkey.rs`, not enforced.

## Goals / Non-Goals

**Goals:**
- **A write can't hit anything but a user profile sector,** and every write is verified by reading it back.
- **The mouse stays in onboard mode,** and bytes this change doesn't own are preserved.
- **A backup and a restore path** exist before the first write.

**Non-Goals:**
- Buttons, macros, profile lighting, host mode.
- Changing which profiles are enabled in the directory.

## Decisions

### D1. Per-feature function lists, enforced in `Device::request`

`Feature::allows(function) -> bool`:
- **OnboardProfiles** `0x8100`: functions 0, 2, 3, 4, 5, 6, 7, 8 and 11. Function 1 (`setOnboardMode`) and 12 (`setCurrentDpiIndex`) are excluded.
- **AdjustableDpi** `0x2201`: functions 0, 1 and 2.
- **PerKeyLighting** `0x8080`: functions 3 and 5, as its spec already says. The convention becomes enforced. The `perkey` test that sends function 1 to the fake now expects `FunctionNotAllowed`.
- **Every other allowlisted feature:** any function, as today.

`Device::request` returns `Error::FunctionNotAllowed { feature, function }` before writing a byte. A test covers every disallowed function.

### D2. `g512_hidpp::profiles`

```rust
pub struct ProfilesInfo { pub profile_count: u8, pub sector_count: u8, pub sector_size: u16, pub format: u8 }
pub struct UserSector(u16);              // only from ProfilesInfo::user_sector(n), n in 1..=profile_count
pub struct ProfileSector { bytes: Vec<u8> }   // a whole sector, CRC checked on parse
impl ProfileSector {
    pub fn report_rate_ms(&self) -> u8; pub fn default_index(&self) -> u8; pub fn dpis(&self) -> Vec<u16>;
    pub fn edited(&self, report_rate_ms: u8, default_index: u8, dpis: &[u16]) -> ProfileSector; // touches bytes 0, 1, 3..13, 254..256
}
pub fn read_info(dev) -> Result<ProfilesInfo>;          // fn0; refuses formats other than 2 (UnsupportedProfileFormat)
pub fn read_sector(dev, &ProfilesInfo, UserSector) -> Result<ProfileSector>;   // fn5 in 16-byte reads
pub fn write_sector(dev, &ProfilesInfo, UserSector, &ProfileSector) -> Result<()>; // fn6, fn7 ×N, fn8, then read back; Err if different
pub fn current_profile(dev) -> Result<u8>;  pub fn set_current_profile(dev, &ProfilesInfo, n: u8) -> Result<()>;
pub fn dpi_range(dev) -> Result<DpiRange>;  // 0x2201 fn1: min, max, step
```

- **Only `UserSector` can be written.** Its constructor rejects sector 0, `0x01xx`, and anything above `profile_count`.
- **Byte 2 (switched DPI index)** is kept, unless it points past the new list; then it's set to the new default.
- **Unused DPI slots** are written as 0.

### D3. Backup and restore

- **Backup:** on the first mouse connection where `~/.config/g512-control/mouse-profiles-backup.txt` doesn't exist, the service reads sectors 0 to `profile_count` and writes them in the spike 6 format, one hex line per sector with its CRC state. It's only ever written once, so it holds the profiles as they were before g512d's first write.
- **Restore:** `g512d --restore-mouse-profiles FILE` opens the mouse directly and, for each user sector in the file with a valid CRC, writes it and verifies the read-back. It prints what it did. It works while the service is running.

### D4. Service: profiles beside the mouse lighting

- **Connection:** `MouseConnection` gains `profiles: Option<MouseProfiles>`, holding the info, the five parsed sectors, the active profile and the DPI range. It's read on connect. If reading fails (another format, or a bad CRC), lighting still works and the profile members report empty.
- **`set_mouse_active_profile(n)`:** `setCurrentProfile`, re-read the current profile, then re-apply the mouse lighting.
- **`set_mouse_profile(n, rate, default, dpis)`:**
  1. validate (D5);
  2. make sure the backup exists;
  3. edit, write and verify;
  4. update the cached sector;
  5. re-apply the mouse lighting if `n` is the active profile.
- **Failures** go to the caller as `SetError::Device`, and the cached sector stays as it was. Unlike lighting, a profile write is an explicit request, so the caller needs to hear that it failed.

### D5. Validation

- 1 to 5 DPI steps, each within the sensor's range on its step grid (100–25600, multiples of 50 here).
- A default index below the number of steps.
- A report rate of 1, 2, 4 or 8 ms.
- A profile number from 1 to `profile_count`.

### D6. D-Bus

| Member | Type | Meaning |
|---|---|---|
| `MouseProfiles` | `a(yyyaq)` | per profile: number, report rate in ms, default index, DPI steps; empty when unknown |
| `MouseActiveProfile` | `y` | 1–5; 0 when unknown |
| `MouseDpiRange` | `(qqq)` | min, max, step; zeros when unknown |
| `SetMouseActiveProfile(y)` | | switch the active profile |
| `SetMouseProfile(y n, y rate_ms, y default_index, aq dpis)` | | validate, back up once, write, verify |

`emit_all` includes them. The existing members are unchanged.

### D7. App

The Mouse section, shown while the mouse is connected and has profiles, gains:
- **Profile:** an `AdwComboRow` with "Profile 1" to "Profile 5". Choosing one calls `SetMouseActiveProfile`.
- **Report rate:** an `AdwComboRow` with 125, 250, 500 and 1000 Hz, mapped to 8, 4, 2 and 1 ms.
- **DPI steps:** one `AdwSpinRow` per step (range and step from `MouseDpiRange`), each with a default-step check button and a remove button, then an "Add Step" button (at most 5).

Any edit restarts a 2 s timer. When it fires, the app sends one `SetMouseProfile` for the active profile. A refresh doesn't overwrite rows while an edit is pending.

## Risks / Trade-offs

- **Flash wear.** Each write rewrites one 256-byte sector. The 2 s debounce makes a drag or a typed number one write. Use stays far below typical flash endurance (thousands of cycles), but a script calling `SetMouseProfile` in a loop could still wear it. The service doesn't rate-limit that.
- **Partial writes.** If the mouse is unplugged mid-write, the sector may be left with a bad CRC. The firmware then falls back to the ROM profile for that slot, as libratbag documents. The restore command fixes it.
- **Profile format.** Only format 2 is handled. Any other format is refused, and lighting still works.
- **Lighting after a write or switch** may briefly show the profile's stored lighting until g512d re-applies its own.

## Migration Plan

- No settings change: profiles live on the mouse. The backup file appears on the first connection after this change is installed.
- Rollback: older services ignore the profiles. What's already written stays on the mouse, and the restore command can bring back the backup.

## Open Questions

None. The user decided the debounce, choose-to-activate, and 1–5 spin-button steps.
