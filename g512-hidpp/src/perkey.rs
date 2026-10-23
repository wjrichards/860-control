//! PerKeyLighting (`0x8080`): whole-keyboard per-key colour frames.
//!
//! "Set keys" (function 3) carries 14 `key R G B` entries in a very long
//! report and "commit" (function 5) shows them. The layout follows
//! g810-led's G512/G513 path and was confirmed on the user's G512 by
//! watching the keyboard (`add-per-key-lighting/evidence/per-key.md`). No
//! other `0x8080` function is called.

use std::fmt;

use crate::{Device, Error, Feature, Result, Transport, effects::Rgb};

const FN_SET_KEYS: u8 = 3;
const FN_COMMIT: u8 = 5;
/// Reserved byte, key group 1 (the main keys), reserved byte, entry count.
const SET_KEYS_HEADER: [u8; 4] = [0x00, 0x01, 0x00, 0x0e];
const KEYS_PER_REPORT: usize = 14;

/// Number of keys in the ANSI 104-key set.
pub const KEY_COUNT: usize = 104;

/// A key, by its USB HID keyboard usage ID, from the ANSI 104-key set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyId(u8);

/// Usage ranges of the ANSI 104-key set, in frame order.
const RANGES: [(u8, u8); 7] = [
    (0x04, 0x31), // A..Z, 1..0, Enter, Esc, Backspace, Tab, Space, - = [ ] \
    (0x33, 0x39), // ; ' ` , . / Caps Lock
    (0x3a, 0x45), // F1..F12
    (0x46, 0x52), // Print Screen..Up
    (0x53, 0x63), // Num Lock..keypad .
    (0x65, 0x65), // Menu
    (0xe0, 0xe7), // Left Ctrl..Right GUI
];

const fn all_keys() -> [KeyId; KEY_COUNT] {
    let mut keys = [KeyId(0); KEY_COUNT];
    let (mut r, mut i) = (0, 0);
    while r < RANGES.len() {
        let mut usage = RANGES[r].0;
        while usage <= RANGES[r].1 {
            keys[i] = KeyId(usage);
            i += 1;
            usage += 1;
        }
        r += 1;
    }
    assert!(i == KEY_COUNT);
    keys
}

impl KeyId {
    /// Every key, in frame order.
    pub const ALL: [KeyId; KEY_COUNT] = all_keys();

    pub const LEFT_CTRL: KeyId = KeyId(0xe0);

    /// The key with this usage ID, if it is in the ANSI 104-key set.
    pub fn new(usage: u8) -> Result<KeyId> {
        Self::ALL.iter().copied().find(|k| k.0 == usage).ok_or(Error::InvalidKey(usage))
    }

    pub const fn usage(self) -> u8 {
        self.0
    }

    /// Position in [`Self::ALL`] and in a [`Frame`].
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&k| k == self).expect("KeyId values come from ALL")
    }
}

impl fmt::Display for KeyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02x}", self.0)
    }
}

/// A colour for every key in [`KeyId::ALL`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame([Rgb; KEY_COUNT]);

impl Frame {
    pub fn filled(color: Rgb) -> Self {
        Frame([color; KEY_COUNT])
    }

    pub fn get(&self, key: KeyId) -> Rgb {
        self.0[key.index()]
    }

    pub fn set(&mut self, key: KeyId, color: Rgb) {
        self.0[key.index()] = color;
    }

    pub fn iter(&self) -> impl Iterator<Item = (KeyId, Rgb)> + '_ {
        KeyId::ALL.into_iter().zip(self.0.iter().copied())
    }
}

/// The "set keys" parameter blocks for `frame`: 8 reports of up to 14
/// entries, unused entries zero.
pub fn set_keys_blocks(frame: &Frame) -> Vec<[u8; 60]> {
    let entries: Vec<_> = frame.iter().collect();
    entries
        .chunks(KEYS_PER_REPORT)
        .map(|chunk| {
            let mut block = [0u8; 60];
            block[..4].copy_from_slice(&SET_KEYS_HEADER);
            for (i, (key, c)) in chunk.iter().enumerate() {
                block[4 + 4 * i..8 + 4 * i].copy_from_slice(&[key.usage(), c.r, c.g, c.b]);
            }
            block
        })
        .collect()
}

/// Show `frame` on the keyboard: every key, then commit. Fails on the first
/// error; a frame is applied only when every request is acknowledged.
pub fn send_frame<T: Transport>(dev: &mut Device<T>, frame: &Frame) -> Result<()> {
    for block in set_keys_blocks(frame) {
        dev.request_very_long(Feature::PerKeyLighting, FN_SET_KEYS, &block)?;
    }
    dev.request(Feature::PerKeyLighting, FN_COMMIT, &[])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeKeyboard;

    const BLUE: Rgb = Rgb::new(0, 0, 0xff);

    #[test]
    fn ansi_104_key_set() {
        assert_eq!(KeyId::ALL.len(), 104);
        let mut usages: Vec<u8> = KeyId::ALL.iter().map(|k| k.usage()).collect();
        usages.dedup();
        assert_eq!(usages.len(), 104, "no duplicates");
        assert_eq!(KeyId::new(0xe0).unwrap(), KeyId::LEFT_CTRL);
        assert!(matches!(KeyId::new(0x64), Err(Error::InvalidKey(0x64))), "ISO key rejected");
        assert!(matches!(KeyId::new(0x32), Err(Error::InvalidKey(0x32))), "ISO # rejected");
        assert!(matches!(KeyId::new(0x00), Err(Error::InvalidKey(0x00))));
        assert_eq!(KeyId::new(0x29).unwrap().index(), 0x29 - 0x04);
    }

    /// Spike 5's all-blue frame, in its key order (`spike5_per_key.py`).
    #[test]
    fn blue_frame_matches_spike() {
        let blocks = set_keys_blocks(&Frame::filled(BLUE));
        assert_eq!(blocks.len(), 8);
        let mut first = vec![0x00, 0x01, 0x00, 0x0e];
        for usage in 0x04..=0x11 {
            first.extend([usage, 0, 0, 0xff]);
        }
        assert_eq!(blocks[0].to_vec(), first);
        // 104 = 7 x 14 + 6: the last report holds Left Alt..Right GUI.
        let mut last = vec![0x00, 0x01, 0x00, 0x0e];
        for usage in 0xe2..=0xe7 {
            last.extend([usage, 0, 0, 0xff]);
        }
        last.resize(60, 0);
        assert_eq!(blocks[7].to_vec(), last);
    }

    #[test]
    fn frame_is_sent_then_committed() {
        let mut dev = Device::open(FakeKeyboard::g512()).unwrap();
        let mut frame = Frame::filled(Rgb::default());
        frame.set(KeyId::LEFT_CTRL, Rgb::new(0xff, 0, 0));
        send_frame(&mut dev, &frame).unwrap();

        let sent = dev.transport_mut().sent();
        let requests = &sent[sent.len() - 9..];
        for r in &requests[..8] {
            assert_eq!(r.len(), 64);
            assert_eq!(r[..3], [0x12, 0xff, 0x0c]);
            assert_eq!(r[3] >> 4, FN_SET_KEYS);
            assert_eq!(r[4..8], SET_KEYS_HEADER);
        }
        assert_eq!(requests[8].len(), 20);
        assert_eq!(requests[8][..3], [0x11, 0xff, 0x0c]);
        assert_eq!(requests[8][3] >> 4, FN_COMMIT);

        let committed = &dev.transport_mut().frames;
        assert_eq!(committed.len(), 1);
        assert_eq!(committed[0].len(), 104);
        assert!(committed[0].contains(&[0xe0, 0xff, 0, 0]));
        assert!(committed[0].contains(&[0x29, 0, 0, 0]), "unset keys sent as off");
    }

    #[test]
    fn rejected_set_keys_fails_without_commit() {
        let mut fake = FakeKeyboard::g512();
        fake.reject_set_keys = Some(0x02);
        let mut dev = Device::open(fake).unwrap();
        match send_frame(&mut dev, &Frame::filled(BLUE)) {
            Err(Error::Hidpp { feature: Feature::PerKeyLighting, function: FN_SET_KEYS, code: 0x02 }) => {}
            other => panic!("expected HID++ error, got {other:?}"),
        }
        assert!(dev.transport_mut().frames.is_empty(), "nothing committed");
    }

    #[test]
    fn other_per_key_functions_are_refused_by_the_fake() {
        let mut dev = Device::open(FakeKeyboard::g512()).unwrap();
        assert!(matches!(
            dev.request(Feature::PerKeyLighting, 1, &[]),
            Err(Error::Hidpp { feature: Feature::PerKeyLighting, function: 1, code: 0x07 })
        ));
    }
}
