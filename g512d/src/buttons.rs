//! Mouse button actions as front ends send and see them: (kind, value,
//! modifiers), where the kind is `mouse`, `key`, `media`, `special`,
//! `disabled` or `other` (D3).

use g512_hidpp::profiles::{ButtonAction, Special};

/// Mouse buttons a button can act as: left, right, middle, back, forward.
const MOUSE_BUTTONS: [u16; 5] = [1, 2, 4, 8, 16];
/// Keyboard usages from `a` to right GUI.
const KEY_USAGES: std::ops::RangeInclusive<u16> = 0x04..=0xe7;
/// Consumer usages, e.g. `0x00e9` Volume Up (spike 9) or `0x00cd` Play/Pause.
const MEDIA_USAGES: std::ops::RangeInclusive<u16> = 0x0001..=0x0fff;

/// An action as sent on the bus.
pub fn to_bus(action: ButtonAction) -> (&'static str, u16, u8) {
    match action {
        ButtonAction::Mouse(mask) => ("mouse", mask, 0),
        ButtonAction::Key { modifiers, usage } => ("key", u16::from(usage), modifiers),
        ButtonAction::Media(usage) => ("media", usage, 0),
        ButtonAction::Special(s) => ("special", u16::from(s as u8), 0),
        ButtonAction::Disabled => ("disabled", 0, 0),
        ButtonAction::Other(_) => ("other", 0, 0),
    }
}

/// An action from the bus; anything the mouse shouldn't be given is refused.
pub fn from_bus(kind: &str, value: u16, modifiers: u8) -> Result<ButtonAction, String> {
    if kind != "key" && modifiers != 0 {
        return Err(format!("modifiers only go with a key, not {kind:?}"));
    }
    match kind {
        "mouse" if MOUSE_BUTTONS.contains(&value) => Ok(ButtonAction::Mouse(value)),
        "key" if KEY_USAGES.contains(&value) => Ok(ButtonAction::Key { modifiers, usage: value as u8 }),
        "media" if MEDIA_USAGES.contains(&value) => Ok(ButtonAction::Media(value)),
        "special" => u8::try_from(value)
            .ok()
            .and_then(Special::from_code)
            .map(ButtonAction::Special)
            .ok_or_else(|| format!("no mouse function {value:#04x}")),
        "disabled" if value == 0 => Ok(ButtonAction::Disabled),
        "mouse" | "key" | "media" | "disabled" => Err(format!("{value:#06x} is not a valid {kind} value")),
        "other" => Err("\"other\" entries can't be written".into()),
        _ => Err(format!("unknown kind {kind:?}; use mouse, key, media, special or disabled")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_writable_action_round_trips() {
        let actions = [
            ButtonAction::Mouse(4),
            ButtonAction::Key { modifiers: 1, usage: 0x06 },
            ButtonAction::Media(0x00e9),
            ButtonAction::Special(Special::ProfileCycle),
            ButtonAction::Disabled,
        ];
        for a in actions {
            let (kind, value, modifiers) = to_bus(a);
            assert_eq!(from_bus(kind, value, modifiers), Ok(a), "{a:?}");
        }
        assert_eq!(to_bus(ButtonAction::Other([0, 1, 2, 3])), ("other", 0, 0));
    }

    #[test]
    fn bad_values_are_refused() {
        for (kind, value, modifiers) in [
            ("mouse", 0, 0),
            ("mouse", 3, 0),
            ("mouse", 32, 0),
            ("key", 0x03, 0),
            ("key", 0xe8, 0),
            ("key", 0x106, 0),
            ("media", 0, 0),
            ("media", 0x1000, 0),
            ("special", 0x06, 0),
            ("special", 0x103, 0),
            ("disabled", 1, 0),
            ("mouse", 4, 1),
            ("other", 0, 0),
            ("macro", 1, 0),
        ] {
            assert!(from_bus(kind, value, modifiers).is_err(), "{kind} {value} {modifiers}");
        }
    }
}
