//! Keyboard hotplug (kernel uevents over netlink) and resume (logind).

use std::{
    io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    thread,
    time::Duration,
};

use crate::dbus::{Shared, notify};

/// Let udev apply the `uaccess` ACL before opening a new node.
const HOTPLUG_SETTLE: Duration = Duration::from_millis(500);
/// Let USB re-enumerate after resume.
const RESUME_SETTLE: Duration = Duration::from_secs(1);
const RETRY_DELAY: Duration = Duration::from_millis(500);
const ATTEMPTS: u32 = 3;

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

/// Re-apply the saved state with retries, then publish the result.
fn reapply_with_retries(ctl: &Shared, conn: &zbus::blocking::Connection) {
    let mut done = false;
    for attempt in 1..=ATTEMPTS {
        done = ctl.lock().unwrap_or_else(|e| e.into_inner()).reapply();
        if done {
            eprintln!("re-applied saved state (attempt {attempt})");
            break;
        }
        if attempt < ATTEMPTS {
            thread::sleep(RETRY_DELAY);
        }
    }
    if !done {
        eprintln!("could not re-apply saved state after {ATTEMPTS} attempts");
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
                    let connected = ctl.lock().unwrap_or_else(|e| e.into_inner()).device_path().is_some();
                    if !connected {
                        thread::sleep(HOTPLUG_SETTLE);
                        reapply_with_retries(&ctl, &conn);
                    }
                }
                "remove" => {
                    let mut c = ctl.lock().unwrap_or_else(|e| e.into_inner());
                    let ours = c.device_path().and_then(|p| p.file_name()).is_some_and(|f| *f == *ev.devname);
                    if ours {
                        c.disconnect();
                        drop(c);
                        notify(&conn);
                    }
                }
                _ => {}
            }
        }
    })?;
    Ok(())
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
