//! Owns the keyboard: connection state, applying lighting, and the saved state.

use std::{
    io,
    path::{Path, PathBuf},
};

use g512_hidpp::{
    Device, Error as HidppError, Feature, Hidraw, Transport,
    discovery::{SYSFS_HIDRAW, find_lighting_interface},
    effects::{self, EffectKind, EffectParams, Zone},
    info::{self, DeviceInfo},
    perkey::{self, Frame},
};

use crate::{
    keymap::{Corrections, KeyMap, MapName, compose, map_frame},
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

/// Finds and opens the keyboard; a fake in tests.
pub trait Opener {
    type Transport: Transport;
    fn open(&mut self) -> Opened<Self::Transport>;
}

pub struct HidrawOpener;

impl Opener for HidrawOpener {
    type Transport = Hidraw;

    fn open(&mut self) -> Opened<Hidraw> {
        let path = match find_lighting_interface(Path::new(SYSFS_HIDRAW)) {
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
}

struct Connection<T> {
    device: Device<T>,
    path: PathBuf,
    zone: Zone,
    info: DeviceInfo,
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
        let (transport, path) = match self.opener.open() {
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

    /// Replace a key map; if it is the one shown, the keyboard follows.
    pub fn set_keys(&mut self, name: MapName, map: KeyMap) -> Result<(), SetError> {
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
        true
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

    /// Hands out queued outcomes; the last fake keyboard opened is shared so
    /// tests can inspect what was applied.
    struct FakeOpener {
        outcomes: VecDeque<&'static str>,
        last: Rc<RefCell<Option<FakeKeyboardHandle>>>,
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
        fn open(&mut self) -> Opened<SharedFake> {
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
        let opener = FakeOpener { outcomes: outcomes.iter().copied().collect(), last: last.clone() };
        (Controller::new(opener, Store::new(dir.join("state.toml"))), last)
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
        let mut c2 = Controller::new(FakeOpener { outcomes: ["g512"].into(), last: last.clone() }, store);
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
}
