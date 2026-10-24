//! The G502 HERO mouse: its settings, and what it shows given the keyboard's.

use g512_hidpp::{
    effects::{EffectKind, Rgb},
    perkey::KeyId,
};

use crate::{
    keymap::KeyMap,
    state::{BRIGHTNESS_MAX, Effect, LightingState},
};

/// The mouse's effects (spike 6): both zones offer these four.
pub const MOUSE_EFFECTS: [EffectKind; 4] = [EffectKind::Off, EffectKind::Static, EffectKind::Breathe, EffectKind::Cycle];

/// A map's colour for the mouse to show: the W key's.
const MAP_KEY: u8 = 0x1a;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MouseSettings {
    /// Follow the keyboard (the default) or show `own`.
    pub match_keyboard: bool,
    /// The mouse's own setting; `None` until one is chosen.
    pub own: Option<LightingState>,
}

impl Default for MouseSettings {
    fn default() -> Self {
        MouseSettings { match_keyboard: true, own: None }
    }
}

/// The mouse's own setting as front ends see it: the chosen one, or before
/// any is chosen, Static in the keyboard's colour at its speed and brightness.
pub fn own_or_default(mouse: &MouseSettings, keyboard: Option<&LightingState>) -> LightingState {
    mouse.own.unwrap_or_else(|| {
        let base = keyboard.copied().unwrap_or(LightingState {
            effect: Effect::Zone(EffectKind::Static),
            color: Rgb::new(0xff, 0xff, 0xff),
            speed: 50,
            brightness: BRIGHTNESS_MAX,
        });
        LightingState { effect: Effect::Zone(EffectKind::Static), ..base }
    })
}

/// What the keyboard is showing, as far as the mouse is concerned.
pub struct KeyboardView<'a> {
    pub lights_on: bool,
    /// The Gaming map is shown because a game is running.
    pub gaming_shown: bool,
    pub saved: Option<&'a LightingState>,
    pub custom: &'a KeyMap,
    pub gaming: &'a KeyMap,
}

/// What the mouse should show; `None` leaves it as it is (nothing saved yet
/// to match). The effect is always one of [`MOUSE_EFFECTS`].
pub fn mouse_state(kb: &KeyboardView, mouse: &MouseSettings) -> Option<LightingState> {
    let off = LightingState { effect: Effect::Zone(EffectKind::Off), color: Rgb::default(), speed: 50, brightness: 0 };
    if !kb.lights_on {
        return Some(off);
    }
    if !mouse.match_keyboard {
        return Some(own_or_default(mouse, kb.saved));
    }
    let from_map = |map: &KeyMap, brightness: u8| match KeyId::new(MAP_KEY).ok().and_then(|w| map.get(w)) {
        Some(color) => LightingState { effect: Effect::Zone(EffectKind::Static), color, speed: 50, brightness },
        None => off,
    };
    if kb.gaming_shown {
        return Some(from_map(kb.gaming, kb.saved.map_or(BRIGHTNESS_MAX, |s| s.brightness)));
    }
    let saved = *kb.saved?;
    Some(match saved.effect {
        Effect::Custom => from_map(kb.custom, saved.brightness),
        Effect::Zone(EffectKind::Wave) => LightingState { effect: Effect::Zone(EffectKind::Cycle), ..saved },
        Effect::Zone(EffectKind::Ripple) => LightingState { effect: Effect::Zone(EffectKind::Static), ..saved },
        Effect::Zone(_) => saved,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(kind: EffectKind, rgb: (u8, u8, u8), speed: u8, brightness: u8) -> LightingState {
        LightingState { effect: Effect::Zone(kind), color: Rgb::new(rgb.0, rgb.1, rgb.2), speed, brightness }
    }

    fn view<'a>(saved: Option<&'a LightingState>, custom: &'a KeyMap, gaming: &'a KeyMap) -> KeyboardView<'a> {
        KeyboardView { lights_on: true, gaming_shown: false, saved, custom, gaming }
    }

    fn kind(s: Option<LightingState>) -> Option<EffectKind> {
        match s?.effect {
            Effect::Zone(k) => Some(k),
            Effect::Custom => None,
        }
    }

    #[test]
    fn supported_effects_match_unchanged() {
        let (c, g) = (KeyMap::default(), KeyMap::default_gaming());
        for k in MOUSE_EFFECTS {
            let s = state(k, (1, 2, 3), 40, 70);
            assert_eq!(mouse_state(&view(Some(&s), &c, &g), &MouseSettings::default()), Some(s), "{k:?}");
        }
    }

    #[test]
    fn wave_becomes_cycle_at_the_same_speed() {
        let (c, g) = (KeyMap::default(), KeyMap::default_gaming());
        let wave = state(EffectKind::Wave, (0, 0, 0), 80, 60);
        let shown = mouse_state(&view(Some(&wave), &c, &g), &MouseSettings::default()).unwrap();
        assert_eq!(shown, state(EffectKind::Cycle, (0, 0, 0), 80, 60));
    }

    #[test]
    fn ripple_becomes_static_in_its_colour() {
        let (c, g) = (KeyMap::default(), KeyMap::default_gaming());
        let ripple = state(EffectKind::Ripple, (0x3d, 0x38, 0x46), 52, 100);
        let shown = mouse_state(&view(Some(&ripple), &c, &g), &MouseSettings::default()).unwrap();
        assert_eq!(shown, state(EffectKind::Static, (0x3d, 0x38, 0x46), 52, 100));
    }

    #[test]
    fn custom_uses_the_w_colour_or_off() {
        let g = KeyMap::default_gaming();
        let custom = LightingState { effect: Effect::Custom, ..state(EffectKind::Off, (0, 0, 0), 50, 80) };
        let with_w = KeyMap::from_entries([(0x1a, Rgb::new(0, 0xff, 0)), (0x04, Rgb::new(9, 9, 9))]).unwrap();
        let shown = mouse_state(&view(Some(&custom), &with_w, &g), &MouseSettings::default()).unwrap();
        assert_eq!(shown, state(EffectKind::Static, (0, 0xff, 0), 50, 80));
        let without_w = KeyMap::from_entries([(0x04, Rgb::new(9, 9, 9))]).unwrap();
        assert_eq!(kind(mouse_state(&view(Some(&custom), &without_w, &g), &MouseSettings::default())), Some(EffectKind::Off));
    }

    #[test]
    fn gaming_uses_the_gaming_w_colour_at_the_saved_brightness() {
        let (c, g) = (KeyMap::default(), KeyMap::default_gaming());
        let ripple = state(EffectKind::Ripple, (1, 1, 1), 50, 40);
        let kb = KeyboardView { gaming_shown: true, ..view(Some(&ripple), &c, &g) };
        let shown = mouse_state(&kb, &MouseSettings::default()).unwrap();
        assert_eq!(shown, state(EffectKind::Static, (0xff, 0x20, 0), 50, 40));
    }

    #[test]
    fn lights_off_wins_over_everything() {
        let (c, g) = (KeyMap::default(), KeyMap::default_gaming());
        let s = state(EffectKind::Static, (9, 9, 9), 50, 100);
        let own = MouseSettings { match_keyboard: false, own: Some(s) };
        for mouse in [MouseSettings::default(), own] {
            let kb = KeyboardView { lights_on: false, gaming_shown: true, ..view(Some(&s), &c, &g) };
            assert_eq!(kind(mouse_state(&kb, &mouse)), Some(EffectKind::Off));
        }
    }

    #[test]
    fn own_setting_ignores_the_keyboard() {
        let (c, g) = (KeyMap::default(), KeyMap::default_gaming());
        let kb_state = state(EffectKind::Wave, (0, 0, 0), 50, 100);
        let blue = state(EffectKind::Static, (0, 0, 0xff), 50, 90);
        let mouse = MouseSettings { match_keyboard: false, own: Some(blue) };
        let kb = KeyboardView { gaming_shown: true, ..view(Some(&kb_state), &c, &g) };
        assert_eq!(mouse_state(&kb, &mouse), Some(blue));
    }

    #[test]
    fn own_defaults_to_static_in_the_keyboard_colour() {
        let (c, g) = (KeyMap::default(), KeyMap::default_gaming());
        let ripple = state(EffectKind::Ripple, (0x3d, 0x38, 0x46), 52, 70);
        let mouse = MouseSettings { match_keyboard: false, own: None };
        let expected = state(EffectKind::Static, (0x3d, 0x38, 0x46), 52, 70);
        assert_eq!(mouse_state(&view(Some(&ripple), &c, &g), &mouse), Some(expected));
    }

    #[test]
    fn nothing_saved_leaves_the_mouse_alone_while_matching() {
        let (c, g) = (KeyMap::default(), KeyMap::default_gaming());
        assert_eq!(mouse_state(&view(None, &c, &g), &MouseSettings::default()), None);
    }
}
