//! ColorLedEffects (`0x8070`): zones and the effects each zone offers.

use crate::{Device, Feature, Result, Transport};

const FN_GET_INFO: u8 = 0;
const FN_GET_ZONE_INFO: u8 = 1;
const FN_GET_ZONE_EFFECT_INFO: u8 = 2;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeKeyboard;

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
