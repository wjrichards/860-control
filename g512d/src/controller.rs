//! Owns the keyboard and the mouse: connection state, applying lighting, and
//! the saved state.

use std::{
    io,
    path::{Path, PathBuf},
};

use g512_hidpp::{
    Device, Error as HidppError, Feature, Hidraw, Transport,
    discovery::{Model, SYSFS_HIDRAW, find_lighting_interface},
    effects::{self, EffectKind, EffectParams, Zone},
    info::{self, DeviceInfo},
    perkey::{self, Frame},
    profiles::{self, DpiRange, ProfileSector, ProfilesInfo},
};

use crate::{
    keymap::{Corrections, KeyMap, MapName, compose, map_frame},
    mouse::{self, KeyboardView, MOUSE_EFFECTS, MouseSettings},
    profile_backup,
    state::{BRIGHTNESS_MAX, Effect, InvalidState, LightingState},
    store::{Saved, Settings, Store},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceStatus {
    Connected,
    Disconnected,
    PermissionDenied,
    Unsupported,
}

impl DeviceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            DeviceStatus::Connected => "connected",
            DeviceStatus::Disconnected => "disconnected",
            DeviceStatus::PermissionDenied => "permission-denied",
            DeviceStatus::Unsupported => "unsupported",
        }
    }
}

pub enum Opened<T> {
    Device { transport: T, path: PathBuf },
    NotFound,
    PermissionDenied,
    Failed(io::Error),
}

/// Finds and opens a device's lighting interface; a fake in tests.
pub trait Opener {
    type Transport: Transport;
    fn open(&mut self, model: Model) -> Opened<Self::Transport>;
}

pub struct HidrawOpener;

impl Opener for HidrawOpener {
    type Transport = Hidraw;

    fn open(&mut self, model: Model) -> Opened<Hidraw> {
        let path = match find_lighting_interface(Path::new(SYSFS_HIDRAW), model) {
            Ok(Some(p)) => p,
            Ok(None) => return Opened::NotFound,
            Err(e) => return Opened::Failed(e),
        };
        match Hidraw::open(&path) {
            Ok(transport) => Opened::Device { transport, path },
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => Opened::PermissionDenied,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Opened::NotFound,
            Err(e) => Opened::Failed(e),
        }
    }
}

#[derive(Debug)]
pub enum SetError {
    Invalid(InvalidState),
    Device(HidppError),
    /// A mouse profile request the service refuses (bad values, no profiles).
    Profile(String),
    /// The profile backup could not be saved, so nothing was written.
    Backup(io::Error),
}

struct Connection<T> {
    device: Device<T>,
    path: PathBuf,
    zone: Zone,
    info: DeviceInfo,
}

/// The mouse: both zones get the same effect.
struct MouseConnection<T> {
    device: Device<T>,
    path: PathBuf,
    zones: Vec<Zone>,
    info: DeviceInfo,
    /// `None` when the profiles couldn't be read; lighting still works.
    profiles: Option<MouseProfiles>,
}

/// The mouse's onboard profiles as last read or written.
pub struct MouseProfiles {
    info: ProfilesInfo,
    /// Profile 1 first.
    sectors: Vec<ProfileSector>,
    /// 1-based.
    pub active: u8,
    pub dpi_range: DpiRange,
}

/// One profile as front ends see it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileView {
    pub number: u8,
    pub report_rate_ms: u8,
    pub default_index: u8,
    pub dpis: Vec<u16>,
}

impl MouseProfiles {
    pub fn views(&self) -> Vec<ProfileView> {
        (1..)
            .zip(&self.sectors)
            .map(|(number, s)| ProfileView {
                number,
                report_rate_ms: s.report_rate_ms(),
                default_index: s.default_index(),
                dpis: s.dpis(),
            })
            .collect()
    }
}

/// What is sent to the keyboard: a zone effect, or a per-key frame.
enum Output {
    Zone(EffectKind, EffectParams),
    Frame(Box<Frame>),
}

pub struct Controller<O: Opener> {
    opener: O,
    conn: Option<Connection<O::Transport>>,
    status: DeviceStatus,
    settings: Settings,
    corrections: Corrections,
    store: Store,
    /// The Gaming map is shown because a game started with the switch on.
    game_override: bool,
    /// GameMode reports at least one registered game.
    game_running: bool,
    mouse: Option<MouseConnection<O::Transport>>,
    mouse_status: DeviceStatus,
}

impl<O: Opener> Controller<O> {
    pub fn new(opener: O, store: Store) -> Self {
        let settings = store.load();
        let corrections = Corrections::load(&store.corrections_path());
        if !corrections.is_empty() {
            eprintln!("{} brightness corrections from {}", corrections.len(), store.corrections_path().display());
        }
        Controller {
            opener,
            conn: None,
            status: DeviceStatus::Disconnected,
            settings,
            corrections,
            store,
            game_override: false,
            game_running: false,
            mouse: None,
            mouse_status: DeviceStatus::Disconnected,
        }
    }

    pub fn status(&self) -> DeviceStatus {
        self.status
    }

    pub fn info(&self) -> Option<&DeviceInfo> {
        self.conn.as_ref().map(|c| &c.info)
    }

    /// The hidraw node in use, when connected.
    pub fn device_path(&self) -> Option<&Path> {
        self.conn.as_ref().map(|c| c.path.as_path())
    }

    /// Selectable effects on the connected keyboard; empty when not connected.
    pub fn offered(&self) -> Vec<Effect> {
        let Some(c) = &self.conn else { return Vec::new() };
        let mut offered: Vec<Effect> = c.zone.selectable().filter_map(|e| e.kind).map(Effect::Zone).collect();
        if c.device.supports(Feature::PerKeyLighting) {
            offered.push(Effect::Custom);
        }
        offered
    }

    pub fn saved(&self) -> Option<Saved> {
        self.settings.saved
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Whether the Gaming map is shown because a game is running.
    pub fn game_active(&self) -> bool {
        self.game_override && self.settings.lights_on
    }

    /// Open the keyboard if it is not open, and apply what is due.
    /// Returns whether the keyboard is connected afterwards.
    pub fn connect(&mut self) -> bool {
        if self.conn.is_some() {
            return true;
        }
        let (transport, path) = match self.opener.open(Model::G512) {
            Opened::Device { transport, path } => (transport, path),
            Opened::NotFound => return self.set_status(DeviceStatus::Disconnected),
            Opened::PermissionDenied => return self.set_status(DeviceStatus::PermissionDenied),
            Opened::Failed(e) => {
                eprintln!("cannot open keyboard: {e}");
                return self.set_status(DeviceStatus::Disconnected);
            }
        };
        match Self::setup(transport, path) {
            Ok(conn) => {
                eprintln!(
                    "connected: {} on {}, firmware {}",
                    conn.info.name,
                    conn.path.display(),
                    conn.info.application_firmware().map_or_else(|| "unknown".into(), |f| f.to_string())
                );
                self.conn = Some(conn);
                self.status = DeviceStatus::Connected;
                self.apply_on_connect();
                true
            }
            Err(HidppError::FeatureMissing(f)) => {
                eprintln!("keyboard lacks {f}; unsupported");
                self.set_status(DeviceStatus::Unsupported)
            }
            Err(e) => {
                eprintln!("keyboard setup failed: {e}");
                self.set_status(DeviceStatus::Disconnected)
            }
        }
    }

    pub fn disconnect(&mut self) {
        if self.conn.take().is_some() {
            eprintln!("disconnected");
        }
        self.status = DeviceStatus::Disconnected;
    }

    /// Validate, apply and save a new state. Returns whether it is pending
    /// (saved, to be applied when the keyboard connects). An explicit state
    /// replaces the Gaming map until the next game starts, and turns the
    /// lights on.
    pub fn set_state(&mut self, state: LightingState) -> Result<bool, SetError> {
        let pending = self.set_keyboard_state(state)?;
        self.apply_mouse_logged();
        Ok(pending)
    }

    fn set_keyboard_state(&mut self, state: LightingState) -> Result<bool, SetError> {
        let offered = if self.conn.is_some() {
            self.offered()
        } else {
            EffectKind::ALL.into_iter().map(Effect::Zone).chain([Effect::Custom]).collect()
        };
        state.validate(&offered).map_err(SetError::Invalid)?;
        self.game_override = false;
        if self.conn.is_none() {
            self.settings.lights_on = true;
            self.save(Saved { state, pending: true });
            return Ok(true);
        }
        match self.send(self.state_output(&state)) {
            Ok(()) => {
                self.settings.lights_on = true;
                self.save(Saved { state, pending: false });
                Ok(false)
            }
            Err(e) if is_link_error(&e) => {
                eprintln!("keyboard went away while applying: {e}");
                self.disconnect();
                self.settings.lights_on = true;
                self.save(Saved { state, pending: true });
                Ok(true)
            }
            Err(e) => Err(SetError::Device(e)),
        }
    }

    /// Replace a key map; if it is the one shown, the keyboard (and a
    /// matching mouse) follows.
    pub fn set_keys(&mut self, name: MapName, map: KeyMap) -> Result<(), SetError> {
        self.set_keyboard_keys(name, map)?;
        self.apply_mouse_logged();
        Ok(())
    }

    fn set_keyboard_keys(&mut self, name: MapName, map: KeyMap) -> Result<(), SetError> {
        let previous = match name {
            MapName::Custom => std::mem::replace(&mut self.settings.custom, map),
            MapName::Gaming => {
                let old = self.settings.gaming();
                self.settings.gaming = Some(map);
                old
            }
        };
        if self.conn.is_some() && self.shows(name) {
            let out = self.due_output().expect("a shown map is due");
            match self.send(out) {
                Ok(()) => {}
                Err(e) if is_link_error(&e) => {
                    eprintln!("keyboard went away while applying: {e}");
                    self.disconnect();
                }
                Err(e) => {
                    match name {
                        MapName::Custom => self.settings.custom = previous,
                        MapName::Gaming => self.settings.gaming = Some(previous),
                    }
                    return Err(SetError::Device(e));
                }
            }
        }
        self.persist();
        Ok(())
    }

    /// Turn the lights off (Off on the keyboard, everything else kept) or
    /// back on (whatever is due). Saved either way; while disconnected it is
    /// applied on connect.
    pub fn set_lights_on(&mut self, on: bool) {
        self.settings.lights_on = on;
        self.persist();
        self.apply_due_logged();
        self.apply_mouse_logged();
    }

    /// Turn the game switch on or off. Turning it on during a game shows
    /// the Gaming map; turning it off returns to the saved state.
    pub fn set_game_switch(&mut self, on: bool) {
        self.settings.game_switch = on;
        self.persist();
        let show_gaming = on && self.game_running;
        if show_gaming != self.game_override {
            self.game_override = show_gaming;
            if self.settings.lights_on {
                self.apply_due_logged();
                self.apply_mouse_logged();
            }
        }
    }

    /// GameMode's registered-game count changed. Returns whether what is
    /// shown changed.
    pub fn set_game_count(&mut self, count: u32) -> bool {
        let running = count > 0;
        let started = running && !self.game_running;
        self.game_running = running;
        let show_gaming = if started { self.settings.game_switch } else { running && self.game_override };
        if show_gaming == self.game_override {
            return false;
        }
        self.game_override = show_gaming;
        // Games are still tracked while the lights are off, so turning them
        // on mid-game shows the Gaming map.
        if !self.settings.lights_on {
            eprintln!("game {}; the lights are off", if show_gaming { "started" } else { "ended" });
            return false;
        }
        if show_gaming {
            eprintln!("game started; showing the Gaming map");
        } else {
            eprintln!("last game ended; restoring the saved state");
        }
        self.apply_due_logged();
        self.apply_mouse_logged();
        true
    }

    pub fn mouse_status(&self) -> DeviceStatus {
        self.mouse_status
    }

    pub fn mouse_info(&self) -> Option<&DeviceInfo> {
        self.mouse.as_ref().map(|m| &m.info)
    }

    /// The mouse's hidraw node, when connected.
    pub fn mouse_path(&self) -> Option<&Path> {
        self.mouse.as_ref().map(|m| m.path.as_path())
    }

    /// Effects every mouse zone offers, among [`MOUSE_EFFECTS`]; empty when
    /// the mouse is not connected.
    pub fn mouse_offered(&self) -> Vec<Effect> {
        let Some(m) = &self.mouse else { return Vec::new() };
        MOUSE_EFFECTS
            .into_iter()
            .filter(|&k| m.zones.iter().all(|z| z.find(k).is_some()))
            .map(Effect::Zone)
            .collect()
    }

    /// The mouse's own setting as front ends see it (D5).
    pub fn mouse_own(&self) -> LightingState {
        mouse::own_or_default(&self.settings.mouse, self.saved().map(|s| s.state).as_ref())
    }

    /// Open the mouse if it is not open, and apply what it should show.
    /// Returns whether it is connected afterwards.
    pub fn connect_mouse(&mut self) -> bool {
        if self.mouse.is_some() {
            return true;
        }
        let (transport, path) = match self.opener.open(Model::G502Hero) {
            Opened::Device { transport, path } => (transport, path),
            Opened::NotFound => return self.set_mouse_status(DeviceStatus::Disconnected),
            Opened::PermissionDenied => return self.set_mouse_status(DeviceStatus::PermissionDenied),
            Opened::Failed(e) => {
                eprintln!("cannot open mouse: {e}");
                return self.set_mouse_status(DeviceStatus::Disconnected);
            }
        };
        match Self::setup_mouse(transport, path) {
            Ok(m) => {
                eprintln!("mouse connected: {} on {}", m.info.name, m.path.display());
                self.mouse = Some(m);
                self.mouse_status = DeviceStatus::Connected;
                self.apply_mouse_logged();
                self.mouse.is_some()
            }
            Err(HidppError::FeatureMissing(f)) => {
                eprintln!("mouse lacks {f}; unsupported");
                self.set_mouse_status(DeviceStatus::Unsupported)
            }
            Err(e) => {
                eprintln!("mouse setup failed: {e}");
                self.set_mouse_status(DeviceStatus::Disconnected)
            }
        }
    }

    pub fn disconnect_mouse(&mut self) {
        if self.mouse.take().is_some() {
            eprintln!("mouse disconnected");
        }
        self.mouse_status = DeviceStatus::Disconnected;
    }

    /// Re-apply what the mouse shows, reconnecting first if needed. Returns
    /// whether the mouse is connected afterwards.
    pub fn reapply_mouse(&mut self) -> bool {
        if self.mouse.is_none() {
            return self.connect_mouse();
        }
        self.apply_mouse_logged();
        if self.mouse.is_none() {
            // The node may have been re-created (resume, replug): reopen.
            return self.connect_mouse();
        }
        true
    }

    /// The mouse's profiles; `None` when not connected or not readable.
    pub fn mouse_profiles(&self) -> Option<&MouseProfiles> {
        self.mouse.as_ref().and_then(|m| m.profiles.as_ref())
    }

    /// Make profile `number` (1-based) the mouse's active one, then re-apply
    /// the mouse lighting (a profile switch may bring back its stored lighting).
    pub fn set_mouse_active_profile(&mut self, number: u8) -> Result<(), SetError> {
        let m = self.mouse.as_mut().ok_or_else(|| SetError::Profile("the mouse is not connected".into()))?;
        let p = m.profiles.as_mut().ok_or_else(|| SetError::Profile("the mouse's profiles are not available".into()))?;
        if number == 0 || number > p.info.profile_count {
            return Err(SetError::Profile(format!("no profile {number}")));
        }
        let result = profiles::set_current_profile(&mut m.device, &p.info, number)
            .and_then(|()| profiles::current_profile(&mut m.device));
        match result {
            Ok(active) => p.active = active,
            Err(e) => return Err(self.mouse_device_error(e)),
        }
        self.apply_mouse_logged();
        Ok(())
    }

    /// Change profile `number`'s report rate, DPI steps and default step on
    /// the mouse. Saves the backup first (once ever); a failed write keeps
    /// the previous values.
    pub fn set_mouse_profile(
        &mut self,
        number: u8,
        report_rate_ms: u8,
        default_index: u8,
        dpis: &[u16],
    ) -> Result<(), SetError> {
        let backup = self.store.path().with_file_name("mouse-profiles-backup.txt");
        let m = self.mouse.as_mut().ok_or_else(|| SetError::Profile("the mouse is not connected".into()))?;
        let p = m.profiles.as_mut().ok_or_else(|| SetError::Profile("the mouse's profiles are not available".into()))?;
        let sector = p.info.user_sector(number).map_err(|e| SetError::Profile(e.to_string()))?;
        if let Some(bad) = dpis.iter().find(|&&d| !p.dpi_range.contains(d)) {
            let r = p.dpi_range;
            return Err(SetError::Profile(format!("{bad} DPI is not {} to {} in steps of {}", r.min, r.max, r.step)));
        }
        let index = usize::from(number - 1);
        let edited = p.sectors[index]
            .edited(report_rate_ms, default_index, dpis)
            .map_err(|e| SetError::Profile(e.to_string()))?;
        match profile_backup::save_once(&mut m.device, &p.info, &backup) {
            Ok(true) => eprintln!("saved the mouse's profiles to {}", backup.display()),
            Ok(false) => {}
            Err(e) => return Err(SetError::Backup(e)),
        }
        if let Err(e) = profiles::write_sector(&mut m.device, &p.info, sector, &edited) {
            return Err(self.mouse_device_error(e));
        }
        p.sectors[index] = edited;
        let active = p.active == number;
        eprintln!("mouse profile {number}: {report_rate_ms} ms, DPI {dpis:?}, default step {}", default_index + 1);
        if active {
            self.apply_mouse_logged();
        }
        Ok(())
    }

    /// A failed profile request: a lost link disconnects the mouse.
    fn mouse_device_error(&mut self, e: HidppError) -> SetError {
        if is_link_error(&e) {
            self.disconnect_mouse();
        }
        SetError::Device(e)
    }

    /// Give the mouse its own setting: saved, matching off, lights on.
    /// Returns whether it is pending (the mouse is not connected).
    pub fn set_mouse_lighting(&mut self, state: LightingState) -> Result<bool, SetError> {
        let offered: Vec<Effect> = MOUSE_EFFECTS.into_iter().map(Effect::Zone).collect();
        state.validate(&offered).map_err(SetError::Invalid)?;
        self.settings.mouse = MouseSettings { match_keyboard: false, own: Some(state) };
        let was_off = !self.settings.lights_on;
        self.settings.lights_on = true;
        self.persist();
        if was_off {
            self.apply_due_logged();
        }
        self.apply_mouse_logged();
        Ok(self.mouse.is_none())
    }

    /// Follow the keyboard, or go back to the mouse's own setting.
    pub fn set_mouse_match(&mut self, on: bool) {
        self.settings.mouse.match_keyboard = on;
        self.persist();
        self.apply_mouse_logged();
    }

    /// Re-apply what is due, reconnecting first if needed. Returns whether
    /// the keyboard is connected with it applied.
    pub fn reapply(&mut self) -> bool {
        if self.conn.is_none() {
            return self.connect() && !self.saved().is_some_and(|s| s.pending);
        }
        let Some(out) = self.due_output() else { return true };
        match self.send(out) {
            Ok(()) => true,
            Err(e) if is_link_error(&e) => {
                // The node may have been re-created (resume, replug): reopen.
                self.disconnect();
                self.connect() && !self.saved().is_some_and(|s| s.pending)
            }
            Err(e) => {
                eprintln!("re-applying failed: {e}");
                false
            }
        }
    }

    fn setup(transport: O::Transport, path: PathBuf) -> Result<Connection<O::Transport>, HidppError> {
        let mut device = Device::open(transport)?;
        if !device.supports(Feature::ColorLedEffects) {
            return Err(HidppError::FeatureMissing(Feature::ColorLedEffects));
        }
        let info = info::read_info(&mut device)?;
        let mut lighting = effects::discover(&mut device)?;
        if lighting.zones.is_empty() {
            return Err(HidppError::FeatureMissing(Feature::ColorLedEffects));
        }
        Ok(Connection { device, path, zone: lighting.zones.remove(0), info })
    }

    fn setup_mouse(transport: O::Transport, path: PathBuf) -> Result<MouseConnection<O::Transport>, HidppError> {
        let mut device = Device::open(transport)?;
        if !device.supports(Feature::ColorLedEffects) {
            return Err(HidppError::FeatureMissing(Feature::ColorLedEffects));
        }
        let info = info::read_info(&mut device)?;
        let zones = effects::discover(&mut device)?.zones;
        if zones.is_empty() {
            return Err(HidppError::FeatureMissing(Feature::ColorLedEffects));
        }
        let profiles = match Self::read_profiles(&mut device) {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!("mouse profiles not available: {e}");
                None
            }
        };
        Ok(MouseConnection { device, path, zones, info, profiles })
    }

    fn read_profiles(device: &mut Device<O::Transport>) -> Result<MouseProfiles, HidppError> {
        let info = profiles::read_info(device)?;
        let sectors = (1..=info.profile_count)
            .map(|n| profiles::read_sector(device, &info, info.user_sector(n)?))
            .collect::<Result<Vec<_>, _>>()?;
        let active = profiles::current_profile(device)?;
        let dpi_range = profiles::dpi_range(device)?;
        Ok(MouseProfiles { info, sectors, active, dpi_range })
    }

    /// What the mouse should show, from the keyboard's settings and its own.
    fn mouse_due(&self) -> Option<LightingState> {
        let gaming = self.settings.gaming();
        let saved = self.saved().map(|s| s.state);
        let view = KeyboardView {
            lights_on: self.settings.lights_on,
            gaming_shown: self.game_override,
            saved: saved.as_ref(),
            custom: &self.settings.custom,
            gaming: &gaming,
        };
        mouse::mouse_state(&view, &self.settings.mouse)
    }

    /// Show on the mouse what is due. Failures are logged and never reach
    /// the keyboard's callers; a lost link disconnects the mouse only.
    fn apply_mouse_logged(&mut self) {
        let Some(state) = self.mouse_due() else { return };
        let Some(m) = self.mouse.as_mut() else { return };
        let Effect::Zone(kind) = state.effect else { return };
        let result =
            effects::set_effect_all(&mut m.device, &m.zones, state.device_effect(kind), &state.device_params(kind));
        if let Err(e) = result {
            eprintln!("applying mouse lighting failed: {e}");
            if is_link_error(&e) {
                self.disconnect_mouse();
            }
        }
    }

    fn set_mouse_status(&mut self, status: DeviceStatus) -> bool {
        self.mouse_status = status;
        false
    }

    /// Whether `name` is the map on the keyboard right now.
    fn shows(&self, name: MapName) -> bool {
        if !self.settings.lights_on {
            return false;
        }
        match name {
            MapName::Gaming => self.game_override,
            MapName::Custom => !self.game_override && self.saved().is_some_and(|s| s.state.effect == Effect::Custom),
        }
    }

    /// What should be on the keyboard; `None` leaves it as it is.
    fn due_output(&self) -> Option<Output> {
        if !self.settings.lights_on {
            return Some(Output::Zone(EffectKind::Off, EffectParams::default()));
        }
        if self.game_override {
            let brightness = self.saved().map_or(BRIGHTNESS_MAX, |s| s.state.brightness);
            return Some(self.map_output(&self.settings.gaming(), brightness));
        }
        self.saved().map(|s| self.state_output(&s.state))
    }

    fn state_output(&self, state: &LightingState) -> Output {
        let per_key = self.conn.as_ref().is_some_and(|c| c.device.supports(Feature::PerKeyLighting));
        match state.effect {
            Effect::Custom => self.map_output(&self.settings.custom, state.brightness),
            // Static as a frame, so per-key corrections apply.
            Effect::Zone(EffectKind::Static) if per_key && !self.corrections.is_empty() && state.brightness > 0 => {
                Output::Frame(Box::new(compose(|_| state.color, state.brightness, &self.corrections)))
            }
            Effect::Zone(kind) => Output::Zone(state.device_effect(kind), state.device_params(kind)),
        }
    }

    fn map_output(&self, map: &KeyMap, brightness: u8) -> Output {
        if brightness == 0 {
            Output::Zone(EffectKind::Off, EffectParams::default())
        } else {
            Output::Frame(Box::new(map_frame(map, brightness, &self.corrections)))
        }
    }

    fn send(&mut self, out: Output) -> Result<(), HidppError> {
        let conn = self.conn.as_mut().expect("send needs a connection");
        match out {
            Output::Zone(kind, params) => effects::set_effect(&mut conn.device, &conn.zone, kind, &params),
            Output::Frame(frame) => perkey::send_frame(&mut conn.device, &frame),
        }
    }

    fn apply_on_connect(&mut self) {
        if self.settings.lights_on && !self.game_override {
            if let Some(Err(e)) = self.saved().map(|s| s.state.validate(&self.offered())) {
                eprintln!("saved state not applied: {e}");
                return;
            }
        }
        let Some(out) = self.due_output() else { return };
        match self.send(out) {
            Ok(()) => {
                if let Some(saved) = self.saved().filter(|s| s.pending) {
                    self.save(Saved { pending: false, ..saved });
                }
            }
            Err(e) => eprintln!("applying saved state failed: {e}"),
        }
    }

    fn apply_due_logged(&mut self) {
        if self.conn.is_none() {
            return;
        }
        let Some(out) = self.due_output() else { return };
        if let Err(e) = self.send(out) {
            eprintln!("applying lighting failed: {e}");
            if is_link_error(&e) {
                self.disconnect();
            }
        }
    }

    fn save(&mut self, saved: Saved) {
        self.settings.saved = Some(saved);
        self.persist();
    }

    fn persist(&mut self) {
        if let Err(e) = self.store.save(&self.settings) {
            eprintln!("warning: cannot save {}: {e}", self.store.path().display());
        }
    }

    fn set_status(&mut self, status: DeviceStatus) -> bool {
        self.status = status;
        false
    }
}

/// Errors that mean the keyboard is gone rather than that it refused.
fn is_link_error(e: &HidppError) -> bool {
    matches!(e, HidppError::Io(_) | HidppError::Timeout { .. })
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::VecDeque, env, fs, rc::Rc};

    use g512_hidpp::{effects::Rgb, fake::FakeKeyboard};

    use super::*;

    /// Hands out queued outcomes, one queue per model; the last fake opened
    /// for each is shared so tests can inspect what was applied.
    struct FakeOpener {
        outcomes: VecDeque<&'static str>,
        last: Rc<RefCell<Option<FakeKeyboardHandle>>>,
        mouse_outcomes: VecDeque<&'static str>,
        mouse_last: Rc<RefCell<Option<FakeKeyboardHandle>>>,
    }

    type FakeKeyboardHandle = Rc<RefCell<FakeKeyboard>>;

    struct SharedFake(FakeKeyboardHandle);

    impl Transport for SharedFake {
        fn send(&mut self, report: &[u8]) -> io::Result<()> {
            self.0.borrow_mut().send(report)
        }
        fn recv(&mut self, buf: &mut [u8], timeout: std::time::Duration) -> io::Result<Option<usize>> {
            self.0.borrow_mut().recv(buf, timeout)
        }
    }

    impl Opener for FakeOpener {
        type Transport = SharedFake;
        fn open(&mut self, model: Model) -> Opened<SharedFake> {
            if model == Model::G502Hero {
                return match self.mouse_outcomes.pop_front().unwrap_or("absent") {
                    kind @ ("g502" | "g502-format3" | "g502-corrupt") => {
                        let mut mouse = FakeKeyboard::g502_hero();
                        mouse.profile_format = if kind == "g502-format3" { 3 } else { 2 };
                        mouse.corrupt_next_write = kind == "g502-corrupt";
                        let fake = Rc::new(RefCell::new(mouse));
                        *self.mouse_last.borrow_mut() = Some(fake.clone());
                        Opened::Device { transport: SharedFake(fake), path: "/dev/hidraw10".into() }
                    }
                    "denied" => Opened::PermissionDenied,
                    _ => Opened::NotFound,
                };
            }
            match self.outcomes.pop_front().unwrap_or("absent") {
                "g512" => {
                    let fake = Rc::new(RefCell::new(FakeKeyboard::g512()));
                    *self.last.borrow_mut() = Some(fake.clone());
                    Opened::Device { transport: SharedFake(fake), path: "/dev/hidraw1".into() }
                }
                "no-effects" => {
                    let mut kb = FakeKeyboard::g512();
                    kb.remove_feature(0x8070);
                    Opened::Device { transport: SharedFake(Rc::new(RefCell::new(kb))), path: "/dev/hidraw1".into() }
                }
                "denied" => Opened::PermissionDenied,
                _ => Opened::NotFound,
            }
        }
    }

    fn controller(name: &str, outcomes: &[&'static str]) -> (Controller<FakeOpener>, Rc<RefCell<Option<FakeKeyboardHandle>>>) {
        controller_with_corrections(name, outcomes, None)
    }

    fn controller_with_corrections(
        name: &str,
        outcomes: &[&'static str],
        corrections: Option<&str>,
    ) -> (Controller<FakeOpener>, Rc<RefCell<Option<FakeKeyboardHandle>>>) {
        let dir = env::temp_dir().join(format!("g512d-ctl-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        if let Some(text) = corrections {
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("corrections.toml"), text).unwrap();
        }
        let last = Rc::new(RefCell::new(None));
        let opener = FakeOpener {
            outcomes: outcomes.iter().copied().collect(),
            last: last.clone(),
            mouse_outcomes: VecDeque::new(),
            mouse_last: Rc::new(RefCell::new(None)),
        };
        (Controller::new(opener, Store::new(dir.join("state.toml"))), last)
    }

    type Handle = Rc<RefCell<Option<FakeKeyboardHandle>>>;

    /// A controller with a keyboard and a mouse queued; returns both handles.
    fn with_mouse(name: &str, outcomes: &[&'static str], mouse: &[&'static str]) -> (Controller<FakeOpener>, Handle, Handle) {
        let (mut c, kb) = controller(name, outcomes);
        c.opener.mouse_outcomes = mouse.iter().copied().collect();
        let m = c.opener.mouse_last.clone();
        (c, kb, m)
    }

    /// The last mouse write: (effect index, R, G, B) on zone 0, after
    /// checking zone 1 got the same.
    fn mouse_shown(m: &Handle) -> (u8, u8, u8, u8) {
        let applied = applied(m);
        let (z1, z0) = (applied[applied.len() - 1], applied[applied.len() - 2]);
        assert_eq!((z0[0], z1[0]), (0, 1), "both zones, in order");
        assert_eq!(z0[1..], z1[1..], "same effect on both zones");
        (z0[1], z0[2], z0[3], z0[4])
    }

    fn red() -> LightingState {
        LightingState { effect: Effect::Zone(EffectKind::Static), color: Rgb::new(0xff, 0, 0), speed: 50, brightness: 100 }
    }

    fn ripple() -> LightingState {
        LightingState { effect: Effect::Zone(EffectKind::Ripple), ..red() }
    }

    fn custom(brightness: u8) -> LightingState {
        LightingState { effect: Effect::Custom, brightness, ..red() }
    }

    fn applied(last: &Rc<RefCell<Option<FakeKeyboardHandle>>>) -> Vec<[u8; 16]> {
        last.borrow().as_ref().unwrap().borrow().applied.clone()
    }

    /// Committed per-key frames, as (key, r, g, b) entries.
    fn frames(last: &Rc<RefCell<Option<FakeKeyboardHandle>>>) -> Vec<Vec<[u8; 4]>> {
        last.borrow().as_ref().unwrap().borrow().frames.clone()
    }

    fn w_red() -> KeyMap {
        KeyMap::from_entries([(0x1a, Rgb::new(0xff, 0, 0))]).unwrap()
    }

    /// Zone effect index of the last `setZoneEffect` (6 is Ripple, 0 Off).
    fn last_zone_effect(last: &Rc<RefCell<Option<FakeKeyboardHandle>>>) -> u8 {
        applied(last).last().unwrap()[1]
    }

    const GAMING_W: [u8; 4] = [0x1a, 0xff, 0x20, 0x00];

    #[test]
    fn device_states() {
        let (mut c, _) = controller("states", &["denied", "absent", "no-effects", "g512"]);
        assert!(!c.connect());
        assert_eq!(c.status(), DeviceStatus::PermissionDenied);
        assert!(!c.connect());
        assert_eq!(c.status(), DeviceStatus::Disconnected);
        assert!(!c.connect());
        assert_eq!(c.status(), DeviceStatus::Unsupported);
        assert!(c.connect());
        assert_eq!(c.status(), DeviceStatus::Connected);
        assert_eq!(c.info().unwrap().name, "G512 RGB MECHANICAL GAMING KEYBOARD");
        assert_eq!(c.offered().len(), 7);
        assert_eq!(c.offered().last(), Some(&Effect::Custom));
    }

    #[test]
    fn set_applies_and_saves() {
        let (mut c, last) = controller("set", &["g512"]);
        c.connect();
        assert!(!c.set_state(red()).unwrap());
        assert_eq!(applied(&last).last().unwrap()[..6], [0, 1, 0xff, 0, 0, 0]);
        assert_eq!(c.saved(), Some(Saved { state: red(), pending: false }));
    }

    #[test]
    fn invalid_state_leaves_keyboard_unchanged() {
        let (mut c, last) = controller("invalid", &["g512"]);
        c.connect();
        let bad = LightingState { brightness: 150, ..red() };
        assert!(matches!(c.set_state(bad), Err(SetError::Invalid(InvalidState::Brightness(150)))));
        assert!(applied(&last).is_empty());
        assert_eq!(c.saved(), None);
    }

    #[test]
    fn device_rejection_is_not_saved() {
        let (mut c, last) = controller("reject", &["g512"]);
        c.connect();
        last.borrow().as_ref().unwrap().borrow_mut().reject_set_effect = Some(0x02);
        assert!(matches!(c.set_state(red()), Err(SetError::Device(HidppError::Hidpp { code: 0x02, .. }))));
        assert_eq!(c.saved(), None);
    }

    #[test]
    fn set_while_disconnected_is_pending_then_applied_on_connect() {
        let (mut c, last) = controller("pending", &["absent", "g512"]);
        c.connect();
        assert!(c.set_state(red()).unwrap());
        assert!(c.saved().unwrap().pending);
        assert!(c.reapply());
        assert_eq!(applied(&last).last().unwrap()[..6], [0, 1, 0xff, 0, 0, 0]);
        assert!(!c.saved().unwrap().pending);
    }

    #[test]
    fn saved_state_restored_on_start() {
        let (mut c, _) = controller("restore", &["g512"]);
        c.connect();
        let blue = LightingState { color: Rgb::new(0, 0, 0xff), ..red() };
        c.set_state(blue).unwrap();
        // A new controller with the same store, as after a login.
        let store = Store::new(c.store.path().to_path_buf());
        let last = Rc::new(RefCell::new(None));
        let opener = FakeOpener {
            outcomes: ["g512"].into(),
            last: last.clone(),
            mouse_outcomes: VecDeque::new(),
            mouse_last: Rc::new(RefCell::new(None)),
        };
        let mut c2 = Controller::new(opener, store);
        assert!(c2.connect());
        assert_eq!(applied(&last).last().unwrap()[..6], [0, 1, 0, 0, 0xff, 0]);
    }

    #[test]
    fn no_saved_state_leaves_keyboard_alone() {
        let (mut c, last) = controller("untouched", &["g512"]);
        assert!(c.connect());
        assert!(applied(&last).is_empty());
    }

    #[test]
    fn reapply_reconnects_after_link_loss() {
        let (mut c, last) = controller("relink", &["g512", "g512"]);
        c.connect();
        c.set_state(red()).unwrap();
        // The next reply never comes, as when the node vanished during suspend.
        last.borrow().as_ref().unwrap().borrow_mut().drop_next_reply();
        assert!(c.reapply());
        assert_eq!(c.status(), DeviceStatus::Connected);
        assert_eq!(applied(&last).len(), 1, "the reopened keyboard got the saved state");
    }

    #[test]
    fn custom_effect_sends_the_custom_map() {
        let (mut c, last) = controller("custom", &["g512"]);
        c.connect();
        c.set_keys(MapName::Custom, w_red()).unwrap();
        assert!(frames(&last).is_empty(), "Custom is not shown yet");
        c.set_state(custom(50)).unwrap();
        let frame = frames(&last).pop().unwrap();
        assert_eq!(frame.len(), 104);
        assert!(frame.contains(&[0x1a, 0x80, 0, 0]), "W at half brightness");
        assert!(frame.contains(&[0x29, 0, 0, 0]), "keys not in the map are off");
        assert!(applied(&last).is_empty(), "no zone effect");
    }

    #[test]
    fn custom_at_brightness_zero_is_zone_off() {
        let (mut c, last) = controller("custom-off", &["g512"]);
        c.connect();
        c.set_state(custom(0)).unwrap();
        assert_eq!(last_zone_effect(&last), 0);
        assert!(frames(&last).is_empty());
    }

    #[test]
    fn static_with_corrections_is_a_frame() {
        let (mut c, last) = controller_with_corrections("corrected", &["g512"], Some("e0 = 85\n"));
        c.connect();
        c.set_state(LightingState { color: Rgb::new(0xff, 0xff, 0xff), ..red() }).unwrap();
        let frame = frames(&last).pop().unwrap();
        assert!(frame.contains(&[0xe0, 0xd9, 0xd9, 0xd9]), "Left Ctrl corrected");
        assert!(frame.contains(&[0x29, 0xff, 0xff, 0xff]));
        assert!(applied(&last).is_empty());
        // Effects the keyboard runs itself stay zone effects.
        c.set_state(ripple()).unwrap();
        assert_eq!(last_zone_effect(&last), 6);
        // Brightness 0 is still zone Off.
        c.set_state(LightingState { brightness: 0, ..red() }).unwrap();
        assert_eq!(last_zone_effect(&last), 0);
    }

    #[test]
    fn editing_a_map_updates_the_keyboard_only_when_shown() {
        let (mut c, last) = controller("edit", &["g512"]);
        c.connect();
        c.set_state(red()).unwrap();
        c.set_keys(MapName::Gaming, w_red()).unwrap();
        assert!(frames(&last).is_empty(), "Gaming is not shown");
        assert_eq!(c.settings().gaming, Some(w_red()), "but it is saved");
        c.set_state(custom(100)).unwrap();
        c.set_keys(MapName::Custom, w_red()).unwrap();
        assert_eq!(frames(&last).len(), 2, "the shown Custom map follows the edit");
        assert!(frames(&last)[1].contains(&[0x1a, 0xff, 0, 0]));
    }

    #[test]
    fn device_rejecting_a_shown_map_keeps_the_old_one() {
        let (mut c, last) = controller("edit-reject", &["g512"]);
        c.connect();
        c.set_state(custom(100)).unwrap();
        last.borrow().as_ref().unwrap().borrow_mut().reject_set_keys = Some(0x02);
        assert!(matches!(c.set_keys(MapName::Custom, w_red()), Err(SetError::Device(_))));
        assert_eq!(c.settings().custom, KeyMap::default());
    }

    #[test]
    fn game_shows_gaming_map_then_restores() {
        let (mut c, last) = controller("game", &["g512"]);
        c.connect();
        c.set_state(ripple()).unwrap();
        c.set_game_switch(true);
        assert!(c.set_game_count(1));
        assert!(c.game_active());
        assert!(frames(&last).pop().unwrap().contains(&GAMING_W), "default Gaming map shown");
        assert!(!c.set_game_count(2), "a second game changes nothing");
        assert!(c.set_game_count(0));
        assert!(!c.game_active());
        assert_eq!(last_zone_effect(&last), 6, "Ripple back");
    }

    #[test]
    fn game_with_switch_off_changes_nothing() {
        let (mut c, last) = controller("game-off", &["g512"]);
        c.connect();
        c.set_state(ripple()).unwrap();
        assert!(!c.set_game_count(1));
        assert!(!c.game_active());
        assert!(frames(&last).is_empty());
    }

    #[test]
    fn user_change_during_a_game_wins_until_the_next_game() {
        let (mut c, last) = controller("game-user", &["g512"]);
        c.connect();
        c.set_state(ripple()).unwrap();
        c.set_game_switch(true);
        c.set_game_count(1);
        let blue = LightingState { color: Rgb::new(0, 0, 0xff), ..red() };
        c.set_state(blue).unwrap();
        assert!(!c.game_active());
        assert_eq!(applied(&last).last().unwrap()[..6], [0, 1, 0, 0, 0xff, 0]);
        assert!(!c.set_game_count(2), "still the same session of games");
        assert!(!c.set_game_count(0), "nothing to restore");
        assert_eq!(applied(&last).last().unwrap()[..6], [0, 1, 0, 0, 0xff, 0], "blue stays");
        assert!(c.set_game_count(1), "the next game shows Gaming again");
        assert!(c.game_active());
    }

    #[test]
    fn switch_toggled_during_a_game() {
        let (mut c, last) = controller("game-toggle", &["g512"]);
        c.connect();
        c.set_state(ripple()).unwrap();
        c.set_game_count(1);
        c.set_game_switch(true);
        assert!(c.game_active(), "turning it on mid-game shows Gaming");
        assert!(frames(&last).pop().unwrap().contains(&GAMING_W));
        c.set_game_switch(false);
        assert!(!c.game_active());
        assert_eq!(last_zone_effect(&last), 6, "off returns to Ripple");
        assert!(c.saved().is_some() && !c.settings().game_switch);
    }

    #[test]
    fn replug_during_a_game_shows_gaming_again() {
        let (mut c, last) = controller("game-replug", &["g512", "g512"]);
        c.connect();
        c.set_state(ripple()).unwrap();
        c.set_game_switch(true);
        c.set_game_count(1);
        last.borrow().as_ref().unwrap().borrow_mut().drop_next_reply();
        assert!(c.reapply());
        let reopened = frames(&last);
        assert_eq!(reopened.len(), 1, "the reopened keyboard got the Gaming map");
        assert!(reopened[0].contains(&GAMING_W));
        assert!(applied(&last).is_empty(), "not the saved Ripple");
    }

    #[test]
    fn gaming_uses_saved_brightness() {
        let (mut c, last) = controller("game-dim", &["g512"]);
        c.connect();
        c.set_state(LightingState { brightness: 50, ..ripple() }).unwrap();
        c.set_game_switch(true);
        c.set_game_count(1);
        assert!(frames(&last).pop().unwrap().contains(&[0x1a, 0x80, 0x10, 0x00]));
    }

    #[test]
    fn lights_off_then_on_restores_the_state() {
        let (mut c, last) = controller("lights", &["g512"]);
        c.connect();
        c.set_state(ripple()).unwrap();
        c.set_lights_on(false);
        assert_eq!(last_zone_effect(&last), 0, "Off");
        assert!(!c.settings().lights_on);
        assert_eq!(c.saved().unwrap().state, ripple(), "the saved state is kept");
        c.set_lights_on(true);
        assert_eq!(last_zone_effect(&last), 6, "Ripple back");
    }

    #[test]
    fn set_state_turns_the_lights_on() {
        let (mut c, last) = controller("lights-set", &["g512"]);
        c.connect();
        c.set_state(ripple()).unwrap();
        c.set_lights_on(false);
        c.set_state(red()).unwrap();
        assert!(c.settings().lights_on);
        assert_ne!(last_zone_effect(&last), 0);
    }

    #[test]
    fn rejected_state_leaves_the_lights_off() {
        let (mut c, _) = controller("lights-invalid", &["g512"]);
        c.connect();
        c.set_lights_on(false);
        assert!(c.set_state(LightingState { brightness: 150, ..red() }).is_err());
        assert!(!c.settings().lights_on);
    }

    #[test]
    fn game_with_the_lights_off_stays_dark() {
        let (mut c, last) = controller("lights-game", &["g512"]);
        c.connect();
        c.set_state(ripple()).unwrap();
        c.set_game_switch(true);
        c.set_lights_on(false);
        let writes = applied(&last).len();
        assert!(!c.set_game_count(1), "nothing a front end sees changes");
        assert!(!c.game_active());
        assert_eq!(applied(&last).len(), writes, "no write to the keyboard");
        assert!(frames(&last).is_empty());
        c.set_lights_on(true);
        assert!(c.game_active(), "turning on mid-game shows Gaming");
        assert!(frames(&last).pop().unwrap().contains(&GAMING_W));
    }

    #[test]
    fn replug_with_the_lights_off_sends_off() {
        let (mut c, last) = controller("lights-replug", &["g512", "g512"]);
        c.connect();
        c.set_state(ripple()).unwrap();
        c.set_lights_on(false);
        last.borrow().as_ref().unwrap().borrow_mut().drop_next_reply();
        assert!(c.reapply());
        assert_eq!(applied(&last).len(), 1, "one write to the reopened keyboard");
        assert_eq!(last_zone_effect(&last), 0, "Off, not the firmware default or Ripple");
    }

    #[test]
    fn lights_off_while_disconnected_applies_on_connect() {
        let (mut c, last) = controller("lights-pending", &["g512"]);
        assert!(c.set_state(ripple()).unwrap(), "pending");
        c.set_lights_on(false);
        assert!(c.connect());
        assert_eq!(last_zone_effect(&last), 0);
    }

    #[test]
    fn map_edits_and_game_switch_keep_the_lights_off() {
        let (mut c, last) = controller("lights-keys", &["g512"]);
        c.connect();
        c.set_state(custom(100)).unwrap();
        c.set_lights_on(false);
        let shown = frames(&last).len();
        c.set_keys(MapName::Custom, w_red()).unwrap();
        c.set_game_switch(true);
        assert!(!c.settings().lights_on);
        assert_eq!(frames(&last).len(), shown, "an edit while dark is saved, not shown");
        assert_eq!(c.settings().custom, w_red());
    }

    // Mouse effect indexes (spike 6): Off 0, Static 1, Breathe 2, Cycle 3.

    #[test]
    fn mouse_matches_the_keyboard() {
        let (mut c, _, m) = with_mouse("mouse-match", &["g512"], &["g502"]);
        c.connect();
        assert!(c.connect_mouse());
        assert_eq!(c.mouse_status(), DeviceStatus::Connected);
        assert!(applied(&m).is_empty(), "nothing saved yet: the mouse is left alone");
        c.set_state(red()).unwrap();
        assert_eq!(mouse_shown(&m), (1, 0xff, 0, 0), "Static red on both zones");
        c.set_state(LightingState { effect: Effect::Zone(EffectKind::Breathe), ..red() }).unwrap();
        assert_eq!(mouse_shown(&m).0, 2, "Breathe");
        c.set_state(LightingState { effect: Effect::Zone(EffectKind::Wave), ..red() }).unwrap();
        assert_eq!(mouse_shown(&m).0, 3, "Wave becomes Cycle");
        c.set_state(ripple()).unwrap();
        assert_eq!(mouse_shown(&m), (1, 0xff, 0, 0), "Ripple becomes Static in its colour");
    }

    #[test]
    fn mouse_follows_custom_and_gaming_maps() {
        let (mut c, _, m) = with_mouse("mouse-maps", &["g512"], &["g502"]);
        c.connect();
        c.connect_mouse();
        c.set_keys(MapName::Custom, w_red()).unwrap();
        c.set_state(custom(100)).unwrap();
        assert_eq!(mouse_shown(&m), (1, 0xff, 0, 0), "Custom: W's colour");
        c.set_keys(MapName::Custom, KeyMap::from_entries([(0x04, Rgb::new(9, 9, 9))]).unwrap()).unwrap();
        assert_eq!(mouse_shown(&m).0, 0, "Custom without W: Off");
        c.set_state(ripple()).unwrap();
        c.set_game_switch(true);
        c.set_game_count(1);
        assert_eq!(mouse_shown(&m), (1, 0xff, 0x20, 0), "Gaming: the default map's W colour");
        c.set_game_count(0);
        assert_eq!(mouse_shown(&m), (1, 0xff, 0, 0), "game over: matching Ripple again");
    }

    #[test]
    fn mouse_lights_off_and_on() {
        let (mut c, _, m) = with_mouse("mouse-lights", &["g512"], &["g502"]);
        c.connect();
        c.connect_mouse();
        c.set_state(red()).unwrap();
        c.set_lights_on(false);
        assert_eq!(mouse_shown(&m).0, 0, "Off");
        c.set_lights_on(true);
        assert_eq!(mouse_shown(&m), (1, 0xff, 0, 0));
    }

    #[test]
    fn mouse_own_setting() {
        let (mut c, kb, m) = with_mouse("mouse-own", &["g512"], &["g502"]);
        c.connect();
        c.connect_mouse();
        c.set_state(red()).unwrap();
        c.set_lights_on(false);
        let keyboard_writes = applied(&kb).len();
        let blue = LightingState { color: Rgb::new(0, 0, 0xff), ..red() };
        assert!(!c.set_mouse_lighting(blue).unwrap(), "connected: not pending");
        assert!(!c.settings().mouse.match_keyboard && c.settings().lights_on);
        assert_eq!(mouse_shown(&m), (1, 0, 0, 0xff));
        assert_eq!(last_zone_effect(&kb), 1, "lights back on: the keyboard shows Static red again");
        assert!(applied(&kb).len() > keyboard_writes);
        c.set_state(ripple()).unwrap();
        assert_eq!(mouse_shown(&m), (1, 0, 0, 0xff), "the keyboard changing doesn't move the mouse");
        c.set_mouse_match(true);
        assert_eq!(mouse_shown(&m), (1, 0xff, 0, 0), "matching again");
        assert!(matches!(
            c.set_mouse_lighting(LightingState { effect: Effect::Zone(EffectKind::Ripple), ..red() }),
            Err(SetError::Invalid(_))
        ));
    }

    #[test]
    fn mouse_errors_never_reach_the_keyboard() {
        let (mut c, kb, m) = with_mouse("mouse-error", &["g512"], &["g502"]);
        c.connect();
        c.connect_mouse();
        m.borrow().as_ref().unwrap().borrow_mut().reject_set_effect = Some(0x02);
        assert!(!c.set_state(red()).unwrap(), "the keyboard call still succeeds");
        assert_eq!(last_zone_effect(&kb), 1);
        assert_eq!(c.mouse_status(), DeviceStatus::Connected, "a refusal isn't a lost link");
        m.borrow().as_ref().unwrap().borrow_mut().reject_set_effect = None;
        m.borrow().as_ref().unwrap().borrow_mut().drop_next_reply();
        c.set_state(ripple()).unwrap();
        assert_eq!(c.mouse_status(), DeviceStatus::Disconnected, "a lost link disconnects the mouse only");
        assert_eq!(c.status(), DeviceStatus::Connected);
    }

    #[test]
    fn mouse_replug_and_permission() {
        let (mut c, _, m) = with_mouse("mouse-replug", &["g512"], &["denied", "g502"]);
        c.connect();
        c.set_state(red()).unwrap();
        assert!(!c.connect_mouse());
        assert_eq!(c.mouse_status(), DeviceStatus::PermissionDenied);
        assert!(c.reapply_mouse(), "access arrived: connected and applied");
        assert_eq!(mouse_shown(&m), (1, 0xff, 0, 0));
        c.disconnect_mouse();
        assert_eq!(c.mouse_status(), DeviceStatus::Disconnected);
        assert!(!c.reapply_mouse(), "nothing plugged in");
    }

    #[test]
    fn no_mouse_changes_nothing_for_the_keyboard() {
        let (mut c, kb, _) = with_mouse("mouse-none", &["g512"], &[]);
        c.connect();
        assert!(!c.connect_mouse());
        assert_eq!(c.mouse_status(), DeviceStatus::Disconnected);
        c.set_state(red()).unwrap();
        assert_eq!(last_zone_effect(&kb), 1);
        assert!(c.mouse_offered().is_empty());
    }

    #[test]
    fn mouse_offers_its_four_effects() {
        let (mut c, _, _) = with_mouse("mouse-offered", &["g512"], &["g502"]);
        c.connect_mouse();
        let names: Vec<_> = c.mouse_offered().into_iter().map(Effect::name).collect();
        assert_eq!(names, ["Off", "Static", "Breathe", "Cycle"]);
        assert_eq!(c.mouse_info().unwrap().name, "G502 HERO Gaming Mouse");
    }

    fn backup_path(c: &Controller<FakeOpener>) -> PathBuf {
        c.store.path().with_file_name("mouse-profiles-backup.txt")
    }

    fn views(c: &Controller<FakeOpener>) -> Vec<(u8, u8, u8, Vec<u16>)> {
        let views = c.mouse_profiles().unwrap().views();
        views.into_iter().map(|v| (v.number, v.report_rate_ms, v.default_index, v.dpis)).collect()
    }

    #[test]
    fn mouse_profiles_are_read_on_connect() {
        let (mut c, _, _) = with_mouse("profiles-read", &[], &["g502"]);
        c.connect_mouse();
        let p = c.mouse_profiles().unwrap();
        assert_eq!((p.active, p.dpi_range.min, p.dpi_range.max, p.dpi_range.step), (4, 100, 25600, 50));
        assert_eq!(views(&c)[0], (1, 1, 1, vec![1200, 2400, 3200, 6400]));
        assert_eq!(views(&c)[3], (4, 1, 0, vec![3200]));
    }

    #[test]
    fn mouse_profile_edit_writes_after_a_backup() {
        let (mut c, _, m) = with_mouse("profiles-edit", &[], &["g502"]);
        c.connect_mouse();
        assert!(!backup_path(&c).exists());
        c.set_mouse_profile(5, 2, 1, &[800, 1600, 3200]).unwrap();
        let backup = fs::read_to_string(backup_path(&c)).unwrap();
        assert!(backup.contains("sector 0x0005: crc ok"), "the backup holds the profiles from before the write");
        assert_eq!(views(&c)[4], (5, 2, 1, vec![800, 1600, 3200]));
        let fake = m.borrow().as_ref().unwrap().clone();
        assert_eq!(fake.borrow().profile_writes, [5]);
        // A second edit leaves the backup as it was.
        c.set_mouse_profile(5, 1, 0, &[6400]).unwrap();
        assert_eq!(fs::read_to_string(backup_path(&c)).unwrap(), backup);
        assert_eq!(fake.borrow().profile_writes, [5, 5]);
    }

    #[test]
    fn mouse_profile_edits_are_validated() {
        let (mut c, _, m) = with_mouse("profiles-invalid", &[], &["g502"]);
        c.connect_mouse();
        for (number, rate, default, dpis) in [
            (6, 1, 0, vec![800]),
            (0, 1, 0, vec![800]),
            (1, 3, 0, vec![800]),
            (1, 1, 0, vec![]),
            (1, 1, 0, vec![100, 200, 300, 400, 500, 600]),
            (1, 1, 2, vec![800, 1600]),
            (1, 1, 0, vec![125]),
            (1, 1, 0, vec![25650]),
            (1, 1, 0, vec![50]),
        ] {
            let result = c.set_mouse_profile(number, rate, default, &dpis);
            assert!(matches!(result, Err(SetError::Profile(_))), "{number} {rate} {default} {dpis:?}");
        }
        assert!(m.borrow().as_ref().unwrap().borrow().profile_writes.is_empty());
        assert!(!backup_path(&c).exists(), "nothing written, not even the backup");
    }

    #[test]
    fn failed_profile_write_keeps_the_old_values() {
        let (mut c, _, _) = with_mouse("profiles-corrupt", &[], &["g502-corrupt"]);
        c.connect_mouse();
        let before = views(&c);
        let result = c.set_mouse_profile(2, 1, 0, &[800]);
        assert!(matches!(result, Err(SetError::Device(HidppError::WriteNotVerified { sector: 2 }))));
        assert_eq!(views(&c), before);
        assert_eq!(c.mouse_status(), DeviceStatus::Connected, "a failed check isn't a lost link");
    }

    #[test]
    fn switching_profiles_reapplies_the_lighting() {
        let (mut c, _, m) = with_mouse("profiles-switch", &["g512"], &["g502"]);
        c.connect();
        c.connect_mouse();
        c.set_state(red()).unwrap();
        let writes = applied(&m).len();
        c.set_mouse_active_profile(1).unwrap();
        assert_eq!(c.mouse_profiles().unwrap().active, 1);
        assert_eq!(applied(&m).len(), writes + 2, "both zones re-applied");
        assert_eq!(mouse_shown(&m), (1, 0xff, 0, 0));
        assert!(matches!(c.set_mouse_active_profile(6), Err(SetError::Profile(_))));
        assert_eq!(c.mouse_profiles().unwrap().active, 1);
    }

    #[test]
    fn editing_the_active_profile_reapplies_the_lighting() {
        let (mut c, _, m) = with_mouse("profiles-active", &["g512"], &["g502"]);
        c.connect();
        c.connect_mouse();
        c.set_state(red()).unwrap();
        let writes = applied(&m).len();
        c.set_mouse_profile(5, 1, 0, &[6400]).unwrap();
        assert_eq!(applied(&m).len(), writes, "an inactive profile: lighting left alone");
        c.set_mouse_profile(4, 1, 0, &[1600, 3200]).unwrap();
        assert_eq!(applied(&m).len(), writes + 2, "the active profile: lighting re-applied");
    }

    #[test]
    fn mouse_without_readable_profiles_still_does_lighting() {
        let (mut c, _, m) = with_mouse("profiles-none", &["g512"], &["g502-format3"]);
        c.connect();
        assert!(c.connect_mouse());
        assert!(c.mouse_profiles().is_none());
        c.set_state(red()).unwrap();
        assert_eq!(mouse_shown(&m), (1, 0xff, 0, 0));
        assert!(matches!(c.set_mouse_profile(1, 1, 0, &[800]), Err(SetError::Profile(_))));
        assert!(matches!(c.set_mouse_active_profile(1), Err(SetError::Profile(_))));
    }

    #[test]
    fn profile_requests_without_a_mouse() {
        let (mut c, _, _) = with_mouse("profiles-nomouse", &["g512"], &[]);
        c.connect();
        assert!(c.mouse_profiles().is_none());
        assert!(matches!(c.set_mouse_profile(1, 1, 0, &[800]), Err(SetError::Profile(_))));
    }
}
