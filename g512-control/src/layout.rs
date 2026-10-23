//! The G512's physical ANSI layout, in quarter-key units, for drawing the
//! key-map editor. Keys carry their USB HID usage ID; Fn has none (it has no
//! lighting address) and is drawn but not selectable.

pub struct Key {
    pub usage: Option<u8>,
    /// Text on the drawn key.
    pub label: &'static str,
    /// Spoken name, e.g. "Left Shift".
    pub name: &'static str,
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

const fn k(usage: u8, label: &'static str, name: &'static str, x: u16, y: u16, w: u16) -> Key {
    Key { usage: Some(usage), label, name, x, y, w, h: 4 }
}

const fn tall(usage: u8, label: &'static str, name: &'static str, x: u16, y: u16) -> Key {
    Key { usage: Some(usage), label, name, x, y, w: 4, h: 8 }
}

/// Width and height of the whole layout, in quarter-key units.
pub const WIDTH: u16 = 90;
pub const HEIGHT: u16 = 26;

// Row tops: the F-row, then a quarter-key gap, then five rows.
const F: u16 = 0;
const R1: u16 = 6;
const R2: u16 = 10;
const R3: u16 = 14;
const R4: u16 = 18;
const R5: u16 = 22;
/// Navigation cluster and numpad left edges.
const NAV: u16 = 61;
const PAD: u16 = 74;

#[rustfmt::skip]
pub const KEYS: &[Key] = &[
    k(0x29, "Esc", "Escape", 0, F, 4),
    k(0x3a, "F1", "F1", 8, F, 4), k(0x3b, "F2", "F2", 12, F, 4), k(0x3c, "F3", "F3", 16, F, 4), k(0x3d, "F4", "F4", 20, F, 4),
    k(0x3e, "F5", "F5", 26, F, 4), k(0x3f, "F6", "F6", 30, F, 4), k(0x40, "F7", "F7", 34, F, 4), k(0x41, "F8", "F8", 38, F, 4),
    k(0x42, "F9", "F9", 44, F, 4), k(0x43, "F10", "F10", 48, F, 4), k(0x44, "F11", "F11", 52, F, 4), k(0x45, "F12", "F12", 56, F, 4),
    k(0x46, "PrtSc", "Print Screen", NAV, F, 4), k(0x47, "ScrLk", "Scroll Lock", NAV + 4, F, 4), k(0x48, "Pause", "Pause", NAV + 8, F, 4),

    k(0x35, "`", "Grave", 0, R1, 4),
    k(0x1e, "1", "1", 4, R1, 4), k(0x1f, "2", "2", 8, R1, 4), k(0x20, "3", "3", 12, R1, 4), k(0x21, "4", "4", 16, R1, 4),
    k(0x22, "5", "5", 20, R1, 4), k(0x23, "6", "6", 24, R1, 4), k(0x24, "7", "7", 28, R1, 4), k(0x25, "8", "8", 32, R1, 4),
    k(0x26, "9", "9", 36, R1, 4), k(0x27, "0", "0", 40, R1, 4), k(0x2d, "-", "Minus", 44, R1, 4), k(0x2e, "=", "Equals", 48, R1, 4),
    k(0x2a, "Backspace", "Backspace", 52, R1, 8),
    k(0x49, "Ins", "Insert", NAV, R1, 4), k(0x4a, "Home", "Home", NAV + 4, R1, 4), k(0x4b, "PgUp", "Page Up", NAV + 8, R1, 4),
    k(0x53, "Num", "Num Lock", PAD, R1, 4), k(0x54, "/", "Keypad Divide", PAD + 4, R1, 4),
    k(0x55, "*", "Keypad Multiply", PAD + 8, R1, 4), k(0x56, "-", "Keypad Minus", PAD + 12, R1, 4),

    k(0x2b, "Tab", "Tab", 0, R2, 6),
    k(0x14, "Q", "Q", 6, R2, 4), k(0x1a, "W", "W", 10, R2, 4), k(0x08, "E", "E", 14, R2, 4), k(0x15, "R", "R", 18, R2, 4),
    k(0x17, "T", "T", 22, R2, 4), k(0x1c, "Y", "Y", 26, R2, 4), k(0x18, "U", "U", 30, R2, 4), k(0x0c, "I", "I", 34, R2, 4),
    k(0x12, "O", "O", 38, R2, 4), k(0x13, "P", "P", 42, R2, 4), k(0x2f, "[", "Left Bracket", 46, R2, 4),
    k(0x30, "]", "Right Bracket", 50, R2, 4), k(0x31, "\\", "Backslash", 54, R2, 6),
    k(0x4c, "Del", "Delete", NAV, R2, 4), k(0x4d, "End", "End", NAV + 4, R2, 4), k(0x4e, "PgDn", "Page Down", NAV + 8, R2, 4),
    k(0x5f, "7", "Keypad 7", PAD, R2, 4), k(0x60, "8", "Keypad 8", PAD + 4, R2, 4), k(0x61, "9", "Keypad 9", PAD + 8, R2, 4),
    tall(0x57, "+", "Keypad Plus", PAD + 12, R2),

    k(0x39, "Caps", "Caps Lock", 0, R3, 7),
    k(0x04, "A", "A", 7, R3, 4), k(0x16, "S", "S", 11, R3, 4), k(0x07, "D", "D", 15, R3, 4), k(0x09, "F", "F", 19, R3, 4),
    k(0x0a, "G", "G", 23, R3, 4), k(0x0b, "H", "H", 27, R3, 4), k(0x0d, "J", "J", 31, R3, 4), k(0x0e, "K", "K", 35, R3, 4),
    k(0x0f, "L", "L", 39, R3, 4), k(0x33, ";", "Semicolon", 43, R3, 4), k(0x34, "'", "Apostrophe", 47, R3, 4),
    k(0x28, "Enter", "Enter", 51, R3, 9),
    k(0x5c, "4", "Keypad 4", PAD, R3, 4), k(0x5d, "5", "Keypad 5", PAD + 4, R3, 4), k(0x5e, "6", "Keypad 6", PAD + 8, R3, 4),

    k(0xe1, "Shift", "Left Shift", 0, R4, 9),
    k(0x1d, "Z", "Z", 9, R4, 4), k(0x1b, "X", "X", 13, R4, 4), k(0x06, "C", "C", 17, R4, 4), k(0x19, "V", "V", 21, R4, 4),
    k(0x05, "B", "B", 25, R4, 4), k(0x11, "N", "N", 29, R4, 4), k(0x10, "M", "M", 33, R4, 4), k(0x36, ",", "Comma", 37, R4, 4),
    k(0x37, ".", "Period", 41, R4, 4), k(0x38, "/", "Slash", 45, R4, 4), k(0xe5, "Shift", "Right Shift", 49, R4, 11),
    k(0x52, "↑", "Up", NAV + 4, R4, 4),
    k(0x59, "1", "Keypad 1", PAD, R4, 4), k(0x5a, "2", "Keypad 2", PAD + 4, R4, 4), k(0x5b, "3", "Keypad 3", PAD + 8, R4, 4),
    tall(0x58, "Enter", "Keypad Enter", PAD + 12, R4),

    k(0xe0, "Ctrl", "Left Ctrl", 0, R5, 5), k(0xe3, "Super", "Left Super", 5, R5, 5), k(0xe2, "Alt", "Left Alt", 10, R5, 5),
    k(0x2c, "Space", "Space", 15, R5, 25), k(0xe6, "Alt", "Right Alt", 40, R5, 5),
    Key { usage: None, label: "Fn", name: "Fn", x: 45, y: R5, w: 5, h: 4 },
    k(0x65, "Menu", "Menu", 50, R5, 5), k(0xe4, "Ctrl", "Right Ctrl", 55, R5, 5),
    k(0x50, "←", "Left", NAV, R5, 4), k(0x51, "↓", "Down", NAV + 4, R5, 4), k(0x4f, "→", "Right", NAV + 8, R5, 4),
    k(0x62, "0", "Keypad 0", PAD, R5, 8), k(0x63, ".", "Keypad Period", PAD + 8, R5, 4),
];

/// One-click selections in the editor.
pub const GROUPS: &[(&str, &[u8])] = &[
    ("WASD", &[0x1a, 0x04, 0x16, 0x07]),
    ("Arrows", &[0x4f, 0x50, 0x51, 0x52]),
    ("F-keys", &[0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45]),
    ("Number Row", &[0x35, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x2d, 0x2e]),
    ("Numpad", &[0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x5b, 0x5c, 0x5d, 0x5e, 0x5f, 0x60, 0x61, 0x62, 0x63]),
    ("Modifiers", &[0xe0, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6]),
];

/// The selectable key whose rectangle contains (x, y), in quarter-key units.
pub fn key_at(x: f64, y: f64) -> Option<u8> {
    KEYS.iter()
        .find(|k| {
            let (x0, y0) = (f64::from(k.x), f64::from(k.y));
            x >= x0 && x < x0 + f64::from(k.w) && y >= y0 && y < y0 + f64::from(k.h)
        })
        .and_then(|k| k.usage)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use g512_hidpp::perkey::KeyId;

    use super::*;

    #[test]
    fn every_key_is_addressable_and_unique() {
        let usages: Vec<u8> = KEYS.iter().filter_map(|k| k.usage).collect();
        let unique: HashSet<u8> = usages.iter().copied().collect();
        assert_eq!(unique.len(), usages.len(), "duplicate usage");
        for u in &usages {
            assert!(KeyId::new(*u).is_ok(), "{u:#04x} is not in the key set");
        }
        // Every addressable key except Right Super, which the G512 has not got.
        assert_eq!(usages.len(), 103);
        assert!(!unique.contains(&0xe7));
        for (name, group) in GROUPS {
            assert!(group.iter().all(|u| unique.contains(u)), "group {name}");
        }
    }

    #[test]
    fn keys_do_not_overlap_and_fit() {
        for (i, a) in KEYS.iter().enumerate() {
            assert!(a.x + a.w <= WIDTH && a.y + a.h <= HEIGHT, "{} outside", a.name);
            for b in &KEYS[i + 1..] {
                let apart = a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y;
                assert!(apart, "{} overlaps {}", a.name, b.name);
            }
        }
    }

    #[test]
    fn main_rows_are_fifteen_keys_wide() {
        for row in [R1, R2, R3, R4, R5] {
            let right = KEYS.iter().filter(|k| k.y == row && k.x < NAV).map(|k| k.x + k.w).max();
            assert_eq!(right, Some(60), "row at {row}");
        }
    }

    #[test]
    fn hit_testing() {
        assert_eq!(key_at(10.5, 11.0), Some(0x1a), "W");
        assert_eq!(key_at(87.0, 16.0), Some(0x57), "lower half of keypad plus");
        assert_eq!(key_at(46.0, 23.0), None, "Fn");
        assert_eq!(key_at(5.0, 1.0), None, "gap after Esc");
    }
}
