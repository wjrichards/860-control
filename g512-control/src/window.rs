//! The main window: status pages for problems, otherwise the lighting
//! controls, with the key-map editor as a second page.

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::{Duration, Instant},
};

use adw::prelude::*;
use futures_util::StreamExt;
use gtk::{gdk, glib};
use zbus::fdo::{DBusProxy, PropertiesProxy};

use crate::{
    button_section::ButtonSection,
    editor::{Editor, MapName},
    mouse_section::MouseSection,
    profile_section::ProfileSection,
    service::{self, BUS_NAME, CUSTOM, DeviceState, EffectInfo, Keys, LightingProxy, LightingState, OBJECT_PATH, PerKey, Snapshot},
    throttle::Throttle,
    udev,
};

pub(crate) const UPDATE_INTERVAL: Duration = Duration::from_millis(100);

enum Status {
    ServiceMissing,
    Disconnected,
    PermissionDenied,
    Unsupported,
    NoSessionBus(String),
}

struct Bus {
    lighting: LightingProxy<'static>,
    props: PropertiesProxy<'static>,
}

pub struct Ui {
    window: adw::ApplicationWindow,
    toasts: adw::ToastOverlay,
    stack: gtk::Stack,
    status: adw::StatusPage,
    status_detail: gtk::Label,
    retry: gtk::Button,
    device_row: adw::ActionRow,
    lights_row: adw::SwitchRow,
    effect_row: adw::ComboRow,
    effect_names: gtk::StringList,
    color_row: adw::ActionRow,
    color_button: gtk::ColorDialogButton,
    speed_row: adw::ActionRow,
    speed: gtk::Scale,
    brightness: gtk::Scale,
    nav: adw::NavigationView,
    editor: Rc<Editor>,
    mouse_section: Rc<MouseSection>,
    profile_section: Rc<ProfileSection>,
    button_section: Rc<ButtonSection>,
    keymaps_row: adw::ActionRow,
    keymaps_button: gtk::Button,
    games_group: adw::PreferencesGroup,
    game_row: adw::SwitchRow,
    banner: adw::Banner,
    effects: RefCell<Vec<EffectInfo>>,
    /// Set while the controls are updated from the service, so their change
    /// handlers do not send that state back.
    syncing: Cell<bool>,
    throttle: RefCell<Throttle<LightingState>>,
    flush_scheduled: Cell<bool>,
    keys_throttle: RefCell<Throttle<(MapName, Keys)>>,
    keys_flush_scheduled: Cell<bool>,
    bus: RefCell<Option<Rc<Bus>>>,
}

pub fn present(app: &adw::Application) {
    if let Some(window) = app.active_window() {
        window.present();
        return;
    }
    let ui = Rc::new(Ui::build(app));
    ui.connect_controls();
    // The widgets' handlers hold weak references; the window holds the
    // strong one until it is destroyed.
    let keep = ui.clone();
    ui.window.connect_destroy(move |_| {
        let _ = &keep;
    });
    ui.window.present();
    glib::spawn_future_local(ui.run());
}

pub(crate) fn labelled_scale(label: &str, min: f64, max: f64) -> gtk::Scale {
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, min, max, 1.0);
    scale.set_width_request(200);
    scale.set_valign(gtk::Align::Center);
    scale.set_draw_value(true);
    scale.set_value_pos(gtk::PositionType::Left);
    scale.set_digits(0);
    scale.update_property(&[gtk::accessible::Property::Label(label)]);
    scale
}

pub(crate) fn to_rgba((r, g, b): (u8, u8, u8)) -> gdk::RGBA {
    gdk::RGBA::new(f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0, 1.0)
}

pub(crate) fn from_rgba(c: &gdk::RGBA) -> (u8, u8, u8) {
    let ch = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    (ch(c.red()), ch(c.green()), ch(c.blue()))
}

impl Ui {
    fn build(app: &adw::Application) -> Self {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("G512 Control")
            .default_width(860)
            .default_height(640)
            .build();

        let status_detail = gtk::Label::builder()
            .selectable(true)
            .wrap(true)
            .wrap_mode(gtk::pango::WrapMode::Char)
            .css_classes(["monospace", "card"])
            .build();
        status_detail.set_margin_start(12);
        status_detail.set_margin_end(12);
        let retry = gtk::Button::builder()
            .label("Try Again")
            .halign(gtk::Align::Center)
            .css_classes(["pill", "suggested-action"])
            .build();
        let status_box = gtk::Box::new(gtk::Orientation::Vertical, 18);
        status_box.append(&status_detail);
        status_box.append(&retry);
        let status = adw::StatusPage::builder().child(&status_box).build();

        let device_row = adw::ActionRow::new();
        device_row.add_prefix(&gtk::Image::from_icon_name("input-keyboard-symbolic"));
        let device_group = adw::PreferencesGroup::builder().title("Keyboard").build();
        device_group.add(&device_row);

        let effect_names = gtk::StringList::new(&[]);
        let effect_row = adw::ComboRow::builder().title("Effect").model(&effect_names).build();

        let color_button = gtk::ColorDialogButton::new(Some(
            gtk::ColorDialog::builder().title("Lighting Colour").with_alpha(false).build(),
        ));
        color_button.set_valign(gtk::Align::Center);
        color_button.update_property(&[gtk::accessible::Property::Label("Colour")]);
        let color_row = adw::ActionRow::builder().title("Colour").activatable_widget(&color_button).build();
        color_row.add_suffix(&color_button);

        let speed = labelled_scale("Speed", 1.0, 100.0);
        let speed_row = adw::ActionRow::builder().title("Speed").build();
        speed_row.add_suffix(&speed);

        let brightness = labelled_scale("Brightness", 0.0, 100.0);
        let brightness_row = adw::ActionRow::builder().title("Brightness").build();
        brightness_row.add_suffix(&brightness);

        let keymaps_button = gtk::Button::builder()
            .icon_name("go-next-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();
        keymaps_button.update_property(&[gtk::accessible::Property::Label("Open Key Maps")]);
        let keymaps_row = adw::ActionRow::builder()
            .title("Key Maps")
            .subtitle("Colour single keys for Custom and the Gaming layout")
            .activatable_widget(&keymaps_button)
            .build();
        keymaps_row.add_suffix(&keymaps_button);

        let lights_row = adw::SwitchRow::builder().title("Lights").build();
        let lighting_group = adw::PreferencesGroup::builder().title("Lighting").build();
        lighting_group.add(&lights_row);
        lighting_group.add(&effect_row);
        lighting_group.add(&color_row);
        lighting_group.add(&speed_row);
        lighting_group.add(&brightness_row);
        lighting_group.add(&keymaps_row);

        let game_row = adw::SwitchRow::builder()
            .title("Switch to Gaming Layout While a Game Runs")
            .subtitle("Games must use GameMode, for example with the Steam launch option gamemoderun %command%")
            .build();
        let games_group = adw::PreferencesGroup::builder().title("Games").build();
        games_group.add(&game_row);

        let page = adw::PreferencesPage::new();
        page.add(&device_group);
        page.add(&lighting_group);
        page.add(&games_group);
        let mouse_section = MouseSection::new();
        page.add(&mouse_section.group);
        let profile_section = ProfileSection::new();
        page.add(&profile_section.group);
        let button_section = ButtonSection::new();
        page.add(&button_section.group);

        let stack = gtk::Stack::new();
        stack.add_named(&status, Some("status"));
        stack.add_named(&page, Some("main"));

        let banner = adw::Banner::new("Gaming layout active while a game runs");
        banner.set_visible(false);
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&adw::HeaderBar::new());
        toolbar.add_top_bar(&banner);
        toolbar.set_content(Some(&stack));
        let main_page = adw::NavigationPage::builder().title("G512 Control").tag("main").child(&toolbar).build();

        let editor = Editor::new();
        let nav = adw::NavigationView::new();
        nav.add(&main_page);
        nav.add(&editor.page);
        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&nav));
        window.set_content(Some(&toasts));

        Ui {
            window,
            toasts,
            stack,
            status,
            status_detail,
            retry,
            device_row,
            lights_row,
            effect_row,
            effect_names,
            color_row,
            color_button,
            speed_row,
            speed,
            brightness,
            nav,
            editor,
            mouse_section,
            profile_section,
            button_section,
            keymaps_row,
            keymaps_button,
            games_group,
            game_row,
            banner,
            effects: RefCell::new(Vec::new()),
            syncing: Cell::new(false),
            throttle: RefCell::new(Throttle::new(UPDATE_INTERVAL)),
            flush_scheduled: Cell::new(false),
            keys_throttle: RefCell::new(Throttle::new(UPDATE_INTERVAL)),
            keys_flush_scheduled: Cell::new(false),
            bus: RefCell::new(None),
        }
    }

    fn connect_controls(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        let on_change = move |w: &Weak<Ui>| {
            if let Some(ui) = w.upgrade() {
                ui.user_changed();
            }
        };
        let w = weak.clone();
        self.effect_row.connect_selected_notify(move |_| on_change(&w));
        let w = weak.clone();
        self.color_button.connect_rgba_notify(move |_| on_change(&w));
        let w = weak.clone();
        self.speed.connect_value_changed(move |_| on_change(&w));
        let w = weak.clone();
        self.brightness.connect_value_changed(move |_| on_change(&w));
        let w = weak.clone();
        self.keymaps_button.connect_clicked(move |_| {
            if let Some(ui) = w.upgrade() {
                if ui.selected_effect().is_some_and(|e| e.name == CUSTOM) {
                    ui.editor.set_target(MapName::Custom);
                }
                ui.nav.push_by_tag("keymaps");
            }
        });
        let w = weak.clone();
        self.lights_row.connect_active_notify(move |row| {
            let Some(ui) = w.upgrade() else { return };
            if ui.syncing.get() {
                return;
            }
            let Some(bus) = ui.bus.borrow().clone() else { return };
            let on = row.is_active();
            glib::spawn_future_local(async move {
                if let Err(e) = bus.lighting.set_lights_on(on).await {
                    ui.toasts.add_toast(adw::Toast::new(&service::error_message(&e)));
                    ui.refresh().await;
                }
            });
        });
        let w = weak.clone();
        self.game_row.connect_active_notify(move |row| {
            let Some(ui) = w.upgrade() else { return };
            if ui.syncing.get() {
                return;
            }
            let Some(bus) = ui.bus.borrow().clone() else { return };
            let on = row.is_active();
            glib::spawn_future_local(async move {
                if let Err(e) = bus.lighting.set_game_switch(on).await {
                    ui.toasts.add_toast(adw::Toast::new(&service::error_message(&e)));
                    ui.refresh().await;
                }
            });
        });
        let w = weak.clone();
        self.editor.connect_edit(move |map, keys| {
            if let Some(ui) = w.upgrade() {
                ui.keys_changed(map, keys);
            }
        });
        let (on_switch, on_save) = (weak.clone(), weak.clone());
        self.profile_section.connect(
            move |number| {
                let Some(ui) = on_switch.upgrade() else { return };
                let Some(bus) = ui.bus.borrow().clone() else { return };
                glib::spawn_future_local(async move {
                    if let Err(e) = bus.lighting.set_mouse_active_profile(number).await {
                        ui.toasts.add_toast(adw::Toast::new(&service::error_message(&e)));
                    }
                    ui.refresh().await;
                });
            },
            move |(number, edit)| {
                let Some(ui) = on_save.upgrade() else { return };
                let Some(bus) = ui.bus.borrow().clone() else { return };
                glib::spawn_future_local(async move {
                    let result =
                        bus.lighting.set_mouse_profile(number, edit.report_rate_ms, edit.default_index, &edit.dpis).await;
                    if let Err(e) = result {
                        ui.toasts.add_toast(adw::Toast::new(&service::error_message(&e)));
                    }
                    ui.refresh().await;
                });
            },
        );
        let on_set = weak.clone();
        self.button_section.connect(move |(profile, button, action)| {
            let Some(ui) = on_set.upgrade() else { return };
            let Some(bus) = ui.bus.borrow().clone() else { return };
            glib::spawn_future_local(async move {
                let result =
                    bus.lighting.set_mouse_button(profile, button, &action.kind, action.value, action.modifiers).await;
                if let Err(e) = result {
                    ui.toasts.add_toast(adw::Toast::new(&service::error_message(&e)));
                }
                ui.refresh().await;
            });
        });
        let (on_match, on_lighting, on_idle) = (weak.clone(), weak.clone(), weak.clone());
        self.mouse_section.connect(
            move |on| {
                let Some(ui) = on_match.upgrade() else { return };
                let Some(bus) = ui.bus.borrow().clone() else { return };
                glib::spawn_future_local(async move {
                    if let Err(e) = bus.lighting.set_mouse_match(on).await {
                        ui.toasts.add_toast(adw::Toast::new(&service::error_message(&e)));
                        ui.refresh().await;
                    }
                });
            },
            move |state| {
                let Some(ui) = on_lighting.upgrade() else { return };
                let Some(bus) = ui.bus.borrow().clone() else { return };
                glib::spawn_future_local(async move {
                    let result =
                        bus.lighting.set_mouse_lighting(&state.effect, state.color, state.speed, state.brightness).await;
                    if let Err(e) = result {
                        ui.toasts.add_toast(adw::Toast::new(&service::error_message(&e)));
                        ui.refresh().await;
                    }
                });
            },
            move |()| {
                if let Some(ui) = on_idle.upgrade() {
                    glib::spawn_future_local(async move { ui.refresh().await });
                }
            },
        );
        let w = weak;
        self.retry.connect_clicked(move |_| {
            if let Some(ui) = w.upgrade() {
                glib::spawn_future_local(async move { ui.refresh().await });
            }
        });
    }

    async fn run(self: Rc<Self>) {
        let conn = match zbus::Connection::session().await {
            Ok(c) => c,
            Err(e) => return self.show_status(Status::NoSessionBus(e.to_string())),
        };
        let bus = async {
            let lighting = LightingProxy::builder(&conn).cache_properties(zbus::proxy::CacheProperties::No).build().await?;
            let props = PropertiesProxy::builder(&conn).destination(BUS_NAME)?.path(OBJECT_PATH)?.build().await?;
            zbus::Result::Ok(Bus { lighting, props })
        };
        let bus = match bus.await {
            Ok(b) => Rc::new(b),
            Err(e) => return self.show_status(Status::NoSessionBus(e.to_string())),
        };
        *self.bus.borrow_mut() = Some(bus.clone());
        self.refresh().await;

        // The service restarting, or any property change, re-reads everything.
        if let Ok(dbus) = DBusProxy::new(&conn).await
            && let Ok(mut owners) = dbus.receive_name_owner_changed_with_args(&[(0, BUS_NAME)]).await
        {
            let ui = Rc::downgrade(&self);
            glib::spawn_future_local(async move {
                while owners.next().await.is_some() {
                    let Some(ui) = ui.upgrade() else { break };
                    ui.refresh().await;
                }
            });
        }
        match bus.props.receive_properties_changed().await {
            Ok(mut changes) => {
                while changes.next().await.is_some() {
                    self.refresh().await;
                }
            }
            Err(e) => eprintln!("cannot follow lighting changes: {e}"),
        }
    }

    async fn refresh(&self) {
        let Some(bus) = self.bus.borrow().clone() else { return };
        match service::snapshot(&bus.props).await {
            Err(e) => {
                eprintln!("lighting service unavailable: {e}");
                self.show_status(Status::ServiceMissing);
            }
            Ok(Snapshot { keyboard, mouse }) => {
                self.mouse_section.show(mouse.as_deref());
                self.profile_section.show(mouse.as_deref());
                self.button_section.show(mouse.as_deref());
                match keyboard {
                    DeviceState::Connected { name, firmware, effects, state, per_key, lights_on } => {
                        self.show_lights(lights_on);
                        self.show_per_key(&effects, &per_key);
                        self.show_controls(&name, &firmware, effects, &state)
                    }
                    DeviceState::Disconnected => self.show_status(Status::Disconnected),
                    DeviceState::PermissionDenied => self.show_status(Status::PermissionDenied),
                    DeviceState::Unsupported => self.show_status(Status::Unsupported),
                }
            }
        }
    }

    fn show_status(&self, status: Status) {
        let (icon, title, description, detail, retry) = match status {
            Status::ServiceMissing => (
                "system-run-symbolic",
                "Lighting Service Not Running",
                "The g512d service controls the keyboard. Start it, then try again.",
                "systemctl --user start g512d".to_string(),
                true,
            ),
            Status::Disconnected => (
                "input-keyboard-symbolic",
                "Keyboard Not Connected",
                "Plug in the Logitech G512. Your saved lighting is applied when it connects.",
                String::new(),
                false,
            ),
            Status::PermissionDenied => (
                "dialog-password-symbolic",
                "No Access to the Keyboard",
                "Install the udev rule once, then unplug and replug the keyboard.",
                udev::install_command(udev::RULES),
                false,
            ),
            Status::Unsupported => (
                "dialog-warning-symbolic",
                "Keyboard Not Supported",
                "This keyboard's firmware does not offer the lighting effects this app uses.",
                String::new(),
                false,
            ),
            Status::NoSessionBus(e) => (
                "dialog-error-symbolic",
                "Cannot Reach the Session Bus",
                "The app talks to the lighting service over D-Bus.",
                e,
                true,
            ),
        };
        self.status.set_icon_name(Some(icon));
        self.status.set_title(title);
        self.status.set_description(Some(description));
        self.status_detail.set_label(&detail);
        self.status_detail.set_visible(!detail.is_empty());
        self.retry.set_visible(retry);
        self.stack.set_visible_child_name("status");
    }

    fn show_controls(&self, name: &str, firmware: &str, effects: Vec<EffectInfo>, state: &LightingState) {
        self.device_row.set_title(name);
        self.device_row.set_subtitle(&format!("Firmware {firmware}"));
        self.stack.set_visible_child_name("main");
        // Mid-drag the controls lead; the timer re-syncs once the user stops.
        if self.throttle.borrow().busy(Instant::now()) {
            return;
        }
        self.syncing.set(true);
        if *self.effects.borrow() != effects {
            let names: Vec<&str> = effects.iter().map(|e| e.name.as_str()).collect();
            self.effect_names.splice(0, self.effect_names.n_items(), &names);
            *self.effects.borrow_mut() = effects;
        }
        let index = self.effects.borrow().iter().position(|e| e.name == state.effect).unwrap_or(0);
        self.effect_row.set_selected(index as u32);
        self.color_button.set_rgba(&to_rgba(state.color));
        self.speed.set_value(f64::from(state.speed));
        self.brightness.set_value(f64::from(state.brightness));
        self.update_visibility();
        self.syncing.set(false);
    }

    /// The Lights switch, hidden on a service without the setting.
    fn show_lights(&self, lights_on: Option<bool>) {
        self.lights_row.set_visible(lights_on.is_some());
        self.syncing.set(true);
        self.lights_row.set_active(lights_on.unwrap_or(true));
        self.syncing.set(false);
    }

    /// Key maps, the game switch and the banner. Hidden on a service or
    /// keyboard without the Custom effect.
    fn show_per_key(&self, effects: &[EffectInfo], per_key: &PerKey) {
        let supported = effects.iter().any(|e| e.name == CUSTOM);
        self.keymaps_row.set_visible(supported);
        self.games_group.set_visible(supported);
        // Hidden as well as unrevealed, so screen readers skip it.
        self.banner.set_visible(per_key.game_active);
        self.banner.set_revealed(per_key.game_active);
        self.syncing.set(true);
        self.game_row.set_active(per_key.game_switch);
        self.syncing.set(false);
        if !self.keys_throttle.borrow().busy(Instant::now()) {
            self.editor.set_maps(&per_key.custom, &per_key.gaming);
        }
    }

    fn selected_effect(&self) -> Option<EffectInfo> {
        self.effects.borrow().get(self.effect_row.selected() as usize).cloned()
    }

    fn update_visibility(&self) {
        let Some(effect) = self.selected_effect() else { return };
        self.color_row.set_visible(effect.accepts_color);
        self.speed_row.set_visible(effect.accepts_speed);
        if effect.accepts_speed {
            self.speed.set_range(f64::from(effect.min_speed), f64::from(effect.max_speed));
        }
    }

    fn current_state(&self) -> Option<LightingState> {
        let effect = self.selected_effect()?;
        Some(LightingState {
            effect: effect.name,
            color: from_rgba(&self.color_button.rgba()),
            speed: self.speed.value().round() as u8,
            brightness: self.brightness.value().round() as u8,
        })
    }

    fn user_changed(self: Rc<Self>) {
        if self.syncing.get() {
            return;
        }
        self.update_visibility();
        if self.selected_effect().is_some_and(|e| e.name == CUSTOM) {
            self.editor.set_target(MapName::Custom);
        }
        let Some(state) = self.current_state() else { return };
        let ready = self.throttle.borrow_mut().push(Instant::now(), state);
        if let Some(state) = ready {
            self.send(state);
        }
        self.schedule_flush();
    }

    fn schedule_flush(self: &Rc<Self>) {
        if self.flush_scheduled.replace(true) {
            return;
        }
        let weak = Rc::downgrade(self);
        glib::timeout_add_local_once(UPDATE_INTERVAL, move || {
            let Some(ui) = weak.upgrade() else { return };
            ui.flush_scheduled.set(false);
            let held = ui.throttle.borrow_mut().flush(Instant::now());
            match held {
                Some(state) => {
                    ui.send(state);
                    ui.schedule_flush();
                }
                // The user has stopped: show what the service actually has.
                None => {
                    glib::spawn_future_local(async move { ui.refresh().await });
                }
            }
        });
    }

    fn send(self: &Rc<Self>, state: LightingState) {
        let Some(bus) = self.bus.borrow().clone() else { return };
        let ui = self.clone();
        glib::spawn_future_local(async move {
            let result =
                bus.lighting.set_state(&state.effect, state.color, state.speed, state.brightness).await;
            if let Err(e) = result {
                ui.toasts.add_toast(adw::Toast::new(&service::error_message(&e)));
                ui.refresh().await;
            }
        });
    }

    fn keys_changed(self: Rc<Self>, map: MapName, keys: Keys) {
        let ready = self.keys_throttle.borrow_mut().push(Instant::now(), (map, keys));
        if let Some(edit) = ready {
            self.send_keys(edit);
        }
        self.schedule_keys_flush();
    }

    fn schedule_keys_flush(self: &Rc<Self>) {
        if self.keys_flush_scheduled.replace(true) {
            return;
        }
        let weak = Rc::downgrade(self);
        glib::timeout_add_local_once(UPDATE_INTERVAL, move || {
            let Some(ui) = weak.upgrade() else { return };
            ui.keys_flush_scheduled.set(false);
            let held = ui.keys_throttle.borrow_mut().flush(Instant::now());
            match held {
                Some(edit) => {
                    ui.send_keys(edit);
                    ui.schedule_keys_flush();
                }
                None => {
                    glib::spawn_future_local(async move { ui.refresh().await });
                }
            }
        });
    }

    fn send_keys(self: &Rc<Self>, (map, keys): (MapName, Keys)) {
        let Some(bus) = self.bus.borrow().clone() else { return };
        let ui = self.clone();
        glib::spawn_future_local(async move {
            if let Err(e) = bus.lighting.set_keys(map.as_str(), &keys).await {
                ui.toasts.add_toast(adw::Toast::new(&service::error_message(&e)));
                ui.refresh().await;
            }
        });
    }
}
