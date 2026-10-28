//! G512 Control: a GNOME app for the Logitech G512 keyboard's lighting. It
//! only talks to the g512d service over D-Bus and never opens the device.

mod art;
mod art_area;
mod button_actions;
mod button_section;
mod editor;
mod keyboard_art;
mod layout;
mod mouse_section;
mod profile_edit;
mod profile_section;
mod service;
mod throttle;
mod udev;
mod window;

use adw::prelude::*;
use gtk::{gio, glib};

const APP_ID: &str = "io.github.g512control.Control";

fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(window::present);

    let quit = gio::ActionEntry::builder("quit").activate(|app: &adw::Application, _, _| app.quit()).build();
    app.add_action_entries([quit]);
    app.set_accels_for_action("app.quit", &["<Control>q"]);
    app.set_accels_for_action("window.close", &["<Control>w"]);

    app.run()
}
