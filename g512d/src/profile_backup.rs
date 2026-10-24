//! The mouse's profile backup, saved once before the first profile write,
//! and restoring it (`g512d --restore-mouse-profiles FILE`).
//!
//! The file uses spike 6's format, so that backup restores too:
//!
//! ```text
//! sector 0x0001: crc ok
//!   <the sector's bytes in hex>
//! ```

use std::{fmt::Write as _, fs, io, path::Path};

use g512_hidpp::{
    Device, Result as HidppResult, Transport,
    profiles::{self, ProfileSector, ProfilesInfo},
};

/// Sector number and bytes, in file order.
pub type Sectors = Vec<(u16, Vec<u8>)>;

pub fn format(sectors: &[(u16, Vec<u8>)]) -> String {
    let mut out = String::from("# G502 HERO onboard profile backup, written by g512d before its first profile write.\n");
    for (sector, data) in sectors {
        let state = if ProfileSector::parse(data.clone()).is_ok() { "crc ok" } else { "crc not valid" };
        let hex: String = data.iter().map(|b| format!("{b:02x}")).collect();
        let _ = writeln!(out, "sector {sector:#06x}: {state}\n  {hex}");
    }
    out
}

/// Every sector in a backup file; lines that aren't sector data are ignored.
pub fn parse(text: &str) -> Sectors {
    let mut sectors = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        let Some(number) = line.strip_prefix("sector 0x").and_then(|l| l.split(':').next()) else { continue };
        let (Ok(sector), Some(hex)) = (u16::from_str_radix(number, 16), lines.next()) else { continue };
        let hex = hex.trim();
        let bytes: Option<Vec<u8>> = (0..hex.len())
            .step_by(2)
            .map(|i| hex.get(i..i + 2).and_then(|b| u8::from_str_radix(b, 16).ok()))
            .collect();
        if let Some(bytes) = bytes.filter(|b| !b.is_empty()) {
            sectors.push((sector, bytes));
        }
    }
    sectors
}

/// Write the backup to `path` unless a file is already there. Returns
/// whether it was written.
pub fn save_once<T: Transport>(dev: &mut Device<T>, info: &ProfilesInfo, path: &Path) -> io::Result<bool> {
    if path.exists() {
        return Ok(false);
    }
    let mut sectors = Vec::new();
    for sector in 0..=u16::from(info.profile_count) {
        let data = profiles::read_raw(dev, info, sector).map_err(|e| io::Error::other(e.to_string()))?;
        sectors.push((sector, data));
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, format(&sectors))?;
    fs::rename(&tmp, path)?;
    Ok(true)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Restored {
    Written,
    Unchanged,
    /// Not a user profile sector, or its CRC isn't valid: left alone.
    Skipped(&'static str),
}

/// Write each user profile sector from `sectors` whose CRC is valid and
/// which differs from the mouse, verifying each by reading it back.
pub fn restore<T: Transport>(dev: &mut Device<T>, sectors: &[(u16, Vec<u8>)]) -> HidppResult<Vec<(u16, Restored)>> {
    let info = profiles::read_info(dev)?;
    let mut done = Vec::new();
    for (number, data) in sectors {
        let user = u8::try_from(*number).ok().and_then(|n| info.user_sector(n).ok());
        let outcome = match (user, ProfileSector::parse(data.clone())) {
            (None, _) => Restored::Skipped("not a user profile sector"),
            (Some(_), Err(_)) => Restored::Skipped("CRC not valid"),
            (Some(sector), Ok(wanted)) => {
                if profiles::read_raw(dev, &info, *number)? == wanted.bytes() {
                    Restored::Unchanged
                } else {
                    profiles::write_sector(dev, &info, sector, &wanted)?;
                    Restored::Written
                }
            }
        };
        done.push((*number, outcome));
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use g512_hidpp::fake::FakeKeyboard;

    use super::*;

    fn mouse() -> Device<FakeKeyboard> {
        Device::open(FakeKeyboard::g502_hero()).unwrap()
    }

    #[test]
    fn spike_6_backup_parses() {
        let text = include_str!("../../g512-hidpp/src/fixtures/g502_hero_profiles.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
            .map(|l| {
                let (n, hex) = l.split_once(' ').unwrap();
                format!("sector 0x{n}: crc ok\n  {hex}\n")
            })
            .collect::<String>();
        let sectors = parse(&text);
        let shape: Vec<(u16, usize)> = sectors.iter().map(|(n, d)| (*n, d.len())).collect();
        assert_eq!(shape, (0..6).map(|n| (n, 256)).collect::<Vec<_>>());
    }

    #[test]
    fn save_once_never_overwrites() {
        let dir = std::env::temp_dir().join(format!("g512d-backup-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("mouse-profiles-backup.txt");
        let mut dev = mouse();
        let info = profiles::read_info(&mut dev).unwrap();
        assert!(save_once(&mut dev, &info, &path).unwrap());
        let first = fs::read_to_string(&path).unwrap();
        let sectors = parse(&first);
        assert_eq!(sectors.len(), 6, "directory and five profiles");
        assert!(sectors[1..].iter().all(|(_, d)| ProfileSector::parse(d.clone()).is_ok()));
        assert!(first.contains("sector 0x0005: crc ok"));
        // Change the mouse, then try again: the file stays as it was.
        dev.transport_mut().current_profile = 1;
        assert!(!save_once(&mut dev, &info, &path).unwrap());
        assert_eq!(fs::read_to_string(&path).unwrap(), first);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn restore_writes_only_valid_user_sectors() {
        let mut dev = mouse();
        let info = profiles::read_info(&mut dev).unwrap();
        let original: Sectors = (0..=5).map(|n| (n, profiles::read_raw(&mut dev, &info, n).unwrap())).collect();
        // Change profiles 2 and 5 on the mouse.
        for n in [2, 5] {
            let sector = info.user_sector(n).unwrap();
            let edited = profiles::read_sector(&mut dev, &info, sector).unwrap().edited(8, 0, &[100]).unwrap();
            profiles::write_sector(&mut dev, &info, sector, &edited).unwrap();
        }
        let mut backup = original.clone();
        backup[3].1[0] ^= 1; // profile 3 no longer matches its CRC
        backup.push((0x101, vec![0xff; 256]));
        let writes_before = dev.transport_mut().profile_writes.len();
        let done = restore(&mut dev, &backup).unwrap();
        assert_eq!(
            done,
            [
                (0, Restored::Skipped("not a user profile sector")),
                (1, Restored::Unchanged),
                (2, Restored::Written),
                (3, Restored::Skipped("CRC not valid")),
                (4, Restored::Unchanged),
                (5, Restored::Written),
                (0x101, Restored::Skipped("not a user profile sector")),
            ]
        );
        assert_eq!(dev.transport_mut().profile_writes[writes_before..], [2, 5]);
        for n in [2u16, 5] {
            assert_eq!(dev.transport_mut().sector(n).unwrap(), original[usize::from(n)].1.as_slice());
        }
    }
}
