//! The session D-Bus interface `io.github.g512control.Lighting1`.

use std::sync::{Arc, Mutex, MutexGuard};

use g512_hidpp::effects::{EffectKind, Rgb};
use zbus::{fdo, interface, object_server::SignalEmitter};

use crate::{
    buttons,
    controller::{Controller, HidrawOpener, SetError},
    keymap::{KeyMap, MapName},
    macros,
    state::{Effect, InvalidState, LightingState, SPEED_MAX, SPEED_MIN, effect_from_name},
};

pub const BUS_NAME: &str = "io.github.g512control.Lighting1";
pub const OBJECT_PATH: &str = "/io/github/g512control/Lighting1";

pub type Shared = Arc<Mutex<Controller<HidrawOpener>>>;

/// Shown before anything has been saved; not applied to the keyboard.
const DEFAULT_STATE: LightingState = LightingState {
    effect: Effect::Zone(EffectKind::Static),
    color: Rgb::new(0xff, 0xff, 0xff),
    speed: 50,
    brightness: 100,
};

/// A key map as sent on the bus: (key usage, r, g, b), sorted by key.
fn map_entries(map: &KeyMap) -> Vec<(u8, u8, u8, u8)> {
    map.entries().map(|(k, c)| (k.usage(), c.r, c.g, c.b)).collect()
}

/// Effects as sent on the bus: (name, accepts colour, accepts speed, min
/// speed, max speed).
fn effect_list(effects: Vec<Effect>) -> Vec<(String, bool, bool, u8, u8)> {
    effects
        .into_iter()
        .map(|e| {
            let speed = e.has_speed();
            let (min, max) = if speed { (SPEED_MIN, SPEED_MAX) } else { (0, 0) };
            (e.name().to_string(), e.accepts_color(), speed, min, max)
        })
        .collect()
}

fn set_error(e: SetError) -> fdo::Error {
    match e {
        SetError::Invalid(e) => fdo::Error::InvalidArgs(e.to_string()),
        SetError::Device(e) => fdo::Error::Failed(format!("the device refused the change: {e}")),
        SetError::Profile(msg) => fdo::Error::InvalidArgs(msg),
        SetError::Backup(e) => fdo::Error::Failed(format!("could not save the profile backup, so nothing was written: {e}")),
        SetError::MacroTooLong(msg) => fdo::Error::InvalidArgs(format!("the macro is too long: {msg}")),
        SetError::MacroMemoryFull => fdo::Error::Failed("every macro sector on the mouse is in use".into()),
        SetError::WriteFailed(msg) => fdo::Error::Failed(format!("the mouse didn't take the write: {msg}")),
    }
}

/// Errors from the macro calls. The three macro problems have their own
/// names (`io.github.g512control.Error.MacroTooLong`, `.MacroMemoryFull`,
/// `.WriteFailed`), so a front end can tell them apart without reading the
/// text; everything else is the usual `org.freedesktop.DBus.Error` one.
#[derive(Debug)]
pub enum MacroError {
    Named(&'static str, String),
    Fdo(fdo::Error),
}

impl From<SetError> for MacroError {
    fn from(e: SetError) -> Self {
        match e {
            SetError::MacroTooLong(msg) => MacroError::Named("io.github.g512control.Error.MacroTooLong", msg),
            SetError::MacroMemoryFull => MacroError::Named(
                "io.github.g512control.Error.MacroMemoryFull",
                "every macro sector on the mouse is in use".into(),
            ),
            SetError::WriteFailed(msg) => MacroError::Named("io.github.g512control.Error.WriteFailed", msg),
            e => MacroError::Fdo(set_error(e)),
        }
    }
}

impl From<zbus::Error> for MacroError {
    fn from(e: zbus::Error) -> Self {
        MacroError::Fdo(e.into())
    }
}

impl zbus::DBusError for MacroError {
    fn create_reply(&self, call: &zbus::message::Header<'_>) -> zbus::Result<zbus::Message> {
        match self {
            MacroError::Fdo(e) => e.create_reply(call),
            MacroError::Named(name, msg) => zbus::Message::error(call, *name)?.build(&(msg,)),
        }
    }

    fn name(&self) -> zbus::names::ErrorName<'_> {
        match self {
            MacroError::Fdo(e) => e.name(),
            MacroError::Named(name, _) => zbus::names::ErrorName::from_static_str_unchecked(name),
        }
    }

    fn description(&self) -> Option<&str> {
        match self {
            MacroError::Fdo(e) => e.description(),
            MacroError::Named(_, msg) => Some(msg),
        }
    }
}

pub struct Lighting {
    pub ctl: Shared,
}

impl Lighting {
    fn ctl(&self) -> MutexGuard<'_, Controller<HidrawOpener>> {
        self.ctl.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Emit PropertiesChanged for everything a state change can touch.
    pub async fn emit_all(&self, emitter: &SignalEmitter<'_>) -> zbus::Result<()> {
        self.device_state_changed(emitter).await?;
        self.device_name_changed(emitter).await?;
        self.firmware_changed(emitter).await?;
        self.effects_changed(emitter).await?;
        self.state_changed(emitter).await?;
        self.pending_changed(emitter).await?;
        self.custom_keys_changed(emitter).await?;
        self.gaming_keys_changed(emitter).await?;
        self.game_switch_changed(emitter).await?;
        self.game_active_changed(emitter).await?;
        self.lights_on_changed(emitter).await?;
        self.mouse_state_changed(emitter).await?;
        self.mouse_name_changed(emitter).await?;
        self.mouse_effects_changed(emitter).await?;
        self.mouse_match_changed(emitter).await?;
        self.mouse_lighting_changed(emitter).await?;
        self.mouse_profiles_changed(emitter).await?;
        self.mouse_active_profile_changed(emitter).await?;
        self.mouse_dpi_range_changed(emitter).await?;
        self.mouse_buttons_changed(emitter).await?;
        self.mouse_macros_changed(emitter).await?;
        self.mouse_macro_sectors_changed(emitter).await
    }
}

#[interface(name = "io.github.g512control.Lighting1")]
impl Lighting {
    /// `connected`, `disconnected`, `permission-denied` or `unsupported`.
    #[zbus(property)]
    async fn device_state(&self) -> String {
        self.ctl().status().as_str().to_string()
    }

    /// Empty when not connected.
    #[zbus(property)]
    async fn device_name(&self) -> String {
        self.ctl().info().map(|i| i.name.clone()).unwrap_or_default()
    }

    /// Application firmware, e.g. `U1 19.01 build 0014`; empty when not connected.
    #[zbus(property)]
    async fn firmware(&self) -> String {
        let ctl = self.ctl();
        ctl.info().and_then(|i| i.application_firmware()).map(|f| f.to_string()).unwrap_or_default()
    }

    /// Selectable effects: (name, accepts colour, accepts speed, min speed, max speed).
    /// Speed 100 is the fastest. Empty when not connected.
    #[zbus(property)]
    async fn effects(&self) -> Vec<(String, bool, bool, u8, u8)> {
        effect_list(self.ctl().offered())
    }

    /// The saved lighting state: (effect, (r, g, b), speed, brightness).
    #[zbus(property)]
    async fn state(&self) -> (String, (u8, u8, u8), u8, u8) {
        let s = self.ctl().saved().map_or(DEFAULT_STATE, |s| s.state);
        (s.effect.name().to_string(), (s.color.r, s.color.g, s.color.b), s.speed, s.brightness)
    }

    /// True when the state is saved but not yet applied (keyboard not connected).
    #[zbus(property)]
    async fn pending(&self) -> bool {
        self.ctl().saved().is_some_and(|s| s.pending)
    }

    /// Set the lighting. Returns whether the state is pending.
    async fn set_state(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        effect: String,
        color: (u8, u8, u8),
        speed: u8,
        brightness: u8,
    ) -> fdo::Result<bool> {
        let effect = effect_from_name(&effect).map_err(|e| fdo::Error::InvalidArgs(e.to_string()))?;
        let state = LightingState { effect, color: Rgb::new(color.0, color.1, color.2), speed, brightness };
        let result = self.ctl().set_state(state);
        let pending = result.map_err(set_error)?;
        self.emit_all(&emitter).await?;
        Ok(pending)
    }

    /// The Custom key map: (key usage, r, g, b), sorted by key; keys not
    /// listed are off.
    #[zbus(property)]
    async fn custom_keys(&self) -> Vec<(u8, u8, u8, u8)> {
        map_entries(&self.ctl().settings().custom)
    }

    /// The Gaming key map, as `CustomKeys`.
    #[zbus(property)]
    async fn gaming_keys(&self) -> Vec<(u8, u8, u8, u8)> {
        map_entries(&self.ctl().settings().gaming())
    }

    /// Replace the `"custom"` or `"gaming"` key map. Keys are USB HID usage
    /// IDs from the ANSI 104-key set; each may appear once.
    async fn set_keys(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        map: String,
        keys: Vec<(u8, u8, u8, u8)>,
    ) -> fdo::Result<()> {
        let name = MapName::parse(&map)
            .ok_or_else(|| fdo::Error::InvalidArgs(format!("unknown map {map:?}; use \"custom\" or \"gaming\"")))?;
        let keys = KeyMap::from_entries(keys.into_iter().map(|(k, r, g, b)| (k, Rgb::new(r, g, b))))
            .map_err(|e| fdo::Error::InvalidArgs(e.to_string()))?;
        let result = self.ctl().set_keys(name, keys);
        result.map_err(set_error)?;
        self.emit_all(&emitter).await?;
        Ok(())
    }

    /// Whether the Gaming map is shown while GameMode reports a game.
    #[zbus(property)]
    async fn game_switch(&self) -> bool {
        self.ctl().settings().game_switch
    }

    async fn set_game_switch(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>, on: bool) -> fdo::Result<()> {
        self.ctl().set_game_switch(on);
        self.emit_all(&emitter).await?;
        Ok(())
    }

    /// True while the Gaming map is shown because a game is running.
    #[zbus(property)]
    async fn game_active(&self) -> bool {
        self.ctl().game_active()
    }

    /// False while the keyboard is dark; the state, maps and game switch
    /// are kept and come back when it is turned on.
    #[zbus(property)]
    async fn lights_on(&self) -> bool {
        self.ctl().settings().lights_on
    }

    async fn set_lights_on(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>, on: bool) -> fdo::Result<()> {
        self.ctl().set_lights_on(on);
        self.emit_all(&emitter).await?;
        Ok(())
    }

    /// The G502 HERO: `connected`, `disconnected`, `permission-denied` or
    /// `unsupported`.
    #[zbus(property)]
    async fn mouse_state(&self) -> String {
        self.ctl().mouse_status().as_str().to_string()
    }

    /// Empty when the mouse is not connected.
    #[zbus(property)]
    async fn mouse_name(&self) -> String {
        self.ctl().mouse_info().map(|i| i.name.clone()).unwrap_or_default()
    }

    /// The mouse's selectable effects, as `Effects`; empty when not connected.
    #[zbus(property)]
    async fn mouse_effects(&self) -> Vec<(String, bool, bool, u8, u8)> {
        effect_list(self.ctl().mouse_offered())
    }

    /// True while the mouse follows the keyboard's lighting.
    #[zbus(property)]
    async fn mouse_match(&self) -> bool {
        self.ctl().settings().mouse.match_keyboard
    }

    /// The mouse's own setting, shown while it isn't matching: (effect,
    /// (r, g, b), speed, brightness).
    #[zbus(property)]
    async fn mouse_lighting(&self) -> (String, (u8, u8, u8), u8, u8) {
        let s = self.ctl().mouse_own();
        (s.effect.name().to_string(), (s.color.r, s.color.g, s.color.b), s.speed, s.brightness)
    }

    /// The mouse's onboard profiles: (number, report period in ms, default
    /// step index, DPI steps). Empty when unknown.
    #[zbus(property)]
    async fn mouse_profiles(&self) -> Vec<(u8, u8, u8, Vec<u16>)> {
        let ctl = self.ctl();
        let views = ctl.mouse_profiles().map(|p| p.views()).unwrap_or_default();
        views.into_iter().map(|v| (v.number, v.report_rate_ms, v.default_index, v.dpis)).collect()
    }

    /// The active profile, 1-based; 0 when unknown.
    #[zbus(property)]
    async fn mouse_active_profile(&self) -> u8 {
        self.ctl().mouse_profiles().map_or(0, |p| p.active)
    }

    /// The sensor's DPI range: (min, max, step); zeros when unknown.
    #[zbus(property)]
    async fn mouse_dpi_range(&self) -> (u16, u16, u16) {
        self.ctl().mouse_profiles().map_or((0, 0, 0), |p| (p.dpi_range.min, p.dpi_range.max, p.dpi_range.step))
    }

    /// Each profile's buttons: (profile number, [(kind, value, modifiers)]),
    /// buttons in the mouse's order, left click first. Kinds: `mouse` (value
    /// 1 left, 2 right, 4 middle, 8 back, 16 forward), `key` (HID usage, with
    /// modifier bits Ctrl 1, Shift 2, Alt 4, Super 8), `media` (consumer
    /// usage), `special` (mouse function code), `macro` (the macro's sector,
    /// see `MouseMacros`), `disabled` and `other` (an entry this service
    /// doesn't write). Empty when unknown.
    #[zbus(property)]
    async fn mouse_buttons(&self) -> Vec<(u8, Vec<(String, u16, u8)>)> {
        let ctl = self.ctl();
        let profiles = ctl.mouse_profiles().map(|p| p.buttons()).unwrap_or_default();
        (1..)
            .zip(profiles)
            .map(|(number, actions)| {
                let entries = actions
                    .into_iter()
                    .map(|a| {
                        let (kind, value, modifiers) = buttons::to_bus(a);
                        (kind.to_string(), value, modifiers)
                    })
                    .collect();
                (number, entries)
            })
            .collect()
    }

    /// The macros stored on the mouse: (sector, name, play mode, steps).
    /// The name is empty when this service doesn't know it. Play modes:
    /// `once`, `held` (repeats while held) and `toggle` (repeats until pressed
    /// again). Steps are (kind, value, modifiers): `key-down` and `key-up`
    /// (HID usage, modifier bits as for buttons), `button-down` and
    /// `button-up` (mouse-button mask), `media-down` and `media-up` (consumer
    /// usage), `pause` (milliseconds) and `wait-release` (0). Empty when
    /// unknown.
    #[zbus(property)]
    async fn mouse_macros(&self) -> Vec<(u8, String, String, Vec<(String, u16, u8)>)> {
        let ctl = self.ctl();
        ctl.mouse_macros()
            .into_iter()
            .map(|m| {
                let steps = m
                    .content
                    .steps
                    .into_iter()
                    .map(|step| {
                        let (kind, value, modifiers) = macros::step_to_bus(step);
                        (kind.to_string(), value, modifiers)
                    })
                    .collect();
                (m.sector, m.name, macros::mode_to_bus(m.content.mode).to_string(), steps)
            })
            .collect()
    }

    /// (used, total) macro sectors on the mouse, one macro each; (0, 0)
    /// when unknown.
    #[zbus(property)]
    async fn mouse_macro_sectors(&self) -> (u8, u8) {
        self.ctl().mouse_profiles().map_or((0, 0), |p| p.macro_sectors())
    }

    /// Save a macro on the mouse and return its sector: a new one when
    /// `sector` is 0, otherwise a new version of the macro in `sector`. The
    /// first macro write ever saves a backup of the macro sectors. Fails with
    /// `io.github.g512control.Error.MacroTooLong`, `.MacroMemoryFull` or
    /// `.WriteFailed` (the mouse is left as it was).
    async fn save_mouse_macro(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        sector: u8,
        name: String,
        mode: String,
        steps: Vec<(String, u16, u8)>,
    ) -> Result<u8, MacroError> {
        let result = self.ctl().save_mouse_macro(sector, &name, &mode, &steps);
        let saved = result?;
        self.emit_all(&emitter).await?;
        Ok(saved)
    }

    /// Delete the macro in `sector`. Every button that plays it, in every
    /// profile, is set to disabled first.
    async fn delete_mouse_macro(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>, sector: u8) -> Result<(), MacroError> {
        let result = self.ctl().delete_mouse_macro(sector);
        result?;
        self.emit_all(&emitter).await?;
        Ok(())
    }

    /// Change one button of a profile (1-based) on the mouse. Button 0 (left
    /// click), the kind `other`, and a `macro` whose sector holds none are
    /// refused. The first write ever saves a backup.
    async fn set_mouse_button(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        profile: u8,
        button: u8,
        kind: String,
        value: u16,
        modifiers: u8,
    ) -> fdo::Result<()> {
        let result = self.ctl().set_mouse_button(profile, button, &kind, value, modifiers);
        result.map_err(set_error)?;
        self.emit_all(&emitter).await?;
        Ok(())
    }

    /// Make a profile (1-based) the mouse's active one.
    async fn set_mouse_active_profile(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        profile: u8,
    ) -> fdo::Result<()> {
        let result = self.ctl().set_mouse_active_profile(profile);
        result.map_err(set_error)?;
        self.emit_all(&emitter).await?;
        Ok(())
    }

    /// Write a profile's report period (1, 2, 4 or 8 ms), default step and
    /// 1 to 5 DPI steps to the mouse. The first write ever saves a backup.
    async fn set_mouse_profile(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        profile: u8,
        report_rate_ms: u8,
        default_index: u8,
        dpis: Vec<u16>,
    ) -> fdo::Result<()> {
        let result = self.ctl().set_mouse_profile(profile, report_rate_ms, default_index, &dpis);
        result.map_err(set_error)?;
        self.emit_all(&emitter).await?;
        Ok(())
    }

    async fn set_mouse_match(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>, on: bool) -> fdo::Result<()> {
        self.ctl().set_mouse_match(on);
        self.emit_all(&emitter).await?;
        Ok(())
    }

    /// Give the mouse its own setting; turns matching off and the lights on.
    /// Returns whether it is pending (the mouse is not connected).
    async fn set_mouse_lighting(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        effect: String,
        color: (u8, u8, u8),
        speed: u8,
        brightness: u8,
    ) -> fdo::Result<bool> {
        let effect = effect_from_name(&effect).map_err(|e| fdo::Error::InvalidArgs(e.to_string()))?;
        let state = LightingState { effect, color: Rgb::new(color.0, color.1, color.2), speed, brightness };
        let result = self.ctl().set_mouse_lighting(state);
        let pending = result.map_err(|e| match e {
            SetError::Invalid(InvalidState::EffectNotOffered(effect)) => {
                fdo::Error::InvalidArgs(format!("the mouse does not offer {}", effect.name()))
            }
            e => set_error(e),
        })?;
        self.emit_all(&emitter).await?;
        Ok(pending)
    }
}

/// Emit property changes from outside a D-Bus call (hotplug, resume).
pub fn notify(conn: &zbus::blocking::Connection) {
    let result = conn.object_server().interface::<_, Lighting>(OBJECT_PATH).and_then(|iface| {
        let emitter = iface.signal_emitter().clone();
        zbus::block_on(iface.get().emit_all(&emitter))
    });
    if let Err(e) = result {
        eprintln!("warning: cannot emit property changes: {e}");
    }
}

#[cfg(test)]
mod tests {
    use zbus::DBusError;

    use super::*;

    #[test]
    fn macro_errors_have_their_own_names() {
        let name = |e: SetError| MacroError::from(e).name().to_string();
        assert_eq!(name(SetError::MacroTooLong("x".into())), "io.github.g512control.Error.MacroTooLong");
        assert_eq!(name(SetError::MacroMemoryFull), "io.github.g512control.Error.MacroMemoryFull");
        assert_eq!(name(SetError::WriteFailed("x".into())), "io.github.g512control.Error.WriteFailed");
        assert_eq!(name(SetError::Profile("x".into())), "org.freedesktop.DBus.Error.InvalidArgs");
        let full = MacroError::from(SetError::MacroMemoryFull);
        assert_eq!(full.description(), Some("every macro sector on the mouse is in use"));
    }
}
