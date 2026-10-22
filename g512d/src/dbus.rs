//! The session D-Bus interface `io.github.g512control.Lighting1`.

use std::sync::{Arc, Mutex, MutexGuard};

use g512_hidpp::effects::{EffectKind, Rgb};
use zbus::{fdo, interface, object_server::SignalEmitter};

use crate::{
    controller::{Controller, HidrawOpener, SetError},
    state::{LightingState, SPEED_MAX, SPEED_MIN, effect_from_name},
};

pub const BUS_NAME: &str = "io.github.g512control.Lighting1";
pub const OBJECT_PATH: &str = "/io/github/g512control/Lighting1";

pub type Shared = Arc<Mutex<Controller<HidrawOpener>>>;

/// Shown before anything has been saved; not applied to the keyboard.
const DEFAULT_STATE: LightingState =
    LightingState { effect: EffectKind::Static, color: Rgb::new(0xff, 0xff, 0xff), speed: 50, brightness: 100 };

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
        self.pending_changed(emitter).await
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
        self.ctl()
            .offered()
            .into_iter()
            .map(|k| {
                let speed = k.speed_range().is_some();
                let (min, max) = if speed { (SPEED_MIN, SPEED_MAX) } else { (0, 0) };
                (k.name().to_string(), k.accepts_color(), speed, min, max)
            })
            .collect()
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
        let pending = match result {
            Ok(pending) => pending,
            Err(SetError::Invalid(e)) => return Err(fdo::Error::InvalidArgs(e.to_string())),
            Err(SetError::Device(e)) => return Err(fdo::Error::Failed(format!("keyboard refused the change: {e}"))),
        };
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
