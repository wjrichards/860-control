//! Tests against the real keyboard. Run with `cargo test -p g512-hidpp -- --ignored`
//! after installing `packaging/70-g512-control.rules`. They change the
//! lighting in RAM only; a replug restores the keyboard's own lighting.

use std::path::Path;

use g512_hidpp::{
    Device, Hidraw,
    discovery::{SYSFS_HIDRAW, find_lighting_interface},
    effects::{self, EffectKind, EffectParams, Rgb},
    info,
    perkey::{self, Frame, KeyId},
};

fn open_keyboard() -> (Device<Hidraw>, std::path::PathBuf) {
    let path = find_lighting_interface(Path::new(SYSFS_HIDRAW))
        .unwrap()
        .expect("G512 lighting interface not found");
    (Device::open(Hidraw::open(&path).unwrap()).unwrap(), path)
}

#[test]
#[ignore = "needs the G512 connected and the udev rule installed"]
fn sets_static_green_on_real_keyboard() {
    let (mut dev, path) = open_keyboard();

    let info = info::read_info(&mut dev).unwrap();
    assert!(info.name.contains("G512"), "unexpected device name {:?}", info.name);
    eprintln!("{} on {}, firmware {}", info.name, path.display(), info.application_firmware().unwrap());

    let lighting = effects::discover(&mut dev).unwrap();
    let green = EffectParams { color: Rgb::new(0, 0xff, 0), ..Default::default() };
    effects::set_effect(&mut dev, &lighting.zones[0], EffectKind::Static, &green).unwrap();
}

/// Shows W, A, S and D red with every other key dim white.
#[test]
#[ignore = "needs the G512 connected and the udev rule installed"]
fn sets_wasd_frame_on_real_keyboard() {
    let (mut dev, _) = open_keyboard();
    let mut frame = Frame::filled(Rgb::new(0x30, 0x30, 0x30));
    for usage in [0x1a, 0x04, 0x16, 0x07] {
        frame.set(KeyId::new(usage).unwrap(), Rgb::new(0xff, 0, 0));
    }
    perkey::send_frame(&mut dev, &frame).unwrap();
}
