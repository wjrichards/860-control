//! Onboard macros on the G502 HERO, macro format 1 (spike 10): one macro per
//! macro sector, starting at byte 0, played by a button entry `00 ss 00 00`.
//!
//! A macro is an optional repeat mode, its steps, and the end command `ff`.
//! Steps are three bytes, except wait-for-release (`01`), which is one. The
//! rest of the sector is `ff` padding and the CRC, as in profile sectors.
//! The jump command (`60`) is never written, so a macro never continues into
//! another sector.

use crate::{Error, Result, profiles::crc};

const WAIT_FOR_RELEASE: u8 = 0x01;
const REPEAT_WHILE_HELD: u8 = 0x02;
const REPEAT_UNTIL_PRESSED: u8 = 0x03;
const PAUSE: u8 = 0x40;
const BUTTON_DOWN: u8 = 0x41;
const BUTTON_UP: u8 = 0x42;
const KEY_DOWN: u8 = 0x43;
const KEY_UP: u8 = 0x44;
const MEDIA_DOWN: u8 = 0x45;
const MEDIA_UP: u8 = 0x46;
const END: u8 = 0xff;

/// How the macro plays when its button is pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayMode {
    /// Once per press.
    Once,
    /// `02`: over and over while the button is held.
    WhileHeld,
    /// `03`: over and over until the button is pressed again.
    UntilPressedAgain,
}

/// One step of a macro.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// `43 mm kk`: a key goes down, HID usage `kk` with modifier bits `mm`.
    KeyDown { modifiers: u8, usage: u8 },
    /// `44 mm kk`: a key comes up.
    KeyUp { modifiers: u8, usage: u8 },
    /// `41 hi lo`: mouse buttons go down (1 left, 2 right, 4 middle, 8 back, 16 forward).
    ButtonDown(u16),
    /// `42 hi lo`: mouse buttons come up.
    ButtonUp(u16),
    /// `45 hi lo`: a consumer (media) usage goes down.
    MediaDown(u16),
    /// `46 hi lo`: a consumer usage comes up.
    MediaUp(u16),
    /// `40 hi lo`: wait this many milliseconds, 1 to 65535.
    Pause(u16),
    /// `01`: the steps after it play when the button is let go.
    WaitForRelease,
}

/// A macro as the mouse stores it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Macro {
    pub mode: PlayMode,
    pub steps: Vec<Step>,
}

/// What a macro sector holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SectorContent {
    /// Nothing but `ff` before the CRC, or a CRC that doesn't match (new
    /// mice ship these sectors as zeros): nothing is stored here.
    Free,
    Macro(Macro),
    /// A valid CRC, but commands this module doesn't know: kept as it is.
    Unknown,
}

impl Macro {
    /// The commands, ending with `ff`. Refuses a pause of 0 ms and an empty
    /// mouse-button mask.
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut out = match self.mode {
            PlayMode::Once => vec![],
            PlayMode::WhileHeld => vec![REPEAT_WHILE_HELD],
            PlayMode::UntilPressedAgain => vec![REPEAT_UNTIL_PRESSED],
        };
        for step in &self.steps {
            let three = |command: u8, [hi, lo]: [u8; 2]| [command, hi, lo];
            match *step {
                Step::KeyDown { modifiers, usage } => out.extend([KEY_DOWN, modifiers, usage]),
                Step::KeyUp { modifiers, usage } => out.extend([KEY_UP, modifiers, usage]),
                Step::ButtonDown(0) | Step::ButtonUp(0) => {
                    return Err(Error::InvalidProfile("an empty mouse-button mask".into()));
                }
                Step::ButtonDown(mask) => out.extend(three(BUTTON_DOWN, mask.to_be_bytes())),
                Step::ButtonUp(mask) => out.extend(three(BUTTON_UP, mask.to_be_bytes())),
                Step::MediaDown(usage) => out.extend(three(MEDIA_DOWN, usage.to_be_bytes())),
                Step::MediaUp(usage) => out.extend(three(MEDIA_UP, usage.to_be_bytes())),
                Step::Pause(0) => return Err(Error::InvalidProfile("a pause of 0 ms".into())),
                Step::Pause(ms) => out.extend(three(PAUSE, ms.to_be_bytes())),
                Step::WaitForRelease => out.push(WAIT_FOR_RELEASE),
            }
        }
        out.push(END);
        Ok(out)
    }

    /// A whole sector of `size` bytes: the commands, `ff` padding and the
    /// CRC. Refuses a macro that doesn't fit before the CRC.
    pub fn sector(&self, size: usize) -> Result<Vec<u8>> {
        let mut bytes = self.encode()?;
        if bytes.len() > size.saturating_sub(2) {
            return Err(Error::InvalidProfile(format!("a macro of {} bytes in a {size}-byte sector", bytes.len())));
        }
        bytes.resize(size - 2, END);
        bytes.extend(crc(&bytes).to_be_bytes());
        Ok(bytes)
    }

    /// The commands from the start of `bytes` up to `ff`, or `None` when they
    /// include anything this module doesn't know.
    pub fn decode(bytes: &[u8]) -> Option<Macro> {
        let (mode, mut rest) = match *bytes.first()? {
            REPEAT_WHILE_HELD => (PlayMode::WhileHeld, &bytes[1..]),
            REPEAT_UNTIL_PRESSED => (PlayMode::UntilPressedAgain, &bytes[1..]),
            _ => (PlayMode::Once, bytes),
        };
        let mut steps = Vec::new();
        loop {
            let (&command, after) = rest.split_first()?;
            if command == END {
                return Some(Macro { mode, steps });
            }
            if command == WAIT_FOR_RELEASE {
                steps.push(Step::WaitForRelease);
                rest = after;
                continue;
            }
            let [a, b] = [*after.first()?, *after.get(1)?];
            let value = u16::from_be_bytes([a, b]);
            steps.push(match command {
                KEY_DOWN => Step::KeyDown { modifiers: a, usage: b },
                KEY_UP => Step::KeyUp { modifiers: a, usage: b },
                BUTTON_DOWN => Step::ButtonDown(value),
                BUTTON_UP => Step::ButtonUp(value),
                MEDIA_DOWN => Step::MediaDown(value),
                MEDIA_UP => Step::MediaUp(value),
                PAUSE => Step::Pause(value),
                _ => return None,
            });
            rest = &after[2..];
        }
    }
}

impl SectorContent {
    /// What a whole macro sector, as read from the mouse, holds.
    pub fn of(sector: &[u8]) -> SectorContent {
        let Some(body_len) = sector.len().checked_sub(2) else {
            return SectorContent::Free;
        };
        let (body, stored) = sector.split_at(body_len);
        // A cleared sector (`free_sector`) has a valid CRC but nothing before it.
        if body.iter().all(|&b| b == END) || crc(body) != u16::from_be_bytes([stored[0], stored[1]]) {
            return SectorContent::Free;
        }
        Macro::decode(body).map_or(SectorContent::Unknown, SectorContent::Macro)
    }

    /// The bytes of a free sector of `size` bytes: `ff` with a valid CRC.
    /// The mouse refuses to finish a write whose CRC is wrong (spike 10), so
    /// clearing a sector writes this, never zeros.
    pub fn free_sector(size: usize) -> Vec<u8> {
        let mut bytes = vec![END; size - 2];
        bytes.extend(crc(&bytes).to_be_bytes());
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tap(modifiers: u8, usage: u8) -> [Step; 2] {
        [Step::KeyDown { modifiers, usage }, Step::KeyUp { modifiers, usage }]
    }

    fn copy_line() -> Macro {
        let mut steps = Vec::new();
        steps.extend(tap(0, 0x4a));
        steps.push(Step::Pause(50));
        steps.extend(tap(0x02, 0x4d));
        steps.push(Step::Pause(50));
        steps.extend(tap(0x01, 0x06));
        Macro { mode: PlayMode::Once, steps }
    }

    #[test]
    fn copy_line_encodes_as_the_spec_says() {
        let bytes = copy_line().encode().unwrap();
        assert_eq!(
            bytes,
            [
                0x43, 0x00, 0x4a, 0x44, 0x00, 0x4a, 0x40, 0x00, 0x32, 0x43, 0x02, 0x4d, 0x44, 0x02, 0x4d, 0x40, 0x00,
                0x32, 0x43, 0x01, 0x06, 0x44, 0x01, 0x06, 0xff
            ]
        );
        let sector = copy_line().sector(256).unwrap();
        assert_eq!(sector.len(), 256);
        assert!(sector[25..254].iter().all(|&b| b == 0xff), "ff padding");
        assert_eq!(SectorContent::of(&sector), SectorContent::Macro(copy_line()));
    }

    #[test]
    fn spike_10_macros_round_trip() {
        // The bytes written to sector 6 in spike 10, each played on the user's mouse.
        let ab = [0x43, 0, 0x04, 0x44, 0, 0x04, 0x43, 0, 0x05, 0x44, 0, 0x05];
        let cases: [(&[u8], &[u8], PlayMode); 4] = [
            (&[], &[], PlayMode::Once),
            (&[], &[0x01], PlayMode::Once),
            (&[0x02], &[], PlayMode::WhileHeld),
            (&[0x03], &[], PlayMode::UntilPressedAgain),
        ];
        for (prefix, middle, mode) in cases {
            let mut bytes = prefix.to_vec();
            bytes.extend(&ab[..6]);
            bytes.extend(middle);
            bytes.extend(&ab[6..]);
            bytes.push(0xff);
            let m = Macro::decode(&bytes).unwrap();
            assert_eq!(m.mode, mode);
            assert_eq!(m.steps.contains(&Step::WaitForRelease), !middle.is_empty());
            assert_eq!(m.encode().unwrap(), bytes);
        }
        let paused = [0x43, 0, 0x04, 0x44, 0, 0x04, 0x40, 0x07, 0xd0, 0x43, 0, 0x05, 0x44, 0, 0x05, 0xff];
        assert_eq!(Macro::decode(&paused).unwrap().steps[2], Step::Pause(2000));
    }

    #[test]
    fn every_step_round_trips() {
        let m = Macro {
            mode: PlayMode::WhileHeld,
            steps: vec![
                Step::ButtonDown(1),
                Step::ButtonUp(1),
                Step::MediaDown(0x00e9),
                Step::MediaUp(0x00e9),
                Step::Pause(65535),
                Step::WaitForRelease,
                Step::KeyDown { modifiers: 0x05, usage: 0x2c },
                Step::KeyUp { modifiers: 0x05, usage: 0x2c },
            ],
        };
        assert_eq!(Macro::decode(&m.encode().unwrap()), Some(m));
    }

    #[test]
    fn too_long_is_refused() {
        // 42 taps and the end command are 253 bytes; with a mode command 254,
        // the most a 256-byte sector holds before its CRC. 43 taps are 259.
        let taps = |n: usize| Macro { mode: PlayMode::Once, steps: (0..n).flat_map(|_| tap(0, 0x04)).collect() };
        assert_eq!(taps(42).sector(256).unwrap().len(), 256);
        let full = Macro { mode: PlayMode::WhileHeld, steps: taps(42).steps };
        assert_eq!(full.encode().unwrap().len(), 254);
        assert_eq!(SectorContent::of(&full.sector(256).unwrap()), SectorContent::Macro(full));
        assert!(matches!(taps(43).sector(256), Err(Error::InvalidProfile(_))));
    }

    #[test]
    fn bad_values_are_refused() {
        for step in [Step::Pause(0), Step::ButtonDown(0), Step::ButtonUp(0)] {
            let m = Macro { mode: PlayMode::Once, steps: vec![step] };
            assert!(matches!(m.encode(), Err(Error::InvalidProfile(_))), "{step:?}");
        }
    }

    #[test]
    fn sector_contents() {
        // New mice ship macro sectors as zeros with a CRC that doesn't match.
        assert_eq!(SectorContent::of(&[0; 256]), SectorContent::Free);
        assert_eq!(SectorContent::of(&[0xff; 256]), SectorContent::Free);
        let free = SectorContent::free_sector(256);
        assert_eq!(SectorContent::of(&free), SectorContent::Free);
        assert_eq!(&free[..254], [0xff; 254]);
        // A valid CRC around commands this module doesn't know: a 00 no-op, a
        // 60 jump, and key presses that never reach the end command.
        let with_crc = |mut body: Vec<u8>, pad: u8| {
            body.resize(254, pad);
            body.extend(crc(&body).to_be_bytes());
            body
        };
        let unknown = [
            with_crc(vec![0x00, 0xff], 0xff),
            with_crc(vec![0x60, 0x00, 0x07, 0xff], 0xff),
            with_crc([0x43, 0x00, 0x04].repeat(84), 0x43),
        ];
        for sector in unknown {
            assert_eq!(SectorContent::of(&sector), SectorContent::Unknown, "{:02x?}", &sector[..4]);
        }
    }
}
