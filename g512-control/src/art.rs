//! What the device illustrations show: which keys are lit and in what
//! colour, and the colour the drawings use. Pure, so it can be tested
//! without a display; the widgets only draw what this returns.

use crate::{
    layout::{KEYS, Key},
    motion::{self, Loop},
    service::{CUSTOM, Keys, LightingState},
};

pub type Rgb = (u8, u8, u8);

/// The illustration card's background in the dark style (#1D1D20,
/// `color.bg.illustration`).
pub const CARD: Rgb = (0x1d, 0x1d, 0x20);
/// The card in the light style (#F6F6F7), like GNOME Settings' illustrations.
pub const CARD_LIGHT: Rgb = (0xf6, 0xf6, 0xf7);

/// The card for the system's style.
pub fn card(dark: bool) -> Rgb {
    if dark { CARD } else { CARD_LIGHT }
}

/// Dim lines for an unlit drawing on `card`: white on the dark card, near-black
/// on the light one.
pub fn dim_ink(card: Rgb) -> Rgb {
    if luminance(card) > 0.5 { (0, 0, 6) } else { (255, 255, 255) }
}
/// Least contrast between a drawing colour and the card (non-text, WCAG).
const MIN_CONTRAST: f64 = 3.0;

/// What the keyboard is showing, as far as the drawing cares.
#[derive(Clone, Debug, PartialEq)]
pub enum Shown<'a> {
    /// Lights off, or the Off effect.
    Dark,
    /// A zone effect in one colour (Static, Breathe, Ripple).
    Colour { effect: &'a str, colour: Rgb },
    /// An effect that makes its own colours (Cycle, Wave).
    Rainbow { effect: &'a str },
    /// A key map: the Custom map, or the Gaming map during a game.
    Map(&'a Keys),
}

/// What the keyboard shows, from the service's view of it.
pub fn shown<'a>(lights_on: bool, gaming_shown: bool, state: &'a LightingState, custom: &'a Keys, gaming: &'a Keys) -> Shown<'a> {
    if !lights_on || state.brightness == 0 {
        return Shown::Dark;
    }
    if gaming_shown {
        return Shown::Map(gaming);
    }
    match state.effect.as_str() {
        "Off" => Shown::Dark,
        CUSTOM => Shown::Map(custom),
        "Cycle" | "Wave" => Shown::Rainbow { effect: &state.effect },
        effect => Shown::Colour { effect, colour: state.color },
    }
}

/// The key whose colour a matching mouse takes from a key map (W), as in
/// `g512d/src/mouse.rs`.
const MOUSE_MAP_KEY: u8 = 0x1a;

/// What the mouse shows, derived as g512d does: dark while the keyboard's
/// lights are off; its own setting while not matching; otherwise the
/// keyboard's, translated to what the mouse has (Wave as Cycle, Ripple as
/// Static, a key map as Static in its W key's colour). `keyboard` is `None`
/// while the keyboard is missing, and the mouse keeps its own setting.
pub fn mouse_shown<'a>(
    lights_on: bool,
    matching: bool,
    own: &'a LightingState,
    keyboard: Option<Shown<'a>>,
) -> Shown<'a> {
    if !lights_on {
        return Shown::Dark;
    }
    match keyboard {
        Some(kb) if matching => match kb {
            Shown::Map(keys) => keys
                .iter()
                .find(|k| k.0 == MOUSE_MAP_KEY)
                .map_or(Shown::Dark, |&(_, r, g, b)| Shown::Colour { effect: "Static", colour: (r, g, b) }),
            Shown::Rainbow { .. } => Shown::Rainbow { effect: "Cycle" },
            Shown::Colour { effect: "Ripple", colour } => Shown::Colour { effect: "Static", colour },
            other => other,
        },
        _ => shown(true, false, own, &NO_KEYS, &NO_KEYS),
    }
}

static NO_KEYS: Keys = Vec::new();

/// The loop the shown effect runs, if it moves.
pub fn shown_loop(shown: &Shown) -> Option<Loop> {
    match shown {
        Shown::Colour { effect, .. } | Shown::Rainbow { effect } => motion::effect_loop(effect),
        Shown::Dark | Shown::Map(_) => None,
    }
}

/// Each key's fill, in `KEYS` order: `Some((colour, level))` for a lit key
/// (`level` 0 to 1 from the animation; 1 when still), `None` for an
/// outline. `level_at` gives the animation's level for a key; Fn has no
/// light and is always `None`. `cycle` is Cycle's hue, in degrees.
pub fn key_colours(shown: &Shown, level_at: &dyn Fn(&Key) -> f64, cycle: f64) -> Vec<Option<(Rgb, f64)>> {
    KEYS.iter()
        .map(|key| {
            let usage = key.usage?;
            let colour = match shown {
                Shown::Dark => return None,
                Shown::Colour { colour, .. } => *colour,
                Shown::Rainbow { effect: "Cycle" } => hue(cycle),
                Shown::Rainbow { .. } => hue(f64::from(key.x) / f64::from(crate::layout::WIDTH) * 360.0),
                Shown::Map(keys) => keys.iter().find(|k| k.0 == usage).map(|&(_, r, g, b)| (r, g, b))?,
            };
            let level = level_at(key).clamp(0.0, 1.0);
            (level > 0.02).then_some((colour, level))
        })
        .collect()
}

/// The colour the case outline takes: the effect's colour, the map's most
/// common colour, or `None` when nothing is lit. `cycle` is Cycle's hue.
pub fn main_colour(shown: &Shown, cycle: f64) -> Option<Rgb> {
    match shown {
        Shown::Dark => None,
        Shown::Colour { colour, .. } => Some(*colour),
        Shown::Rainbow { .. } => Some(hue(cycle)),
        Shown::Map(keys) => {
            let mut counts: Vec<(Rgb, usize)> = Vec::new();
            for &(_, r, g, b) in keys.iter() {
                match counts.iter_mut().find(|(c, _)| *c == (r, g, b)) {
                    Some((_, n)) => *n += 1,
                    None => counts.push(((r, g, b), 1)),
                }
            }
            counts.into_iter().max_by_key(|&(_, n)| n).map(|(c, _)| c)
        }
    }
}

/// A fully saturated colour at `degrees` on the hue wheel.
pub fn hue(degrees: f64) -> Rgb {
    let h = degrees.rem_euclid(360.0) / 60.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    let (r, g, b) = match h as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    let ch = |v: f64| (v * 255.0).round() as u8;
    (ch(r), ch(g), ch(b))
}

fn luminance((r, g, b): Rgb) -> f64 {
    let ch = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.039_28 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b)
}

pub fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// The colour a drawing uses for `led`: unchanged when it reaches 3:1 on
/// the card, otherwise mixed away from the card (toward white on the dark
/// card, toward black on the light one), keeping its hue, until it does.
/// Only the drawing changes; the device keeps the real colour.
pub fn drawing_colour(led: Rgb, card: Rgb) -> Rgb {
    let toward = if luminance(card) > 0.5 { 0.0 } else { 255.0 };
    let mix = |t: f64| {
        let ch = |v: u8| (f64::from(v) + (toward - f64::from(v)) * t).round() as u8;
        (ch(led.0), ch(led.1), ch(led.2))
    };
    let end = mix(1.0);
    (0..=100).map(|step| mix(f64::from(step) / 100.0)).find(|&c| contrast(c, card) >= MIN_CONTRAST).unwrap_or(end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(effect: &str) -> LightingState {
        LightingState { effect: effect.into(), color: (0x26, 0xa2, 0x69), speed: 50, brightness: 100 }
    }

    fn lit(colours: &[Option<(Rgb, f64)>]) -> usize {
        colours.iter().filter(|c| c.is_some()).count()
    }

    const STILL: &dyn Fn(&Key) -> f64 = &|_| 1.0;
    static EMPTY: Keys = Vec::new();

    #[test]
    fn custom_map_lights_only_its_keys() {
        let wasd: Keys = [0x1a, 0x04, 0x16, 0x07].iter().map(|&u| (u, 0x26, 0xa2, 0x69)).collect();
        let s = state(CUSTOM);
        let shown = shown(true, false, &s, &wasd, &EMPTY);
        let colours = key_colours(&shown, STILL, 0.0);
        assert_eq!(lit(&colours), 4);
        assert_eq!(main_colour(&shown, 0.0), Some((0x26, 0xa2, 0x69)));
    }

    #[test]
    fn effect_lights_every_key_but_fn() {
        let s = state("Ripple");
        let colours = key_colours(&shown(true, false, &s, &EMPTY, &EMPTY), STILL, 0.0);
        assert_eq!(colours.len(), KEYS.len());
        assert_eq!(lit(&colours), 103, "all but Fn");
        let fn_index = KEYS.iter().position(|k| k.usage.is_none()).unwrap();
        assert_eq!(colours[fn_index], None);
    }

    #[test]
    fn lights_off_lights_nothing() {
        let s = state("Static");
        for shown in [shown(false, false, &s, &EMPTY, &EMPTY), shown(true, false, &state("Off"), &EMPTY, &EMPTY)] {
            assert_eq!(lit(&key_colours(&shown, STILL, 0.0)), 0);
            assert_eq!(main_colour(&shown, 0.0), None);
        }
        let dim = LightingState { brightness: 0, ..state("Static") };
        assert_eq!(shown(true, false, &dim, &EMPTY, &EMPTY), Shown::Dark);
    }

    #[test]
    fn gaming_map_wins_during_a_game() {
        let gaming: Keys = vec![(0x1a, 0xff, 0x20, 0x00)];
        let s = state("Ripple");
        assert_eq!(shown(true, true, &s, &EMPTY, &gaming), Shown::Map(&gaming));
    }

    #[test]
    fn animation_level_is_carried() {
        let s = state("Breathe");
        let colours = key_colours(&shown(true, false, &s, &EMPTY, &EMPTY), &|_| 0.5, 0.0);
        assert!(colours.iter().flatten().all(|&(_, level)| level == 0.5));
        assert_eq!(lit(&key_colours(&shown(true, false, &s, &EMPTY, &EMPTY), &|_| 0.0, 0.0)), 0, "a level of 0 is unlit");
    }

    #[test]
    fn mouse_follows_the_service_rules() {
        let own = state("Breathe");
        let kb = state("Ripple");
        let kb_shown = || Some(shown(true, false, &kb, &EMPTY, &EMPTY));
        assert_eq!(mouse_shown(true, false, &own, kb_shown()), Shown::Colour { effect: "Breathe", colour: own.color });
        assert_eq!(mouse_shown(true, true, &own, kb_shown()), Shown::Colour { effect: "Static", colour: kb.color });
        assert_eq!(mouse_shown(false, false, &own, kb_shown()), Shown::Dark, "keyboard lights off");
        assert_eq!(
            mouse_shown(true, true, &own, None),
            Shown::Colour { effect: "Breathe", colour: own.color },
            "no keyboard"
        );
        let wave = state("Wave");
        assert_eq!(
            mouse_shown(true, true, &own, Some(shown(true, false, &wave, &EMPTY, &EMPTY))),
            Shown::Rainbow { effect: "Cycle" }
        );
        let map: Keys = vec![(0x04, 1, 2, 3), (0x1a, 0xff, 0x20, 0x00)];
        let custom = state(CUSTOM);
        let from_map = mouse_shown(true, true, &own, Some(shown(true, false, &custom, &map, &EMPTY)));
        assert_eq!(from_map, Shown::Colour { effect: "Static", colour: (0xff, 0x20, 0x00) }, "the W key's colour");
        let no_w: Keys = vec![(0x04, 1, 2, 3)];
        assert_eq!(mouse_shown(true, true, &own, Some(shown(true, false, &custom, &no_w, &EMPTY))), Shown::Dark);
    }

    #[test]
    fn drawing_colour_keeps_contrast() {
        assert_eq!(drawing_colour((0x26, 0xa2, 0x69), CARD), (0x26, 0xa2, 0x69), "green is already fine");
        assert_eq!(drawing_colour((0x1c, 0x71, 0xd8), CARD), (0x1c, 0x71, 0xd8), "deep blue is already fine");
        let lifted = drawing_colour((0x10, 0x10, 0x10), CARD);
        assert!(contrast(lifted, CARD) >= 3.0, "{lifted:?}");
        assert!(lifted.0 == lifted.1 && lifted.1 == lifted.2, "a grey stays grey");
        let navy = drawing_colour((0, 0, 0x50), CARD);
        assert!(contrast(navy, CARD) >= 3.0 && navy.2 > navy.0, "navy stays blue: {navy:?}");
    }

    #[test]
    fn drawing_colour_darkens_on_the_light_card() {
        for led in [(255, 255, 255), (0xf6, 0xd3, 0x2d), (0x26, 0xa2, 0x69), (0x99, 0xc1, 0xf1)] {
            let c = drawing_colour(led, CARD_LIGHT);
            assert!(contrast(c, CARD_LIGHT) >= 3.0, "{led:?} -> {c:?}");
            assert!(c.0 <= led.0 && c.1 <= led.1 && c.2 <= led.2, "darkened, not lightened: {led:?} -> {c:?}");
        }
        assert_eq!(drawing_colour((0x1c, 0x71, 0xd8), CARD_LIGHT), (0x1c, 0x71, 0xd8), "deep blue is already fine");
        let white = drawing_colour((255, 255, 255), CARD_LIGHT);
        assert!(white.0 == white.1 && white.1 == white.2, "white becomes a grey: {white:?}");
    }

    #[test]
    fn card_follows_the_style() {
        assert_eq!(card(true), CARD);
        assert_eq!(card(false), CARD_LIGHT);
        assert_eq!(dim_ink(CARD), (255, 255, 255));
        assert_eq!(dim_ink(CARD_LIGHT), (0, 0, 6));
    }

    #[test]
    fn hue_wheel() {
        assert_eq!(hue(0.0), (255, 0, 0));
        assert_eq!(hue(120.0), (0, 255, 0));
        assert_eq!(hue(240.0), (0, 0, 255));
        assert_eq!(hue(360.0), (255, 0, 0));
    }
}
