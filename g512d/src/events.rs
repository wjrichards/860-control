//! Keyboard and mouse hotplug (kernel uevents over netlink), resume (logind),
//! and access arriving just after login.

use std::{
    io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    path::Path,
    thread,
    time::Duration,
};

use crate::{
    controller::DeviceStatus,
    dbus::{Shared, notify},
};

/// Let udev apply the `uaccess` ACL before opening a new node.
const HOTPLUG_SETTLE: Duration = Duration::from_millis(500);
/// Let USB re-enumerate after resume.
const RESUME_SETTLE: Duration = Duration::from_secs(1);
const RETRY_DELAY: Duration = Duration::from_millis(500);
const ATTEMPTS: u32 = 3;
/// At login the session's `uaccess` ACL can land just after the service
/// starts, with no uevent to say so: keep trying for 15 s.
const LOGIN_ATTEMPTS: u32 = 30;

#[derive(Debug, PartialEq, Eq)]
pub struct Uevent {
    pub action: String,
    pub subsystem: String,
    pub devname: String,
}

/// Parse a kernel uevent datagram: `action@devpath\0KEY=value\0...`.
pub fn parse_uevent(buf: &[u8]) -> Option<Uevent> {
    let mut fields = buf.split(|&b| b == 0).map(String::from_utf8_lossy);
    let header = fields.next()?;
    if !header.contains('@') {
        return None; // udev's own (libudev-format) messages are not used
    }
    let (mut action, mut subsystem, mut devname) = (None, None, None);
    for f in fields {
        if let Some(v) = f.strip_prefix("ACTION=") {
            action = Some(v.to_string());
        } else if let Some(v) = f.strip_prefix("SUBSYSTEM=") {
            subsystem = Some(v.to_string());
        } else if let Some(v) = f.strip_prefix("DEVNAME=") {
            devname = Some(v.to_string());
        }
    }
    Some(Uevent { action: action?, subsystem: subsystem?, devname: devname.unwrap_or_default() })
}

struct UeventSocket(OwnedFd);

impl UeventSocket {
    fn open() -> io::Result<Self> {
        // SAFETY: plain socket(2) call; the returned fd is owned below.
        let fd = unsafe {
            libc::socket(libc::AF_NETLINK, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, libc::NETLINK_KOBJECT_UEVENT)
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: `fd` is a fresh descriptor we exclusively own.
        let sock = UeventSocket(unsafe { OwnedFd::from_raw_fd(fd) });
        // SAFETY: zeroed sockaddr_nl is a valid value; fields set below.
        let mut addr: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
        addr.nl_family = libc::AF_NETLINK as u16;
        addr.nl_groups = 1; // kernel uevent broadcast group
        // SAFETY: `addr` is a valid sockaddr_nl and its size is passed.
        let rc = unsafe {
            libc::bind(
                sock.0.as_raw_fd(),
                (&addr as *const libc::sockaddr_nl).cast(),
                std::mem::size_of::<libc::sockaddr_nl>() as u32,
            )
        };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(sock)
    }

    fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        // SAFETY: `buf` is valid for writes of `buf.len()` bytes.
        let n = unsafe { libc::recv(self.0.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len(), 0) };
        if n < 0 { Err(io::Error::last_os_error()) } else { Ok(n as usize) }
    }
}

/// Which device a removed hidraw node belonged to.
#[derive(Debug, PartialEq, Eq)]
enum Removed {
    Keyboard,
    Mouse,
}

/// Match a removed node's name against the keyboard's and the mouse's paths.
fn removed_device(devname: &str, keyboard: Option<&Path>, mouse: Option<&Path>) -> Option<Removed> {
    let ours = |p: Option<&Path>| p.and_then(|p| p.file_name()).is_some_and(|f| *f == *devname);
    if ours(keyboard) {
        Some(Removed::Keyboard)
    } else if ours(mouse) {
        Some(Removed::Mouse)
    } else {
        None
    }
}

/// Re-apply the mouse with retries while access is still being granted.
/// Quiet when there is simply no mouse.
fn reapply_mouse_with_retries(ctl: &Shared, conn: &zbus::blocking::Connection) {
    for attempt in 1..=ATTEMPTS {
        let mut c = ctl.lock().unwrap_or_else(|e| e.into_inner());
        if c.reapply_mouse() {
            eprintln!("re-applied mouse lighting (attempt {attempt})");
            break;
        }
        if c.mouse_status() == DeviceStatus::Disconnected {
            break;
        }
        drop(c);
        if attempt < ATTEMPTS {
            thread::sleep(RETRY_DELAY);
        }
    }
    notify(conn);
}

/// Re-apply what is due (the saved state, or the Gaming map during a game)
/// with retries, then publish the result.
fn reapply_with_retries(ctl: &Shared, conn: &zbus::blocking::Connection) {
    let mut done = false;
    for attempt in 1..=ATTEMPTS {
        done = ctl.lock().unwrap_or_else(|e| e.into_inner()).reapply();
        if done {
            eprintln!("re-applied lighting (attempt {attempt})");
            break;
        }
        if attempt < ATTEMPTS {
            thread::sleep(RETRY_DELAY);
        }
    }
    if !done {
        let status = ctl.lock().unwrap_or_else(|e| e.into_inner()).status();
        eprintln!("could not re-apply lighting after {ATTEMPTS} attempts (keyboard {})", status.as_str());
    }
    notify(conn);
}

pub fn spawn_hotplug(ctl: Shared, conn: zbus::blocking::Connection) -> io::Result<()> {
    let sock = UeventSocket::open()?;
    thread::Builder::new().name("hotplug".into()).spawn(move || {
        let mut buf = vec![0u8; 8192];
        loop {
            let n = match sock.recv(&mut buf) {
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    // ENOBUFS: events were dropped; resync by trying to reconnect.
                    eprintln!("uevent socket: {e}");
                    reapply_with_retries(&ctl, &conn);
                    continue;
                }
            };
            let Some(ev) = parse_uevent(&buf[..n]) else { continue };
            if ev.subsystem != "hidraw" {
                continue;
            }
            match ev.action.as_str() {
                "add" => {
                    let (keyboard, mouse) = {
                        let c = ctl.lock().unwrap_or_else(|e| e.into_inner());
                        (c.device_path().is_some(), c.mouse_path().is_some())
                    };
                    if !keyboard || !mouse {
                        thread::sleep(HOTPLUG_SETTLE);
                    }
                    if !keyboard {
                        reapply_with_retries(&ctl, &conn);
                    }
                    if !mouse {
                        reapply_mouse_with_retries(&ctl, &conn);
                    }
                }
                "remove" => {
                    let mut c = ctl.lock().unwrap_or_else(|e| e.into_inner());
                    match removed_device(&ev.devname, c.device_path(), c.mouse_path()) {
                        Some(Removed::Keyboard) => c.disconnect(),
                        Some(Removed::Mouse) => c.disconnect_mouse(),
                        None => continue,
                    }
                    drop(c);
                    notify(&conn);
                }
                _ => {}
            }
        }
    })?;
    Ok(())
}

/// If the keyboard or mouse was there but not ours to open when the service
/// started, retry until access arrives (or give up after `LOGIN_ATTEMPTS`).
pub fn spawn_login_retry(ctl: Shared, conn: zbus::blocking::Connection) -> io::Result<()> {
    thread::Builder::new().name("login-retry".into()).spawn(move || {
        let (attempt, status) = retry_while_denied(LOGIN_ATTEMPTS, || {
            thread::sleep(RETRY_DELAY);
            let mut c = ctl.lock().unwrap_or_else(|e| e.into_inner());
            c.connect();
            c.connect_mouse();
            either_denied(c.status(), c.mouse_status())
        });
        let c = ctl.lock().unwrap_or_else(|e| e.into_inner());
        let (keyboard, mouse) = (c.status().as_str(), c.mouse_status().as_str());
        drop(c);
        if status == DeviceStatus::PermissionDenied {
            eprintln!(
                "still no permission after {attempt} retries (keyboard {keyboard}, mouse {mouse}); \
                 replug it or check the udev rule"
            );
        } else {
            eprintln!("access arrived; keyboard {keyboard}, mouse {mouse} (retry {attempt})");
        }
        notify(&conn);
    })?;
    Ok(())
}

/// `PermissionDenied` while either device is, otherwise the keyboard's status.
pub fn either_denied(keyboard: DeviceStatus, mouse: DeviceStatus) -> DeviceStatus {
    if mouse == DeviceStatus::PermissionDenied { mouse } else { keyboard }
}

/// Call `connect` until it reports something other than permission denied,
/// at most `attempts` times. Returns the number of calls and the last status.
fn retry_while_denied(attempts: u32, mut connect: impl FnMut() -> DeviceStatus) -> (u32, DeviceStatus) {
    let mut status = DeviceStatus::PermissionDenied;
    for attempt in 1..=attempts {
        status = connect();
        if status != DeviceStatus::PermissionDenied {
            return (attempt, status);
        }
    }
    (attempts, status)
}

pub fn spawn_resume(ctl: Shared, conn: zbus::blocking::Connection) -> zbus::Result<()> {
    let system = zbus::blocking::Connection::system()?;
    let proxy = zbus::blocking::Proxy::new(
        &system,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )?;
    let signals = proxy.receive_signal("PrepareForSleep")?;
    thread::Builder::new().name("resume".into()).spawn(move || {
        let _system = system; // keep the connection alive with the iterator
        for msg in signals {
            match msg.body().deserialize::<bool>() {
                Ok(false) => {
                    eprintln!("resumed from suspend; re-applying");
                    thread::sleep(RESUME_SETTLE);
                    reapply_with_retries(&ctl, &conn);
                    reapply_mouse_with_retries(&ctl, &conn);
                }
                Ok(true) => {}
                Err(e) => eprintln!("bad PrepareForSleep signal: {e}"),
            }
        }
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_continues_while_either_device_is_denied() {
        use DeviceStatus::*;
        assert_eq!(either_denied(PermissionDenied, Disconnected), PermissionDenied);
        assert_eq!(either_denied(Connected, PermissionDenied), PermissionDenied);
        assert_eq!(either_denied(Connected, Connected), Connected);
        assert_eq!(either_denied(Connected, Disconnected), Connected, "no mouse: done");
        let mut replies = [(PermissionDenied, PermissionDenied), (Connected, PermissionDenied), (Connected, Connected)]
            .into_iter()
            .map(|(k, m)| either_denied(k, m));
        assert_eq!(retry_while_denied(30, || replies.next().unwrap()), (3, Connected));
    }

    #[test]
    fn remove_disconnects_only_the_matching_device() {
        let (kb, mouse) = (Path::new("/dev/hidraw8"), Path::new("/dev/hidraw10"));
        assert_eq!(removed_device("hidraw8", Some(kb), Some(mouse)), Some(Removed::Keyboard));
        assert_eq!(removed_device("hidraw10", Some(kb), Some(mouse)), Some(Removed::Mouse));
        assert_eq!(removed_device("hidraw9", Some(kb), Some(mouse)), None, "the mouse's boot node");
        assert_eq!(removed_device("hidraw10", Some(kb), None), None);
    }

    #[test]
    fn login_retry_stops_once_access_arrives() {
        let denied = DeviceStatus::PermissionDenied;
        let mut replies = [denied, denied, DeviceStatus::Connected].into_iter();
        assert_eq!(retry_while_denied(30, || replies.next().unwrap()), (3, DeviceStatus::Connected));
    }

    #[test]
    fn login_retry_gives_up() {
        let mut calls = 0;
        let result = retry_while_denied(4, || {
            calls += 1;
            DeviceStatus::PermissionDenied
        });
        assert_eq!((result, calls), ((4, DeviceStatus::PermissionDenied), 4));
    }

    #[test]
    fn login_retry_stops_on_other_states() {
        // Unplugged during the wait: hotplug takes over from here.
        assert_eq!(retry_while_denied(30, || DeviceStatus::Disconnected), (1, DeviceStatus::Disconnected));
    }

    #[test]
    fn parses_kernel_hidraw_add() {
        let msg = b"add@/devices/pci0000:00/0000:00:14.0/usb5/5-1/5-1.1/5-1.1:1.1/0003:046D:C33C.0007/hidraw/hidraw1\0\
                    ACTION=add\0DEVPATH=/devices/.../hidraw/hidraw1\0SUBSYSTEM=hidraw\0MAJOR=241\0MINOR=1\0\
                    DEVNAME=hidraw1\0SEQNUM=5123\0";
        assert_eq!(
            parse_uevent(msg),
            Some(Uevent { action: "add".into(), subsystem: "hidraw".into(), devname: "hidraw1".into() })
        );
    }

    #[test]
    fn ignores_libudev_messages_and_garbage() {
        assert_eq!(parse_uevent(b"libudev\0\xfe\xed\xca\xfe"), None);
        assert_eq!(parse_uevent(b"add@/devices/x\0SUBSYSTEM=usb\0"), None, "no ACTION");
    }
}
