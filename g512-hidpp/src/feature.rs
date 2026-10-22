//! The HID++ 2.0 features this crate is allowed to talk to.
//!
//! Requests are built from a [`Feature`], so a feature outside this list
//! cannot be addressed at all. The G512 also exposes firmware update
//! (`0x00c2`), device reset (`0x1802`) and hidden engineering features; none
//! of them are representable here.

use std::fmt;

/// An allowlisted HID++ 2.0 feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Feature {
    Root,
    FeatureSet,
    DeviceFwVersion,
    DeviceName,
    ColorLedEffects,
    PerKeyLighting,
}

impl Feature {
    /// Every allowlisted feature.
    pub const ALL: [Feature; 6] = [
        Feature::Root,
        Feature::FeatureSet,
        Feature::DeviceFwVersion,
        Feature::DeviceName,
        Feature::ColorLedEffects,
        Feature::PerKeyLighting,
    ];

    /// The 16-bit HID++ feature ID.
    pub const fn id(self) -> u16 {
        match self {
            Feature::Root => 0x0000,
            Feature::FeatureSet => 0x0001,
            Feature::DeviceFwVersion => 0x0003,
            Feature::DeviceName => 0x0005,
            Feature::ColorLedEffects => 0x8070,
            Feature::PerKeyLighting => 0x8080,
        }
    }
}

impl fmt::Display for Feature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?} ({:#06x})", self.id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_matches_spec() {
        let mut ids: Vec<u16> = Feature::ALL.iter().map(|f| f.id()).collect();
        ids.sort_unstable();
        assert_eq!(ids, [0x0000, 0x0001, 0x0003, 0x0005, 0x8070, 0x8080]);
    }

    #[test]
    fn dangerous_features_absent() {
        for id in [0x00c2, 0x1802, 0x1e00, 0x1eb0, 0x1801, 0x1821, 0x1bc0, 0x40a2] {
            assert!(Feature::ALL.iter().all(|f| f.id() != id), "{id:#06x} must not be allowlisted");
        }
    }
}
