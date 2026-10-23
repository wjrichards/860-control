//! An in-memory G512 that answers HID++ requests, for tests.
//!
//! [`FakeKeyboard::g512`] reproduces what the real keyboard reported to the
//! read-only probes (`evidence/feature-table.txt`, `evidence/effects-out.txt`).

use std::{collections::VecDeque, io, time::Duration};

use crate::Transport;

const ERR_INVALID_ARGUMENT: u8 = 0x02;
const ERR_INVALID_FEATURE_INDEX: u8 = 0x06;
const ERR_INVALID_FUNCTION_ID: u8 = 0x07;

#[derive(Clone, Debug)]
pub struct FakeEffect {
    pub id: u16,
    pub capabilities: u16,
    pub period: u16,
}

#[derive(Clone, Debug)]
pub struct FakeZone {
    pub location: u16,
    pub effects: Vec<FakeEffect>,
}

#[derive(Clone, Debug)]
pub struct FakeKeyboard {
    /// Feature IDs by slot.
    features: Vec<u16>,
    name: String,
    firmware: Vec<[u8; 16]>,
    lighting_info: [u8; 5],
    zones: Vec<FakeZone>,
    inbox: VecDeque<Vec<u8>>,
    sent: Vec<Vec<u8>>,
    drop_next: bool,
    dropped: Option<Vec<u8>>,
    /// Parameter bytes of every accepted `setZoneEffect`, in order.
    pub applied: Vec<[u8; 16]>,
    /// When set, `setZoneEffect` is answered with this HID++ error code.
    pub reject_set_effect: Option<u8>,
    /// Per-key entries (key, r, g, b) received since the last commit;
    /// zero-key padding is left out.
    staged_keys: Vec<[u8; 4]>,
    /// Every committed per-key frame, in order.
    pub frames: Vec<Vec<[u8; 4]>>,
    /// When set, per-key "set keys" is answered with this HID++ error code.
    pub reject_set_keys: Option<u8>,
}

impl FakeKeyboard {
    /// The user's G512, firmware U1 19.01 build 0014.
    pub fn g512() -> Self {
        let fx = |id, capabilities, period| FakeEffect { id, capabilities, period };
        FakeKeyboard {
            features: vec![
                0x0000, 0x0001, 0x0003, 0x4522, 0x0005, 0x1e00, 0x4540, 0x1eb0, 0x8060, 0x00c2,
                0x1801, 0x1802, 0x8080, 0x8070, 0x1821, 0x1bc0, 0x1806, 0x40a2, 0x1b04, 0x0020,
            ],
            name: "G512 RGB MECHANICAL GAMING KEYBOARD".into(),
            firmware: vec![
                fw_entry(0, *b"U1 ", 0x19, 0x01, 0x0014),
                fw_entry(1, *b"BOT", 0x65, 0x00, 0x6060),
            ],
            lighting_info: [0x01, 0x00, 0x01, 0x00, 0x07],
            zones: vec![FakeZone {
                location: 0x0001,
                effects: vec![
                    fx(0x0000, 0x0000, 0),
                    fx(0x0001, 0x0005, 0),
                    fx(0x000a, 0xc001, 992),
                    fx(0x0003, 0xc001, 992),
                    fx(0x0004, 0xdce1, 30),
                    fx(0x0005, 0x0000, 0),
                    fx(0x000b, 0xdce1, 30),
                ],
            }],
            inbox: VecDeque::new(),
            sent: Vec::new(),
            drop_next: false,
            dropped: None,
            applied: Vec::new(),
            reject_set_effect: None,
            staged_keys: Vec::new(),
            frames: Vec::new(),
            reject_set_keys: None,
        }
    }

    /// Simulate firmware without `feature_id`; later features move up a slot.
    pub fn remove_feature(&mut self, feature_id: u16) {
        self.features.retain(|&f| f != feature_id);
    }

    /// Move `feature_id` to the end of the feature table (a different slot).
    pub fn move_feature_to_end(&mut self, feature_id: u16) {
        self.remove_feature(feature_id);
        self.features.push(feature_id);
    }

    /// Queue a report that arrives before the next reply.
    pub fn queue_noise(&mut self, report: Vec<u8>) {
        self.inbox.push_back(report);
    }

    /// Hold back the reply to the next request, as if it never arrived.
    pub fn drop_next_reply(&mut self) {
        self.drop_next = true;
    }

    /// Deliver a reply held back by [`Self::drop_next_reply`].
    pub fn deliver_dropped(&mut self) {
        if let Some(r) = self.dropped.take() {
            self.inbox.push_back(r);
        }
    }

    /// Every report written to the fake so far.
    pub fn sent(&self) -> &[Vec<u8>] {
        &self.sent
    }

    fn answer(&mut self, req: &[u8]) -> Vec<u8> {
        let (slot, function_sw) = (req[2], req[3]);
        let p = &req[4..];
        // Replies come back in the request's report type.
        let (report_id, len) = (req[0], req.len());
        let ok = |data: &[u8]| {
            let mut r = vec![report_id, 0xff, slot, function_sw];
            r.extend_from_slice(data);
            r.resize(len, 0);
            r
        };
        let err = |code| {
            let mut r = vec![0x11, 0xff, 0xff, slot, function_sw, code];
            r.resize(20, 0);
            r
        };
        let Some(&feature) = self.features.get(slot as usize) else {
            return err(ERR_INVALID_FEATURE_INDEX);
        };
        match (feature, function_sw >> 4) {
            (0x0000, 0) => {
                let id = u16::from_be_bytes([p[0], p[1]]);
                match self.features.iter().position(|&f| f == id) {
                    Some(i) => ok(&[i as u8, 0, 0]),
                    None => ok(&[0, 0, 0]),
                }
            }
            (0x0000, 1) => ok(&[4, 2, p[2]]),
            (0x0005, 0) => ok(&[self.name.len() as u8]),
            (0x0005, 1) => {
                let start = (p[0] as usize).min(self.name.len());
                let end = (start + 16).min(self.name.len());
                ok(&self.name.as_bytes()[start..end])
            }
            (0x0003, 0) => ok(&[self.firmware.len() as u8]),
            (0x0003, 1) => match self.firmware.get(p[0] as usize) {
                Some(entry) => ok(entry),
                None => err(ERR_INVALID_ARGUMENT),
            },
            (0x8070, 0) => ok(&self.lighting_info.clone()),
            (0x8070, 1) => match self.zones.get(p[0] as usize) {
                Some(z) => {
                    let [hi, lo] = z.location.to_be_bytes();
                    ok(&[p[0], hi, lo, z.effects.len() as u8])
                }
                None => err(ERR_INVALID_ARGUMENT),
            },
            (0x8070, 2) => match self.zones.get(p[0] as usize).and_then(|z| z.effects.get(p[1] as usize)) {
                Some(e) => {
                    let mut d = vec![p[0], p[1]];
                    for v in [e.id, e.capabilities, e.period] {
                        d.extend(v.to_be_bytes());
                    }
                    ok(&d)
                }
                None => err(ERR_INVALID_ARGUMENT),
            },
            (0x8070, 3) => {
                let valid = self.zones.get(p[0] as usize).is_some_and(|z| (p[1] as usize) < z.effects.len());
                match (valid, self.reject_set_effect) {
                    (false, _) => err(ERR_INVALID_ARGUMENT),
                    (true, Some(code)) => err(code),
                    (true, None) => {
                        let mut params = [0u8; 16];
                        params.copy_from_slice(&p[..16]);
                        self.applied.push(params);
                        ok(&p[..2])
                    }
                }
            }
            // "Set keys": header 00 01 00 0e, then 14 x (key, r, g, b).
            (0x8080, 3) => match (req[0], &p[..4], self.reject_set_keys) {
                (0x12, [0x00, 0x01, 0x00, 0x0e], None) => {
                    let entries = p[4..60].chunks_exact(4).filter(|e| e[0] != 0);
                    self.staged_keys.extend(entries.map(|e| [e[0], e[1], e[2], e[3]]));
                    ok(&p[..4])
                }
                (0x12, [0x00, 0x01, 0x00, 0x0e], Some(code)) => err(code),
                _ => err(ERR_INVALID_ARGUMENT),
            },
            (0x8080, 5) => {
                self.frames.push(std::mem::take(&mut self.staged_keys));
                ok(&[])
            }
            _ => err(ERR_INVALID_FUNCTION_ID),
        }
    }
}

fn fw_entry(kind: u8, prefix: [u8; 3], number: u8, revision: u8, build: u16) -> [u8; 16] {
    let mut e = [0u8; 16];
    e[0] = kind;
    e[1..4].copy_from_slice(&prefix);
    e[4] = number;
    e[5] = revision;
    e[6..8].copy_from_slice(&build.to_be_bytes());
    e
}

impl Transport for FakeKeyboard {
    fn send(&mut self, report: &[u8]) -> io::Result<()> {
        match report[0] {
            0x11 => assert_eq!(report.len(), 20, "long reports are 20 bytes"),
            0x12 => assert_eq!(report.len(), 64, "very long reports are 64 bytes"),
            id => panic!("report ID {id:#04x} is not a HID++ request"),
        }
        self.sent.push(report.to_vec());
        let reply = self.answer(report);
        if std::mem::take(&mut self.drop_next) {
            self.dropped = Some(reply);
        } else {
            self.inbox.push_back(reply);
        }
        Ok(())
    }

    fn recv(&mut self, buf: &mut [u8], _timeout: Duration) -> io::Result<Option<usize>> {
        Ok(self.inbox.pop_front().map(|r| {
            buf[..r.len()].copy_from_slice(&r);
            r.len()
        }))
    }
}
