//! The HID++ 2.0 features this crate is allowed to talk to.
//!
//! Requests are built from a [`Feature`], so a feature outside this list
//! cannot be addressed at all, and [`Feature::allows`] limits some features
//! to listed functions. The G512 and G502 HERO also expose firmware update
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
    /// Read only: the sensor's DPI range and current DPI.
    AdjustableDpi,
    /// Onboard profiles: reading and writing user profile sectors, and the
    /// active profile. Never the onboard mode.
    OnboardProfiles,
}

impl Feature {
    /// Every allowlisted feature.
    pub const ALL: [Feature; 8] = [
        Feature::Root,
        Feature::FeatureSet,
        Feature::DeviceFwVersion,
        Feature::DeviceName,
        Feature::ColorLedEffects,
        Feature::PerKeyLighting,
        Feature::AdjustableDpi,
        Feature::OnboardProfiles,
    ];

    /// Whether `function` of this feature may be called. Requests for any
    /// other function are refused before anything is sent.
    pub fn allows(self, function: u8) -> bool {
        match self {
            // set keys, commit
            Feature::PerKeyLighting => matches!(function, 3 | 5),
            // getSensorCount, getSensorDpiList, getSensorDpi
            Feature::AdjustableDpi => matches!(function, 0..=2),
            // getInfo, getMode, set/getCurrentProfile, memoryRead,
            // memoryAddrWrite, memoryWrite, memoryWriteEnd, getCurrentDpiIndex;
            // never 1 (setOnboardMode) or 12 (setCurrentDpiIndex)
            Feature::OnboardProfiles => matches!(function, 0 | 2..=8 | 11),
            _ => true,
        }
    }

    /// The 16-bit HID++ feature ID.
    pub const fn id(self) -> u16 {
        match self {
            Feature::Root => 0x0000,
            Feature::FeatureSet => 0x0001,
            Feature::DeviceFwVersion => 0x0003,
            Feature::DeviceName => 0x0005,
            Feature::ColorLedEffects => 0x8070,
            Feature::PerKeyLighting => 0x8080,
            Feature::AdjustableDpi => 0x2201,
            Feature::OnboardProfiles => 0x8100,
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
        assert_eq!(ids, [0x0000, 0x0001, 0x0003, 0x0005, 0x2201, 0x8070, 0x8080, 0x8100]);
    }

    #[test]
    fn function_lists_match_spec() {
        let allowed = |f: Feature| (0..16).filter(|&n| f.allows(n)).collect::<Vec<u8>>();
        assert_eq!(allowed(Feature::PerKeyLighting), [3, 5]);
        assert_eq!(allowed(Feature::AdjustableDpi), [0, 1, 2]);
        assert_eq!(allowed(Feature::OnboardProfiles), [0, 2, 3, 4, 5, 6, 7, 8, 11]);
        assert!(!Feature::OnboardProfiles.allows(1), "setOnboardMode is never reachable");
        assert_eq!(allowed(Feature::ColorLedEffects).len(), 16);
    }

    #[test]
    fn dangerous_features_absent() {
        // Including the mouse's (spike 6): 0x1300, 0x18a1, 0x1e22, 0x1806, 0x18b1.
        let mouse = [0x1300, 0x18a1, 0x1e22, 0x1806, 0x18b1];
        for id in [0x00c2, 0x1802, 0x1e00, 0x1eb0, 0x1801, 0x1821, 0x1bc0, 0x40a2].into_iter().chain(mouse) {
            assert!(Feature::ALL.iter().all(|f| f.id() != id), "{id:#06x} must not be allowlisted");
        }
    }
}
