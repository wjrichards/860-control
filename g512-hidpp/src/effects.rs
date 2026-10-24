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

    /// Whether the effect has an intensity (brightness) value.
    pub const fn has_intensity(self) -> bool {
        matches!(self, EffectKind::Breathe | EffectKind::Cycle | EffectKind::Wave)
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

/// What an effect is drawn with. Each effect uses only the fields it accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectParams {
    pub color: Rgb,
    /// Device speed value; see [`EffectKind::speed_range`].
    pub speed: u16,
    /// 1–100; dims Breathe, Cycle and Wave (20 and 100 confirmed on the keyboard).
    pub intensity: u8,
}

impl Default for EffectParams {
    fn default() -> Self {
        EffectParams { color: Rgb::default(), speed: 0, intensity: INTENSITY_FULL }
    }
}

/// The 10-byte `setZoneEffect` parameter block for `kind`.
///
/// Fields an effect does not use are ignored; a speed outside
/// [`EffectKind::speed_range`] or an intensity outside 1–100 is rejected.
pub fn parameter_block(kind: EffectKind, p: &EffectParams) -> Result<[u8; 10]> {
    if let Some(range) = kind.speed_range()
        && !range.contains(&p.speed)
    {
        return Err(Error::OutOfRange { kind, what: "speed", value: p.speed });
    }
    if kind.has_intensity() && !(1..=INTENSITY_FULL).contains(&p.intensity) {
        return Err(Error::OutOfRange { kind, what: "intensity", value: p.intensity.into() });
    }
    let Rgb { r, g, b } = p.color;
    let [s_hi, s_lo] = p.speed.to_be_bytes();
    let i = p.intensity;
    let mut block = [0u8; 10];
    let used: &[u8] = match kind {
        EffectKind::Off => &[],
        EffectKind::Static => &[r, g, b, 0],
        EffectKind::Breathe => &[r, g, b, s_hi, s_lo, 0, i],
        EffectKind::Cycle => &[0, 0, 0, 0, 0, s_hi, s_lo, i],
        // Wave takes a one-byte speed; a big-endian period here is rejected.
        EffectKind::Wave => &[0, 0, 0, 0, 0, 0, s_lo, 0, WAVE_DIRECTION, i],
        EffectKind::Ripple => &[r, g, b, 0, s_hi, s_lo],
    };
    block[..used.len()].copy_from_slice(used);
    Ok(block)
}

/// Apply `kind` to `zone` (RAM only). Succeeds only once the device has
/// acknowledged the request.
pub fn set_effect<T: Transport>(dev: &mut Device<T>, zone: &Zone, kind: EffectKind, p: &EffectParams) -> Result<()> {
    let effect = zone.find(kind).ok_or(Error::EffectUnavailable(kind))?;
    let block = parameter_block(kind, p)?;
    let mut params = [0u8; 13];
    params[0] = zone.index;
    params[1] = effect.index;
    params[2..12].copy_from_slice(&block);
    params[12] = PERSIST_RAM;
    dev.request(Feature::ColorLedEffects, FN_SET_ZONE_EFFECT, &params)?;
    Ok(())
}

/// Apply `kind` to every zone in `zones` that offers it, in zone order (RAM
/// only). Fails on the first zone that refuses, or when no zone offers it.
pub fn set_effect_all<T: Transport>(
    dev: &mut Device<T>,
    zones: &[Zone],
    kind: EffectKind,
    p: &EffectParams,
) -> Result<()> {
    let mut offered = zones.iter().filter(|z| z.find(kind).is_some()).peekable();
    if offered.peek().is_none() {
        return Err(Error::EffectUnavailable(kind));
    }
    offered.try_for_each(|zone| set_effect(dev, zone, kind, p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeKeyboard;

    fn hex(s: &str) -> Vec<u8> {
        s.split_whitespace().map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
    }

    fn params(color: Rgb, speed: u16, intensity: u8) -> EffectParams {
        EffectParams { color, speed, intensity }
    }

    /// Apply one effect to the fake and return the 13 parameter bytes sent.
    fn applied_params(kind: EffectKind, p: EffectParams) -> Vec<u8> {
        let mut dev = Device::open(FakeKeyboard::g512()).unwrap();
        let zone = discover(&mut dev).unwrap().zones.remove(0);
        set_effect(&mut dev, &zone, kind, &p).unwrap();
        dev.transport_mut().applied.last().unwrap()[..13].to_vec()
    }

    // Expected bytes are the `sent` lines (after `11 ff 0d 3a`) of requests
    // the keyboard accepted and the user confirmed visually: spike-out.txt,
    // spike2-out.txt, spike3-out.txt, and spike 4 (intensity 20, whose
    // replies were not saved).
    #[test]
    fn bytes_match_confirmed_requests() {
        let none = Rgb::default();
        let cases = [
            (EffectKind::Static, params(Rgb::new(0xff, 0, 0), 0, 100), "00 01 ff 00 00 00 00 00 00 00 00 00 00"),
            (EffectKind::Breathe, params(Rgb::new(0xff, 0, 0xff), 4000, 100), "00 02 ff 00 ff 0f a0 00 64 00 00 00 00"),
            (EffectKind::Breathe, params(Rgb::new(0xff, 0xff, 0xff), 8000, 100), "00 02 ff ff ff 1f 40 00 64 00 00 00 00"),
            (EffectKind::Cycle, params(none, 8000, 100), "00 03 00 00 00 00 00 1f 40 64 00 00 00"),
            (EffectKind::Cycle, params(none, 3000, 20), "00 03 00 00 00 00 00 0b b8 14 00 00 00"),
            (EffectKind::Wave, params(none, 30, 100), "00 04 00 00 00 00 00 00 1e 00 01 64 00"),
            (EffectKind::Wave, params(none, 30, 20), "00 04 00 00 00 00 00 00 1e 00 01 14 00"),
            (EffectKind::Wave, params(none, 250, 100), "00 04 00 00 00 00 00 00 fa 00 01 64 00"),
            (EffectKind::Ripple, params(Rgb::new(0, 0xff, 0xff), 1000, 100), "00 06 00 ff ff 00 03 e8 00 00 00 00 00"),
            (EffectKind::Off, EffectParams::default(), "00 00 00 00 00 00 00 00 00 00 00 00 00"),
        ];
        for (kind, p, expected) in cases {
            assert_eq!(applied_params(kind, p), hex(expected), "{kind:?} {p:?}");
        }
    }

    #[test]
    fn unused_fields_ignored() {
        let with = applied_params(EffectKind::Cycle, params(Rgb::new(1, 2, 3), 1000, 100));
        let without = applied_params(EffectKind::Cycle, params(Rgb::default(), 1000, 100));
        assert_eq!(with, without);
        // Static and Ripple have no intensity, so any value (even 0) is ignored.
        let a = applied_params(EffectKind::Ripple, params(Rgb::new(0, 0xff, 0xff), 30, 0));
        let b = applied_params(EffectKind::Ripple, params(Rgb::new(0, 0xff, 0xff), 30, 100));
        assert_eq!(a, b);
    }

    #[test]
    fn out_of_range_values_rejected_before_sending() {
        let mut dev = Device::open(FakeKeyboard::g512()).unwrap();
        let zone = discover(&mut dev).unwrap().zones.remove(0);
        let sent = dev.transport_mut().sent().len();
        let cases = [
            (EffectKind::Wave, params(Rgb::default(), 251, 100), "speed"),
            (EffectKind::Breathe, params(Rgb::default(), 999, 100), "speed"),
            (EffectKind::Ripple, params(Rgb::default(), 1001, 100), "speed"),
            (EffectKind::Cycle, params(Rgb::default(), 1000, 0), "intensity"),
            (EffectKind::Wave, params(Rgb::default(), 30, 101), "intensity"),
        ];
        for (kind, p, field) in cases {
            match set_effect(&mut dev, &zone, kind, &p) {
                Err(Error::OutOfRange { what, .. }) => assert_eq!(what, field),
                other => panic!("{kind:?} {p:?}: {other:?}"),
            }
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
            set_effect(&mut dev, &zone, EffectKind::Static, &params(Rgb::new(0xff, 0, 0), 0, 100)),
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
            set_effect(&mut dev, &zone, EffectKind::Ripple, &params(Rgb::default(), 30, 100)),
            Err(Error::EffectUnavailable(EffectKind::Ripple))
        ));
    }

    #[test]
    fn mouse_zones_from_spike_6() {
        let mut dev = Device::open(FakeKeyboard::g502_hero()).unwrap();
        let info = discover(&mut dev).unwrap();
        assert_eq!(info.zones.iter().map(|z| (z.index, z.location)).collect::<Vec<_>>(), [(0, 1), (1, 2)]);
        for zone in &info.zones {
            let kinds: Vec<_> = zone.selectable().map(|e| e.kind.unwrap()).collect();
            assert_eq!(kinds, [EffectKind::Off, EffectKind::Static, EffectKind::Breathe, EffectKind::Cycle]);
        }
    }

    #[test]
    fn set_effect_all_sends_each_zone() {
        let mut dev = Device::open(FakeKeyboard::g502_hero()).unwrap();
        let zones = discover(&mut dev).unwrap().zones;
        set_effect_all(&mut dev, &zones, EffectKind::Static, &params(Rgb::new(0, 0xff, 0), 0, 100)).unwrap();
        let applied = &dev.transport_mut().applied;
        assert_eq!(applied.len(), 2);
        // zone, effect index 1 (Static), R G B, persistence 0
        assert_eq!(applied[0][..6], [0, 1, 0x00, 0xff, 0x00, 0]);
        assert_eq!(applied[1][..6], [1, 1, 0x00, 0xff, 0x00, 0]);
        assert_eq!((applied[0][12], applied[1][12]), (0, 0));
    }

    #[test]
    fn set_effect_all_fails_if_any_zone_refuses() {
        let mut fake = FakeKeyboard::g502_hero();
        fake.reject_set_effect = Some(0x02);
        fake.reject_zone = Some(1);
        let mut dev = Device::open(fake).unwrap();
        let zones = discover(&mut dev).unwrap().zones;
        let result = set_effect_all(&mut dev, &zones, EffectKind::Cycle, &params(Rgb::default(), 5000, 100));
        assert!(matches!(result, Err(Error::Hidpp { code: 0x02, .. })));
        assert_eq!(dev.transport_mut().applied.len(), 1, "zone 0 accepted, zone 1 refused");
    }

    #[test]
    fn set_effect_all_needs_a_zone_offering_it() {
        let mut dev = Device::open(FakeKeyboard::g502_hero()).unwrap();
        let zones = discover(&mut dev).unwrap().zones;
        assert!(matches!(
            set_effect_all(&mut dev, &zones, EffectKind::Ripple, &params(Rgb::default(), 30, 100)),
            Err(Error::EffectUnavailable(EffectKind::Ripple))
        ));
        assert!(dev.transport_mut().applied.is_empty());
    }

    #[test]
    fn set_effect_all_on_the_keyboard_matches_set_effect() {
        let p = params(Rgb::new(0x12, 0x34, 0x56), 4000, 80);
        let mut one = Device::open(FakeKeyboard::g512()).unwrap();
        let zone = discover(&mut one).unwrap().zones.remove(0);
        set_effect(&mut one, &zone, EffectKind::Breathe, &p).unwrap();
        let mut all = Device::open(FakeKeyboard::g512()).unwrap();
        let zones = discover(&mut all).unwrap().zones;
        set_effect_all(&mut all, &zones, EffectKind::Breathe, &p).unwrap();
        assert_eq!(one.transport_mut().applied, all.transport_mut().applied);
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
