//! g512d: owns the Logitech G512 keyboard and exposes lighting control on
//! the session bus as `io.github.g512control.Lighting1`.

mod controller;
mod dbus;
mod events;
mod gamemode;
mod keymap;
mod state;
mod store;

use std::{
    process::ExitCode,
    sync::{Arc, Mutex},
    thread,
};

use controller::{Controller, HidrawOpener};
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
        _ => {
            eprintln!("usage: g512d [--state <file>] | --version");
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

    ctl.lock().unwrap_or_else(|e| e.into_inner()).connect();
    let status = ctl.lock().unwrap_or_else(|e| e.into_inner()).status();
    eprintln!("g512d {} serving {BUS_NAME}; keyboard {}", env!("CARGO_PKG_VERSION"), status.as_str());
    dbus::notify(&conn);

    if status == controller::DeviceStatus::PermissionDenied {
        if let Err(e) = events::spawn_login_retry(ctl.clone(), conn.clone()) {
            eprintln!("warning: cannot retry opening the keyboard ({e}); replug it to connect");
        }
    }
    if let Err(e) = events::spawn_hotplug(ctl.clone(), conn.clone()) {
        eprintln!("warning: no hotplug events ({e}); replugging the keyboard will not re-apply lighting");
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
fn running_owner() -> String {
    let owner = zbus::blocking::Connection::session().and_then(|c| {
        let dbus = zbus::blocking::fdo::DBusProxy::new(&c)?;
        Ok(dbus.get_name_owner(BUS_NAME.try_into()?)?.to_string())
    });
    owner.unwrap_or_else(|_| "another process".into())
}
