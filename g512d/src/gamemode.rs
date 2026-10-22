//! Follows the GameMode daemon (`com.feralinteractive.GameMode`) on the
//! session bus, so the Gaming map can be shown while a game runs.
//!
//! GameMode is activatable, so nothing here may start it: the count is read
//! from the name's current unique owner only, with property caching off,
//! and only after `GetNameOwner` has shown that it is running.

use std::thread;

use zbus::{
    MatchRule,
    blocking::{Connection, MessageIterator, Proxy, fdo::DBusProxy, proxy::Builder},
    message::Type,
    proxy::CacheProperties,
};

use crate::dbus::{Shared, notify};

const NAME: &str = "com.feralinteractive.GameMode";
const PATH: &str = "/com/feralinteractive/GameMode";
const INTERFACE: &str = "com.feralinteractive.GameMode";

/// Registered games, or 0 when GameMode is not running.
fn client_count(bus: &Connection) -> u32 {
    let read = || -> zbus::Result<i32> {
        let owner = DBusProxy::new(bus)?.get_name_owner(NAME.try_into()?)?;
        let proxy: Proxy<'_> = Builder::new(bus)
            .destination(owner.to_string())?
            .path(PATH)?
            .interface(INTERFACE)?
            .cache_properties(CacheProperties::No)
            .build()?;
        proxy.get_property("ClientCount")
    };
    // No owner (not running) or a read error both mean no game.
    read().map_or(0, |n| n.max(0) as u32)
}

fn refresh(bus: &Connection, ctl: &Shared, conn: &Connection) {
    let count = client_count(bus);
    let changed = ctl.lock().unwrap_or_else(|e| e.into_inner()).set_game_count(count);
    if changed {
        notify(conn);
    }
}

/// Watch GameMode's game signals and its bus name; re-read the count on
/// each, so missed or repeated signals correct themselves.
pub fn spawn_gamemode(ctl: Shared, conn: Connection) -> zbus::Result<()> {
    let bus = Connection::session()?;
    let games = MatchRule::builder().msg_type(Type::Signal).interface(INTERFACE)?.build();
    let owner = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender("org.freedesktop.DBus")?
        .interface("org.freedesktop.DBus")?
        .member("NameOwnerChanged")?
        .add_arg(NAME)?
        .build();
    let streams = [MessageIterator::for_match_rule(games, &bus, None)?, MessageIterator::for_match_rule(owner, &bus, None)?];
    refresh(&bus, &ctl, &conn);
    for (i, stream) in streams.into_iter().enumerate() {
        let (bus, ctl, conn) = (bus.clone(), ctl.clone(), conn.clone());
        thread::Builder::new().name(format!("gamemode-{i}")).spawn(move || {
            for msg in stream {
                let Ok(msg) = msg else { continue };
                let member = msg.header().member().map(|m| m.to_string()).unwrap_or_default();
                if matches!(member.as_str(), "GameRegistered" | "GameUnregistered" | "NameOwnerChanged") {
                    refresh(&bus, &ctl, &conn);
                }
            }
        })?;
    }
    Ok(())
}
