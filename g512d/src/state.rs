//! The lighting state front ends set, its validation, and how it maps onto
//! what is sent to the keyboard.

use std::fmt;

use g512_hidpp::effects::{EffectKind, Rgb};

pub const SPEED_MIN: u8 = 1;
/// The fastest speed.
pub const SPEED_MAX: u8 = 100;
pub const BRIGHTNESS_MAX: u8 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LightingState {
    pub effect: EffectKind,
    pub color: Rgb,
    /// 1 (slowest) to 100 (fastest); ignored by effects without a speed.
    pub speed: u8,
    /// 0 to 100 percent.
    pub brightness: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InvalidState {
    UnknownEffect(String),
    EffectNotOffered(EffectKind),
    Speed(u8),
    Brightness(u8),
}

impl fmt::Display for InvalidState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvalidState::UnknownEffect(name) => write!(f, "unknown effect {name:?}"),
            InvalidState::EffectNotOffered(kind) => write!(f, "the keyboard does not offer {}", kind.name()),
            InvalidState::Speed(s) => write!(f, "speed {s} is outside {SPEED_MIN}..={SPEED_MAX}"),
            InvalidState::Brightness(b) => write!(f, "brightness {b} is outside 0..={BRIGHTNESS_MAX}"),
        }
    }
}

pub fn effect_from_name(name: &str) -> Result<EffectKind, InvalidState> {
    EffectKind::ALL
        .into_iter()
        .find(|k| k.name() == name)
        .ok_or_else(|| InvalidState::UnknownEffect(name.to_string()))
}

impl LightingState {
    /// Check the state against the effects the keyboard offers.
    pub fn validate(&self, offered: &[EffectKind]) -> Result<(), InvalidState> {
        if !offered.contains(&self.effect) {
            return Err(InvalidState::EffectNotOffered(self.effect));
        }
        if !(SPEED_MIN..=SPEED_MAX).contains(&self.speed) {
            return Err(InvalidState::Speed(self.speed));
        }
        if self.brightness > BRIGHTNESS_MAX {
            return Err(InvalidState::Brightness(self.brightness));
        }
        Ok(())
    }

    /// The effect actually sent: brightness 0 turns the lighting off.
    pub fn device_effect(&self) -> EffectKind {
        if self.brightness == 0 { EffectKind::Off } else { self.effect }
    }

    /// The colour sent to the keyboard, each channel scaled by brightness.
    pub fn device_color(&self) -> Rgb {
        let scale = |c: u8| ((u16::from(c) * u16::from(self.brightness) + 50) / 100) as u8;
        Rgb::new(scale(self.color.r), scale(self.color.g), scale(self.color.b))
    }

    /// The raw speed value for the effect, or 0 when it has no speed.
    pub fn device_speed(&self) -> u16 {
        self.effect.speed_range().map_or(0, |range| speed_to_device(self.speed, *range.start(), *range.end()))
    }
}

/// Map speed 1..=100 geometrically onto `fastest..=slowest`, where a bigger
/// device value is slower: 1 gives `slowest`, 100 gives `fastest`.
fn speed_to_device(speed: u8, fastest: u16, slowest: u16) -> u16 {
    let t = f64::from(speed.clamp(SPEED_MIN, SPEED_MAX) - SPEED_MIN) / f64::from(SPEED_MAX - SPEED_MIN);
    let value = f64::from(slowest) * (f64::from(fastest) / f64::from(slowest)).powf(t);
    (value.round() as u16).clamp(fastest, slowest)
}

pub fn format_hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

pub fn parse_hex(s: &str) -> Option<Rgb> {
    let h = s.strip_prefix('#')?;
    if h.len() != 6 || !h.is_ascii() {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    Some(Rgb::new(byte(0)?, byte(2)?, byte(4)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(effect: EffectKind, hex: &str, speed: u8, brightness: u8) -> LightingState {
        LightingState { effect, color: parse_hex(hex).unwrap(), speed, brightness }
    }

    #[test]
    fn half_brightness_scales_colour_not_stored_value() {
        let s = state(EffectKind::Static, "#FF8040", 50, 50);
        assert_eq!(format_hex(s.device_color()), "#804020");
        assert_eq!(format_hex(s.color), "#ff8040");
    }

    #[test]
    fn brightness_zero_is_off() {
        let s = state(EffectKind::Breathe, "#ffffff", 50, 0);
        assert_eq!(s.device_effect(), EffectKind::Off);
        assert_eq!(state(EffectKind::Breathe, "#ffffff", 50, 1).device_effect(), EffectKind::Breathe);
    }

    #[test]
    fn rejects_out_of_range_brightness_and_speed() {
        let offered = EffectKind::ALL;
        assert_eq!(state(EffectKind::Static, "#ffffff", 50, 150).validate(&offered), Err(InvalidState::Brightness(150)));
        assert_eq!(state(EffectKind::Cycle, "#ffffff", 0, 100).validate(&offered), Err(InvalidState::Speed(0)));
        assert_eq!(state(EffectKind::Cycle, "#ffffff", 101, 100).validate(&offered), Err(InvalidState::Speed(101)));
        assert_eq!(state(EffectKind::Cycle, "#ffffff", 100, 100).validate(&offered), Ok(()));
    }

    #[test]
    fn rejects_effect_the_device_did_not_report() {
        let offered = [EffectKind::Off, EffectKind::Static];
        assert_eq!(
            state(EffectKind::Ripple, "#00ffff", 50, 100).validate(&offered),
            Err(InvalidState::EffectNotOffered(EffectKind::Ripple))
        );
    }

    #[test]
    fn speed_maps_onto_tested_ranges() {
        let raw = |effect, speed| state(effect, "#ffffff", speed, 100).device_speed();
        assert_eq!((raw(EffectKind::Breathe, 1), raw(EffectKind::Breathe, 100)), (8000, 1000));
        assert_eq!((raw(EffectKind::Wave, 1), raw(EffectKind::Wave, 100)), (250, 10));
        assert_eq!((raw(EffectKind::Ripple, 1), raw(EffectKind::Ripple, 100)), (1000, 10));
        assert_eq!(raw(EffectKind::Static, 50), 0);
        // Faster speed always gives a smaller (faster) or equal device value.
        for effect in [EffectKind::Breathe, EffectKind::Cycle, EffectKind::Wave, EffectKind::Ripple] {
            let values: Vec<u16> = (1..=100).map(|s| raw(effect, s)).collect();
            assert!(values.windows(2).all(|w| w[1] <= w[0]), "{effect:?}");
            let range = effect.speed_range().unwrap();
            assert!(values.iter().all(|v| range.contains(v)));
        }
    }

    #[test]
    fn hex_colours() {
        assert_eq!(parse_hex("#FF8040"), Some(Rgb::new(0xff, 0x80, 0x40)));
        assert_eq!(parse_hex("ff8040"), None);
        assert_eq!(parse_hex("#ff80"), None);
        assert_eq!(parse_hex("#gg8040"), None);
    }

    #[test]
    fn effect_names() {
        assert_eq!(effect_from_name("Ripple"), Ok(EffectKind::Ripple));
        assert_eq!(effect_from_name("Rainbow"), Err(InvalidState::UnknownEffect("Rainbow".into())));
    }
}
