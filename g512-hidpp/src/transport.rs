//! Moving raw HID reports to and from the keyboard.

use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    os::fd::AsRawFd,
    path::Path,
    time::Duration,
};

/// A channel for raw HID reports, so the protocol can run against a fake.
pub trait Transport {
    fn send(&mut self, report: &[u8]) -> io::Result<()>;
    /// Read one report into `buf`. `Ok(None)` means nothing arrived within `timeout`.
    fn recv(&mut self, buf: &mut [u8], timeout: Duration) -> io::Result<Option<usize>>;
}

/// A `/dev/hidrawN` node.
pub struct Hidraw {
    file: File,
}

impl Hidraw {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        Ok(Hidraw { file })
    }
}

impl Transport for Hidraw {
    fn send(&mut self, report: &[u8]) -> io::Result<()> {
        self.file.write_all(report)
    }

    fn recv(&mut self, buf: &mut [u8], timeout: Duration) -> io::Result<Option<usize>> {
        let mut pfd = libc::pollfd { fd: self.file.as_raw_fd(), events: libc::POLLIN, revents: 0 };
        let ms = timeout.as_millis().min(i32::MAX as u128) as i32;
        // SAFETY: `pfd` is a valid pollfd for the duration of the call and nfds is 1.
        let ready = unsafe { libc::poll(&mut pfd, 1, ms) };
        match ready {
            -1 => Err(io::Error::last_os_error()),
            0 => Ok(None),
            _ if pfd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 => {
                Err(io::Error::new(io::ErrorKind::NotConnected, "hidraw device went away"))
            }
            _ => self.file.read(buf).map(Some),
        }
    }
}
