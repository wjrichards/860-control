//! The Mouse view's Device and Lighting groups: the G502 HERO's status, then
//! Match Keyboard and its own lighting while it isn't matching.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Instant,
};

use adw::prelude::*;
use gtk::glib;

use crate::{
    service::{EffectInfo, LightingState, Mouse},
    throttle::Throttle,
    window::{UPDATE_INTERVAL, from_rgba, labelled_scale, to_rgba},
};

type Handler<T> = RefCell<Option<Box<dyn Fn(T)>>>;

pub struct MouseSection {
    pub group: adw::PreferencesGroup,
    pub lighting_group: adw::PreferencesGroup,
    status_row: adw::ActionRow,
    match_row: adw::SwitchRow,
    effect_row: adw::ComboRow,
    effect_names: gtk::StringList,
    color_row: adw::ActionRow,
    color_button: gtk::ColorDialogButton,
    speed_row: adw::ActionRow,
    speed: gtk::Scale,
    brightness_row: adw::ActionRow,
    brightness: gtk::Scale,
    effects: RefCell<Vec<EffectInfo>>,
    connected: Cell<bool>,
    syncing: Cell<bool>,
    throttle: RefCell<Throttle<LightingState>>,
    flush_scheduled: Cell<bool>,
    on_match: Handler<bool>,
    on_lighting: Handler<LightingState>,
    on_idle: Handler<()>,
}

impl MouseSection {
    pub fn new() -> Rc<Self> {
        let status_row = adw::ActionRow::new();
        status_row.add_prefix(&gtk::Image::from_icon_name("input-mouse-symbolic"));
        let match_row = adw::SwitchRow::builder()
            .title("Match Keyboard")
            .subtitle("Show the keyboard's lighting, or the nearest effect the mouse has")
            .build();

        let effect_names = gtk::StringList::new(&[]);
        let effect_row = adw::ComboRow::builder().title("Effect").model(&effect_names).build();
        let color_button = gtk::ColorDialogButton::new(Some(
            gtk::ColorDialog::builder().title("Mouse Colour").with_alpha(false).build(),
        ));
        color_button.set_valign(gtk::Align::Center);
        color_button.update_property(&[gtk::accessible::Property::Label("Mouse Colour")]);
        let color_row = adw::ActionRow::builder().title("Colour").activatable_widget(&color_button).build();
        color_row.add_suffix(&color_button);
        let speed = labelled_scale("Mouse Speed", 1.0, 100.0);
        let speed_row = adw::ActionRow::builder().title("Speed").build();
        speed_row.add_suffix(&speed);
        let brightness = labelled_scale("Mouse Brightness", 0.0, 100.0);
        let brightness_row = adw::ActionRow::builder().title("Brightness").build();
        brightness_row.add_suffix(&brightness);

        let group = adw::PreferencesGroup::builder().title("Device").visible(false).build();
        group.add(&status_row);
        let lighting_group = adw::PreferencesGroup::builder().title("Lighting").visible(false).build();
        for row in [match_row.upcast_ref::<gtk::Widget>(), effect_row.upcast_ref()] {
            lighting_group.add(row);
        }
        for row in [&color_row, &speed_row, &brightness_row] {
            lighting_group.add(row);
        }

        let section = Rc::new(MouseSection {
            group,
            lighting_group,
            status_row,
            match_row,
            effect_row,
            effect_names,
            color_row,
            color_button,
            speed_row,
            speed,
            brightness_row,
            brightness,
            effects: RefCell::new(Vec::new()),
            connected: Cell::new(false),
            syncing: Cell::new(false),
            throttle: RefCell::new(Throttle::new(UPDATE_INTERVAL)),
            flush_scheduled: Cell::new(false),
            on_match: RefCell::new(None),
            on_lighting: RefCell::new(None),
            on_idle: RefCell::new(None),
        });
        section.connect_controls();
        section
    }

    /// `on_match` and `on_lighting` send to the service; `on_idle` re-reads
    /// it once the user stops changing the controls.
    pub fn connect(
        &self,
        on_match: impl Fn(bool) + 'static,
        on_lighting: impl Fn(LightingState) + 'static,
        on_idle: impl Fn(()) + 'static,
    ) {
        *self.on_match.borrow_mut() = Some(Box::new(on_match));
        *self.on_lighting.borrow_mut() = Some(Box::new(on_lighting));
        *self.on_idle.borrow_mut() = Some(Box::new(on_idle));
    }

    fn connect_controls(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.match_row.connect_active_notify(move |row| {
            let Some(s) = weak.upgrade() else { return };
            if !s.syncing.get() {
                s.update_visibility();
                if let Some(f) = s.on_match.borrow().as_ref() {
                    f(row.is_active());
                }
            }
        });
        let changed = |weak: std::rc::Weak<Self>| {
            move || {
                if let Some(s) = weak.upgrade() {
                    s.user_changed();
                }
            }
        };
        let f = changed(Rc::downgrade(self));
        self.effect_row.connect_selected_notify(move |_| f());
        let f = changed(Rc::downgrade(self));
        self.color_button.connect_rgba_notify(move |_| f());
        let f = changed(Rc::downgrade(self));
        self.speed.connect_value_changed(move |_| f());
        let f = changed(Rc::downgrade(self));
        self.brightness.connect_value_changed(move |_| f());
    }

    /// The service's view of the mouse; hidden when it has no mouse support.
    pub fn show(&self, mouse: Option<&Mouse>) {
        let Some(m) = mouse else {
            self.group.set_visible(false);
            self.lighting_group.set_visible(false);
            return;
        };
        self.group.set_visible(true);
        self.lighting_group.set_visible(true);
        let connected = m.state == "connected";
        self.connected.set(connected);
        let (title, subtitle) = match m.state.as_str() {
            "connected" => (m.name.as_str(), ""),
            "permission-denied" => ("G502 HERO", "No permission: reinstall the udev rule"),
            "unsupported" => ("G502 HERO", "Unsupported"),
            _ => ("Mouse", "Not connected"),
        };
        self.status_row.set_title(title);
        self.status_row.set_subtitle(subtitle);
        // Mid-drag the controls lead; the timer re-syncs once the user stops.
        if self.throttle.borrow().busy(Instant::now()) {
            return;
        }
        self.syncing.set(true);
        self.match_row.set_active(m.matching);
        if *self.effects.borrow() != m.effects {
            let names: Vec<&str> = m.effects.iter().map(|e| e.name.as_str()).collect();
            self.effect_names.splice(0, self.effect_names.n_items(), &names);
            *self.effects.borrow_mut() = m.effects.clone();
        }
        let index = m.effects.iter().position(|e| e.name == m.lighting.effect).unwrap_or(0);
        self.effect_row.set_selected(index as u32);
        self.color_button.set_rgba(&to_rgba(m.lighting.color));
        self.speed.set_value(f64::from(m.lighting.speed));
        self.brightness.set_value(f64::from(m.lighting.brightness));
        self.update_visibility();
        self.syncing.set(false);
    }

    fn selected_effect(&self) -> Option<EffectInfo> {
        self.effects.borrow().get(self.effect_row.selected() as usize).cloned()
    }

    /// The match switch only while connected; the own-setting rows only
    /// while not matching, each when the chosen effect takes it.
    fn update_visibility(&self) {
        let connected = self.connected.get();
        let own = connected && !self.match_row.is_active();
        self.match_row.set_visible(connected);
        self.effect_row.set_visible(own);
        self.brightness_row.set_visible(own);
        let effect = self.selected_effect();
        self.color_row.set_visible(own && effect.as_ref().is_some_and(|e| e.accepts_color));
        self.speed_row.set_visible(own && effect.as_ref().is_some_and(|e| e.accepts_speed));
        if let Some(e) = effect.filter(|e| e.accepts_speed) {
            self.speed.set_range(f64::from(e.min_speed), f64::from(e.max_speed));
        }
    }

    fn user_changed(self: &Rc<Self>) {
        if self.syncing.get() {
            return;
        }
        self.update_visibility();
        let Some(effect) = self.selected_effect() else { return };
        let state = LightingState {
            effect: effect.name,
            color: from_rgba(&self.color_button.rgba()),
            speed: self.speed.value().round() as u8,
            brightness: self.brightness.value().round() as u8,
        };
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
            let Some(s) = weak.upgrade() else { return };
            s.flush_scheduled.set(false);
            let held = s.throttle.borrow_mut().flush(Instant::now());
            match held {
                Some(state) => {
                    s.send(state);
                    s.schedule_flush();
                }
                None => {
                    if let Some(f) = s.on_idle.borrow().as_ref() {
                        f(());
                    }
                }
            }
        });
    }

    fn send(&self, state: LightingState) {
        if let Some(f) = self.on_lighting.borrow().as_ref() {
            f(state);
        }
    }
}
