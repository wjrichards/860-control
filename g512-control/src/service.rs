//! Client side of `io.github.g512control.Lighting1`.

use std::collections::HashMap;

use zbus::{
    fdo::PropertiesProxy,
    names::InterfaceName,
    zvariant::OwnedValue,
};

pub const BUS_NAME: &str = "io.github.g512control.Lighting1";
pub const OBJECT_PATH: &str = "/io/github/g512control/Lighting1";
pub const INTERFACE: &str = "io.github.g512control.Lighting1";

#[zbus::proxy(
    interface = "io.github.g512control.Lighting1",
    default_service = "io.github.g512control.Lighting1",
    default_path = "/io/github/g512control/Lighting1"
)]
pub trait Lighting {
    fn set_state(&self, effect: &str, color: (u8, u8, u8), speed: u8, brightness: u8) -> zbus::Result<bool>;
    fn set_keys(&self, map: &str, keys: &[(u8, u8, u8, u8)]) -> zbus::Result<()>;
    fn set_game_switch(&self, on: bool) -> zbus::Result<()>;
    fn set_lights_on(&self, on: bool) -> zbus::Result<()>;
    fn set_mouse_match(&self, on: bool) -> zbus::Result<()>;
    fn set_mouse_lighting(&self, effect: &str, color: (u8, u8, u8), speed: u8, brightness: u8) -> zbus::Result<bool>;
    fn set_mouse_active_profile(&self, profile: u8) -> zbus::Result<()>;
    fn set_mouse_profile(&self, profile: u8, report_rate_ms: u8, default_index: u8, dpis: &[u16]) -> zbus::Result<()>;
    fn set_mouse_button(&self, profile: u8, button: u8, kind: &str, value: u16, modifiers: u8) -> zbus::Result<()>;
}

/// The Custom effect's name on the bus.
pub const CUSTOM: &str = "Custom";

/// A key map: (key usage, r, g, b); keys not listed are off.
pub type Keys = Vec<(u8, u8, u8, u8)>;

/// Key maps and the game switch. Empty and off when talking to a service
/// without per-key support.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PerKey {
    pub custom: Keys,
    pub gaming: Keys,
    pub game_switch: bool,
    pub game_active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectInfo {
    pub name: String,
    pub accepts_color: bool,
    pub accepts_speed: bool,
    pub min_speed: u8,
    pub max_speed: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LightingState {
    pub effect: String,
    pub color: (u8, u8, u8),
    pub speed: u8,
    pub brightness: u8,
}

/// The G502 HERO as the service reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mouse {
    /// `connected`, `disconnected`, `permission-denied` or `unsupported`.
    pub state: String,
    pub name: String,
    pub effects: Vec<EffectInfo>,
    pub matching: bool,
    /// Its own setting, shown while not matching.
    pub lighting: LightingState,
    /// Onboard profiles; empty on a service without them, or when they
    /// can't be read.
    pub profiles: Vec<MouseProfile>,
    /// 1-based; 0 when unknown.
    pub active_profile: u8,
    /// (min, max, step); zeros when unknown.
    pub dpi_range: (u16, u16, u16),
    /// Each profile's buttons, by profile number; empty on a service
    /// without them.
    pub buttons: Vec<(u8, Vec<ButtonAction>)>,
}

/// What one mouse button does, as the service reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ButtonAction {
    /// `mouse`, `key`, `media`, `special`, `disabled` or `other`.
    pub kind: String,
    pub value: u16,
    pub modifiers: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MouseProfile {
    pub number: u8,
    pub report_rate_ms: u8,
    pub default_index: u8,
    pub dpis: Vec<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeviceState {
    Connected {
        name: String,
        firmware: String,
        effects: Vec<EffectInfo>,
        state: LightingState,
        per_key: PerKey,
        /// `None` on a service without the lights setting.
        lights_on: Option<bool>,
    },
    Disconnected,
    PermissionDenied,
    Unsupported,
}

/// Both devices as the service reports them. The mouse is read whatever the
/// keyboard's state: g512d drives each device on its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub keyboard: DeviceState,
    /// `None` on a service without mouse support.
    pub mouse: Option<Box<Mouse>>,
}

/// Read everything in one `GetAll`. An error means the service is not
/// reachable (not installed, not running, or crashed).
pub async fn snapshot(props: &PropertiesProxy<'_>) -> zbus::Result<Snapshot> {
    let all = props.get_all(InterfaceName::from_static_str_unchecked(INTERFACE)).await?;
    parse_snapshot(&all)
}

fn parse_snapshot(all: &HashMap<String, OwnedValue>) -> zbus::Result<Snapshot> {
    Ok(Snapshot { keyboard: parse_keyboard(all)?, mouse: parse_mouse(all)? })
}

fn get(all: &HashMap<String, OwnedValue>, key: &str) -> zbus::Result<OwnedValue> {
    all.get(key).map(OwnedValue::try_clone).transpose()?.ok_or_else(|| zbus::Error::Failure(format!("missing property {key}")))
}

fn parse_keyboard(all: &HashMap<String, OwnedValue>) -> zbus::Result<DeviceState> {
    let get = |key: &str| get(all, key);
    let device_state: String = get("DeviceState")?.try_into()?;
    Ok(match device_state.as_str() {
        "connected" => DeviceState::Connected {
            name: get("DeviceName")?.try_into()?,
            firmware: get("Firmware")?.try_into()?,
            effects: parse_effects(get("Effects")?)?,
            state: parse_state(get("State")?)?,
            per_key: PerKey {
                custom: optional(all, "CustomKeys")?.unwrap_or_default(),
                gaming: optional(all, "GamingKeys")?.unwrap_or_default(),
                game_switch: optional(all, "GameSwitch")?.unwrap_or_default(),
                game_active: optional(all, "GameActive")?.unwrap_or_default(),
            },
            lights_on: optional(all, "LightsOn")?,
        },
        "permission-denied" => DeviceState::PermissionDenied,
        "unsupported" => DeviceState::Unsupported,
        _ => DeviceState::Disconnected,
    })
}

fn parse_mouse(all: &HashMap<String, OwnedValue>) -> zbus::Result<Option<Box<Mouse>>> {
    let get = |key: &str| get(all, key);
    let Some(state) = optional::<String>(all, "MouseState")? else { return Ok(None) };
    Ok(Some(Box::new(Mouse {
        state,
        name: get("MouseName")?.try_into()?,
        effects: parse_effects(get("MouseEffects")?)?,
        matching: get("MouseMatch")?.try_into()?,
        lighting: parse_state(get("MouseLighting")?)?,
        profiles: optional::<Vec<(u8, u8, u8, Vec<u16>)>>(all, "MouseProfiles")?
            .unwrap_or_default()
            .into_iter()
            .map(|(number, report_rate_ms, default_index, dpis)| MouseProfile {
                number,
                report_rate_ms,
                default_index,
                dpis,
            })
            .collect(),
        active_profile: optional(all, "MouseActiveProfile")?.unwrap_or(0),
        dpi_range: optional(all, "MouseDpiRange")?.unwrap_or((0, 0, 0)),
        buttons: optional::<Vec<(u8, Vec<(String, u16, u8)>)>>(all, "MouseButtons")?
            .unwrap_or_default()
            .into_iter()
            .map(|(number, entries)| {
                let actions = entries
                    .into_iter()
                    .map(|(kind, value, modifiers)| ButtonAction { kind, value, modifiers })
                    .collect();
                (number, actions)
            })
            .collect(),
    })))
}

/// A property added after v1: absent on older services.
fn optional<T>(all: &HashMap<String, OwnedValue>, key: &str) -> zbus::Result<Option<T>>
where
    T: TryFrom<OwnedValue>,
    T::Error: Into<zbus::Error>,
{
    all.get(key).map(|v| T::try_from(v.try_clone()?).map_err(Into::into)).transpose()
}

fn parse_effects(v: OwnedValue) -> zbus::Result<Vec<EffectInfo>> {
    let raw: Vec<(String, bool, bool, u8, u8)> = v.try_into()?;
    Ok(raw
        .into_iter()
        .map(|(name, accepts_color, accepts_speed, min_speed, max_speed)| EffectInfo {
            name,
            accepts_color,
            accepts_speed,
            min_speed,
            max_speed,
        })
        .collect())
}

fn parse_state(v: OwnedValue) -> zbus::Result<LightingState> {
    let (effect, color, speed, brightness): (String, (u8, u8, u8), u8, u8) = v.try_into()?;
    Ok(LightingState { effect, color, speed, brightness })
}

/// The human part of a D-Bus error, e.g. "brightness 150 is outside 0..=100".
pub fn error_message(e: &zbus::Error) -> String {
    match e {
        zbus::Error::MethodError(_, Some(msg), _) => msg.clone(),
        zbus::Error::FDO(fdo) => match fdo.as_ref() {
            zbus::fdo::Error::InvalidArgs(msg) | zbus::fdo::Error::Failed(msg) => msg.clone(),
            other => other.to_string(),
        },
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use zbus::zvariant::Value;

    use super::*;

    fn props(entries: Vec<(&str, Value<'static>)>) -> HashMap<String, OwnedValue> {
        entries.into_iter().map(|(k, v)| (k.to_string(), OwnedValue::try_from(v).unwrap())).collect()
    }

    fn mouse_props() -> Vec<(&'static str, Value<'static>)> {
        vec![
            ("MouseState", Value::from("connected")),
            ("MouseName", Value::from("G502 HERO Gaming Mouse")),
            ("MouseEffects", Value::from(vec![("Static".to_string(), true, false, 0u8, 0u8)])),
            ("MouseMatch", Value::from(true)),
            ("MouseLighting", Value::from(("Static".to_string(), (38u8, 162u8, 105u8), 50u8, 100u8))),
        ]
    }

    #[test]
    fn mouse_is_read_while_the_keyboard_is_missing() {
        let mut entries = vec![("DeviceState", Value::from("disconnected"))];
        entries.extend(mouse_props());
        let snap = parse_snapshot(&props(entries)).unwrap();
        assert_eq!(snap.keyboard, DeviceState::Disconnected);
        let mouse = snap.mouse.expect("the mouse is reported without the keyboard");
        assert_eq!((mouse.state.as_str(), mouse.name.as_str()), ("connected", "G502 HERO Gaming Mouse"));
        assert_eq!(mouse.lighting.color, (38, 162, 105));
        assert!(mouse.profiles.is_empty() && mouse.buttons.is_empty(), "optional members default to empty");
    }

    #[test]
    fn older_service_has_no_mouse() {
        let snap = parse_snapshot(&props(vec![("DeviceState", Value::from("permission-denied"))])).unwrap();
        assert_eq!(snap.keyboard, DeviceState::PermissionDenied);
        assert_eq!(snap.mouse, None);
    }

    #[test]
    fn missing_mouse_member_is_an_error() {
        let mut entries = vec![("DeviceState", Value::from("unsupported"))];
        entries.extend(mouse_props().into_iter().filter(|(k, _)| *k != "MouseName"));
        assert!(parse_snapshot(&props(entries)).is_err());
    }
}
