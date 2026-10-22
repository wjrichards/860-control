//! Device name and firmware versions.

use std::fmt;

use crate::{Device, Error, Feature, Result, Transport};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirmwareVersion {
    /// 0 = main application, 1 = bootloader, others are device-specific.
    pub kind: u8,
    pub prefix: String,
    pub number: u8,
    pub revision: u8,
    pub build: u16,
}

impl fmt::Display for FirmwareVersion {
    /// Number, revision and build are BCD on the wire, so hex prints them as decimal digits.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {:02x}.{:02x} build {:04x}", self.prefix, self.number, self.revision, self.build)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceInfo {
    pub name: String,
    pub firmware: Vec<FirmwareVersion>,
}

impl DeviceInfo {
    /// The main application firmware, if the device reported one.
    pub fn application_firmware(&self) -> Option<&FirmwareVersion> {
        self.firmware.iter().find(|f| f.kind == 0)
    }
}

pub fn read_info<T: Transport>(dev: &mut Device<T>) -> Result<DeviceInfo> {
    Ok(DeviceInfo { name: read_name(dev)?, firmware: read_firmware(dev)? })
}

fn read_name<T: Transport>(dev: &mut Device<T>) -> Result<String> {
    let len = dev.request(Feature::DeviceName, 0, &[])?[0] as usize;
    let mut name = Vec::with_capacity(len);
    while name.len() < len {
        let chunk = dev.request(Feature::DeviceName, 1, &[name.len() as u8])?;
        let take = (len - name.len()).min(chunk.len());
        if take == 0 {
            return Err(Error::Protocol("empty device name chunk".into()));
        }
        name.extend_from_slice(&chunk[..take]);
    }
    Ok(String::from_utf8_lossy(&name).trim_end_matches('\0').to_string())
}

fn read_firmware<T: Transport>(dev: &mut Device<T>) -> Result<Vec<FirmwareVersion>> {
    let count = dev.request(Feature::DeviceFwVersion, 0, &[])?[0];
    (0..count)
        .map(|entity| {
            let r = dev.request(Feature::DeviceFwVersion, 1, &[entity])?;
            let prefix = String::from_utf8_lossy(&r[1..4]).trim_matches(|c| c == '\0' || c == ' ').to_string();
            Ok(FirmwareVersion {
                kind: r[0],
                prefix,
                number: r[4],
                revision: r[5],
                build: u16::from_be_bytes([r[6], r[7]]),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeKeyboard;

    #[test]
    fn reads_name_and_firmware() {
        let mut dev = Device::open(FakeKeyboard::g512()).unwrap();
        let info = read_info(&mut dev).unwrap();
        assert_eq!(info.name, "G512 RGB MECHANICAL GAMING KEYBOARD");
        assert_eq!(info.application_firmware().unwrap().to_string(), "U1 19.01 build 0014");
        assert_eq!(info.firmware[1].to_string(), "BOT 65.00 build 6060");
    }
}
