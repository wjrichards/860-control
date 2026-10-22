//! The main window: status pages for problems, otherwise the lighting controls.

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
    service::{self, BUS_NAME, DeviceState, EffectInfo, LightingProxy, LightingState, OBJECT_PATH},
    throttle::Throttle,
};

const UPDATE_INTERVAL: Duration = Duration::from_millis(100);
const UDEV_RULES: &str = include_str!("../../packaging/70-g512-control.rules");

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
    effect_row: adw::ComboRow,
    effect_names: gtk::StringList,
    color_row: adw::ActionRow,
    color_button: gtk::ColorDialogButton,
    speed_row: adw::ActionRow,
    speed: gtk::Scale,
    brightness: gtk::Scale,
    effects: RefCell<Vec<EffectInfo>>,
    /// Set while the controls are updated from the service, so their change
    /// handlers do not send that state back.
    syncing: Cell<bool>,
    throttle: RefCell<Throttle<LightingState>>,
    flush_scheduled: Cell<bool>,
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

fn labelled_scale(label: &str, min: f64, max: f64) -> gtk::Scale {
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, min, max, 1.0);
    scale.set_width_request(200);
    scale.set_valign(gtk::Align::Center);
    scale.set_draw_value(true);
    scale.set_value_pos(gtk::PositionType::Left);
    scale.set_digits(0);
    scale.update_property(&[gtk::accessible::Property::Label(label)]);
    scale
}

fn to_rgba((r, g, b): (u8, u8, u8)) -> gdk::RGBA {
    gdk::RGBA::new(f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0, 1.0)
}

fn from_rgba(c: &gdk::RGBA) -> (u8, u8, u8) {
    let ch = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    (ch(c.red()), ch(c.green()), ch(c.blue()))
}

impl Ui {
    fn build(app: &adw::Application) -> Self {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("G512 Control")
            .default_width(460)
            .default_height(560)
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

        let lighting_group = adw::PreferencesGroup::builder().title("Lighting").build();
        lighting_group.add(&effect_row);
        lighting_group.add(&color_row);
        lighting_group.add(&speed_row);
        lighting_group.add(&brightness_row);

        let page = adw::PreferencesPage::new();
        page.add(&device_group);
        page.add(&lighting_group);

        let stack = gtk::Stack::new();
        stack.add_named(&status, Some("status"));
        stack.add_named(&page, Some("main"));

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&adw::HeaderBar::new());
        toolbar.set_content(Some(&stack));
        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&toolbar));
        window.set_content(Some(&toasts));

        Ui {
            window,
            toasts,
            stack,
            status,
            status_detail,
            retry,
            device_row,
            effect_row,
            effect_names,
            color_row,
            color_button,
            speed_row,
            speed,
            brightness,
            effects: RefCell::new(Vec::new()),
            syncing: Cell::new(false),
            throttle: RefCell::new(Throttle::new(UPDATE_INTERVAL)),
            flush_scheduled: Cell::new(false),
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
            Ok(DeviceState::Connected { name, firmware, effects, state }) => {
                self.show_controls(&name, &firmware, effects, &state)
            }
            Ok(DeviceState::Disconnected) => self.show_status(Status::Disconnected),
            Ok(DeviceState::PermissionDenied) => self.show_status(Status::PermissionDenied),
            Ok(DeviceState::Unsupported) => self.show_status(Status::Unsupported),
        }
    }

    fn show_status(&self, status: Status) {
        let rule = UDEV_RULES.lines().rfind(|l| !l.trim().is_empty() && !l.starts_with('#')).unwrap_or_default();
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
                format!(
                    "echo '{rule}' | sudo tee /etc/udev/rules.d/70-g512-control.rules\nsudo udevadm control --reload && sudo udevadm trigger --subsystem-match=hidraw"
                ),
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
}
