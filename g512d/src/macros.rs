//! Mouse macros as front ends send and see them: the play mode as `once`,
//! `held` or `toggle`, and each step as (kind, value, modifiers), where the
//! kind is `key-down`, `key-up`, `button-down`, `button-up`, `media-down`,
//! `media-up`, `pause` (milliseconds) or `wait-release`.

use g512_hidpp::macros::{PlayMode, Step};

use crate::buttons::{KEY_USAGES, MEDIA_USAGES, MOUSE_BUTTONS};

pub fn mode_to_bus(mode: PlayMode) -> &'static str {
    match mode {
        PlayMode::Once => "once",
        PlayMode::WhileHeld => "held",
        PlayMode::UntilPressedAgain => "toggle",
    }
}

pub fn mode_from_bus(mode: &str) -> Result<PlayMode, String> {
    match mode {
        "once" => Ok(PlayMode::Once),
        "held" => Ok(PlayMode::WhileHeld),
        "toggle" => Ok(PlayMode::UntilPressedAgain),
        _ => Err(format!("unknown play mode {mode:?}; use once, held or toggle")),
    }
}

pub fn step_to_bus(step: Step) -> (&'static str, u16, u8) {
    match step {
        Step::KeyDown { modifiers, usage } => ("key-down", u16::from(usage), modifiers),
        Step::KeyUp { modifiers, usage } => ("key-up", u16::from(usage), modifiers),
        Step::ButtonDown(mask) => ("button-down", mask, 0),
        Step::ButtonUp(mask) => ("button-up", mask, 0),
        Step::MediaDown(usage) => ("media-down", usage, 0),
        Step::MediaUp(usage) => ("media-up", usage, 0),
        Step::Pause(ms) => ("pause", ms, 0),
        Step::WaitForRelease => ("wait-release", 0, 0),
    }
}

/// A step from the bus; anything the mouse shouldn't be given is refused.
pub fn step_from_bus(kind: &str, value: u16, modifiers: u8) -> Result<Step, String> {
    if !kind.starts_with("key-") && modifiers != 0 {
        return Err(format!("modifiers only go with a key step, not {kind:?}"));
    }
    let key = |value: u16| KEY_USAGES.contains(&value).then_some(value as u8);
    let step = match kind {
        "key-down" => key(value).map(|usage| Step::KeyDown { modifiers, usage }),
        "key-up" => key(value).map(|usage| Step::KeyUp { modifiers, usage }),
        "button-down" => MOUSE_BUTTONS.contains(&value).then_some(Step::ButtonDown(value)),
        "button-up" => MOUSE_BUTTONS.contains(&value).then_some(Step::ButtonUp(value)),
        "media-down" => MEDIA_USAGES.contains(&value).then_some(Step::MediaDown(value)),
        "media-up" => MEDIA_USAGES.contains(&value).then_some(Step::MediaUp(value)),
        "pause" => (value != 0).then_some(Step::Pause(value)),
        "wait-release" => (value == 0).then_some(Step::WaitForRelease),
        _ => return Err(format!("unknown step kind {kind:?}")),
    };
    step.ok_or_else(|| format!("{value:#06x} is not a valid {kind} value"))
}

/// A short fingerprint of a macro's bytes (64-bit FNV-1a), stored with its
/// name: when the mouse's macro no longer matches, it was changed elsewhere
/// and the name no longer applies.
pub fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_round_trip() {
        let steps = [
            Step::KeyDown { modifiers: 1, usage: 0x06 },
            Step::KeyUp { modifiers: 1, usage: 0x06 },
            Step::ButtonDown(1),
            Step::ButtonUp(16),
            Step::MediaDown(0x00e9),
            Step::MediaUp(0x00e9),
            Step::Pause(50),
            Step::WaitForRelease,
        ];
        for step in steps {
            let (kind, value, modifiers) = step_to_bus(step);
            assert_eq!(step_from_bus(kind, value, modifiers), Ok(step), "{kind}");
        }
        for mode in [PlayMode::Once, PlayMode::WhileHeld, PlayMode::UntilPressedAgain] {
            assert_eq!(mode_from_bus(mode_to_bus(mode)), Ok(mode));
        }
    }

    #[test]
    fn bad_steps_are_refused() {
        for (kind, value, modifiers) in [
            ("key-down", 0x03, 0),
            ("key-up", 0x100, 0),
            ("button-down", 3, 0),
            ("media-up", 0, 0),
            ("pause", 0, 0),
            ("wait-release", 1, 0),
            ("pause", 50, 1),
            ("jump", 6, 0),
        ] {
            assert!(step_from_bus(kind, value, modifiers).is_err(), "{kind} {value} {modifiers}");
        }
        assert!(mode_from_bus("release").is_err());
    }

    #[test]
    fn fingerprint_follows_the_bytes() {
        assert_eq!(fingerprint(b""), 0xcbf2_9ce4_8422_2325);
        assert_ne!(fingerprint(&[0x43, 0, 4, 0xff]), fingerprint(&[0x43, 0, 5, 0xff]));
    }
}
