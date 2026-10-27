//! g512d: owns the Logitech G512 keyboard and exposes lighting control on
//! the session bus as `io.github.g512control.Lighting1`.

mod buttons;
mod controller;
mod dbus;
mod events;
mod gamemode;
mod keymap;
mod mouse;
mod profile_backup;
mod state;
mod store;

use std::{
    process::ExitCode,
    sync::{Arc, Mutex},
    thread,
};

use controller::{Controller, HidrawOpener, Opened, Opener};
use g512_hidpp::{Device, discovery::Model};
use dbus::{BUS_NAME, Lighting, OBJECT_PATH};
use store::Store;
use zbus::fdo::{RequestNameFlags, RequestNameReply};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let state_arg = match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        [] => None,
        ["--version"] => {
            println!("g512d {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        ["--state", path] => Some(std::path::PathBuf::from(path)),
        ["--restore-mouse-profiles", file] => return restore_mouse_profiles(file),
        _ => {
            eprintln!("usage: g512d [--state <file>] | --restore-mouse-profiles <backup> | --version");
            return ExitCode::FAILURE;
        }
    };
    let Some(path) = state_arg.or_else(Store::default_path) else {
        eprintln!("g512d: cannot locate the config directory (neither XDG_CONFIG_HOME nor HOME is set)");
        return ExitCode::FAILURE;
    };
    let ctl = Arc::new(Mutex::new(Controller::new(HidrawOpener, Store::new(path))));

    // Own the bus name before touching the keyboard, so a second instance
    // exits without opening it. Only DoNotQueue: the builder's default also
    // sets ReplaceExisting/AllowReplacement, which lets a new instance steal
    // the name from a running one.
    let conn = match zbus::blocking::connection::Builder::session()
        .and_then(|b| b.serve_at(OBJECT_PATH, Lighting { ctl: ctl.clone() }))
        .and_then(|b| b.build())
    {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("g512d: cannot set up the session bus: {e}");
            return ExitCode::FAILURE;
        }
    };
    match conn.request_name_with_flags(BUS_NAME, RequestNameFlags::DoNotQueue.into()) {
        Ok(RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner) => {}
        Ok(_) | Err(zbus::Error::NameTaken) => {
            eprintln!("g512d: already running as {} ({BUS_NAME} is taken)", running_owner());
            return ExitCode::FAILURE;
        }
        Err(e) => {
            eprintln!("g512d: cannot own {BUS_NAME}: {e}");
            return ExitCode::FAILURE;
        }
    }

    let (status, mouse) = {
        let mut c = ctl.lock().unwrap_or_else(|e| e.into_inner());
        c.connect();
        c.connect_mouse();
        (c.status(), c.mouse_status())
    };
    eprintln!(
        "g512d {} serving {BUS_NAME}; keyboard {}, mouse {}",
        env!("CARGO_PKG_VERSION"),
        status.as_str(),
        mouse.as_str()
    );
    dbus::notify(&conn);

    if events::either_denied(status, mouse) == controller::DeviceStatus::PermissionDenied {
        if let Err(e) = events::spawn_login_retry(ctl.clone(), conn.clone()) {
            eprintln!("warning: cannot retry opening the keyboard ({e}); replug it to connect");
        }
    }
    if let Err(e) = events::spawn_hotplug(ctl.clone(), conn.clone()) {
        eprintln!("warning: no hotplug events ({e}); replugging the keyboard will not re-apply lighting");
    }
    if let Err(e) = events::spawn_profile_watch(ctl.clone(), conn.clone()) {
        eprintln!("warning: not following the mouse's active profile ({e}); its profile button won't show here");
    }
    if let Err(e) = events::spawn_resume(ctl.clone(), conn.clone()) {
        eprintln!("warning: no resume events ({e}); lighting will not be re-applied after suspend");
    }
    if let Err(e) = gamemode::spawn_gamemode(ctl.clone(), conn.clone()) {
        eprintln!("warning: not following GameMode ({e}); the Gaming map will not switch on with games");
    }

    loop {
        thread::park();
    }
}

/// The unique bus name currently owning ours, for the "already running" message.
/// Write the user profile sectors from a backup file back to the mouse.
/// Runs alongside the service; both can talk to the mouse.
fn restore_mouse_profiles(file: &str) -> ExitCode {
    let text = match std::fs::read_to_string(file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("cannot read {file}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let sectors = profile_backup::parse(&text);
    if sectors.is_empty() {
        eprintln!("{file} has no profile sectors");
        return ExitCode::FAILURE;
    }
    let transport = match HidrawOpener.open(Model::G502Hero) {
        Opened::Device { transport, path } => {
            println!("mouse on {}", path.display());
            transport
        }
        Opened::NotFound => {
            eprintln!("no G502 HERO found");
            return ExitCode::FAILURE;
        }
        Opened::PermissionDenied => {
            eprintln!("no permission to open the mouse; install the udev rule (packaging/INSTALL-udev.md)");
            return ExitCode::FAILURE;
        }
        Opened::Failed(e) => {
            eprintln!("cannot open the mouse: {e}");
            return ExitCode::FAILURE;
        }
    };
    let result = Device::open(transport).and_then(|mut dev| {
        let done = profile_backup::restore(&mut dev, &sectors)?;
        Ok((done, g512_hidpp::profiles::current_profile(&mut dev)?))
    });
    match result {
        Ok((done, active)) => {
            for (sector, outcome) in done {
                println!("sector {sector:#06x}: {outcome:?}");
            }
            // A running service holds the profiles in memory: switching to the
            // active profile makes it read them again.
            let refreshed = zbus::blocking::Connection::session().and_then(|conn| {
                conn.call_method(Some(BUS_NAME), OBJECT_PATH, Some(BUS_NAME), "SetMouseActiveProfile", &(active,))
            });
            match refreshed {
                Ok(_) => println!("the running service has re-read the profiles"),
                Err(e) => println!("the service wasn't refreshed ({e}); it reads the profiles when it starts"),
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("restore stopped: {e}");
            ExitCode::FAILURE
        }
    }
}

fn running_owner() -> String {
    let owner = zbus::blocking::Connection::session().and_then(|c| {
        let dbus = zbus::blocking::fdo::DBusProxy::new(&c)?;
        Ok(dbus.get_name_owner(BUS_NAME.try_into()?)?.to_string())
    });
    owner.unwrap_or_else(|_| "another process".into())
}
