//! Mouse button names, what each action is called ("Ctrl+C", "DPI up",
//! "Middle click"), and the keys a shortcut can use: GDK keyvals to HID
//! usages.

use gtk::gdk::{self, Key};

use crate::service::ButtonAction;

/// The G502 HERO's buttons in the mouse's order (spike 8).
const BUTTON_NAMES: [&str; 11] = [
    "Left Click",
    "Right Click",
    "Wheel Click",
    "Back",
    "Forward",
    "Sniper",
    "DPI Down",
    "DPI Up",
    "G9",
    "Wheel Tilt Right",
    "Wheel Tilt Left",
];

pub const MOUSE_BUTTONS: [(u16, &str); 5] =
    [(1, "Left click"), (2, "Right click"), (4, "Middle click"), (8, "Back"), (16, "Forward")];

/// Consumer usages; Volume Up was checked on the mouse (spike 9).
pub const MEDIA_KEYS: [(u16, &str); 6] = [
    (0x00cd, "Play/Pause"),
    (0x00b5, "Next track"),
    (0x00b6, "Previous track"),
    (0x00e9, "Volume up"),
    (0x00ea, "Volume down"),
    (0x00e2, "Mute"),
];

pub const SPECIALS: [(u16, &str); 8] = [
    (0x03, "DPI up"),
    (0x04, "DPI down"),
    (0x05, "DPI cycle"),
    (0x07, "DPI shift (sniper)"),
    (0x0a, "Next profile"),
    (0x01, "Scroll left"),
    (0x02, "Scroll right"),
    (0x0b, "G-shift"),
];

/// HID modifier bits; the right-hand keys use the same bits shifted by 4.
const MODIFIERS: [(u8, &str, gdk::ModifierType); 4] = [
    (1, "Ctrl", gdk::ModifierType::CONTROL_MASK),
    (2, "Shift", gdk::ModifierType::SHIFT_MASK),
    (4, "Alt", gdk::ModifierType::ALT_MASK),
    (8, "Super", gdk::ModifierType::SUPER_MASK),
];

/// Keys a shortcut can use: GDK keyval, HID usage, label.
const KEYS: [(Key, u8, &str); 59] = [
    (Key::a, 0x04, "A"),
    (Key::b, 0x05, "B"),
    (Key::c, 0x06, "C"),
    (Key::d, 0x07, "D"),
    (Key::e, 0x08, "E"),
    (Key::f, 0x09, "F"),
    (Key::g, 0x0a, "G"),
    (Key::h, 0x0b, "H"),
    (Key::i, 0x0c, "I"),
    (Key::j, 0x0d, "J"),
    (Key::k, 0x0e, "K"),
    (Key::l, 0x0f, "L"),
    (Key::m, 0x10, "M"),
    (Key::n, 0x11, "N"),
    (Key::o, 0x12, "O"),
    (Key::p, 0x13, "P"),
    (Key::q, 0x14, "Q"),
    (Key::r, 0x15, "R"),
    (Key::s, 0x16, "S"),
    (Key::t, 0x17, "T"),
    (Key::u, 0x18, "U"),
    (Key::v, 0x19, "V"),
    (Key::w, 0x1a, "W"),
    (Key::x, 0x1b, "X"),
    (Key::y, 0x1c, "Y"),
    (Key::z, 0x1d, "Z"),
    (Key::_1, 0x1e, "1"),
    (Key::_2, 0x1f, "2"),
    (Key::_3, 0x20, "3"),
    (Key::_4, 0x21, "4"),
    (Key::_5, 0x22, "5"),
    (Key::_6, 0x23, "6"),
    (Key::_7, 0x24, "7"),
    (Key::_8, 0x25, "8"),
    (Key::_9, 0x26, "9"),
    (Key::_0, 0x27, "0"),
    (Key::Return, 0x28, "Enter"),
    (Key::Escape, 0x29, "Esc"),
    (Key::BackSpace, 0x2a, "Backspace"),
    (Key::Tab, 0x2b, "Tab"),
    (Key::space, 0x2c, "Space"),
    (Key::minus, 0x2d, "-"),
    (Key::equal, 0x2e, "="),
    (Key::bracketleft, 0x2f, "["),
    (Key::bracketright, 0x30, "]"),
    (Key::backslash, 0x31, "\\"),
    (Key::semicolon, 0x33, ";"),
    (Key::apostrophe, 0x34, "'"),
    (Key::grave, 0x35, "`"),
    (Key::comma, 0x36, ","),
    (Key::period, 0x37, "."),
    (Key::slash, 0x38, "/"),
    (Key::Insert, 0x49, "Insert"),
    (Key::Home, 0x4a, "Home"),
    (Key::Page_Up, 0x4b, "Page Up"),
    (Key::Delete, 0x4c, "Delete"),
    (Key::End, 0x4d, "End"),
    (Key::Page_Down, 0x4e, "Page Down"),
    (Key::Print, 0x46, "Print Screen"),
];

/// F1 to F12 and the arrows, which are runs of usages.
const F_KEYS: [Key; 12] =
    [Key::F1, Key::F2, Key::F3, Key::F4, Key::F5, Key::F6, Key::F7, Key::F8, Key::F9, Key::F10, Key::F11, Key::F12];
const ARROWS: [(Key, u8, &str); 4] =
    [(Key::Right, 0x4f, "Right"), (Key::Left, 0x50, "Left"), (Key::Down, 0x51, "Down"), (Key::Up, 0x52, "Up")];

pub fn button_name(index: usize) -> String {
    BUTTON_NAMES.get(index).map_or_else(|| format!("Button {}", index + 1), |n| (*n).to_string())
}

/// The HID usage for a key without its modifiers, e.g. `c` → `0x06`.
pub fn usage(key: Key) -> Option<u8> {
    let key = key.to_lower();
    if let Some(i) = F_KEYS.iter().position(|&k| k == key) {
        return Some(0x3a + i as u8);
    }
    KEYS.iter().chain(&ARROWS).find(|(k, ..)| *k == key).map(|&(_, usage, _)| usage)
}

/// HID modifier bits for the modifiers held.
pub fn modifier_bits(state: gdk::ModifierType) -> u8 {
    MODIFIERS.iter().filter(|(.., m)| state.contains(*m)).fold(0, |bits, (bit, ..)| bits | bit)
}

/// Whether the key is itself a modifier, so a shortcut isn't finished yet.
pub fn is_modifier(key: Key) -> bool {
    [
        Key::Control_L,
        Key::Control_R,
        Key::Shift_L,
        Key::Shift_R,
        Key::Alt_L,
        Key::Alt_R,
        Key::Super_L,
        Key::Super_R,
        Key::Meta_L,
        Key::Meta_R,
        Key::ISO_Level3_Shift,
    ]
    .contains(&key)
}

fn key_label(usage: u8) -> String {
    if (0x3a..=0x45).contains(&usage) {
        return format!("F{}", usage - 0x39);
    }
    KEYS.iter()
        .chain(&ARROWS)
        .find(|&&(_, u, _)| u == usage)
        .map_or_else(|| format!("Key {usage:#04x}"), |&(.., label)| label.to_string())
}

/// A shortcut as people write it, e.g. "Ctrl+Shift+T".
pub fn shortcut_label(modifiers: u8, usage: u8) -> String {
    let held = modifiers | (modifiers >> 4);
    let mut parts: Vec<String> =
        MODIFIERS.iter().filter(|(bit, ..)| held & bit != 0).map(|(_, name, _)| (*name).to_string()).collect();
    parts.push(key_label(usage));
    parts.join("+")
}

fn named(list: &[(u16, &str)], value: u16) -> Option<String> {
    list.iter().find(|&&(v, _)| v == value).map(|&(_, name)| name.to_string())
}

/// What a button does, for its row: "Ctrl+C", "DPI up", "Middle click".
pub fn describe(action: &ButtonAction) -> String {
    let v = action.value;
    match action.kind.as_str() {
        "mouse" => named(&MOUSE_BUTTONS, v).unwrap_or_else(|| format!("Mouse buttons {v:#04x}")),
        "key" => shortcut_label(action.modifiers, v as u8),
        "media" => named(&MEDIA_KEYS, v).unwrap_or_else(|| format!("Media key {v:#06x}")),
        "special" => named(&SPECIALS, v).unwrap_or_else(|| format!("Mouse function {v:#04x}")),
        "disabled" => "Disabled".into(),
        _ => "Set elsewhere (kept as it is)".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(kind: &str, value: u16, modifiers: u8) -> ButtonAction {
        ButtonAction { kind: kind.into(), value, modifiers }
    }

    #[test]
    fn descriptions() {
        assert_eq!(describe(&action("key", 0x06, 1)), "Ctrl+C");
        assert_eq!(describe(&action("key", 0x19, 1)), "Ctrl+V");
        assert_eq!(describe(&action("key", 0x17, 0x03)), "Ctrl+Shift+T");
        assert_eq!(describe(&action("key", 0x68, 0)), "Key 0x68");
        assert_eq!(describe(&action("key", 0x3a, 0x0c)), "Alt+Super+F1");
        assert_eq!(describe(&action("key", 0x4f, 0x10)), "Ctrl+Right", "right Ctrl");
        assert_eq!(describe(&action("special", 0x03, 0)), "DPI up");
        assert_eq!(describe(&action("special", 0x0a, 0)), "Next profile");
        assert_eq!(describe(&action("mouse", 4, 0)), "Middle click");
        assert_eq!(describe(&action("mouse", 3, 0)), "Mouse buttons 0x03");
        assert_eq!(describe(&action("media", 0x00e9, 0)), "Volume up");
        assert_eq!(describe(&action("disabled", 0, 0)), "Disabled");
        assert_eq!(describe(&action("other", 0, 0)), "Set elsewhere (kept as it is)");
    }

    #[test]
    fn keys_map_to_hid_usages() {
        assert_eq!(usage(Key::c), Some(0x06));
        assert_eq!(usage(Key::C), Some(0x06), "Shift held gives the capital");
        assert_eq!(usage(Key::_1), Some(0x1e));
        assert_eq!(usage(Key::_0), Some(0x27));
        assert_eq!(usage(Key::F1), Some(0x3a));
        assert_eq!(usage(Key::F12), Some(0x45));
        assert_eq!(usage(Key::Up), Some(0x52));
        assert_eq!(usage(Key::space), Some(0x2c));
        assert_eq!(usage(Key::slash), Some(0x38));
        assert_eq!(usage(Key::AudioRaiseVolume), None);
        assert_eq!(usage(Key::Control_L), None);
        // Every usage has a label of its own.
        for &(key, u, label) in KEYS.iter().chain(&ARROWS) {
            assert_eq!(usage(key), Some(u));
            assert_eq!(key_label(u), label);
        }
        assert_eq!(key_label(0x45), "F12");
    }

    #[test]
    fn modifiers() {
        use gdk::ModifierType as M;
        assert_eq!(modifier_bits(M::CONTROL_MASK), 1);
        assert_eq!(modifier_bits(M::CONTROL_MASK | M::SHIFT_MASK | M::ALT_MASK | M::SUPER_MASK), 0x0f);
        assert_eq!(modifier_bits(M::LOCK_MASK), 0, "Caps Lock isn't part of a shortcut");
        assert!(is_modifier(Key::Shift_L) && !is_modifier(Key::a));
    }

    #[test]
    fn button_names() {
        assert_eq!(button_name(0), "Left Click");
        assert_eq!(button_name(8), "G9");
        assert_eq!(button_name(11), "Button 12");
    }
}
