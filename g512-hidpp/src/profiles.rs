//! Onboard profiles (`0x8100`) on the G502 HERO: reading the user profile
//! sectors, editing their DPI steps, report rate and button actions, writing
//! them back with a read-back check, and switching the active profile
//! (spikes 7 and 8).
//!
//! Only user profile sectors can be written: [`UserSector`] can be made for
//! sectors 1 to the profile count and nothing else, so the profile directory
//! (sector 0) and the ROM sectors (`0x01xx`) are out of reach. The onboard
//! mode is never changed ([`Feature::allows`] refuses `setOnboardMode`).
//! Left click (button entry 0) can't be changed, and entries this module
//! doesn't understand are kept byte for byte.

use crate::{Device, Error, Feature, Result, Transport};

const FN_GET_INFO: u8 = 0;
const FN_SET_CURRENT_PROFILE: u8 = 3;
const FN_GET_CURRENT_PROFILE: u8 = 4;
const FN_MEMORY_READ: u8 = 5;
const FN_MEMORY_ADDR_WRITE: u8 = 6;
const FN_MEMORY_WRITE: u8 = 7;
const FN_MEMORY_WRITE_END: u8 = 8;
const DPI_FN_GET_LIST: u8 = 1;

/// The only profile layout this module understands.
const PROFILE_FORMAT: u8 = 2;
const CHUNK: usize = 16;
/// Profile format 2: report rate, default DPI index, switched DPI index,
/// then five little-endian DPI values.
const RATE: usize = 0;
const DEFAULT_INDEX: usize = 1;
const SWITCHED_INDEX: usize = 2;
const DPI: usize = 3;
pub const MAX_DPI_STEPS: usize = 5;
/// Report periods the profile can hold, in milliseconds (1000 to 125 Hz).
pub const REPORT_RATES_MS: [u8; 4] = [1, 2, 4, 8];
/// Profile format 2: the button table, 4 bytes per button, room for 16.
const BUTTONS: usize = 0x20;
const BUTTON_SIZE: usize = 4;
const MAX_BUTTONS: u8 = 16;
/// Marks a step size, rather than a value, in the sensor's DPI list.
const DPI_STEP_MARKER: u16 = 0xe000;

/// What `getInfo` reports about the profile memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProfilesInfo {
    pub format: u8,
    pub profile_count: u8,
    pub button_count: u8,
    pub sector_count: u8,
    pub sector_size: u16,
}

impl ProfilesInfo {
    /// The sector holding user profile `profile` (1-based); the only kind
    /// of sector that can be written.
    pub fn user_sector(&self, profile: u8) -> Result<UserSector> {
        if profile == 0 || profile > self.profile_count || profile >= self.sector_count {
            return Err(Error::InvalidProfile(format!("no user profile {profile}")));
        }
        Ok(UserSector(u16::from(profile)))
    }
}

/// A user profile sector, 1 to the profile count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UserSector(u16);

impl UserSector {
    pub fn number(self) -> u16 {
        self.0
    }
}

/// The sensor's DPI range: `min` to `max` in steps of `step`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DpiRange {
    pub min: u16,
    pub max: u16,
    pub step: u16,
}

impl DpiRange {
    pub fn contains(&self, dpi: u16) -> bool {
        (self.min..=self.max).contains(&dpi) && (dpi - self.min) % self.step == 0
    }
}

/// A mouse function a button can run (`90 cc`, spike 8 and libratbag).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Special {
    WheelLeft = 0x01,
    WheelRight = 0x02,
    DpiUp = 0x03,
    DpiDown = 0x04,
    DpiCycle = 0x05,
    DpiShift = 0x07,
    ProfileCycle = 0x0a,
    GShift = 0x0b,
}

impl Special {
    pub const ALL: [Special; 8] = [
        Special::WheelLeft,
        Special::WheelRight,
        Special::DpiUp,
        Special::DpiDown,
        Special::DpiCycle,
        Special::DpiShift,
        Special::ProfileCycle,
        Special::GShift,
    ];

    pub fn from_code(code: u8) -> Option<Special> {
        Special::ALL.into_iter().find(|s| *s as u8 == code)
    }
}

/// What one button does, as one 4-byte entry of the button table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonAction {
    /// `80 01 hi lo`: mouse buttons (1 left, 2 right, 4 middle, 8 back, 16 forward).
    Mouse(u16),
    /// `80 02 mm kk`: a key, HID usage `kk`, with modifier bits `mm`
    /// (Ctrl 1, Shift 2, Alt 4, Super 8).
    Key { modifiers: u8, usage: u8 },
    /// `80 03 hi lo`: a consumer (media) usage.
    Media(u16),
    /// `90 cc 00 00`: a mouse function.
    Special(Special),
    /// `80 01 00 00`: no mouse buttons, so the button does nothing.
    Disabled,
    /// Anything else (macros, unknown types): shown, never written.
    Other([u8; 4]),
}

impl ButtonAction {
    pub fn decode(entry: [u8; 4]) -> ButtonAction {
        match entry {
            [0x80, 0x01, 0, 0] => ButtonAction::Disabled,
            [0x80, 0x01, hi, lo] => ButtonAction::Mouse(u16::from_be_bytes([hi, lo])),
            [0x80, 0x02, modifiers, usage] => ButtonAction::Key { modifiers, usage },
            [0x80, 0x03, hi, lo] => ButtonAction::Media(u16::from_be_bytes([hi, lo])),
            [0x90, code, _, _] => Special::from_code(code).map_or(ButtonAction::Other(entry), ButtonAction::Special),
            _ => ButtonAction::Other(entry),
        }
    }

    /// The entry for this action; `Other` and an empty mouse mask have none.
    pub fn encode(self) -> Result<[u8; 4]> {
        Ok(match self {
            ButtonAction::Mouse(0) => return Err(Error::InvalidProfile("an empty mouse-button mask".into())),
            ButtonAction::Mouse(mask) => {
                let [hi, lo] = mask.to_be_bytes();
                [0x80, 0x01, hi, lo]
            }
            ButtonAction::Key { modifiers, usage } => [0x80, 0x02, modifiers, usage],
            ButtonAction::Media(usage) => {
                let [hi, lo] = usage.to_be_bytes();
                [0x80, 0x03, hi, lo]
            }
            ButtonAction::Special(s) => [0x90, s as u8, 0, 0],
            ButtonAction::Disabled => [0x80, 0x01, 0, 0],
            ButtonAction::Other(_) => return Err(Error::InvalidProfile("an entry this program can't write".into())),
        })
    }
}

/// A whole profile sector whose CRC has been checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileSector(Vec<u8>);

impl ProfileSector {
    pub fn parse(bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() < BUTTONS + BUTTON_SIZE * usize::from(MAX_BUTTONS) + 2 {
            return Err(Error::InvalidProfile(format!("{} bytes is too short for a profile", bytes.len())));
        }
        let (body, stored) = bytes.split_at(bytes.len() - 2);
        if crc(body) != u16::from_be_bytes([stored[0], stored[1]]) {
            return Err(Error::InvalidProfile("CRC does not match".into()));
        }
        Ok(ProfileSector(bytes))
    }

    pub fn bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn report_rate_ms(&self) -> u8 {
        self.0[RATE]
    }

    pub fn default_index(&self) -> u8 {
        self.0[DEFAULT_INDEX]
    }

    /// The DPI steps in order, up to the first unused (0 or `ffff`) slot.
    pub fn dpis(&self) -> Vec<u16> {
        (0..MAX_DPI_STEPS)
            .map(|i| u16::from_le_bytes([self.0[DPI + 2 * i], self.0[DPI + 2 * i + 1]]))
            .take_while(|&d| d != 0 && d != 0xffff)
            .collect()
    }

    /// The first `count` button entries (`getInfo`'s button count).
    pub fn buttons(&self, count: u8) -> Vec<ButtonAction> {
        (0..usize::from(count.min(MAX_BUTTONS)))
            .map(|i| self.button_entry(i))
            .map(ButtonAction::decode)
            .collect()
    }

    fn button_entry(&self, index: usize) -> [u8; 4] {
        let at = BUTTONS + BUTTON_SIZE * index;
        [self.0[at], self.0[at + 1], self.0[at + 2], self.0[at + 3]]
    }

    /// This profile with button `index` set to `action`. Every other byte is
    /// kept and the CRC is recomputed. Left click (entry 0) is refused.
    pub fn with_button(&self, index: u8, action: ButtonAction) -> Result<ProfileSector> {
        if index == 0 {
            return Err(Error::InvalidProfile("left click can't be changed".into()));
        }
        if index >= MAX_BUTTONS {
            return Err(Error::InvalidProfile(format!("no button {index}")));
        }
        let entry = action.encode()?;
        let at = BUTTONS + BUTTON_SIZE * usize::from(index);
        let mut b = self.0.clone();
        b[at..at + BUTTON_SIZE].copy_from_slice(&entry);
        Ok(ProfileSector::with_crc(b))
    }

    fn with_crc(mut b: Vec<u8>) -> ProfileSector {
        let n = b.len();
        let c = crc(&b[..n - 2]);
        b[n - 2..].copy_from_slice(&c.to_be_bytes());
        ProfileSector(b)
    }

    /// This profile with a new report rate, default step and DPI steps.
    /// Every other byte is kept; the switched DPI index too, unless it
    /// would point past the new steps. The CRC is recomputed.
    pub fn edited(&self, report_rate_ms: u8, default_index: u8, dpis: &[u16]) -> Result<ProfileSector> {
        if !REPORT_RATES_MS.contains(&report_rate_ms) {
            return Err(Error::InvalidProfile(format!("report rate {report_rate_ms} ms")));
        }
        if dpis.is_empty() || dpis.len() > MAX_DPI_STEPS || dpis.contains(&0) {
            return Err(Error::InvalidProfile(format!("{} DPI steps", dpis.len())));
        }
        if usize::from(default_index) >= dpis.len() {
            return Err(Error::InvalidProfile(format!("default step {default_index} of {}", dpis.len())));
        }
        let mut b = self.0.clone();
        b[RATE] = report_rate_ms;
        b[DEFAULT_INDEX] = default_index;
        if usize::from(b[SWITCHED_INDEX]) >= dpis.len() {
            b[SWITCHED_INDEX] = default_index;
        }
        for i in 0..MAX_DPI_STEPS {
            let d = dpis.get(i).copied().unwrap_or(0);
            b[DPI + 2 * i..DPI + 2 * i + 2].copy_from_slice(&d.to_le_bytes());
        }
        Ok(ProfileSector::with_crc(b))
    }
}

/// CRC-CCITT (initial value `0xffff`), as the profile sectors store it.
pub fn crc(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xffff;
    for &byte in data {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

/// Read `getInfo`; any profile format but 2 is refused.
pub fn read_info<T: Transport>(dev: &mut Device<T>) -> Result<ProfilesInfo> {
    let r = dev.request(Feature::OnboardProfiles, FN_GET_INFO, &[])?;
    let info = ProfilesInfo {
        format: r[1],
        profile_count: r[3],
        button_count: r[5],
        sector_count: r[6],
        sector_size: u16::from_be_bytes([r[7], r[8]]),
    };
    if info.format != PROFILE_FORMAT {
        return Err(Error::UnsupportedProfileFormat(info.format));
    }
    if info.button_count > MAX_BUTTONS {
        return Err(Error::Protocol(format!("{} buttons", info.button_count)));
    }
    if info.sector_size == 0 || usize::from(info.sector_size) % CHUNK != 0 {
        return Err(Error::Protocol(format!("sector size {}", info.sector_size)));
    }
    Ok(info)
}

/// Read any sector, raw (for the backup); reading never changes the mouse.
pub fn read_raw<T: Transport>(dev: &mut Device<T>, info: &ProfilesInfo, sector: u16) -> Result<Vec<u8>> {
    let mut data = Vec::with_capacity(usize::from(info.sector_size));
    for offset in (0..info.sector_size).step_by(CHUNK) {
        let [s_hi, s_lo] = sector.to_be_bytes();
        let [o_hi, o_lo] = offset.to_be_bytes();
        data.extend(dev.request(Feature::OnboardProfiles, FN_MEMORY_READ, &[s_hi, s_lo, o_hi, o_lo])?);
    }
    Ok(data)
}

pub fn read_sector<T: Transport>(dev: &mut Device<T>, info: &ProfilesInfo, sector: UserSector) -> Result<ProfileSector> {
    ProfileSector::parse(read_raw(dev, info, sector.0)?)
}

/// Write a whole user sector, then read it back; anything but an exact
/// match is an error.
pub fn write_sector<T: Transport>(
    dev: &mut Device<T>,
    info: &ProfilesInfo,
    sector: UserSector,
    data: &ProfileSector,
) -> Result<()> {
    if data.0.len() != usize::from(info.sector_size) {
        return Err(Error::InvalidProfile(format!("{} bytes for a {}-byte sector", data.0.len(), info.sector_size)));
    }
    let [s_hi, s_lo] = sector.0.to_be_bytes();
    let [l_hi, l_lo] = info.sector_size.to_be_bytes();
    dev.request(Feature::OnboardProfiles, FN_MEMORY_ADDR_WRITE, &[s_hi, s_lo, 0, 0, l_hi, l_lo])?;
    for chunk in data.0.chunks(CHUNK) {
        dev.request(Feature::OnboardProfiles, FN_MEMORY_WRITE, chunk)?;
    }
    dev.request(Feature::OnboardProfiles, FN_MEMORY_WRITE_END, &[])?;
    if read_raw(dev, info, sector.0)? != data.0 {
        return Err(Error::WriteNotVerified { sector: sector.0 });
    }
    Ok(())
}

/// The active profile, 1-based.
pub fn current_profile<T: Transport>(dev: &mut Device<T>) -> Result<u8> {
    Ok(dev.request(Feature::OnboardProfiles, FN_GET_CURRENT_PROFILE, &[])?[1])
}

/// Make `profile` (1-based) active.
pub fn set_current_profile<T: Transport>(dev: &mut Device<T>, info: &ProfilesInfo, profile: u8) -> Result<()> {
    info.user_sector(profile)?;
    dev.request(Feature::OnboardProfiles, FN_SET_CURRENT_PROFILE, &[0, profile])?;
    Ok(())
}

/// The sensor's DPI range, from AdjustableDpi's list for sensor 0: values
/// with a step marker (`e0 xx`) between them, e.g. `00 64 e0 32 64 00` for
/// 100 to 25600 in steps of 50.
pub fn dpi_range<T: Transport>(dev: &mut Device<T>) -> Result<DpiRange> {
    let r = dev.request(Feature::AdjustableDpi, DPI_FN_GET_LIST, &[0])?;
    let values: Vec<u16> = r[1..]
        .chunks_exact(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .take_while(|&v| v != 0)
        .collect();
    match values.as_slice() {
        [min, step, max] if step & DPI_STEP_MARKER == DPI_STEP_MARKER && step & !DPI_STEP_MARKER > 0 => {
            Ok(DpiRange { min: *min, max: *max, step: step & !DPI_STEP_MARKER })
        }
        _ => Err(Error::Protocol(format!("DPI list {values:04x?}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeKeyboard;

    fn mouse() -> Device<FakeKeyboard> {
        Device::open(FakeKeyboard::g502_hero()).unwrap()
    }

    #[test]
    fn reads_the_real_profiles() {
        let mut dev = mouse();
        let info = read_info(&mut dev).unwrap();
        assert_eq!(
            (info.format, info.profile_count, info.button_count, info.sector_count, info.sector_size),
            (2, 5, 11, 16, 256)
        );
        let profiles: Vec<_> = (1..=5)
            .map(|n| read_sector(&mut dev, &info, info.user_sector(n).unwrap()).unwrap())
            .map(|p| (p.report_rate_ms(), p.default_index(), p.dpis()))
            .collect();
        assert_eq!(
            profiles,
            [
                (1, 1, vec![1200, 2400, 3200, 6400]),
                (1, 2, vec![400, 800, 1600, 2400]),
                (2, 0, vec![2400]),
                (1, 0, vec![3200]),
                (1, 0, vec![6400]),
            ]
        );
    }

    #[test]
    fn crc_matches_the_real_sectors() {
        assert_eq!(crc(b"123456789"), 0x29b1, "CRC-CCITT check value");
        let mut dev = mouse();
        let info = read_info(&mut dev).unwrap();
        for sector in 0..=5 {
            let raw = read_raw(&mut dev, &info, sector).unwrap();
            assert_eq!(crc(&raw[..254]), u16::from_be_bytes([raw[254], raw[255]]), "sector {sector}");
        }
    }

    #[test]
    fn edited_changes_only_its_bytes() {
        let mut dev = mouse();
        let info = read_info(&mut dev).unwrap();
        let p1 = read_sector(&mut dev, &info, info.user_sector(1).unwrap()).unwrap();
        let e = p1.edited(2, 1, &[800, 1600, 3200]).unwrap();
        assert_eq!((e.report_rate_ms(), e.default_index(), e.dpis()), (2, 1, vec![800, 1600, 3200]));
        let changed: Vec<usize> = (0..256).filter(|&i| e.bytes()[i] != p1.bytes()[i]).collect();
        assert!(changed.iter().all(|&i| i <= 12 || i >= 254), "only header, DPI and CRC: {changed:?}");
        assert!(ProfileSector::parse(e.bytes().to_vec()).is_ok(), "the new CRC is valid");
        // Unused slots are written as 0.
        assert_eq!(&e.bytes()[9..13], [0, 0, 0, 0]);
        // A switched index past the new steps follows the default.
        let mut odd = p1.bytes().to_vec();
        odd[SWITCHED_INDEX] = 3;
        let n = odd.len();
        let c = crc(&odd[..n - 2]);
        odd[n - 2..].copy_from_slice(&c.to_be_bytes());
        let odd = ProfileSector::parse(odd).unwrap();
        assert_eq!(odd.edited(1, 0, &[1600]).unwrap().bytes()[SWITCHED_INDEX], 0);
    }

    fn profile(n: u8) -> ProfileSector {
        let mut dev = mouse();
        let info = read_info(&mut dev).unwrap();
        read_sector(&mut dev, &info, info.user_sector(n).unwrap()).unwrap()
    }

    #[test]
    fn decodes_the_real_buttons() {
        use ButtonAction::{Key, Mouse};
        let special = ButtonAction::Special;
        let mouse_buttons = [Mouse(1), Mouse(2), Mouse(4), Mouse(8), Mouse(16)];
        let wheel = [special(Special::WheelRight), special(Special::WheelLeft)];
        let p1 = profile(1).buttons(11);
        assert_eq!(p1[..5], mouse_buttons);
        assert_eq!(
            p1[5..9],
            [
                special(Special::DpiShift),
                special(Special::DpiDown),
                special(Special::DpiUp),
                special(Special::ProfileCycle)
            ]
        );
        assert_eq!(p1[9..], wheel);
        let p3 = profile(3).buttons(11);
        assert_eq!(p3[..5], mouse_buttons);
        assert_eq!(
            p3[5..9],
            [
                special(Special::GShift),
                Key { modifiers: 1, usage: 0x06 },
                Key { modifiers: 1, usage: 0x19 },
                special(Special::ProfileCycle)
            ],
            "G-shift, Ctrl+C, Ctrl+V"
        );
        assert_eq!(p3[9..], wheel);
    }

    #[test]
    fn with_button_changes_only_its_entry() {
        let p5 = profile(5);
        let f13 = ButtonAction::Key { modifiers: 0, usage: 0x68 };
        let e = p5.with_button(8, f13).unwrap();
        let changed: Vec<usize> = (0..256).filter(|&i| e.bytes()[i] != p5.bytes()[i]).collect();
        assert!(changed.iter().all(|&i| (0x40..0x44).contains(&i) || i >= 254), "{changed:?}");
        assert_eq!(&e.bytes()[0x40..0x44], [0x80, 0x02, 0x00, 0x68]);
        assert!(ProfileSector::parse(e.bytes().to_vec()).is_ok(), "the new CRC is valid");
        assert_eq!(e.buttons(11)[8], f13);
        // Untouched specials keep their ff ff tails.
        assert_eq!(&e.bytes()[0x34..0x38], [0x90, 0x07, 0xff, 0xff]);
    }

    #[test]
    fn with_button_refuses_left_click_and_other() {
        let p = profile(4);
        assert!(matches!(p.with_button(0, ButtonAction::Mouse(1)), Err(Error::InvalidProfile(_))));
        assert!(matches!(p.with_button(16, ButtonAction::Mouse(1)), Err(Error::InvalidProfile(_))));
        assert!(matches!(p.with_button(3, ButtonAction::Other([0, 1, 2, 3])), Err(Error::InvalidProfile(_))));
        assert!(matches!(p.with_button(3, ButtonAction::Mouse(0)), Err(Error::InvalidProfile(_))));
    }

    #[test]
    fn actions_round_trip() {
        let actions = [
            ButtonAction::Mouse(4),
            ButtonAction::Key { modifiers: 0x05, usage: 0x2c },
            ButtonAction::Media(0x00cd),
            ButtonAction::Disabled,
        ]
        .into_iter()
        .chain(Special::ALL.map(ButtonAction::Special));
        for a in actions {
            assert_eq!(ButtonAction::decode(a.encode().unwrap()), a, "{a:?}");
        }
        // Unknown entries decode as Other with their bytes.
        for entry in [[0x00, 0x01, 0x02, 0x03], [0x90, 0x06, 0, 0], [0xff; 4]] {
            assert_eq!(ButtonAction::decode(entry), ButtonAction::Other(entry));
        }
    }

    #[test]
    fn edited_rejects_bad_values() {
        let mut dev = mouse();
        let info = read_info(&mut dev).unwrap();
        let p = read_sector(&mut dev, &info, info.user_sector(4).unwrap()).unwrap();
        for (rate, default, dpis) in [
            (3, 0, vec![800]),
            (1, 0, vec![]),
            (1, 0, vec![1, 2, 3, 4, 5, 6]),
            (1, 2, vec![800, 1600]),
            (1, 0, vec![0]),
        ] {
            assert!(matches!(p.edited(rate, default, &dpis), Err(Error::InvalidProfile(_))), "{rate} {default} {dpis:?}");
        }
    }

    #[test]
    fn only_user_sectors_can_be_named() {
        let mut dev = mouse();
        let info = read_info(&mut dev).unwrap();
        for profile in [0, 6, 0x10, 0xff] {
            assert!(matches!(info.user_sector(profile), Err(Error::InvalidProfile(_))), "{profile}");
        }
        assert_eq!(info.user_sector(5).unwrap().number(), 5);
    }

    #[test]
    fn write_then_read_back() {
        let mut dev = mouse();
        let info = read_info(&mut dev).unwrap();
        let sector = info.user_sector(5).unwrap();
        let edited = read_sector(&mut dev, &info, sector).unwrap().edited(1, 0, &[6000]).unwrap();
        write_sector(&mut dev, &info, sector, &edited).unwrap();
        assert_eq!(read_sector(&mut dev, &info, sector).unwrap().dpis(), [6000]);
        assert_eq!(dev.transport_mut().profile_writes, [5]);
    }

    #[test]
    fn read_back_mismatch_fails() {
        let mut fake = FakeKeyboard::g502_hero();
        fake.corrupt_next_write = true;
        let mut dev = Device::open(fake).unwrap();
        let info = read_info(&mut dev).unwrap();
        let sector = info.user_sector(2).unwrap();
        let edited = read_sector(&mut dev, &info, sector).unwrap().edited(1, 0, &[800]).unwrap();
        assert!(matches!(write_sector(&mut dev, &info, sector, &edited), Err(Error::WriteNotVerified { sector: 2 })));
    }

    #[test]
    fn unknown_format_is_refused() {
        let mut fake = FakeKeyboard::g502_hero();
        fake.profile_format = 3;
        let mut dev = Device::open(fake).unwrap();
        assert!(matches!(read_info(&mut dev), Err(Error::UnsupportedProfileFormat(3))));
    }

    #[test]
    fn switching_profiles() {
        let mut dev = mouse();
        let info = read_info(&mut dev).unwrap();
        assert_eq!(current_profile(&mut dev).unwrap(), 4);
        set_current_profile(&mut dev, &info, 1).unwrap();
        assert_eq!(current_profile(&mut dev).unwrap(), 1);
        assert!(matches!(set_current_profile(&mut dev, &info, 6), Err(Error::InvalidProfile(_))));
        assert_eq!(current_profile(&mut dev).unwrap(), 1, "a refused switch sends nothing");
    }

    #[test]
    fn dpi_range_from_the_sensor() {
        let mut dev = mouse();
        let range = dpi_range(&mut dev).unwrap();
        assert_eq!(range, DpiRange { min: 100, max: 25600, step: 50 });
        assert!(range.contains(6000) && !range.contains(125) && !range.contains(25650) && !range.contains(50));
    }
}
