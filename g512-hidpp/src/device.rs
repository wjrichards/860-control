//! HID++ 2.0 requests and replies over a [`Transport`].

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::{Error, Feature, Result, Transport};

/// Device index used by wired HID++ devices.
pub const DEVICE_INDEX: u8 = 0xff;
pub const REPLY_TIMEOUT: Duration = Duration::from_secs(1);

const REPORT_LONG: u8 = 0x11;
const REPORT_VERY_LONG: u8 = 0x12;
const LONG_LEN: usize = 20;
const VERY_LONG_LEN: usize = 64;
const MAX_PARAMS: usize = LONG_LEN - 4;
/// Parameter bytes a very long (`0x12`) request carries.
pub const VERY_LONG_PARAMS: usize = VERY_LONG_LEN - 4;
/// Marks a HID++ 2.0 error reply in the feature-index position.
const ERROR_MARKER: u8 = 0xff;

/// Root is at slot 0 by definition of the protocol; everything else is
/// resolved from the device.
const ROOT_SLOT: u8 = 0;
const ROOT_GET_FEATURE: u8 = 0;

/// An opened G512 with its feature slots resolved.
pub struct Device<T> {
    transport: T,
    slots: HashMap<Feature, u8>,
    software_id: u8,
}

impl<T: Transport> Device<T> {
    /// Resolve the slot of every allowlisted feature the device implements.
    pub fn open(transport: T) -> Result<Self> {
        let mut device = Device { transport, slots: HashMap::new(), software_id: 0 };
        device.slots.insert(Feature::Root, ROOT_SLOT);
        for feature in Feature::ALL.into_iter().filter(|&f| f != Feature::Root) {
            let id = feature.id().to_be_bytes();
            let reply = device.request(Feature::Root, ROOT_GET_FEATURE, &id)?;
            if reply[0] != 0 {
                device.slots.insert(feature, reply[0]);
            }
        }
        Ok(device)
    }

    pub fn supports(&self, feature: Feature) -> bool {
        self.slots.contains_key(&feature)
    }

    /// The slot the device reported for `feature`, if it has one.
    pub fn slot(&self, feature: Feature) -> Option<u8> {
        self.slots.get(&feature).copied()
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// Call `function` of `feature` and return the 16 reply parameter bytes.
    pub fn request(&mut self, feature: Feature, function: u8, params: &[u8]) -> Result<[u8; 16]> {
        assert!(params.len() <= MAX_PARAMS, "long report carries at most {MAX_PARAMS} parameter bytes");
        self.send_request::<LONG_LEN>(REPORT_LONG, feature, function, params)
    }

    /// Like [`Self::request`], sent as a very long (`0x12`, 64-byte) report
    /// for up to [`VERY_LONG_PARAMS`] parameter bytes.
    pub fn request_very_long(&mut self, feature: Feature, function: u8, params: &[u8]) -> Result<[u8; 16]> {
        assert!(params.len() <= VERY_LONG_PARAMS, "very long report carries at most {VERY_LONG_PARAMS} bytes");
        self.send_request::<VERY_LONG_LEN>(REPORT_VERY_LONG, feature, function, params)
    }

    fn send_request<const N: usize>(
        &mut self,
        report_id: u8,
        feature: Feature,
        function: u8,
        params: &[u8],
    ) -> Result<[u8; 16]> {
        assert!(function < 16, "HID++ function numbers are 4 bits");
        let slot = self.slot(feature).ok_or(Error::FeatureMissing(feature))?;
        debug_assert!(Feature::ALL.contains(&feature));

        // Rotate 1..=15 so a late reply to an earlier request is not taken
        // as the answer to this one.
        self.software_id = self.software_id % 15 + 1;
        let function_sw = function << 4 | self.software_id;

        let mut report = [0u8; N];
        report[..4].copy_from_slice(&[report_id, DEVICE_INDEX, slot, function_sw]);
        report[4..4 + params.len()].copy_from_slice(params);
        self.transport.send(&report)?;

        let deadline = Instant::now() + REPLY_TIMEOUT;
        let mut buf = [0u8; 64];
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let Some(n) = self.transport.recv(&mut buf, remaining)? else {
                return Err(Error::Timeout { feature, function });
            };
            let rep = &buf[..n];
            if n < LONG_LEN || !matches!(rep[0], REPORT_LONG | REPORT_VERY_LONG) || rep[1] != DEVICE_INDEX {
                continue; // key reports, notifications from other interfaces' IDs, etc.
            }
            if rep[2] == ERROR_MARKER && rep[3] == slot && rep[4] == function_sw {
                return Err(Error::Hidpp { feature, function, code: rep[5] });
            }
            if rep[2] == slot && rep[3] == function_sw {
                let mut out = [0u8; 16];
                out.copy_from_slice(&rep[4..LONG_LEN]);
                return Ok(out);
            }
            if Instant::now() >= deadline {
                return Err(Error::Timeout { feature, function });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeKeyboard;

    #[test]
    fn resolves_slots_from_device() {
        let dev = Device::open(FakeKeyboard::g512()).unwrap();
        assert_eq!(dev.slot(Feature::ColorLedEffects), Some(0x0d));
        assert_eq!(dev.slot(Feature::PerKeyLighting), Some(0x0c));
        assert_eq!(dev.slot(Feature::DeviceName), Some(0x04));
    }

    #[test]
    fn normal_reply() {
        let mut dev = Device::open(FakeKeyboard::g512()).unwrap();
        let info = dev.request(Feature::ColorLedEffects, 0, &[]).unwrap();
        assert_eq!(info[..5], [0x01, 0x00, 0x01, 0x00, 0x07]);
    }

    #[test]
    fn error_reply_carries_code() {
        let mut dev = Device::open(FakeKeyboard::g512()).unwrap();
        // Function 9 is not implemented by the fake, which answers 0x07 (invalid function).
        match dev.request(Feature::ColorLedEffects, 9, &[]) {
            Err(Error::Hidpp { feature: Feature::ColorLedEffects, function: 9, code: 0x07 }) => {}
            other => panic!("expected HID++ error, got {other:?}"),
        }
    }

    #[test]
    fn skips_interleaved_key_report() {
        let mut dev = Device::open(FakeKeyboard::g512()).unwrap();
        let mut key_report = vec![0x01];
        key_report.extend([0x04; 20]); // NKRO report ID 1, 'a' held
        dev.transport_mut().queue_noise(key_report);
        // A reply to some other slot is skipped as well.
        let mut other = vec![REPORT_LONG, DEVICE_INDEX, 0x0c, 0x01];
        other.resize(LONG_LEN, 0);
        dev.transport_mut().queue_noise(other);
        let info = dev.request(Feature::ColorLedEffects, 0, &[]).unwrap();
        assert_eq!(info[0], 1);
    }

    #[test]
    fn very_long_request_is_64_bytes() {
        let mut dev = Device::open(FakeKeyboard::g512()).unwrap();
        let mut params = vec![0x00, 0x01, 0x00, 0x0e];
        params.resize(VERY_LONG_PARAMS, 0);
        let reply = dev.request_very_long(Feature::PerKeyLighting, 3, &params).unwrap();
        assert_eq!(reply[..4], [0x00, 0x01, 0x00, 0x0e]);
        let sent = dev.transport_mut().sent().last().unwrap().clone();
        assert_eq!(sent.len(), 64);
        assert_eq!(sent[..3], [REPORT_VERY_LONG, DEVICE_INDEX, 0x0c]);
        assert_eq!(sent[3] >> 4, 3);
    }

    #[test]
    fn timeout_leaves_device_usable() {
        let mut dev = Device::open(FakeKeyboard::g512()).unwrap();
        dev.transport_mut().drop_next_reply();
        assert!(matches!(
            dev.request(Feature::ColorLedEffects, 0, &[]),
            Err(Error::Timeout { feature: Feature::ColorLedEffects, function: 0 })
        ));
        // The late reply turns up now; it has a stale software ID and is ignored.
        dev.transport_mut().deliver_dropped();
        let zone = dev.request(Feature::ColorLedEffects, 1, &[0]).unwrap();
        assert_eq!(zone[..4], [0x00, 0x00, 0x01, 0x07]);
    }

    #[test]
    fn missing_feature_sends_nothing() {
        let mut fake = FakeKeyboard::g512();
        fake.remove_feature(Feature::ColorLedEffects.id());
        let mut dev = Device::open(fake).unwrap();
        let sent_before = dev.transport_mut().sent().len();
        assert!(matches!(
            dev.request(Feature::ColorLedEffects, 0, &[]),
            Err(Error::FeatureMissing(Feature::ColorLedEffects))
        ));
        assert_eq!(dev.transport_mut().sent().len(), sent_before);
    }

    #[test]
    fn only_allowlisted_features_are_resolved_or_called() {
        let mut dev = Device::open(FakeKeyboard::g512()).unwrap();
        dev.request(Feature::ColorLedEffects, 0, &[]).unwrap();
        let allowed: Vec<u8> = Feature::ALL.iter().filter_map(|&f| dev.slot(f)).collect();
        for report in dev.transport_mut().sent() {
            assert!(allowed.contains(&report[2]), "request to non-allowlisted slot {:#04x}", report[2]);
        }
        // getFeature was only ever asked about allowlisted IDs.
        for report in dev.transport_mut().sent().iter().filter(|r| r[2] == ROOT_SLOT) {
            let id = u16::from_be_bytes([report[4], report[5]]);
            assert!(Feature::ALL.iter().any(|f| f.id() == id), "getFeature({id:#06x})");
        }
    }
}
