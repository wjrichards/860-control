//! ColorLedEffects (`0x8070`): zones, the effects each zone offers, and
//! applying an effect.
//!
//! The `setZoneEffect` parameter layouts were confirmed on the user's G512 by
//! watching the keyboard (`evidence/set-zone-effect.md`).

use std::ops::RangeInclusive;

use crate::{Device, Error, Feature, Result, Transport};

const FN_GET_INFO: u8 = 0;
const FN_GET_ZONE_INFO: u8 = 1;
const FN_GET_ZONE_EFFECT_INFO: u8 = 2;
const FN_SET_ZONE_EFFECT: u8 = 3;

/// Written to RAM only; a replug restores the keyboard's own lighting.
const PERSIST_RAM: u8 = 0;
const INTENSITY_FULL: u8 = 100;
/// The only wave direction tested; 0 is rejected.
const WAVE_DIRECTION: u8 = 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb { r, g, b }
    }
}

/// Effects the controller knows how to drive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EffectKind {
    Off,
    Static,
    Breathe,
    Cycle,
    Wave,
    Ripple,
}

impl EffectKind {
    pub const ALL: [EffectKind; 6] = [
        EffectKind::Off,
        EffectKind::Static,
        EffectKind::Breathe,
        EffectKind::Cycle,
        EffectKind::Wave,
        EffectKind::Ripple,
    ];

    /// The effect ID the device reports for this effect.
    pub const fn id(self) -> u16 {
        match self {
            EffectKind::Off => 0x0000,
            EffectKind::Static => 0x0001,
            EffectKind::Cycle => 0x0003,
            EffectKind::Wave => 0x0004,
            EffectKind::Breathe => 0x000a,
            EffectKind::Ripple => 0x000b,
        }
    }

    pub fn from_id(id: u16) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.id() == id)
    }

    /// Whether the effect takes a colour.
    pub const fn accepts_color(self) -> bool {
        matches!(self, EffectKind::Static | EffectKind::Breathe | EffectKind::Ripple)
    }

    /// The speed values the effect accepts, as tested on the keyboard
    /// (firmware U1 19.01). Bigger is slower. `None` for effects without speed.
    pub const fn speed_range(self) -> Option<RangeInclusive<u16>> {
        match self {
            EffectKind::Breathe | EffectKind::Cycle => Some(1000..=8000),
            EffectKind::Wave => Some(10..=250),
            EffectKind::Ripple => Some(10..=1000),
            EffectKind::Off | EffectKind::Static => None,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            EffectKind::Off => "Off",
            EffectKind::Static => "Static",
            EffectKind::Breathe => "Breathe",
            EffectKind::Cycle => "Cycle",
            EffectKind::Wave => "Wave",
            EffectKind::Ripple => "Ripple",
        }
    }
}

/// One effect as reported by the device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZoneEffect {
    /// Position in the zone's effect list; `setZoneEffect` addresses effects by this.
    pub index: u8,
    pub id: u16,
    pub capabilities: u16,
    pub default_period: u16,
    /// `None` for an effect ID this crate does not recognise.
    pub kind: Option<EffectKind>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Zone {
    pub index: u8,
    pub location: u16,
    pub effects: Vec<ZoneEffect>,
}

impl Zone {
    /// Effects that can be offered for selection, in device order.
    pub fn selectable(&self) -> impl Iterator<Item = &ZoneEffect> {
        self.effects.iter().filter(|e| e.kind.is_some())
    }

    pub fn unrecognised(&self) -> impl Iterator<Item = &ZoneEffect> {
        self.effects.iter().filter(|e| e.kind.is_none())
    }

    pub fn find(&self, kind: EffectKind) -> Option<&ZoneEffect> {
        self.effects.iter().find(|e| e.kind == Some(kind))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LightingInfo {
    pub nv_capabilities: u16,
    pub ext_capabilities: u16,
    pub zones: Vec<Zone>,
}

/// Read the zones and effects using ColorLedEffects getters only.
pub fn discover<T: Transport>(dev: &mut Device<T>) -> Result<LightingInfo> {
    let info = dev.request(Feature::ColorLedEffects, FN_GET_INFO, &[])?;
    let zone_count = info[0];
    let mut zones = Vec::with_capacity(zone_count as usize);
    for z in 0..zone_count {
        let zi = dev.request(Feature::ColorLedEffects, FN_GET_ZONE_INFO, &[z])?;
        let mut effects = Vec::with_capacity(zi[3] as usize);
        for e in 0..zi[3] {
            let ei = dev.request(Feature::ColorLedEffects, FN_GET_ZONE_EFFECT_INFO, &[z, e])?;
            let id = u16::from_be_bytes([ei[2], ei[3]]);
            effects.push(ZoneEffect {
                index: e,
                id,
                capabilities: u16::from_be_bytes([ei[4], ei[5]]),
                default_period: u16::from_be_bytes([ei[6], ei[7]]),
                kind: EffectKind::from_id(id),
            });
        }
        zones.push(Zone { index: z, location: u16::from_be_bytes([zi[1], zi[2]]), effects });
    }
    Ok(LightingInfo {
        nv_capabilities: u16::from_be_bytes([info[1], info[2]]),
        ext_capabilities: u16::from_be_bytes([info[3], info[4]]),
        zones,
    })
}

/// The 10-byte `setZoneEffect` parameter block for `kind`.
///
/// `color` is ignored by effects without a colour and `speed` by effects
/// without a speed; a speed outside [`EffectKind::speed_range`] is rejected.
pub fn parameter_block(kind: EffectKind, color: Rgb, speed: u16) -> Result<[u8; 10]> {
    if let Some(range) = kind.speed_range()
        && !range.contains(&speed)
    {
        return Err(Error::OutOfRange { kind, value: speed });
    }
    let Rgb { r, g, b } = color;
    let [s_hi, s_lo] = speed.to_be_bytes();
    let mut block = [0u8; 10];
    let used: &[u8] = match kind {
        EffectKind::Off => &[],
        EffectKind::Static => &[r, g, b, 0],
        EffectKind::Breathe => &[r, g, b, s_hi, s_lo, 0, INTENSITY_FULL],
        EffectKind::Cycle => &[0, 0, 0, 0, 0, s_hi, s_lo, INTENSITY_FULL],
        // Wave takes a one-byte speed; a big-endian period here is rejected.
        EffectKind::Wave => &[0, 0, 0, 0, 0, 0, s_lo, 0, WAVE_DIRECTION, INTENSITY_FULL],
        EffectKind::Ripple => &[r, g, b, 0, s_hi, s_lo],
    };
    block[..used.len()].copy_from_slice(used);
    Ok(block)
}

/// Apply `kind` to `zone` (RAM only). Succeeds only once the device has
/// acknowledged the request.
pub fn set_effect<T: Transport>(
    dev: &mut Device<T>,
    zone: &Zone,
    kind: EffectKind,
    color: Rgb,
    speed: u16,
) -> Result<()> {
    let effect = zone.find(kind).ok_or(Error::EffectUnavailable(kind))?;
    let block = parameter_block(kind, color, speed)?;
    let mut params = [0u8; 13];
    params[0] = zone.index;
    params[1] = effect.index;
    params[2..12].copy_from_slice(&block);
    params[12] = PERSIST_RAM;
    dev.request(Feature::ColorLedEffects, FN_SET_ZONE_EFFECT, &params)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeKeyboard;

    fn hex(s: &str) -> Vec<u8> {
        s.split_whitespace().map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
    }

    /// Apply one effect to the fake and return the 13 parameter bytes sent.
    fn applied_params(kind: EffectKind, color: Rgb, speed: u16) -> Vec<u8> {
        let mut dev = Device::open(FakeKeyboard::g512()).unwrap();
        let zone = discover(&mut dev).unwrap().zones.remove(0);
        set_effect(&mut dev, &zone, kind, color, speed).unwrap();
        dev.transport_mut().applied.last().unwrap()[..13].to_vec()
    }

    // Expected bytes are the `sent` lines (after `11 ff 0d 3a`) of requests
    // the keyboard accepted and the user confirmed visually: spike-out.txt,
    // spike2-out.txt, spike3-out.txt.
    #[test]
    fn bytes_match_confirmed_requests() {
        let cases = [
            (EffectKind::Static, Rgb::new(0xff, 0, 0), 0, "00 01 ff 00 00 00 00 00 00 00 00 00 00"),
            (EffectKind::Breathe, Rgb::new(0xff, 0, 0xff), 4000, "00 02 ff 00 ff 0f a0 00 64 00 00 00 00"),
            (EffectKind::Breathe, Rgb::new(0xff, 0xff, 0xff), 8000, "00 02 ff ff ff 1f 40 00 64 00 00 00 00"),
            (EffectKind::Cycle, Rgb::default(), 8000, "00 03 00 00 00 00 00 1f 40 64 00 00 00"),
            (EffectKind::Wave, Rgb::default(), 30, "00 04 00 00 00 00 00 00 1e 00 01 64 00"),
            (EffectKind::Wave, Rgb::default(), 250, "00 04 00 00 00 00 00 00 fa 00 01 64 00"),
            (EffectKind::Ripple, Rgb::new(0, 0xff, 0xff), 1000, "00 06 00 ff ff 00 03 e8 00 00 00 00 00"),
            (EffectKind::Off, Rgb::default(), 0, "00 00 00 00 00 00 00 00 00 00 00 00 00"),
        ];
        for (kind, color, speed, expected) in cases {
            assert_eq!(applied_params(kind, color, speed), hex(expected), "{kind:?} speed {speed}");
        }
    }

    #[test]
    fn colour_ignored_where_not_accepted() {
        let with = applied_params(EffectKind::Cycle, Rgb::new(1, 2, 3), 1000);
        let without = applied_params(EffectKind::Cycle, Rgb::default(), 1000);
        assert_eq!(with, without);
    }

    #[test]
    fn speed_outside_tested_range_rejected_before_sending() {
        let mut dev = Device::open(FakeKeyboard::g512()).unwrap();
        let zone = discover(&mut dev).unwrap().zones.remove(0);
        let sent = dev.transport_mut().sent().len();
        for (kind, speed) in [(EffectKind::Wave, 251), (EffectKind::Breathe, 999), (EffectKind::Ripple, 1001)] {
            assert!(matches!(
                set_effect(&mut dev, &zone, kind, Rgb::default(), speed),
                Err(Error::OutOfRange { .. })
            ));
        }
        assert_eq!(dev.transport_mut().sent().len(), sent);
    }

    #[test]
    fn device_rejection_is_reported() {
        let mut fake = FakeKeyboard::g512();
        fake.reject_set_effect = Some(0x02);
        let mut dev = Device::open(fake).unwrap();
        let zone = discover(&mut dev).unwrap().zones.remove(0);
        assert!(matches!(
            set_effect(&mut dev, &zone, EffectKind::Static, Rgb::new(0xff, 0, 0), 0),
            Err(Error::Hidpp { code: 0x02, .. })
        ));
        assert!(dev.transport_mut().applied.is_empty());
    }

    #[test]
    fn unavailable_effect_rejected() {
        let mut dev = Device::open(FakeKeyboard::g512()).unwrap();
        let mut zone = discover(&mut dev).unwrap().zones.remove(0);
        zone.effects.retain(|e| e.kind != Some(EffectKind::Ripple));
        assert!(matches!(
            set_effect(&mut dev, &zone, EffectKind::Ripple, Rgb::default(), 30),
            Err(Error::EffectUnavailable(EffectKind::Ripple))
        ));
    }

    #[test]
    fn effects_on_observed_firmware() {
        let mut dev = Device::open(FakeKeyboard::g512()).unwrap();
        let info = discover(&mut dev).unwrap();
        assert_eq!((info.nv_capabilities, info.ext_capabilities), (0x0001, 0x0007));
        assert_eq!(info.zones.len(), 1);
        let zone = &info.zones[0];
        assert_eq!(zone.location, 0x0001);
        let names: Vec<_> = zone.selectable().map(|e| e.kind.unwrap().name()).collect();
        assert_eq!(names, ["Off", "Static", "Breathe", "Cycle", "Wave", "Ripple"]);
        let unknown: Vec<_> = zone.unrecognised().map(|e| (e.index, e.id)).collect();
        assert_eq!(unknown, [(5, 0x0005)]);
        let breathe = zone.find(EffectKind::Breathe).unwrap();
        assert_eq!((breathe.index, breathe.capabilities, breathe.default_period), (2, 0xc001, 992));
        assert_eq!(zone.find(EffectKind::Ripple).unwrap().index, 6);
    }

    #[test]
    fn moved_slot_still_resolves() {
        let mut fake = FakeKeyboard::g512();
        fake.move_feature_to_end(0x8070);
        let mut dev = Device::open(fake).unwrap();
        assert_eq!(dev.slot(Feature::ColorLedEffects), Some(0x13));
        let info = discover(&mut dev).unwrap();
        assert_eq!(info.zones[0].selectable().count(), 6);
    }
}
