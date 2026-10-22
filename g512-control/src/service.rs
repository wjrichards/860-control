//! Client side of `io.github.g512control.Lighting1`.

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeviceState {
    Connected { name: String, firmware: String, effects: Vec<EffectInfo>, state: LightingState },
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
        },
        "permission-denied" => DeviceState::PermissionDenied,
        "unsupported" => DeviceState::Unsupported,
        _ => DeviceState::Disconnected,
    })
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
