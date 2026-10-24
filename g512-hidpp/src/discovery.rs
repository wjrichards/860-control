//! Finding a supported device's HID++ lighting interface among the hidraw
//! nodes.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub const VENDOR_ID: u16 = 0x046d;
const LONG_REPORT_ID: u8 = 0x11;
const VERY_LONG_REPORT_ID: u8 = 0x12;

pub const SYSFS_HIDRAW: &str = "/sys/class/hidraw";

/// A supported device, and how its HID++ interface is recognised.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    /// G512 RGB keyboard: HID++ on vendor page 0xff43, long and very long reports.
    G512,
    /// G502 HERO mouse: HID++ on vendor page 0xff00, long reports only.
    G502Hero,
}

impl Model {
    pub const ALL: [Model; 2] = [Model::G512, Model::G502Hero];

    pub fn product_id(self) -> u16 {
        match self {
            Model::G512 => 0xc33c,
            Model::G502Hero => 0xc08b,
        }
    }

    fn usage_page(self) -> u16 {
        match self {
            Model::G512 => 0xff43,
            Model::G502Hero => 0xff00,
        }
    }

    fn report_ids(self) -> &'static [u8] {
        match self {
            Model::G512 => &[LONG_REPORT_ID, VERY_LONG_REPORT_ID],
            Model::G502Hero => &[LONG_REPORT_ID],
        }
    }
}

/// Which supported device's lighting interface a hidraw node is, given its
/// sysfs `uevent` text and report descriptor; `None` for anything else.
pub fn is_lighting_interface(uevent: &str, descriptor: &[u8]) -> Option<Model> {
    let (vendor, product) = parse_hid_id(uevent)?;
    let model = Model::ALL.into_iter().find(|m| vendor == VENDOR_ID && product == m.product_id())?;
    let ids = report_ids_on_page(descriptor, model.usage_page());
    model.report_ids().iter().all(|id| ids.contains(id)).then_some(model)
}

/// Scan `sysfs_hidraw` (normally [`SYSFS_HIDRAW`]) and return the `/dev`
/// path of the first lighting interface of `model`, in name order.
pub fn find_lighting_interface(sysfs_hidraw: &Path, model: Model) -> io::Result<Option<PathBuf>> {
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
        if is_lighting_interface(&uevent, &descriptor) == Some(model) {
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
    const MOUSE_IFACE0: &str = "05 01 09 02 a1 01 09 01 a1 00 05 09 19 01 29 10 15 00 25 01 95 10 75 01 81 02 05 01 16 01 80 26 ff 7f 75 10 95 02 09 30 09 31 81 06 15 81 25 7f 75 08 95 01 09 38 81 06 05 0c 0a 38 02 95 01 81 06 c0 c0";
    const MOUSE_IFACE1: &str = "05 01 09 06 a1 01 85 01 05 07 19 e0 29 e7 15 00 25 01 75 01 95 08 81 02 81 03 95 06 75 08 15 00 26 a4 00 19 00 2a a4 00 81 00 c0 05 0c 09 01 a1 01 85 03 75 10 95 02 15 01 26 8c 02 19 01 2a 8c 02 81 00 c0 05 01 09 80 a1 01 85 04 75 02 95 01 15 01 25 03 09 82 09 81 09 83 81 60 75 06 81 03 c0 06 00 ff 09 01 a1 01 85 10 75 08 95 06 15 00 26 ff 00 09 01 81 00 09 01 91 00 c0 06 00 ff 09 02 a1 01 85 11 75 08 95 13 15 00 26 ff 00 09 02 81 00 09 02 91 00 c0";

    fn hex(s: &str) -> Vec<u8> {
        s.split_whitespace().map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
    }

    #[test]
    fn selects_keyboard_interface_1() {
        assert_eq!(is_lighting_interface(KEYBOARD_UEVENT, &hex(KEYBOARD_IFACE1)), Some(Model::G512));
    }

    #[test]
    fn rejects_boot_keyboard_interface_0() {
        assert_eq!(is_lighting_interface(KEYBOARD_UEVENT, &hex(KEYBOARD_IFACE0)), None);
    }

    #[test]
    fn selects_mouse_interface_1() {
        // HID++ report 0x11 on page 0xff00 (descriptors from hidraw9 and 10).
        assert_eq!(is_lighting_interface(MOUSE_UEVENT, &hex(MOUSE_IFACE1)), Some(Model::G502Hero));
        assert_eq!(is_lighting_interface(MOUSE_UEVENT, &hex(MOUSE_IFACE0)), None);
    }

    #[test]
    fn descriptor_must_fit_the_product() {
        // A keyboard ID on the mouse's descriptor (no page 0xff43), and the
        // reverse (page 0xff00 absent), are both rejected.
        assert_eq!(is_lighting_interface(KEYBOARD_UEVENT, &hex(MOUSE_IFACE1)), None);
        assert_eq!(is_lighting_interface(MOUSE_UEVENT, &hex(KEYBOARD_IFACE1)), None);
    }

    #[test]
    fn other_products_are_ignored() {
        // A Logitech webcam (046d:0894) and another vendor's device with the
        // mouse's descriptor.
        let webcam = "HID_ID=0003:0000046D:00000894\n";
        let other = "HID_ID=0003:00001462:00007C92\n";
        assert_eq!(is_lighting_interface(webcam, &hex(MOUSE_IFACE1)), None);
        assert_eq!(is_lighting_interface(other, &hex(KEYBOARD_IFACE1)), None);
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
            ("hidraw2", MOUSE_UEVENT, MOUSE_IFACE0),
            ("hidraw3", MOUSE_UEVENT, MOUSE_IFACE1),
        ];
        for (name, uevent, desc) in nodes {
            let dev = root.join(name).join("device");
            fs::create_dir_all(&dev).unwrap();
            fs::write(dev.join("uevent"), uevent).unwrap();
            fs::write(dev.join("report_descriptor"), hex(desc)).unwrap();
        }
        let keyboard = find_lighting_interface(&root, Model::G512).unwrap();
        let mouse = find_lighting_interface(&root, Model::G502Hero).unwrap();
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(keyboard, Some(PathBuf::from("/dev/hidraw1")));
        assert_eq!(mouse, Some(PathBuf::from("/dev/hidraw3")));
    }
}
