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
        /// `None` on a service without mouse support.
        mouse: Option<Box<Mouse>>,
    },
    Disconnected,
    PermissionDenied,
    Unsupported,
}

/// Read everything in one `GetAll`. An error means the service is not
/// reachable (not installed, not running, or crashed).
pub async fn snapshot(props: &PropertiesProxy<'_>) -> zbus::Result<DeviceState> {
    let all = props.get_all(InterfaceName::from_static_str_unchecked(INTERFACE)).await?;
    let get = |key: &str| all.get(key).cloned().ok_or_else(|| zbus::Error::Failure(format!("missing property {key}")));
    let device_state: String = get("DeviceState")?.try_into()?;
    Ok(match device_state.as_str() {
        "connected" => DeviceState::Connected {
            name: get("DeviceName")?.try_into()?,
            firmware: get("Firmware")?.try_into()?,
            effects: parse_effects(get("Effects")?)?,
            state: parse_state(get("State")?)?,
            per_key: PerKey {
                custom: optional(&all, "CustomKeys")?.unwrap_or_default(),
                gaming: optional(&all, "GamingKeys")?.unwrap_or_default(),
                game_switch: optional(&all, "GameSwitch")?.unwrap_or_default(),
                game_active: optional(&all, "GameActive")?.unwrap_or_default(),
            },
            lights_on: optional(&all, "LightsOn")?,
            mouse: match optional::<String>(&all, "MouseState")? {
                Some(state) => Some(Box::new(Mouse {
                    state,
                    name: get("MouseName")?.try_into()?,
                    effects: parse_effects(get("MouseEffects")?)?,
                    matching: get("MouseMatch")?.try_into()?,
                    lighting: parse_state(get("MouseLighting")?)?,
                })),
                None => None,
            },
        },
        "permission-denied" => DeviceState::PermissionDenied,
        "unsupported" => DeviceState::Unsupported,
        _ => DeviceState::Disconnected,
    })
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
