//! The effect loops the illustrations preview: Ripple's ring, Wave's band,
//! Breathe's level and Cycle's hue. Pure functions of time and speed, with
//! the timings of the `04 Motion` board and `motion-preview/render.py`.

use std::f64::consts::PI;

use crate::layout::Key;

/// An effect that moves; Static, Custom and Off have none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loop {
    Ripple,
    Wave,
    Breathe,
    Cycle,
}

/// Where Ripple's ring starts: the G key's centre, in quarter keys.
const RIPPLE_ORIGIN: (f64, f64) = (25.0, 16.0);
/// How far the ring spreads, and how wide it is.
const RIPPLE_REACH: f64 = 52.0;
const RIPPLE_WIDTH: f64 = 5.0;
/// Wave's band: where it starts, how far it travels, and its half-width.
const WAVE_START: f64 = -6.0;
const WAVE_TRAVEL: f64 = 102.0;
const WAVE_HALF_WIDTH: f64 = 10.0;
/// Breathe's lowest level.
const BREATHE_FLOOR: f64 = 0.15;

pub fn effect_loop(effect: &str) -> Option<Loop> {
    match effect {
        "Ripple" => Some(Loop::Ripple),
        "Wave" => Some(Loop::Wave),
        "Breathe" => Some(Loop::Breathe),
        "Cycle" => Some(Loop::Cycle),
        _ => None,
    }
}

/// Seconds per loop. At speed 50: a ring every 2.4 s, a 3 s sweep, a 4 s
/// breath and an 8 s turn of the hue wheel; the length scales with 50 / speed.
pub fn loop_seconds(l: Loop, speed: u8) -> f64 {
    let base = match l {
        Loop::Ripple => 2.4,
        Loop::Wave => 3.0,
        Loop::Breathe => 4.0,
        Loop::Cycle => 8.0,
    };
    base * 50.0 / f64::from(speed.max(1))
}

/// How far through its loop the effect is at `t` seconds, 0 to 1.
pub fn phase(l: Loop, speed: u8, t: f64) -> f64 {
    let len = loop_seconds(l, speed);
    t.rem_euclid(len) / len
}

pub fn ease_out_cubic(p: f64) -> f64 {
    1.0 - (1.0 - p).powi(3)
}

fn ease_in_out_sine(p: f64) -> f64 {
    -((PI * p).cos() - 1.0) / 2.0
}

/// Breathe's level at phase `p`: up from 15% to full and back.
pub fn breathe_level(p: f64) -> f64 {
    BREATHE_FLOOR + (1.0 - BREATHE_FLOOR) * ease_in_out_sine(1.0 - (1.0 - 2.0 * p).abs())
}

/// Cycle's hue at phase `p`, in degrees.
pub fn cycle_degrees(p: f64) -> f64 {
    p * 360.0
}

/// One key's level at phase `p`, 0 to 1.
pub fn key_level(l: Loop, p: f64, key: &Key) -> f64 {
    let cx = f64::from(key.x) + f64::from(key.w) / 2.0;
    let cy = f64::from(key.y) + f64::from(key.h) / 2.0;
    match l {
        Loop::Ripple => {
            let ring = ease_out_cubic(p) * RIPPLE_REACH;
            let fade = 1.0 - ((p - 0.6) / 0.4).max(0.0);
            let d = (cx - RIPPLE_ORIGIN.0).hypot((cy - RIPPLE_ORIGIN.1) * 1.2);
            fade * (1.0 - (d - ring).abs() / RIPPLE_WIDTH).max(0.0)
        }
        Loop::Wave => (1.0 - (cx - (WAVE_START + p * WAVE_TRAVEL)).abs() / WAVE_HALF_WIDTH).max(0.0),
        Loop::Breathe => breathe_level(p),
        Loop::Cycle => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::KEYS;

    fn key(usage: u8) -> &'static Key {
        KEYS.iter().find(|k| k.usage == Some(usage)).unwrap()
    }

    /// `key`'s level `t` seconds into `l` at the default speed.
    fn at(l: Loop, t: f64, usage: u8) -> f64 {
        key_level(l, phase(l, 50, t), key(usage))
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-3
    }

    // Expected values are render.py's, at the same times into each effect.

    #[test]
    fn ripple_matches_the_preview() {
        for (t, usage, want) in [
            (0.6, 0x43, 0.9884), // F10
            (0.6, 0x29, 0.6840), // Esc
            (1.5, 0x48, 0.8839), // Pause
            (1.5, 0x5c, 0.6108), // Keypad 4
            (2.0, 0x53, 0.4053), // Num Lock
            (2.0, 0x5f, 0.3722), // Keypad 7
            (2.0, 0x0a, 0.0),    // G: the ring has left it
        ] {
            assert!(close(at(Loop::Ripple, t, usage), want), "{t} s, {usage:#x}: {}", at(Loop::Ripple, t, usage));
        }
    }

    #[test]
    fn wave_matches_the_preview() {
        for (t, usage, want) in [
            (0.6, 0x3b, 0.96), // F2
            (0.6, 0x29, 0.0),  // Esc: the band has passed
            (1.5, 0x33, 1.0),  // ;
            (1.5, 0x42, 0.9),  // F9
            (2.0, 0x50, 0.9),  // Left
        ] {
            assert!(close(at(Loop::Wave, t, usage), want), "{t} s, {usage:#x}: {}", at(Loop::Wave, t, usage));
        }
    }

    #[test]
    fn breathe_matches_the_preview() {
        for (t, want) in [(0.6, 0.3252), (1.5, 0.8755), (2.0, 1.0)] {
            assert!(close(at(Loop::Breathe, t, 0x04), want), "{t} s");
        }
        assert!(close(breathe_level(0.0), BREATHE_FLOOR), "starts at the floor");
    }

    #[test]
    fn cycle_turns_the_wheel_in_eight_seconds() {
        assert!(close(cycle_degrees(phase(Loop::Cycle, 50, 2.0)), 90.0));
        assert!(close(cycle_degrees(phase(Loop::Cycle, 50, 8.0)), 0.0));
    }

    #[test]
    fn speed_scales_the_loop() {
        assert!(close(loop_seconds(Loop::Ripple, 100), 1.2), "twice as fast");
        assert!(close(loop_seconds(Loop::Breathe, 25), 8.0), "half as fast");
        assert!(close(at(Loop::Ripple, 0.6, 0x43), key_level(Loop::Ripple, phase(Loop::Ripple, 100, 0.3), key(0x43))));
        assert!(loop_seconds(Loop::Wave, 0).is_finite(), "speed 0 doesn't divide by zero");
    }

    #[test]
    fn still_effects_have_no_loop() {
        for effect in ["Static", "Custom", "Off"] {
            assert_eq!(effect_loop(effect), None);
        }
        assert_eq!(effect_loop("Breathe"), Some(Loop::Breathe));
    }
}
