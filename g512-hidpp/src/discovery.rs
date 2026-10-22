//! Finding the G512's HID++ lighting interface among the hidraw nodes.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub const VENDOR_ID: u16 = 0x046d;
pub const PRODUCT_ID: u16 = 0xc33c;
/// Logitech's HID++ vendor usage page on the G512.
const HIDPP_USAGE_PAGE: u16 = 0xff43;
const LONG_REPORT_ID: u8 = 0x11;
const VERY_LONG_REPORT_ID: u8 = 0x12;

pub const SYSFS_HIDRAW: &str = "/sys/class/hidraw";

/// Whether a hidraw node, given its sysfs `uevent` text and report
/// descriptor, is the G512's lighting interface.
pub fn is_lighting_interface(uevent: &str, descriptor: &[u8]) -> bool {
    if parse_hid_id(uevent) != Some((VENDOR_ID, PRODUCT_ID)) {
        return false;
    }
    let ids = report_ids_on_page(descriptor, HIDPP_USAGE_PAGE);
    ids.contains(&LONG_REPORT_ID) && ids.contains(&VERY_LONG_REPORT_ID)
}

/// Scan `sysfs_hidraw` (normally [`SYSFS_HIDRAW`]) and return the `/dev`
/// path of the first G512 lighting interface, in name order.
pub fn find_lighting_interface(sysfs_hidraw: &Path) -> io::Result<Option<PathBuf>> {
    let mut names: Vec<_> = fs::read_dir(sysfs_hidraw)?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name())
        .collect();
    names.sort();
    for name in names {
        let device = sysfs_hidraw.join(&name).join("device");
        let (Ok(uevent), Ok(descriptor)) = (
            fs::read_to_string(device.join("uevent")),
            fs::read(device.join("report_descriptor")),
        ) else {
            continue;
        };
        if is_lighting_interface(&uevent, &descriptor) {
            return Ok(Some(Path::new("/dev").join(name)));
        }
    }
    Ok(None)
}

/// `HID_ID=0003:0000046D:0000C33C` -> `(0x046d, 0xc33c)`.
fn parse_hid_id(uevent: &str) -> Option<(u16, u16)> {
    let id = uevent.lines().find_map(|l| l.strip_prefix("HID_ID="))?;
    let mut parts = id.trim().split(':').skip(1);
    let vendor = u32::from_str_radix(parts.next()?, 16).ok()?;
    let product = u32::from_str_radix(parts.next()?, 16).ok()?;
    Some((u16::try_from(vendor).ok()?, u16::try_from(product).ok()?))
}

/// Report IDs declared while `page` is the current usage page.
fn report_ids_on_page(descriptor: &[u8], page: u16) -> Vec<u8> {
    const TYPE_GLOBAL: u8 = 1;
    const TAG_USAGE_PAGE: u8 = 0x0;
    const TAG_REPORT_ID: u8 = 0x8;
    const TAG_PUSH: u8 = 0xa;
    const TAG_POP: u8 = 0xb;

    let mut ids = Vec::new();
    let mut current_page: u32 = 0;
    let mut stack = Vec::new();
    let mut i = 0;
    while i < descriptor.len() {
        let prefix = descriptor[i];
        if prefix == 0xfe {
            // Long item: 0xfe, data size, long tag, data.
            let size = *descriptor.get(i + 1).unwrap_or(&0) as usize;
            i += 3 + size;
            continue;
        }
        let size = [0, 1, 2, 4][(prefix & 0x3) as usize];
        let Some(data) = descriptor.get(i + 1..i + 1 + size) else {
            break;
        };
        let value = data.iter().rev().fold(0u32, |acc, &b| acc << 8 | u32::from(b));
        if (prefix >> 2) & 0x3 == TYPE_GLOBAL {
            match prefix >> 4 {
                TAG_USAGE_PAGE => current_page = value,
                TAG_REPORT_ID if current_page == u32::from(page) => ids.push(value as u8),
                TAG_PUSH => stack.push(current_page),
                TAG_POP => current_page = stack.pop().unwrap_or(0),
                _ => {}
            }
        }
        i += 1 + size;
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEYBOARD_UEVENT: &str = "DRIVER=hid-generic\nHID_ID=0003:0000046D:0000C33C\n\
                                   HID_NAME=Logitech G512 RGB MECHANICAL GAMING KEYBOARD\n";
    const MOUSE_UEVENT: &str = "HID_ID=0003:0000046D:0000C08B\nHID_NAME=Logitech G502 HERO Gaming Mouse\n";

    // Descriptors recorded from the user's devices (evidence/descriptors.txt).
    const KEYBOARD_IFACE0: &str = "05 01 09 06 a1 01 05 07 19 e0 29 e7 15 00 25 01 75 01 95 08 81 02 95 01 75 08 81 01 95 03 75 01 05 08 19 01 29 03 91 02 95 05 75 01 91 01 95 06 75 08 15 00 26 ff 00 05 07 19 00 2a ff 00 81 00 c0";
    const KEYBOARD_IFACE1: &str = "05 01 09 06 a1 01 05 07 19 00 29 e7 15 00 26 e7 00 75 08 95 14 85 01 81 00 c0 05 0c 09 01 a1 01 85 02 15 00 25 01 75 01 95 07 09 b5 09 b6 09 b7 09 cd 09 e9 09 ea 09 e2 81 02 95 01 81 01 c0 06 43 ff 0a 02 06 a1 01 85 11 75 08 95 13 15 00 26 ff 00 09 02 81 00 09 02 91 00 c0 06 43 ff 0a 04 06 a1 01 85 12 75 08 95 3f 15 00 26 ff 00 09 04 81 00 09 04 91 00 c0";
    const MOUSE_IFACE1: &str = "05 01 09 06 a1 01 85 01 05 07 19 e0 29 e7 15 00 25 01 75 01 95 08 81 02 81 03 95 06 75 08 15 00 26 a4 00 19 00 2a a4 00 81 00 c0 05 0c 09 01 a1 01 85 03 75 10 95 02 15 01 26 8c 02 19 01 2a 8c 02 81 00 c0 05 01 09 80 a1 01 85 04 75 02 95 01 15 01 25 03 09 82 09 81 09 83 81 60 75 06 81 03 c0 06 00 ff 09 01 a1 01 85 10 75 08 95 06 15 00 26 ff 00 09 01 81 00 09 01 91 00 c0 06 00 ff 09 02 a1 01 85 11 75 08 95 13 15 00 26 ff 00 09 02 81 00 09 02 91 00 c0";

    fn hex(s: &str) -> Vec<u8> {
        s.split_whitespace().map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
    }

    #[test]
    fn selects_keyboard_interface_1() {
        assert!(is_lighting_interface(KEYBOARD_UEVENT, &hex(KEYBOARD_IFACE1)));
    }

    #[test]
    fn rejects_boot_keyboard_interface_0() {
        assert!(!is_lighting_interface(KEYBOARD_UEVENT, &hex(KEYBOARD_IFACE0)));
    }

    #[test]
    fn rejects_mouse_even_with_hidpp_reports() {
        // The mouse declares HID++ report 0x11 on page 0xff00.
        assert!(!is_lighting_interface(MOUSE_UEVENT, &hex(MOUSE_IFACE1)));
        // A keyboard ID on a descriptor without page 0xff43 is still rejected.
        assert!(!is_lighting_interface(KEYBOARD_UEVENT, &hex(MOUSE_IFACE1)));
    }

    #[test]
    fn report_ids_follow_usage_page() {
        assert_eq!(report_ids_on_page(&hex(KEYBOARD_IFACE1), 0xff43), [0x11, 0x12]);
        // Report 1 (NKRO keys) is declared after `05 07`, the keyboard page.
        assert_eq!(report_ids_on_page(&hex(KEYBOARD_IFACE1), 0x0007), [0x01]);
        assert_eq!(report_ids_on_page(&hex(KEYBOARD_IFACE1), 0x000c), [0x02]);
        assert_eq!(report_ids_on_page(&hex(MOUSE_IFACE1), 0xff00), [0x10, 0x11]);
    }

    #[test]
    fn finds_device_in_sysfs_tree() {
        let root = std::env::temp_dir().join(format!("g512-sysfs-{}", std::process::id()));
        let nodes = [
            ("hidraw0", KEYBOARD_UEVENT, KEYBOARD_IFACE0),
            ("hidraw1", KEYBOARD_UEVENT, KEYBOARD_IFACE1),
            ("hidraw3", MOUSE_UEVENT, MOUSE_IFACE1),
        ];
        for (name, uevent, desc) in nodes {
            let dev = root.join(name).join("device");
            fs::create_dir_all(&dev).unwrap();
            fs::write(dev.join("uevent"), uevent).unwrap();
            fs::write(dev.join("report_descriptor"), hex(desc)).unwrap();
        }
        let found = find_lighting_interface(&root).unwrap();
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(found, Some(PathBuf::from("/dev/hidraw1")));
    }
}
