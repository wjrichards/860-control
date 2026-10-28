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
    art,
    button_section::ButtonSection,
    editor::{Editor, MapName},
    keyboard_art::KeyboardArt,
    mouse_art::MouseArt,
    mouse_section::MouseSection,
    profile_section::ProfileSection,
    service::{self, CUSTOM, DeviceState, EffectInfo, Keys, LightingProxy, LightingState, OBJECT_PATH, PerKey, Snapshot},
    throttle::Throttle,
    udev,
};

pub(crate) const UPDATE_INTERVAL: Duration = Duration::from_millis(100);

/// The illustrations sit in flat buttons; focus and hover get the accent ring.
const APP_CSS: &str = "
.device-art { padding: 0; border-radius: 12px; }
.device-art:hover, .device-art:focus-visible { outline: 3px solid alpha(@accent_bg_color, 0.8); outline-offset: 2px; }
.mouse-card { background: #1d1d20; border-radius: 12px; padding: 16px; }
.callout { min-height: 44px; padding: 4px 10px; }
.callout-fixed { border: 1px dashed alpha(white, 0.35); }
";

/// The views' widths, as on the boards: the keyboard card is 694 px wide,
/// and the mouse card 760 px so the callouts fit beside the drawing.
const KEYBOARD_WIDTH: i32 = 694;
const MOUSE_WIDTH: i32 = 760;

/// A view's scrolling column of groups: `AdwPreferencesPage`'s layout, with
/// the view's own width in place of its fixed clamp.
struct Page {
    widget: gtk::ScrolledWindow,
    column: gtk::Box,
}

impl Page {
    fn new(width: i32) -> Self {
        let column = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(24)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(12)
            .margin_end(12)
            .build();
        let clamp =
            adw::Clamp::builder().maximum_size(width).tightening_threshold(width * 3 / 4).child(&column).build();
        let widget = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&clamp)
            .build();
        Page { widget, column }
    }

    fn add(&self, group: &adw::PreferencesGroup) {
        self.column.append(group);
    }
}

/// What covers the whole window: the service can't be reached.
enum Status {
    ServiceMissing,
    NoSessionBus(String),
}

/// A device that isn't usable, shown on its own view's status page.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DeviceProblem {
    Disconnected,
    PermissionDenied,
    Unsupported,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Device {
    Keyboard,
    Mouse,
}

/// A status page with a selectable detail line (for commands).
struct StatusView {
    page: adw::StatusPage,
    detail: gtk::Label,
}

impl StatusView {
    fn new(retry: Option<&gtk::Button>) -> Self {
        let detail = gtk::Label::builder()
            .selectable(true)
            .wrap(true)
            .wrap_mode(gtk::pango::WrapMode::Char)
            .css_classes(["monospace", "card"])
            .build();
        detail.set_margin_start(12);
        detail.set_margin_end(12);
        let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
        content.append(&detail);
        if let Some(retry) = retry {
            content.append(retry);
        }
        StatusView { page: adw::StatusPage::builder().child(&content).build(), detail }
    }

    fn show(&self, icon: &str, title: &str, description: &str, detail: &str) {
        self.page.set_icon_name(Some(icon));
        self.page.set_title(title);
        self.page.set_description(Some(description));
        self.detail.set_label(detail);
        self.detail.set_visible(!detail.is_empty());
    }
}

/// Icon, title, description and detail for a device that isn't usable.
fn problem_text(device: Device, problem: DeviceProblem) -> (&'static str, &'static str, &'static str, String) {
    let (icon, name) = match device {
        Device::Keyboard => ("input-keyboard-symbolic", "keyboard"),
        Device::Mouse => ("input-mouse-symbolic", "mouse"),
    };
    match (device, problem) {
        (Device::Keyboard, DeviceProblem::Disconnected) => (
            icon,
            "Keyboard Not Connected",
            "Plug in the Logitech G512. Your saved lighting is applied when it connects. The Mouse view keeps working meanwhile.",
            String::new(),
        ),
        (Device::Mouse, DeviceProblem::Disconnected) => (
            icon,
            "Mouse Not Connected",
            "Plug in the G502 HERO. Its lighting is applied when it connects, and its profiles and buttons are read from the mouse. The Keyboard view keeps working meanwhile.",
            String::new(),
        ),
        (_, DeviceProblem::PermissionDenied) => (
            "dialog-password-symbolic",
            if device == Device::Keyboard { "No Access to the Keyboard" } else { "No Access to the Mouse" },
            if name == "keyboard" {
                "Install the udev rule once. It covers the keyboard and the mouse. Then unplug and replug the keyboard."
            } else {
                "Install the udev rule once. It covers the keyboard and the mouse. Then unplug and replug the mouse."
            },
            udev::install_command(udev::RULES),
        ),
        (Device::Keyboard, DeviceProblem::Unsupported) => (
            "dialog-warning-symbolic",
            "Keyboard Not Supported",
            "This keyboard's firmware does not offer the lighting effects this app uses.",
            String::new(),
        ),
        (Device::Mouse, DeviceProblem::Unsupported) => (
            "dialog-warning-symbolic",
            "Mouse Not Supported",
            "This mouse's firmware does not offer the lighting this app uses.",
            String::new(),
        ),
    }
}

struct Bus {
    lighting: LightingProxy<'static>,
    props: PropertiesProxy<'static>,
}

pub struct Ui {
    window: adw::ApplicationWindow,
    toasts: adw::ToastOverlay,
    /// The whole window: "status" (service unreachable) or "main".
    stack: gtk::Stack,
    status: StatusView,
    retry: gtk::Button,
    view_stack: adw::ViewStack,
    mouse_view: adw::ViewStackPage,
    view_switcher: adw::ViewSwitcher,
    switcher_bar: adw::ViewSwitcherBar,
    window_title: adw::WindowTitle,
    /// Each view: "status" (its device isn't usable) or "controls".
    keyboard_stack: gtk::Stack,
    keyboard_status: StatusView,
    mouse_stack: gtk::Stack,
    mouse_status: StatusView,
    /// Set while the narrow-window breakpoint applies.
    narrow: Cell<bool>,
    /// The service reports a mouse (it supports one).
    has_mouse: Cell<bool>,
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
    keyboard_art: Rc<KeyboardArt>,
    mouse_art: Rc<MouseArt>,
    /// The Gaming map is on the keyboard because a game is running.
    gaming_shown: Cell<bool>,
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
    breakpoint: RefCell<Option<adw::Breakpoint>>,
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

        let retry = gtk::Button::builder()
            .label("Try Again")
            .halign(gtk::Align::Center)
            .css_classes(["pill", "suggested-action"])
            .build();
        let status = StatusView::new(Some(&retry));

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
            .title("Edit Key Maps…")
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

        if let Some(display) = gdk::Display::default() {
            let css = gtk::CssProvider::new();
            css.load_from_string(APP_CSS);
            gtk::style_context_add_provider_for_display(&display, &css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        }
        let keyboard_art = KeyboardArt::new();
        let art_group = adw::PreferencesGroup::new();
        art_group.add(&keyboard_art.widget);
        let keyboard_page = Page::new(KEYBOARD_WIDTH);
        keyboard_page.add(&art_group);
        keyboard_page.add(&device_group);
        keyboard_page.add(&lighting_group);
        keyboard_page.add(&games_group);
        let mouse_page = Page::new(MOUSE_WIDTH);
        let mouse_art = MouseArt::new();
        let mouse_art_group = adw::PreferencesGroup::new();
        mouse_art_group.add(&mouse_art.widget);
        mouse_page.add(&mouse_art_group);
        let mouse_section = MouseSection::new();
        mouse_page.add(&mouse_section.group);
        let profile_section = ProfileSection::new();
        mouse_page.add(&profile_section.group);
        let button_section = ButtonSection::new();
        mouse_page.add(&button_section.group);

        // Each view swaps its controls for a status page while its device
        // isn't usable; the other view keeps working.
        let keyboard_status = StatusView::new(None);
        let keyboard_stack = gtk::Stack::new();
        keyboard_stack.add_named(&keyboard_status.page, Some("status"));
        keyboard_stack.add_named(&keyboard_page.widget, Some("controls"));
        let mouse_status = StatusView::new(None);
        let mouse_stack = gtk::Stack::new();
        mouse_stack.add_named(&mouse_status.page, Some("status"));
        mouse_stack.add_named(&mouse_page.widget, Some("controls"));

        let view_stack = adw::ViewStack::new();
        view_stack.add_titled_with_icon(&keyboard_stack, Some("keyboard"), "Keyboard", "input-keyboard-symbolic");
        let mouse_view = view_stack.add_titled_with_icon(&mouse_stack, Some("mouse"), "Mouse", "input-mouse-symbolic");
        let view_switcher =
            adw::ViewSwitcher::builder().stack(&view_stack).policy(adw::ViewSwitcherPolicy::Wide).build();
        let switcher_bar = adw::ViewSwitcherBar::builder().stack(&view_stack).build();
        let window_title = adw::WindowTitle::new("G512 Control", "");
        let titles = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        titles.append(&view_switcher);
        titles.append(&window_title);

        let stack = gtk::Stack::new();
        stack.add_named(&status.page, Some("status"));
        stack.add_named(&view_stack, Some("main"));

        let banner = adw::Banner::new("Gaming layout active while a game runs");
        banner.set_visible(false);
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&adw::HeaderBar::builder().title_widget(&titles).build());
        toolbar.add_top_bar(&banner);
        toolbar.set_content(Some(&stack));
        toolbar.add_bottom_bar(&switcher_bar);
        let main_page = adw::NavigationPage::builder().title("G512 Control").tag("main").child(&toolbar).build();

        let editor = Editor::new();
        let nav = adw::NavigationView::new();
        nav.add(&main_page);
        nav.add(&editor.page);
        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&nav));
        window.set_content(Some(&toasts));

        // Narrow windows move the switcher to a bar at the bottom.
        let narrow = adw::Breakpoint::new(adw::BreakpointCondition::parse("max-width: 550sp").expect("valid condition"));
        window.add_breakpoint(narrow.clone());
        let narrow_breakpoint = narrow;

        Ui {
            window,
            toasts,
            stack,
            status,
            retry,
            view_stack,
            mouse_view,
            view_switcher,
            switcher_bar,
            window_title,
            keyboard_stack,
            keyboard_status,
            mouse_stack,
            mouse_status,
            narrow: Cell::new(false),
            has_mouse: Cell::new(false),
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
            keyboard_art,
            mouse_art,
            gaming_shown: Cell::new(false),
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
            breakpoint: RefCell::new(None),
        }
        .with_breakpoint(narrow_breakpoint)
    }

    /// Follow the narrow-window breakpoint to place the switcher.
    fn with_breakpoint(self, breakpoint: adw::Breakpoint) -> Self {
        self.breakpoint.replace(Some(breakpoint));
        self
    }

    /// Show the switcher in the header, in the bottom bar when narrow, or not
    /// at all on a service without a mouse.
    fn update_switchers(&self) {
        let switching = self.has_mouse.get();
        let narrow = self.narrow.get();
        self.view_switcher.set_visible(switching && !narrow);
        self.switcher_bar.set_reveal(switching && narrow);
        self.switcher_bar.set_visible(switching && narrow);
        self.window_title.set_visible(!switching || narrow);
        self.mouse_view.set_visible(switching);
        if !switching {
            self.view_stack.set_visible_child_name("keyboard");
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
        self.keyboard_art.connect_activate(move || {
            if let Some(ui) = w.upgrade() {
                // The map being shown, or Custom when neither is.
                ui.editor.set_target(if ui.gaming_shown.get() { MapName::Gaming } else { MapName::Custom });
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
                match result {
                    Ok(_) => ui.mouse_art.confirm(usize::from(button)),
                    Err(e) => ui.toasts.add_toast(adw::Toast::new(&service::error_message(&e))),
                }
                ui.refresh().await;
            });
        });
        let on_open = weak.clone();
        self.mouse_art.connect_button(move |index| {
            if let Some(ui) = on_open.upgrade() {
                ui.button_section.open_dialog(index);
            }
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
        if let Some(bp) = self.breakpoint.borrow().as_ref() {
            for narrow in [true, false] {
                let w = weak.clone();
                let handler = move |_: &adw::Breakpoint| {
                    if let Some(ui) = w.upgrade() {
                        ui.narrow.set(narrow);
                        ui.update_switchers();
                        ui.mouse_art.set_compact(!narrow);
                    }
                };
                if narrow {
                    bp.connect_apply(handler);
                } else {
                    bp.connect_unapply(handler);
                }
            }
        }
        self.update_switchers();
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
            let name = service::bus_name();
            let lighting = LightingProxy::builder(&conn)
                .destination(name.clone())?
                .cache_properties(zbus::proxy::CacheProperties::No)
                .build()
                .await?;
            let props = PropertiesProxy::builder(&conn).destination(name)?.path(OBJECT_PATH)?.build().await?;
            zbus::Result::Ok(Bus { lighting, props })
        };
        let bus = match bus.await {
            Ok(b) => Rc::new(b),
            Err(e) => return self.show_status(Status::NoSessionBus(e.to_string())),
        };
        *self.bus.borrow_mut() = Some(bus.clone());
        self.refresh().await;
        self.debug_screenshot();
        // For the reduced-motion check: how many effect frames have been drawn.
        if std::env::var_os("G512_CONTROL_TICK_LOG").is_some() {
            glib::timeout_add_seconds_local(2, || {
                let animations = gtk::Settings::default().is_some_and(|s| s.is_gtk_enable_animations());
                let ticks = crate::art_area::TICKS.load(std::sync::atomic::Ordering::Relaxed);
                eprintln!("ticks: {ticks} (animations {animations})");
                glib::ControlFlow::Continue
            });
        }

        // The service restarting, or any property change, re-reads everything.
        if let Ok(dbus) = DBusProxy::new(&conn).await
            && let Ok(mut owners) = dbus.receive_name_owner_changed_with_args(&[(0, service::bus_name().as_str())]).await
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

    /// For checks against the design boards: with `G512_CONTROL_SCREENSHOT`
    /// set to a PNG path, show the view named by `G512_CONTROL_VIEW` (if any)
    /// and render the window to that file once it has drawn. `G512_CONTROL_SIZE`
    /// (e.g. `360x800`) sizes the window first. `G512_CONTROL_FRAMES=N` saves N
    /// frames at about 30 a second instead, to a path with `%04d` in it.
    fn debug_screenshot(&self) {
        let Ok(path) = std::env::var("G512_CONTROL_SCREENSHOT") else { return };
        if let Ok(view) = std::env::var("G512_CONTROL_VIEW") {
            self.view_stack.set_visible_child_name(&view);
        }
        if let Some((w, h)) = std::env::var("G512_CONTROL_SIZE").ok().and_then(|s| {
            let (w, h) = s.split_once('x')?;
            Some((w.parse().ok()?, h.parse().ok()?))
        }) {
            self.window.set_default_size(w, h);
        }
        let frames: u32 = std::env::var("G512_CONTROL_FRAMES").ok().and_then(|n| n.parse().ok()).unwrap_or(1);
        let window = self.window.clone();
        let shot = move |path: &str| {
            let paintable = gtk::WidgetPaintable::new(Some(&window));
            let (w, h) = (window.width(), window.height());
            let snapshot = gtk::Snapshot::new();
            paintable.snapshot(&snapshot, f64::from(w), f64::from(h));
            snapshot.to_node().zip(window.native().and_then(|n| n.renderer())).map(|(node, renderer)| {
                renderer.render_texture(&node, Some(&gtk::graphene::Rect::new(0.0, 0.0, w as f32, h as f32))).save_to_png(path)
            })
        };
        glib::timeout_add_local_once(Duration::from_millis(1500), move || {
            if frames <= 1 {
                eprintln!("screenshot {path}: {:?}", shot(&path));
                return;
            }
            let mut i = 0;
            glib::timeout_add_local(Duration::from_millis(33), move || {
                let saved = shot(&path.replace("%04d", &format!("{i:04}")));
                i += 1;
                if i < frames && matches!(saved, Some(Ok(()))) {
                    return glib::ControlFlow::Continue;
                }
                eprintln!("frames {path}: {i} saved, last {saved:?}");
                glib::ControlFlow::Break
            });
        });
    }

    async fn refresh(&self) {
        let Some(bus) = self.bus.borrow().clone() else { return };
        match service::snapshot(&bus.props).await {
            Err(e) => {
                eprintln!("lighting service unavailable: {e}");
                self.show_status(Status::ServiceMissing);
            }
            Ok(Snapshot { keyboard, mouse }) => {
                self.stack.set_visible_child_name("main");
                self.has_mouse.set(mouse.is_some());
                self.update_switchers();
                self.mouse_section.show(mouse.as_deref());
                self.profile_section.show(mouse.as_deref());
                self.button_section.show(mouse.as_deref());
                if let Some(m) = mouse.as_deref() {
                    let kb = match &keyboard {
                        DeviceState::Connected { state, per_key, lights_on, .. } => {
                            let on = lights_on.unwrap_or(true);
                            let shown = art::shown(on, per_key.game_active, state, &per_key.custom, &per_key.gaming);
                            Some((on, shown, state.speed))
                        }
                        _ => None,
                    };
                    let lights_on = kb.as_ref().is_none_or(|(on, ..)| *on);
                    // A matching mouse runs at the keyboard's speed.
                    let speed = kb.as_ref().filter(|_| m.matching).map_or(m.lighting.speed, |&(.., speed)| speed);
                    let shown = art::mouse_shown(lights_on, m.matching, &m.lighting, kb.map(|(_, s, _)| s));
                    let actions = m
                        .buttons
                        .iter()
                        .find(|(number, _)| *number == m.active_profile)
                        .filter(|_| m.state == "connected")
                        .map(|(_, actions)| actions.as_slice());
                    self.mouse_art.show(&shown, speed, actions);
                }
                match mouse.as_deref().map(|m| m.state.as_str()) {
                    Some("connected") | None => self.mouse_stack.set_visible_child_name("controls"),
                    Some("permission-denied") => self.show_problem(Device::Mouse, DeviceProblem::PermissionDenied),
                    Some("unsupported") => self.show_problem(Device::Mouse, DeviceProblem::Unsupported),
                    Some(_) => self.show_problem(Device::Mouse, DeviceProblem::Disconnected),
                }
                match keyboard {
                    DeviceState::Connected { name, firmware, effects, state, per_key, lights_on } => {
                        self.gaming_shown.set(per_key.game_active);
                        self.keyboard_art.show(
                            lights_on.unwrap_or(true),
                            per_key.game_active,
                            &state,
                            &per_key.custom,
                            &per_key.gaming,
                        );
                        self.show_lights(lights_on);
                        self.show_per_key(&effects, &per_key);
                        self.show_controls(&name, &firmware, effects, &state)
                    }
                    DeviceState::Disconnected => self.show_problem(Device::Keyboard, DeviceProblem::Disconnected),
                    DeviceState::PermissionDenied => self.show_problem(Device::Keyboard, DeviceProblem::PermissionDenied),
                    DeviceState::Unsupported => self.show_problem(Device::Keyboard, DeviceProblem::Unsupported),
                }
            }
        }
    }

    /// Cover the whole window: the service can't be reached.
    fn show_status(&self, status: Status) {
        let (icon, title, description, detail) = match status {
            Status::ServiceMissing => (
                "system-run-symbolic",
                "Lighting Service Not Running",
                "The g512d service controls the keyboard and the mouse. Start it, then try again.",
                "systemctl --user start g512d".to_string(),
            ),
            Status::NoSessionBus(e) => (
                "dialog-error-symbolic",
                "Cannot Reach the Session Bus",
                "The app talks to the lighting service over D-Bus.",
                e,
            ),
        };
        self.status.show(icon, title, description, &detail);
        self.retry.set_visible(true);
        self.stack.set_visible_child_name("status");
    }

    /// Replace one view's controls with a status page for its device.
    fn show_problem(&self, device: Device, problem: DeviceProblem) {
        let (icon, title, description, detail) = problem_text(device, problem);
        let (view, stack) = match device {
            Device::Keyboard => (&self.keyboard_status, &self.keyboard_stack),
            Device::Mouse => (&self.mouse_status, &self.mouse_stack),
        };
        view.show(icon, title, description, &detail);
        stack.set_visible_child_name("status");
    }

    fn show_controls(&self, name: &str, firmware: &str, effects: Vec<EffectInfo>, state: &LightingState) {
        self.device_row.set_title(name);
        self.device_row.set_subtitle(&format!("Firmware {firmware}"));
        self.keyboard_stack.set_visible_child_name("controls");
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
