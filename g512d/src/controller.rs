//! Owns the keyboard: connection state, applying lighting, and the saved state.

use std::{
    io,
    path::{Path, PathBuf},
};

use g512_hidpp::{
    Device, Error as HidppError, Feature, Hidraw, Transport,
    discovery::{SYSFS_HIDRAW, find_lighting_interface},
    effects::{self, EffectKind, Zone},
    info::{self, DeviceInfo},
};

use crate::{
    state::{InvalidState, LightingState},
    store::{Saved, Store},
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

pub struct Controller<O: Opener> {
    opener: O,
    conn: Option<Connection<O::Transport>>,
    status: DeviceStatus,
    saved: Option<Saved>,
    store: Store,
}

impl<O: Opener> Controller<O> {
    pub fn new(opener: O, store: Store) -> Self {
        let saved = store.load();
        Controller { opener, conn: None, status: DeviceStatus::Disconnected, saved, store }
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
    pub fn offered(&self) -> Vec<EffectKind> {
        self.conn.as_ref().map_or_else(Vec::new, |c| c.zone.selectable().filter_map(|e| e.kind).collect())
    }

    pub fn saved(&self) -> Option<Saved> {
        self.saved
    }

    /// Open the keyboard if it is not open, and apply the saved state.
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
                self.apply_saved();
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
    /// (saved, to be applied when the keyboard connects).
    pub fn set_state(&mut self, state: LightingState) -> Result<bool, SetError> {
        let offered = if self.conn.is_some() { self.offered() } else { EffectKind::ALL.to_vec() };
        state.validate(&offered).map_err(SetError::Invalid)?;
        if self.conn.is_none() {
            self.save(Saved { state, pending: true });
            return Ok(true);
        }
        match self.apply(&state) {
            Ok(()) => {
                self.save(Saved { state, pending: false });
                Ok(false)
            }
            Err(e) if is_link_error(&e) => {
                eprintln!("keyboard went away while applying: {e}");
                self.disconnect();
                self.save(Saved { state, pending: true });
                Ok(true)
            }
            Err(e) => Err(SetError::Device(e)),
        }
    }

    /// Re-apply the saved state, reconnecting first if needed. Returns
    /// whether the keyboard is connected with the saved state applied.
    pub fn reapply(&mut self) -> bool {
        if self.conn.is_none() {
            return self.connect() && !self.saved.is_some_and(|s| s.pending);
        }
        let Some(saved) = self.saved else { return true };
        match self.apply(&saved.state) {
            Ok(()) => true,
            Err(e) if is_link_error(&e) => {
                // The node may have been re-created (resume, replug): reopen.
                self.disconnect();
                self.connect() && !self.saved.is_some_and(|s| s.pending)
            }
            Err(e) => {
                eprintln!("re-applying saved state failed: {e}");
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

    fn apply(&mut self, state: &LightingState) -> Result<(), HidppError> {
        let conn = self.conn.as_mut().expect("apply needs a connection");
        effects::set_effect(
            &mut conn.device,
            &conn.zone,
            state.device_effect(),
            state.device_color(),
            state.device_speed(),
        )
    }

    fn apply_saved(&mut self) {
        let Some(saved) = self.saved else { return };
        if let Err(e) = saved.state.validate(&self.offered()) {
            eprintln!("saved state not applied: {e}");
            return;
        }
        match self.apply(&saved.state) {
            Ok(()) if saved.pending => self.save(Saved { pending: false, ..saved }),
            Ok(()) => {}
            Err(e) => eprintln!("applying saved state failed: {e}"),
        }
    }

    fn save(&mut self, saved: Saved) {
        self.saved = Some(saved);
        if let Err(e) = self.store.save(&saved) {
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
        let dir = env::temp_dir().join(format!("g512d-ctl-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let last = Rc::new(RefCell::new(None));
        let opener = FakeOpener { outcomes: outcomes.iter().copied().collect(), last: last.clone() };
        (Controller::new(opener, Store::new(dir.join("state.toml"))), last)
    }

    fn red() -> LightingState {
        LightingState { effect: EffectKind::Static, color: Rgb::new(0xff, 0, 0), speed: 50, brightness: 100 }
    }

    fn applied(last: &Rc<RefCell<Option<FakeKeyboardHandle>>>) -> Vec<[u8; 16]> {
        last.borrow().as_ref().unwrap().borrow().applied.clone()
    }

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
        assert_eq!(c.offered().len(), 6);
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
}
