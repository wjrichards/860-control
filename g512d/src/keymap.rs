//! Key maps (Custom, Gaming), per-key brightness corrections, and composing
//! them into the frame sent to the keyboard.

use std::{collections::BTreeMap, fs, io, path::Path};

use g512_hidpp::{
    effects::Rgb,
    perkey::{Frame, KeyId},
};
use crate::state::{BRIGHTNESS_MAX, scale};

/// Which saved map a front end edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapName {
    Custom,
    Gaming,
}

impl MapName {
    pub fn parse(name: &str) -> Option<MapName> {
        match name {
            "custom" => Some(MapName::Custom),
            "gaming" => Some(MapName::Gaming),
            _ => None,
        }
    }
}

/// Key → colour. Keys not in the map are off.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyMap(BTreeMap<KeyId, Rgb>);

const GAMING_HIGHLIGHT: Rgb = Rgb::new(0xff, 0x20, 0x00);
const GAMING_DIM: Rgb = Rgb::new(0x20, 0x20, 0x20);
/// W A S D, Space, Left Shift, Left Ctrl, 1..5, Right Left Down Up.
const GAMING_KEYS: [u8; 16] =
    [0x1a, 0x04, 0x16, 0x07, 0x2c, 0xe1, 0xe0, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x4f, 0x50, 0x51, 0x52];

#[derive(Debug, PartialEq, Eq)]
pub enum InvalidMap {
    UnknownKey(u8),
    DuplicateKey(u8),
}

impl std::fmt::Display for InvalidMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidMap::UnknownKey(k) => write!(f, "key {k:#04x} is not in the ANSI 104-key set"),
            InvalidMap::DuplicateKey(k) => write!(f, "key {k:#04x} appears more than once"),
        }
    }
}

impl KeyMap {
    /// The Gaming map until the user edits it.
    pub fn default_gaming() -> KeyMap {
        let mut map = KeyMap(KeyId::ALL.into_iter().map(|k| (k, GAMING_DIM)).collect());
        for usage in GAMING_KEYS {
            map.0.insert(KeyId::new(usage).expect("gaming keys are in the key set"), GAMING_HIGHLIGHT);
        }
        map
    }

    /// Build from (usage, colour) entries, as sent over D-Bus.
    pub fn from_entries(entries: impl IntoIterator<Item = (u8, Rgb)>) -> Result<KeyMap, InvalidMap> {
        let mut map = BTreeMap::new();
        for (usage, color) in entries {
            let key = KeyId::new(usage).map_err(|_| InvalidMap::UnknownKey(usage))?;
            if map.insert(key, color).is_some() {
                return Err(InvalidMap::DuplicateKey(usage));
            }
        }
        Ok(KeyMap(map))
    }

    /// (key, colour) pairs, sorted by key.
    pub fn entries(&self) -> impl Iterator<Item = (KeyId, Rgb)> + '_ {
        self.0.iter().map(|(&k, &c)| (k, c))
    }

    pub fn get(&self, key: KeyId) -> Option<Rgb> {
        self.0.get(&key).copied()
    }
}

/// Key → percent (1..=100) applied to every per-key frame, to even out LEDs
/// that are brighter than the rest.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Corrections(BTreeMap<KeyId, u8>);

impl Corrections {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    fn percent(&self, key: KeyId) -> u8 {
        self.0.get(&key).copied().unwrap_or(BRIGHTNESS_MAX)
    }

    /// Read `corrections.toml`: `e0 = 85` lines, key usage in hex. A missing
    /// file is empty; bad entries are skipped with a warning.
    pub fn load(path: &Path) -> Corrections {
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Corrections::default(),
            Err(e) => {
                eprintln!("warning: cannot read {}: {e}; no corrections", path.display());
                return Corrections::default();
            }
        };
        Self::parse(&text).unwrap_or_else(|e| {
            eprintln!("warning: invalid {}: {e}; no corrections", path.display());
            Corrections::default()
        })
    }

    fn parse(text: &str) -> Result<Corrections, String> {
        let raw: BTreeMap<String, toml::Value> = toml::from_str(text).map_err(|e| e.to_string())?;
        let mut out = BTreeMap::new();
        for (name, value) in raw {
            let key = parse_key(&name);
            let percent = value.as_integer().filter(|p| (1..=i64::from(BRIGHTNESS_MAX)).contains(p));
            match (key, percent) {
                (Some(key), Some(p)) => {
                    out.insert(key, p as u8);
                }
                (None, _) => eprintln!("warning: corrections: {name:?} is not a key ID (e.g. e0); skipped"),
                (_, None) => eprintln!("warning: corrections: {name} = {value} is not a percentage 1..=100; skipped"),
            }
        }
        Ok(Corrections(out))
    }
}

/// A two-digit hex usage ID in the key set, e.g. `e0`.
pub fn parse_key(name: &str) -> Option<KeyId> {
    if name.len() != 2 {
        return None;
    }
    u8::from_str_radix(name, 16).ok().and_then(|u| KeyId::new(u).ok())
}

/// The frame for a per-key base colour: brightness, then the key's correction.
pub fn compose(base: impl Fn(KeyId) -> Rgb, brightness: u8, corrections: &Corrections) -> Frame {
    let mut frame = Frame::filled(Rgb::default());
    for key in KeyId::ALL {
        frame.set(key, scale(scale(base(key), brightness), corrections.percent(key)));
    }
    frame
}

/// A key map's frame: keys missing from the map are off.
pub fn map_frame(map: &KeyMap, brightness: u8, corrections: &Corrections) -> Frame {
    compose(|k| map.get(k).unwrap_or_default(), brightness, corrections)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(usage: u8) -> KeyId {
        KeyId::new(usage).unwrap()
    }

    #[test]
    fn custom_at_half_brightness() {
        let map = KeyMap::from_entries([(0x1a, Rgb::new(0xff, 0, 0))]).unwrap();
        let frame = map_frame(&map, 50, &Corrections::default());
        assert_eq!(frame.get(key(0x1a)), Rgb::new(0x80, 0, 0), "W at #800000");
        assert!(frame.iter().filter(|&(k, _)| k != key(0x1a)).all(|(_, c)| c == Rgb::default()), "rest off");
    }

    #[test]
    fn left_ctrl_evened_out() {
        let corrections = Corrections::parse("e0 = 85\n").unwrap();
        let frame = compose(|_| Rgb::new(0xff, 0xff, 0xff), 100, &corrections);
        assert_eq!(frame.get(KeyId::LEFT_CTRL), Rgb::new(0xd9, 0xd9, 0xd9));
        assert_eq!(frame.get(key(0x29)), Rgb::new(0xff, 0xff, 0xff));
    }

    #[test]
    fn default_gaming_map() {
        let map = KeyMap::default_gaming();
        assert_eq!(map.entries().count(), 104);
        assert_eq!(map.get(key(0x1a)), Some(GAMING_HIGHLIGHT), "W");
        assert_eq!(map.get(key(0x52)), Some(GAMING_HIGHLIGHT), "Up");
        assert_eq!(map.get(key(0x29)), Some(GAMING_DIM), "Esc");
        assert_eq!(map.entries().filter(|&(_, c)| c == GAMING_HIGHLIGHT).count(), GAMING_KEYS.len());
    }

    #[test]
    fn map_entries_are_checked() {
        assert_eq!(KeyMap::from_entries([(0x64, Rgb::default())]), Err(InvalidMap::UnknownKey(0x64)));
        assert_eq!(
            KeyMap::from_entries([(0x04, Rgb::default()), (0x04, Rgb::default())]),
            Err(InvalidMap::DuplicateKey(0x04))
        );
        let map = KeyMap::from_entries([(0xe0, Rgb::new(1, 2, 3)), (0x04, Rgb::default())]).unwrap();
        let usages: Vec<u8> = map.entries().map(|(k, _)| k.usage()).collect();
        assert_eq!(usages, [0x04, 0xe0], "sorted by key");
    }

    #[test]
    fn corrections_file() {
        let c = Corrections::parse("# Left Ctrl LED is brighter\ne0 = 85\n04 = 90\n").unwrap();
        assert_eq!((c.percent(KeyId::LEFT_CTRL), c.percent(key(0x04)), c.percent(key(0x05))), (85, 90, 100));
        // Unknown key, 0 %, 150 % and a non-number are skipped; the rest stays.
        let c = Corrections::parse("64 = 80\ne0 = 0\ne1 = 150\ne2 = \"x\"\nzz = 50\ne3 = 70\n").unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c.percent(key(0xe3)), 70);
        assert!(Corrections::parse("not toml [[").is_err());
        let missing = std::env::temp_dir().join(format!("g512d-no-corrections-{}.toml", std::process::id()));
        assert!(Corrections::load(&missing).is_empty());
    }
}
