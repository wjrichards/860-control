//! The main window: a sidebar in the style of GNOME Settings with one
//! section per detected device (Keyboard: Lighting, Key Maps, Games; Mouse:
//! Lighting, DPI & Profiles, Buttons), and the chosen page beside it. In a
//! narrow window the sidebar becomes its own page and pages open with Back.
//! Status pages cover the window when the service can't be reached or no
//! device is connected.

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
    button_actions::describe,
    button_section::ButtonSection,
    editor::{Editor, MapName},
    keyboard_art::KeyboardArt,
    mouse_art::{Badges, HOTSPOTS, MouseArt, Width},
    mouse_section::MouseSection,
    profile_section::ProfileSection,
    service::{
        self, ButtonAction, CUSTOM, DeviceState, EffectInfo, Keys, LightingProxy, LightingState, Mouse, OBJECT_PATH,
        PerKey, Snapshot,
    },
    throttle::Throttle,
    udev,
};

pub(crate) const UPDATE_INTERVAL: Duration = Duration::from_millis(100);

/// The drawings sit on cards that follow the style; the keyboard drawing is
/// a flat button, and focus and hover give it the accent ring.
const APP_CSS: &str = "
.drawing-card { background: #1d1d20; border-radius: 12px; padding: 16px; }
.device-art { padding: 0; border-radius: 12px; }
.device-art:hover, .device-art:focus-visible { outline: 3px solid alpha(@accent_bg_color, 0.8); outline-offset: 2px; }
.callout { min-height: 44px; padding: 4px 10px; }
.callout-fixed { border: 1px dashed alpha(currentColor, 0.35); }
.sidebar-section { margin: 12px 12px 4px 12px; }
.navigation-sidebar > row.current { background-color: alpha(currentColor, 0.1); }
";
/// The light style's card: `color.bg.illustration` and `color.border.illustration`.
const LIGHT_CSS: &str = ".drawing-card { background: #f6f6f7; box-shadow: inset 0 0 0 1px rgb(0 0 6 / 7%); }";

/// The Games page's hint under the Gaming map, and its narrow-window form.
const GAMES_HINT: &str = "Gaming map · click the keyboard to edit it in Key Maps";
const GAMES_HINT_NARROW: &str = "Gaming map · click it to edit in Key Maps";
/// The least scale of the Key Maps keys beside the sidebar or with it folded;
/// a narrow window keeps them full size and scrolls.
const KEY_FLOOR: f64 = 0.6;

/// Badges 6 to 9 (next profile, DPI up, DPI down, DPI shift) on DPI & Profiles.
const DPI_BADGES: &[u8] = &[6, 7, 8, 9];

/// The pages, in sidebar order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PageId {
    KbLighting,
    KbKeyMaps,
    KbGames,
    MsLighting,
    MsDpi,
    MsButtons,
}

impl PageId {
    const ALL: [PageId; 6] =
        [PageId::KbLighting, PageId::KbKeyMaps, PageId::KbGames, PageId::MsLighting, PageId::MsDpi, PageId::MsButtons];

    fn device(self) -> Device {
        match self {
            PageId::KbLighting | PageId::KbKeyMaps | PageId::KbGames => Device::Keyboard,
            _ => Device::Mouse,
        }
    }

    fn tag(self) -> &'static str {
        match self {
            PageId::KbLighting => "keyboard-lighting",
            PageId::KbKeyMaps => "keymaps",
            PageId::KbGames => "games",
            PageId::MsLighting => "mouse-lighting",
            PageId::MsDpi => "dpi",
            PageId::MsButtons => "buttons",
        }
    }

    fn title(self) -> &'static str {
        match self {
            PageId::KbLighting | PageId::MsLighting => "Lighting",
            PageId::KbKeyMaps => "Key Maps",
            PageId::KbGames => "Games",
            PageId::MsDpi => "DPI & Profiles",
            PageId::MsButtons => "Buttons",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            PageId::KbLighting | PageId::MsLighting => "display-brightness-symbolic",
            PageId::KbKeyMaps => "view-grid-symbolic",
            PageId::KbGames => "input-gaming-symbolic",
            PageId::MsDpi => "find-location-symbolic",
            PageId::MsButtons => "input-mouse-symbolic",
        }
    }

    fn index(self) -> usize {
        PageId::ALL.iter().position(|&p| p == self).unwrap_or(0)
    }
}

/// A page beside the sidebar: its scrolling column, swapped for a status
/// page while its device is present but not usable.
struct ContentPage {
    page: adw::NavigationPage,
    toolbar: adw::ToolbarView,
    header: adw::HeaderBar,
    stack: gtk::Stack,
    status: StatusView,
    column: gtk::Box,
}

impl ContentPage {
    fn new(id: PageId) -> Self {
        let column = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(24)
            .margin_top(12)
            .margin_bottom(24)
            .margin_start(32)
            .margin_end(32)
            .vexpand(true)
            .build();
        // The column fills the page; its drawing card takes what the groups
        // leave free, and the page scrolls once nothing is left.
        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&column)
            .build();
        let status = StatusView::new(None);
        let stack = gtk::Stack::new();
        stack.add_named(&status.page, Some("status"));
        stack.add_named(&scrolled, Some("controls"));
        stack.set_visible_child_name("controls");
        let toolbar = adw::ToolbarView::new();
        let header = adw::HeaderBar::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&stack));
        let page = adw::NavigationPage::builder().title(id.title()).tag(id.tag()).child(&toolbar).build();
        ContentPage { page, toolbar, header, stack, status, column }
    }
}

/// A row in the sidebar.
struct SidebarRow {
    id: PageId,
    row: gtk::ListBoxRow,
    /// Shown while the sidebar is its own page: rows open pages with Back.
    chevron: gtk::Image,
}

impl SidebarRow {
    fn new(id: PageId) -> Self {
        let content = gtk::Box::builder().spacing(12).build();
        content.append(&gtk::Image::from_icon_name(id.icon()));
        content.append(&gtk::Label::builder().label(id.title()).xalign(0.0).hexpand(true).build());
        let chevron = gtk::Image::builder().icon_name("go-next-symbolic").visible(false).build();
        content.append(&chevron);
        let row = gtk::ListBoxRow::builder().child(&content).build();
        SidebarRow { id, row, chevron }
    }
}

/// A device's sidebar section: its header, its drawing (narrow only) and its rows.
struct Section {
    widget: gtk::Box,
    list: gtk::ListBox,
    art: gtk::Widget,
}

impl Section {
    fn new(name: &str, model: &str, art: &gtk::Widget, rows: &[&SidebarRow]) -> Self {
        let header = gtk::Box::builder().spacing(6).css_classes(["sidebar-section"]).build();
        header.append(&gtk::Label::builder().label(name).css_classes(["dim-label", "caption-heading"]).build());
        header.append(&gtk::Label::builder().label(format!("· {model}")).css_classes(["dim-label", "caption"]).build());
        header.update_property(&[gtk::accessible::Property::Label(&format!("{name}, {model}"))]);
        // The open page's row is marked by `Ui::open`, not by the list's own
        // selection, which focus and pointer moves would change.
        let list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(["navigation-sidebar"])
            .build();
        for r in rows {
            list.append(&r.row);
        }
        art.set_visible(false);
        art.set_margin_start(6);
        art.set_margin_end(6);
        let widget = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(4).build();
        widget.append(&header);
        widget.append(art);
        widget.append(&list);
        Section { widget, list, art: art.clone() }
    }
}

/// What covers the whole window: the service can't be reached.
enum Status {
    ServiceMissing,
    NoSessionBus(String),
}

/// A device that is present but not usable, shown on its pages' status page.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DeviceProblem {
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
    match (device, problem) {
        (_, DeviceProblem::PermissionDenied) => (
            "dialog-password-symbolic",
            if device == Device::Keyboard { "No Access to the Keyboard" } else { "No Access to the Mouse" },
            if device == Device::Keyboard {
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

/// The DPI & Profiles caption: badges 6 to 9 with each one's current action,
/// one line each beside a narrow drawing, or in a row under a wide one.
fn dpi_caption(actions: &[ButtonAction], legend: bool) -> String {
    let parts: Vec<String> = DPI_BADGES
        .iter()
        .filter_map(|&badge| {
            let spot = HOTSPOTS.iter().find(|s| s.badge == badge)?;
            let action = actions.get(usize::from(spot.button))?;
            let what = describe(action);
            // "6 next profile" in the row; "6  Next profile" as a legend line.
            Some(if legend { format!("{badge}  {what}") } else { format!("{badge} {}", lower_first(&what)) })
        })
        .collect();
    parts.join(if legend { "\n" } else { " · " })
}

/// "Next profile" → "next profile"; "DPI up" and "Ctrl+C" keep their capitals.
/// Only a plain capitalised word (a letter, then lower-case letters) is lowered.
fn lower_first(text: &str) -> String {
    let word = text.split(' ').next().unwrap_or("");
    let mut chars = word.chars();
    let plain = chars.next().is_some_and(char::is_uppercase) && chars.all(char::is_lowercase);
    if !plain {
        return text.to_string();
    }
    let mut chars = text.chars();
    chars.next().map(|first| first.to_lowercase().chain(chars).collect()).unwrap_or_default()
}

struct Bus {
    lighting: LightingProxy<'static>,
    props: PropertiesProxy<'static>,
}

pub struct Ui {
    window: adw::ApplicationWindow,
    toasts: adw::ToastOverlay,
    /// The whole window: "status" (service unreachable), "none" (no device)
    /// or "main".
    stack: gtk::Stack,
    status: StatusView,
    retry: gtk::Button,
    split: adw::NavigationSplitView,
    /// The sidebar page and its content. On Key Maps, between 550 and
    /// 1200 sp, the content moves into `fold` and slides in over the page.
    sidebar_page: adw::NavigationPage,
    sidebar_toolbar: adw::ToolbarView,
    fold: adw::OverlaySplitView,
    /// On the Key Maps header while the sidebar is folded away.
    sidebar_button: gtk::ToggleButton,
    pages: Vec<ContentPage>,
    rows: Vec<SidebarRow>,
    keyboard_section: Section,
    mouse_section_side: Section,
    separator: gtk::Separator,
    /// The page shown beside the sidebar.
    current: Cell<Option<PageId>>,
    /// Whether each device's section was shown after the last refresh (`None`
    /// before the first), so a device leaving is announced once.
    keyboard_present: Cell<Option<bool>>,
    mouse_present: Cell<Option<bool>>,
    width: Cell<Width>,
    keyboard_columns: gtk::Box,
    games_columns: gtk::Box,
    mouse_lighting_row: gtk::Box,
    mouse_dpi_row: gtk::Box,
    css_light: gtk::CssProvider,
    device_row: adw::ActionRow,
    lights_row: adw::SwitchRow,
    effect_row: adw::ComboRow,
    effect_names: gtk::StringList,
    color_row: adw::ActionRow,
    color_button: gtk::ColorDialogButton,
    speed_row: adw::ActionRow,
    speed: gtk::Scale,
    brightness: gtk::Scale,
    editor: Rc<Editor>,
    mouse_section: Rc<MouseSection>,
    profile_section: Rc<ProfileSection>,
    button_section: Rc<ButtonSection>,
    keyboard_art: Rc<KeyboardArt>,
    games_art: Rc<KeyboardArt>,
    list_keyboard_art: Rc<KeyboardArt>,
    mouse_lighting_art: Rc<MouseArt>,
    mouse_dpi_art: Rc<MouseArt>,
    mouse_buttons_art: Rc<MouseArt>,
    list_mouse_art: Rc<MouseArt>,
    /// The active profile's actions, for the DPI & Profiles caption.
    dpi_actions: RefCell<Vec<ButtonAction>>,
    /// The Gaming map is on the keyboard because a game is running.
    gaming_shown: Cell<bool>,
    game_row: adw::SwitchRow,
    now_row: adw::ActionRow,
    edit_gaming_row: adw::ActionRow,
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
    breakpoints: RefCell<Vec<(adw::Breakpoint, Width)>>,
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
            .title("860 Control")
            .default_width(860)
            .default_height(780)
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

        let lights_row = adw::SwitchRow::builder().title("Lights").build();
        let lighting_group = adw::PreferencesGroup::builder().title("Lighting").build();
        lighting_group.add(&lights_row);
        lighting_group.add(&effect_row);
        lighting_group.add(&color_row);
        lighting_group.add(&speed_row);
        lighting_group.add(&brightness_row);

        let game_row = adw::SwitchRow::builder()
            .title("Switch to Gaming Layout While a Game Runs")
            .subtitle("Games must use GameMode, for example with the Steam launch option gamemoderun %command%")
            .build();
        let games_group = adw::PreferencesGroup::builder().title("Games").build();
        games_group.add(&game_row);
        let now_row = adw::ActionRow::builder().title("No game running").build();
        let edit_gaming_row = adw::ActionRow::builder().title("Edit the Gaming Map").activatable(true).build();
        edit_gaming_row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
        let now_group = adw::PreferencesGroup::builder().title("Now").build();
        now_group.add(&now_row);
        now_group.add(&edit_gaming_row);

        let css_light = gtk::CssProvider::new();
        if let Some(display) = gdk::Display::default() {
            let css = gtk::CssProvider::new();
            css.load_from_string(APP_CSS);
            gtk::style_context_add_provider_for_display(&display, &css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
            gtk::style_context_add_provider_for_display(
                &display,
                &css_light,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
            );
        }

        let pages: Vec<ContentPage> = PageId::ALL.iter().map(|&id| ContentPage::new(id)).collect();
        let page = |id: PageId| &pages[id.index()];
        // Two groups side by side when wide, one above the other otherwise.
        let columns = |groups: &[&adw::PreferencesGroup]| {
            let b = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(24).homogeneous(true).build();
            for g in groups {
                g.set_valign(gtk::Align::Start);
                b.append(*g);
            }
            b
        };

        // Keyboard › Lighting
        let keyboard_art = KeyboardArt::new("Click the keyboard to edit Key Maps");
        let keyboard_columns = columns(&[&lighting_group, &device_group]);
        page(PageId::KbLighting).column.append(&keyboard_art.widget);
        page(PageId::KbLighting).column.append(&keyboard_columns);

        // Keyboard › Key Maps
        let editor = Editor::new();
        page(PageId::KbKeyMaps).column.append(&editor.widget);

        // Keyboard › Games
        let games_art = KeyboardArt::new(GAMES_HINT);
        let games_columns = columns(&[&games_group, &now_group]);
        page(PageId::KbGames).column.append(&games_art.widget);
        page(PageId::KbGames).column.append(&games_columns);
        let banner = adw::Banner::new("Gaming layout active while a game runs");
        banner.set_visible(false);
        page(PageId::KbGames).toolbar.add_top_bar(&banner);

        // Mouse pages: the drawing beside the groups when wide, above them otherwise.
        let mouse_row = |art: &gtk::Box, groups: &[&adw::PreferencesGroup]| {
            let column = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(24).hexpand(true).build();
            for g in groups {
                column.append(*g);
            }
            let row = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(24).vexpand(true).build();
            row.append(art);
            row.append(&column);
            row
        };
        let mouse_section = MouseSection::new();
        let mouse_lighting_art = MouseArt::new(Badges::None);
        let mouse_lighting_row =
            mouse_row(&mouse_lighting_art.widget, &[&mouse_section.lighting_group, &mouse_section.group]);
        page(PageId::MsLighting).column.append(&mouse_lighting_row);
        let profile_section = ProfileSection::new();
        let mouse_dpi_art = MouseArt::new(Badges::Only(DPI_BADGES));
        let mouse_dpi_row = mouse_row(&mouse_dpi_art.widget, &[&profile_section.group, &profile_section.steps_group]);
        page(PageId::MsDpi).column.append(&mouse_dpi_row);
        let button_section = ButtonSection::new();
        let mouse_buttons_art = MouseArt::new(Badges::All);
        page(PageId::MsButtons).column.append(&mouse_buttons_art.widget);
        button_section.set_dialog_parent(&page(PageId::MsButtons).page);

        // The sidebar.
        let rows: Vec<SidebarRow> = PageId::ALL.iter().map(|&id| SidebarRow::new(id)).collect();
        let row = |id: PageId| &rows[id.index()];
        let list_keyboard_art = KeyboardArt::new("");
        let list_mouse_art = MouseArt::new(Badges::None);
        list_mouse_art.set_min_fraction(0.3);
        // The keyboard keeps its own height in the list; the mouse takes the rest.
        list_keyboard_art.widget.set_vexpand(false);
        let keyboard_section = Section::new(
            "Keyboard",
            "G512",
            list_keyboard_art.widget.upcast_ref(),
            &[row(PageId::KbLighting), row(PageId::KbKeyMaps), row(PageId::KbGames)],
        );
        let mouse_section_side = Section::new(
            "Mouse",
            "G502 HERO",
            list_mouse_art.widget.upcast_ref(),
            &[row(PageId::MsLighting), row(PageId::MsDpi), row(PageId::MsButtons)],
        );
        let separator = gtk::Separator::builder().margin_top(6).margin_bottom(6).margin_start(12).margin_end(12).build();
        let sidebar_box = gtk::Box::builder().orientation(gtk::Orientation::Vertical).margin_bottom(6).build();
        sidebar_box.append(&keyboard_section.widget);
        sidebar_box.append(&separator);
        sidebar_box.append(&mouse_section_side.widget);
        let sidebar_toolbar = adw::ToolbarView::new();
        sidebar_toolbar.add_top_bar(&adw::HeaderBar::new());
        sidebar_toolbar.set_content(Some(
            &gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).child(&sidebar_box).build(),
        ));
        let sidebar_page =
            adw::NavigationPage::builder().title("860 Control").tag("sidebar").child(&sidebar_toolbar).build();
        let split = adw::NavigationSplitView::builder()
            .sidebar(&sidebar_page)
            .content(&page(PageId::KbLighting).page)
            .min_sidebar_width(220.0)
            .max_sidebar_width(280.0)
            .build();
        // Key Maps needs the window's width for the keyboard: there, the
        // sidebar folds away and this button slides it in over the page.
        let fold = adw::OverlaySplitView::builder()
            .content(&split)
            .collapsed(true)
            .show_sidebar(false)
            .enable_show_gesture(false)
            .min_sidebar_width(220.0)
            .max_sidebar_width(280.0)
            .build();
        let sidebar_button = gtk::ToggleButton::builder()
            .icon_name("sidebar-show-symbolic")
            .tooltip_text("Show Sidebar")
            .visible(false)
            .build();
        sidebar_button.bind_property("active", &fold, "show-sidebar").bidirectional().sync_create().build();
        page(PageId::KbKeyMaps).header.pack_start(&sidebar_button);

        // No device: the window is one status page.
        let none = adw::StatusPage::builder()
            .icon_name("input-keyboard-symbolic")
            .title("No Devices Connected")
            .description(
                "Plug in the Logitech G512 keyboard or the G502 HERO mouse. Each one appears in the sidebar when it connects, and its saved lighting is applied.",
            )
            .build();
        let covered = |child: &gtk::Widget| {
            let t = adw::ToolbarView::new();
            t.add_top_bar(&adw::HeaderBar::new());
            t.set_content(Some(child));
            t
        };
        let stack = gtk::Stack::new();
        stack.add_named(&covered(status.page.upcast_ref()), Some("status"));
        stack.add_named(&covered(none.upcast_ref()), Some("none"));
        stack.add_named(&fold, Some("main"));
        stack.set_visible_child_name("main");
        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&stack));
        window.set_content(Some(&toasts));

        // Narrow: the sidebar becomes its own page (`Ui::update_fold`). Wide:
        // two columns.
        let narrow = adw::Breakpoint::new(adw::BreakpointCondition::parse("max-width: 550sp").expect("valid condition"));
        window.add_breakpoint(narrow.clone());
        // Three callout columns fit beside the sidebar from here.
        let medium = adw::Breakpoint::new(adw::BreakpointCondition::parse("min-width: 720sp").expect("valid condition"));
        window.add_breakpoint(medium.clone());
        let wide = adw::Breakpoint::new(adw::BreakpointCondition::parse("min-width: 1000sp").expect("valid condition"));
        window.add_breakpoint(wide.clone());
        // The whole keyboard fits beside the sidebar from here.
        let full = adw::Breakpoint::new(adw::BreakpointCondition::parse("min-width: 1200sp").expect("valid condition"));
        window.add_breakpoint(full.clone());

        Ui {
            window,
            toasts,
            stack,
            status,
            retry,
            split,
            sidebar_page,
            sidebar_toolbar,
            fold,
            sidebar_button,
            pages,
            rows,
            keyboard_section,
            mouse_section_side,
            separator,
            current: Cell::new(None),
            keyboard_present: Cell::new(None),
            mouse_present: Cell::new(None),
            width: Cell::new(Width::Compact),
            keyboard_columns,
            games_columns,
            mouse_lighting_row,
            mouse_dpi_row,
            css_light,
            device_row,
            lights_row,
            effect_row,
            effect_names,
            color_row,
            color_button,
            speed_row,
            speed,
            brightness,
            editor,
            mouse_section,
            profile_section,
            button_section,
            keyboard_art,
            games_art,
            list_keyboard_art,
            mouse_lighting_art,
            mouse_dpi_art,
            mouse_buttons_art,
            list_mouse_art,
            dpi_actions: RefCell::new(Vec::new()),
            gaming_shown: Cell::new(false),
            game_row,
            now_row,
            edit_gaming_row,
            banner,
            effects: RefCell::new(Vec::new()),
            syncing: Cell::new(false),
            throttle: RefCell::new(Throttle::new(UPDATE_INTERVAL)),
            flush_scheduled: Cell::new(false),
            keys_throttle: RefCell::new(Throttle::new(UPDATE_INTERVAL)),
            keys_flush_scheduled: Cell::new(false),
            bus: RefCell::new(None),
            breakpoints: RefCell::new(vec![
                (narrow, Width::Narrow),
                (medium, Width::Medium),
                (wide, Width::Wide),
                (full, Width::Full),
            ]),
        }
    }

    fn page(&self, id: PageId) -> &ContentPage {
        &self.pages[id.index()]
    }

    fn row(&self, id: PageId) -> &SidebarRow {
        &self.rows[id.index()]
    }

    /// Show `id` beside the sidebar (or open it, when the sidebar is its own
    /// page), and mark its row selected.
    fn open(&self, id: PageId) {
        self.current.set(Some(id));
        for r in &self.rows {
            let current = r.id == id;
            if current {
                r.row.add_css_class("current");
            } else {
                r.row.remove_css_class("current");
            }
            r.row.update_state(&[gtk::accessible::State::Selected(Some(current))]);
        }
        self.split.set_content(Some(&self.page(id).page));
        self.split.set_show_content(true);
        self.update_fold();
    }

    /// Fold the sidebar away on Key Maps between 550 and 1200 sp, so the
    /// keyboard gets the window's width; otherwise put it back beside the
    /// page, or make it its own page in a narrow window.
    fn update_fold(&self) {
        let width = self.width.get();
        let folded = self.current.get() == Some(PageId::KbKeyMaps) && !matches!(width, Width::Narrow | Width::Full);
        let in_fold = self.sidebar_toolbar.parent().as_ref() != Some(self.sidebar_page.upcast_ref());
        let keymaps = self.page(PageId::KbKeyMaps);
        if folded {
            if !in_fold {
                self.sidebar_page.set_child(None::<&gtk::Widget>);
                self.fold.set_sidebar(Some(&self.sidebar_toolbar));
            }
            // The split view shows the page alone; its Back would lead to the
            // now empty sidebar page.
            self.split.set_collapsed(true);
            self.split.set_show_content(true);
            keymaps.header.set_show_back_button(false);
            keymaps.page.set_can_pop(false);
        } else {
            self.fold.set_show_sidebar(false);
            if in_fold {
                self.fold.set_sidebar(None::<&gtk::Widget>);
                self.sidebar_page.set_child(Some(&self.sidebar_toolbar));
            }
            self.split.set_collapsed(width == Width::Narrow);
            keymaps.header.set_show_back_button(true);
            keymaps.page.set_can_pop(true);
        }
        self.fold.set_enable_show_gesture(folded);
        self.sidebar_button.set_visible(folded);
    }

    /// Whether `id`'s row can be chosen: its section is shown and the row is.
    fn available(&self, id: PageId) -> bool {
        let section = match id.device() {
            Device::Keyboard => &self.keyboard_section,
            Device::Mouse => &self.mouse_section_side,
        };
        section.widget.is_visible() && self.row(id).row.is_visible()
    }

    /// Lay the pages out for the window's width.
    fn apply_width(&self, width: Width) {
        self.width.set(width);
        let wide = width.is_wide();
        let narrow = width == Width::Narrow;
        let orientation = if wide { gtk::Orientation::Horizontal } else { gtk::Orientation::Vertical };
        for b in [&self.keyboard_columns, &self.games_columns, &self.mouse_lighting_row, &self.mouse_dpi_row] {
            b.set_orientation(orientation);
        }
        // Beside the groups the mouse card keeps the boards' width.
        for art in [&self.mouse_lighting_art, &self.mouse_dpi_art] {
            art.widget.set_size_request(if wide { 460 } else { -1 }, -1);
            art.widget.set_hexpand(!wide);
        }
        self.mouse_dpi_art.set_legend_beside(narrow);
        self.mouse_dpi_art.set_caption(&dpi_caption(&self.dpi_actions.borrow(), narrow));
        self.mouse_buttons_art.arrange(width);
        self.editor.set_compact(width == Width::Narrow);
        self.editor.set_key_floor(if narrow { 1.0 } else { KEY_FLOOR });
        self.games_art.set_hint(if narrow { GAMES_HINT_NARROW } else { GAMES_HINT });
        let margin = if matches!(width, Width::Narrow | Width::Compact) { 12 } else { 32 };
        for p in &self.pages {
            p.column.set_margin_start(margin);
            p.column.set_margin_end(margin);
        }
        // The list page shows each device's drawing and opens pages with Back.
        self.keyboard_section.art.set_visible(narrow);
        self.mouse_section_side.art.set_visible(narrow);
        self.mouse_section_side.widget.set_vexpand(narrow);
        for r in &self.rows {
            r.chevron.set_visible(narrow);
        }
        self.update_fold();
    }

    /// The light style's card colour follows the system style.
    fn apply_style(&self) {
        let dark = adw::StyleManager::default().is_dark();
        self.css_light.load_from_string(if dark { "" } else { LIGHT_CSS });
    }

    fn connect_controls(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        // The folded sidebar takes focus on its open page's row when it slides
        // in, gives it back to its button when it goes, and Escape closes it.
        let w = weak.clone();
        self.fold.connect_show_sidebar_notify(move |fold| {
            let Some(ui) = w.upgrade() else { return };
            if fold.shows_sidebar() {
                if let Some(id) = ui.current.get() {
                    ui.row(id).row.grab_focus();
                }
            } else if ui.sidebar_button.is_visible() {
                ui.sidebar_button.grab_focus();
            }
        });
        let escape = gtk::EventControllerKey::new();
        let w = weak.clone();
        escape.connect_key_pressed(move |_, key, _, _| {
            let Some(ui) = w.upgrade() else { return glib::Propagation::Proceed };
            if key == gdk::Key::Escape && ui.fold.shows_sidebar() {
                ui.fold.set_show_sidebar(false);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        self.sidebar_toolbar.add_controller(escape);
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
        self.keyboard_art.connect_activate(move || {
            if let Some(ui) = w.upgrade() {
                // The map being shown, or Custom when neither is.
                ui.editor.set_target(if ui.gaming_shown.get() { MapName::Gaming } else { MapName::Custom });
                ui.open(PageId::KbKeyMaps);
            }
        });
        // The Games page's drawing and its "Edit the Gaming Map" row open Gaming.
        let w = weak.clone();
        self.games_art.connect_activate(move || {
            if let Some(ui) = w.upgrade() {
                ui.editor.set_target(MapName::Gaming);
                ui.open(PageId::KbKeyMaps);
            }
        });
        let w = weak.clone();
        self.edit_gaming_row.connect_activated(move |_| {
            if let Some(ui) = w.upgrade() {
                ui.editor.set_target(MapName::Gaming);
                ui.open(PageId::KbKeyMaps);
            }
        });
        let w = weak.clone();
        self.list_keyboard_art.connect_activate(move || {
            if let Some(ui) = w.upgrade() {
                ui.open(PageId::KbLighting);
            }
        });
        for list in [&self.keyboard_section.list, &self.mouse_section_side.list] {
            let w = weak.clone();
            list.connect_row_activated(move |_, row| {
                let Some(ui) = w.upgrade() else { return };
                if let Some(r) = ui.rows.iter().find(|r| &r.row == row) {
                    // Opening Key Maps from the sidebar shows the Custom map while Custom is the effect.
                    if r.id == PageId::KbKeyMaps && ui.selected_effect().is_some_and(|e| e.name == CUSTOM) {
                        ui.editor.set_target(MapName::Custom);
                    }
                    ui.open(r.id);
                }
            });
        }
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
                    Ok(_) => ui.mouse_buttons_art.confirm(usize::from(button)),
                    Err(e) => ui.toasts.add_toast(adw::Toast::new(&service::error_message(&e))),
                }
                ui.refresh().await;
            });
        });
        let on_open = weak.clone();
        self.mouse_buttons_art.connect_button(move |index| {
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
        // Each breakpoint sets its width while it applies (the last matching one
        // wins); from 550 to 720 sp none does, which is Compact.
        for (bp, width) in self.breakpoints.borrow().iter() {
            let (w, width) = (weak.clone(), *width);
            bp.connect_apply(move |_| {
                if let Some(ui) = w.upgrade() {
                    ui.apply_width(width);
                }
            });
            let w = weak.clone();
            bp.connect_unapply(move |_| {
                if let Some(ui) = w.upgrade() {
                    ui.apply_width(Width::Compact);
                }
            });
        }
        self.apply_width(Width::Compact);
        let w = weak.clone();
        adw::StyleManager::default().connect_dark_notify(move |_| {
            if let Some(ui) = w.upgrade() {
                ui.apply_style();
            }
        });
        self.apply_style();
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
        // For the reduced-motion and mapped-drawings checks: how many effect
        // frames have been drawn, and what was showing.
        if std::env::var_os("G512_CONTROL_TICK_LOG").is_some() {
            let w = Rc::downgrade(&self);
            glib::timeout_add_seconds_local(2, move || {
                let Some(ui) = w.upgrade() else { return glib::ControlFlow::Break };
                let animations = gtk::Settings::default().is_some_and(|s| s.is_gtk_enable_animations());
                let ticks = crate::art_area::TICKS.load(std::sync::atomic::Ordering::Relaxed);
                let page = if ui.split.is_collapsed() && !ui.split.shows_content() {
                    "list"
                } else {
                    ui.current.get().map_or("none", PageId::tag)
                };
                let mapped: Vec<&str> = [
                    ("kb", ui.keyboard_art.widget.upcast_ref::<gtk::Widget>()),
                    ("games", ui.games_art.widget.upcast_ref()),
                    ("list-kb", ui.list_keyboard_art.widget.upcast_ref()),
                    ("ms-light", ui.mouse_lighting_art.widget.upcast_ref()),
                    ("ms-dpi", ui.mouse_dpi_art.widget.upcast_ref()),
                    ("ms-buttons", ui.mouse_buttons_art.widget.upcast_ref()),
                    ("list-ms", ui.list_mouse_art.widget.upcast_ref()),
                ]
                .into_iter()
                .filter(|(_, w)| w.is_mapped())
                .map(|(n, _)| n)
                .collect();
                let time = glib::DateTime::now_local().ok().and_then(|t| t.format("%T").ok()).unwrap_or_default();
                eprintln!(
                    "{time} ticks: {ticks} (animations {animations}) page {page} width {:?} {}px mapped [{}]",
                    ui.width.get(),
                    ui.window.width(),
                    mapped.join(" ")
                );
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
    /// set to a PNG path, open the page tagged `G512_CONTROL_VIEW` (if any;
    /// `list` shows the narrow sidebar list) and render the window to that
    /// file once it has drawn. `G512_CONTROL_SIZE` (e.g. `360x800`) sizes the
    /// window first, and `G512_CONTROL_STYLE=light` forces the light style.
    /// `G512_CONTROL_FRAMES=N` saves N frames at about 30 a second instead, to
    /// a path with `%04d` in it. `G512_CONTROL_SIDEBAR=1` slides the folded
    /// sidebar in (Key Maps).
    fn debug_screenshot(&self) {
        let Ok(path) = std::env::var("G512_CONTROL_SCREENSHOT") else { return };
        if std::env::var("G512_CONTROL_STYLE").is_ok_and(|s| s == "light") {
            adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceLight);
        }
        if let Ok(view) = std::env::var("G512_CONTROL_VIEW") {
            match PageId::ALL.iter().find(|p| p.tag() == view) {
                Some(&id) => self.open(id),
                None if view == "list" => self.split.set_show_content(false),
                None => eprintln!("no page tagged {view:?}"),
            }
        }
        if let Some((w, h)) = std::env::var("G512_CONTROL_SIZE").ok().and_then(|s| {
            let (w, h) = s.split_once('x')?;
            Some((w.parse().ok()?, h.parse().ok()?))
        }) {
            self.window.set_default_size(w, h);
        }
        if std::env::var_os("G512_CONTROL_SIDEBAR").is_some() {
            let fold = self.fold.clone();
            glib::timeout_add_local_once(Duration::from_millis(500), move || fold.set_show_sidebar(true));
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
                // A frame with nothing to render (mid page switch) is skipped, not the end.
                if i < frames && !matches!(saved, Some(Err(_))) {
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
                self.mouse_section.show(mouse.as_deref());
                self.profile_section.show(mouse.as_deref());
                self.button_section.show(mouse.as_deref());
                if let Some(m) = mouse.as_deref() {
                    self.show_mouse_art(&keyboard, m);
                }
                let mouse_state = mouse.as_deref().map(|m| m.state.as_str());
                let mouse_present = matches!(mouse_state, Some("connected" | "permission-denied" | "unsupported"));
                match mouse_state {
                    Some("connected") => {
                        self.show_device_controls(Device::Mouse);
                        // Without profiles or buttons there's nothing for these pages.
                        self.row(PageId::MsDpi).row.set_visible(self.profile_section.group.is_visible());
                        self.row(PageId::MsButtons).row.set_visible(self.button_section.has_buttons());
                    }
                    Some("permission-denied") => self.show_problem(Device::Mouse, DeviceProblem::PermissionDenied),
                    Some("unsupported") => self.show_problem(Device::Mouse, DeviceProblem::Unsupported),
                    _ => {}
                }
                if !matches!(mouse_state, Some("connected")) {
                    self.row(PageId::MsDpi).row.set_visible(true);
                    self.row(PageId::MsButtons).row.set_visible(true);
                }
                let keyboard_present = !matches!(keyboard, DeviceState::Disconnected);
                match keyboard {
                    DeviceState::Connected { name, firmware, effects, state, per_key, lights_on } => {
                        self.gaming_shown.set(per_key.game_active);
                        let on = lights_on.unwrap_or(true);
                        for art in [&self.keyboard_art, &self.list_keyboard_art] {
                            art.show(on, per_key.game_active, &state, &per_key.custom, &per_key.gaming);
                        }
                        self.games_art.show_gaming(&per_key.gaming);
                        self.show_lights(lights_on);
                        self.show_per_key(&effects, &per_key);
                        self.show_controls(&name, &firmware, effects, &state)
                    }
                    DeviceState::PermissionDenied => self.show_problem(Device::Keyboard, DeviceProblem::PermissionDenied),
                    DeviceState::Unsupported => self.show_problem(Device::Keyboard, DeviceProblem::Unsupported),
                    DeviceState::Disconnected => {}
                }
                self.show_sections(keyboard_present, mouse_present);
            }
        }
    }

    /// The mouse drawings: what the mouse shows, and the active profile's actions.
    fn show_mouse_art(&self, keyboard: &DeviceState, m: &Mouse) {
        let kb = match keyboard {
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
        let profile = m
            .buttons
            .iter()
            .find(|(number, _)| *number == m.active_profile)
            .filter(|_| m.state == "connected")
            .map(|(number, actions)| (*number, actions.as_slice()));
        self.mouse_lighting_art.show(&shown, speed, None);
        self.list_mouse_art.show(&shown, speed, None);
        self.mouse_dpi_art.show(&shown, speed, profile);
        self.mouse_buttons_art.show(&shown, speed, profile);
        *self.dpi_actions.borrow_mut() = profile.map(|(_, a)| a.to_vec()).unwrap_or_default();
        self.mouse_dpi_art.set_caption(&dpi_caption(&self.dpi_actions.borrow(), self.width.get() == Width::Narrow));
    }

    /// Show the sections of the devices present, announce one that left, and
    /// move off a page that went with it. With neither device, the window
    /// shows "No Devices Connected".
    fn show_sections(&self, keyboard: bool, mouse: bool) {
        self.keyboard_section.widget.set_visible(keyboard);
        self.mouse_section_side.widget.set_visible(mouse);
        self.separator.set_visible(keyboard && mouse);
        for (was, now, message) in [
            (&self.keyboard_present, keyboard, "Keyboard disconnected"),
            (&self.mouse_present, mouse, "Mouse disconnected"),
        ] {
            if was.replace(Some(now)) == Some(true) && !now {
                self.toasts.add_toast(adw::Toast::new(message));
            }
        }
        if !keyboard && !mouse {
            self.stack.set_visible_child_name("none");
            return;
        }
        self.stack.set_visible_child_name("main");
        let current = self.current.get().filter(|&id| self.available(id));
        if current.is_none() {
            // The first page shown: Keyboard › Lighting, or the mouse's when
            // only the mouse is here. In a narrow window the list stays up.
            let showing = self.split.shows_content() || self.current.get().is_none();
            if let Some(&id) = PageId::ALL.iter().find(|&&id| self.available(id)) {
                let collapsed = self.width.get() == Width::Narrow;
                self.open(id);
                if collapsed && !showing {
                    self.split.set_show_content(false);
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

    /// A device that is present but not usable: each of its pages shows why.
    fn show_problem(&self, device: Device, problem: DeviceProblem) {
        let (icon, title, description, detail) = problem_text(device, problem);
        for id in PageId::ALL.into_iter().filter(|id| id.device() == device) {
            let page = self.page(id);
            page.status.show(icon, title, description, &detail);
            page.stack.set_visible_child_name("status");
        }
    }

    /// A usable device: its pages show their controls.
    fn show_device_controls(&self, device: Device) {
        for id in PageId::ALL.into_iter().filter(|id| id.device() == device) {
            self.page(id).stack.set_visible_child_name("controls");
        }
    }

    fn show_controls(&self, name: &str, firmware: &str, effects: Vec<EffectInfo>, state: &LightingState) {
        self.device_row.set_title(name);
        self.device_row.set_subtitle(&format!("Firmware {firmware}"));
        self.show_device_controls(Device::Keyboard);
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

    /// Key Maps and Games, the game switch, the Now row and the banner. The
    /// pages are hidden on a service or keyboard without the Custom effect.
    fn show_per_key(&self, effects: &[EffectInfo], per_key: &PerKey) {
        let supported = effects.iter().any(|e| e.name == CUSTOM);
        self.row(PageId::KbKeyMaps).row.set_visible(supported);
        self.row(PageId::KbGames).row.set_visible(supported);
        // Hidden as well as unrevealed, so screen readers skip it.
        self.banner.set_visible(per_key.game_active);
        self.banner.set_revealed(per_key.game_active);
        if per_key.game_active {
            self.now_row.set_title("Game running");
            self.now_row.set_subtitle("The keyboard shows the Gaming map");
        } else {
            self.now_row.set_title("No game running");
            self.now_row.set_subtitle("The keyboard shows its normal lighting");
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn action(kind: &str, value: u16, modifiers: u8) -> ButtonAction {
        ButtonAction { kind: kind.into(), value, modifiers }
    }

    /// The G502's factory actions, by service index (as the fake service has them).
    fn factory() -> Vec<ButtonAction> {
        [("mouse", 1), ("mouse", 2), ("mouse", 4), ("mouse", 8), ("mouse", 16), ("special", 7), ("special", 4)]
            .into_iter()
            .chain([("special", 3), ("special", 10), ("special", 2), ("special", 1)])
            .map(|(kind, value)| action(kind, value, 0))
            .collect()
    }

    #[test]
    fn dpi_caption_follows_the_buttons() {
        let mut actions = factory();
        assert_eq!(dpi_caption(&actions, false), "6 next profile · 7 DPI up · 8 DPI down · 9 DPI shift (sniper)");
        assert_eq!(dpi_caption(&actions, true), "6  Next profile\n7  DPI up\n8  DPI down\n9  DPI shift (sniper)");
        // G9 (badge 6) remapped to Ctrl+C.
        let g9 = usize::from(HOTSPOTS.iter().find(|s| s.badge == 6).unwrap().button);
        actions[g9] = action("key", 0x06, 1);
        assert!(dpi_caption(&actions, false).starts_with("6 Ctrl+C · 7 DPI up"));
        assert_eq!(dpi_caption(&[], false), "", "no buttons known");
    }

    #[test]
    fn lower_first_keeps_acronyms() {
        assert_eq!(lower_first("Next profile"), "next profile");
        assert_eq!(lower_first("DPI up"), "DPI up");
        assert_eq!(lower_first("Ctrl+C"), "Ctrl+C");
    }

    #[test]
    fn every_page_has_its_own_tag_and_row() {
        let tags: std::collections::HashSet<_> = PageId::ALL.iter().map(|p| p.tag()).collect();
        assert_eq!(tags.len(), PageId::ALL.len());
        for (i, id) in PageId::ALL.iter().enumerate() {
            assert_eq!(id.index(), i);
        }
    }
}
