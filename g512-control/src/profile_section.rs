//! The Mouse view's Profile group: which onboard profile is active, and its report
//! rate and DPI steps. Edits are written to the mouse once the user has
//! stopped changing them for [`SAVE_DELAY`], as one request.
//!
//! All five step rows are built once and shown or hidden as the profile
//! needs; rows added later would not reach the accessibility tree.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use adw::prelude::*;
use gtk::glib;

use crate::{
    profile_edit::{DpiRange, MAX_STEPS, ProfileEdit, REPORT_RATES},
    service::Mouse,
};

/// Flash wears with writes: wait for a pause before writing.
const SAVE_DELAY: Duration = Duration::from_secs(2);

type Handler<T> = RefCell<Option<Box<dyn Fn(T)>>>;

struct StepRow {
    row: adw::SpinRow,
    default: gtk::CheckButton,
    remove: gtk::Button,
}

pub struct ProfileSection {
    pub group: adw::PreferencesGroup,
    profile_row: adw::ComboRow,
    profile_names: gtk::StringList,
    rate_row: adw::ComboRow,
    steps: Vec<StepRow>,
    add_row: adw::ButtonRow,
    edit: RefCell<Option<ProfileEdit>>,
    active: Cell<u8>,
    range: Cell<DpiRange>,
    pending: RefCell<Option<glib::SourceId>>,
    syncing: Cell<bool>,
    on_switch: Handler<u8>,
    on_save: Handler<(u8, ProfileEdit)>,
}

impl ProfileSection {
    pub fn new() -> Rc<Self> {
        let profile_names = gtk::StringList::new(&[]);
        let profile_row = adw::ComboRow::builder().title("Profile").model(&profile_names).build();
        let rate_names: Vec<&str> = REPORT_RATES.iter().map(|&(_, label)| label).collect();
        let rate_row =
            adw::ComboRow::builder().title("Report Rate").model(&gtk::StringList::new(&rate_names)).build();
        let add_row = adw::ButtonRow::builder().title("Add Step").start_icon_name("list-add-symbolic").build();
        let group = adw::PreferencesGroup::builder()
            .title("Profile")
            .description("Stored on the mouse: its DPI buttons cycle through these steps")
            .visible(false)
            .build();
        group.add(&profile_row);
        group.add(&rate_row);

        let mut first: Option<gtk::CheckButton> = None;
        let steps: Vec<StepRow> = (1..=MAX_STEPS)
            .map(|n| {
                let row = adw::SpinRow::with_range(100.0, 25600.0, 50.0);
                row.set_title(&format!("Step {n}"));
                row.set_visible(false);
                let default = gtk::CheckButton::new();
                default.set_group(first.as_ref());
                default.set_tooltip_text(Some("Default step"));
                default.update_property(&[gtk::accessible::Property::Label(&format!("Step {n} is the default"))]);
                first.get_or_insert_with(|| default.clone());
                // Not a minus: the spin row has its own − and + buttons.
                let remove = gtk::Button::builder()
                    .icon_name("user-trash-symbolic")
                    .valign(gtk::Align::Center)
                    .css_classes(["flat"])
                    .build();
                remove.update_property(&[gtk::accessible::Property::Label(&format!("Remove Step {n}"))]);
                row.add_prefix(&default);
                row.add_suffix(&remove);
                group.add(&row);
                StepRow { row, default, remove }
            })
            .collect();
        group.add(&add_row);

        let section = Rc::new(ProfileSection {
            group,
            profile_row,
            profile_names,
            rate_row,
            steps,
            add_row,
            edit: RefCell::new(None),
            active: Cell::new(0),
            range: Cell::new(DpiRange { min: 100, max: 25600, step: 50 }),
            pending: RefCell::new(None),
            syncing: Cell::new(false),
            on_switch: RefCell::new(None),
            on_save: RefCell::new(None),
        });
        section.connect_controls();
        section
    }

    /// `on_switch` makes a profile active; `on_save` writes a profile.
    pub fn connect(&self, on_switch: impl Fn(u8) + 'static, on_save: impl Fn((u8, ProfileEdit)) + 'static) {
        *self.on_switch.borrow_mut() = Some(Box::new(on_switch));
        *self.on_save.borrow_mut() = Some(Box::new(on_save));
    }

    fn connect_controls(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.profile_row.connect_selected_notify(move |row| {
            let Some(s) = weak.upgrade().filter(|s| !s.syncing.get()) else { return };
            s.save_now();
            let number = u8::try_from(row.selected() + 1).unwrap_or(1);
            if let Some(f) = s.on_switch.borrow().as_ref() {
                f(number);
            }
        });
        let weak = Rc::downgrade(self);
        self.rate_row.connect_selected_notify(move |row| {
            let Some(s) = weak.upgrade().filter(|s| !s.syncing.get()) else { return };
            let ms = REPORT_RATES.get(row.selected() as usize).map_or(1, |&(ms, _)| ms);
            s.change(|e, _| e.report_rate_ms = ms);
        });
        let weak = Rc::downgrade(self);
        self.add_row.connect_activated(move |_| {
            if let Some(s) = weak.upgrade() {
                s.change(|e, range| {
                    e.add_step(range);
                });
            }
        });
        for (i, step) in self.steps.iter().enumerate() {
            let weak = Rc::downgrade(self);
            step.row.connect_value_notify(move |row| {
                let Some(s) = weak.upgrade().filter(|s| !s.syncing.get()) else { return };
                let value = row.value().round() as u16;
                s.change(|e, range| e.set_dpi(i, value, range));
            });
            let weak = Rc::downgrade(self);
            step.default.connect_toggled(move |b| {
                let Some(s) = weak.upgrade().filter(|s| !s.syncing.get() && b.is_active()) else { return };
                s.change(|e, _| e.set_default(i));
            });
            let weak = Rc::downgrade(self);
            step.remove.connect_clicked(move |_| {
                if let Some(s) = weak.upgrade() {
                    s.change(|e, _| {
                        e.remove_step(i);
                    });
                }
            });
        }
    }

    /// The service's view of the mouse. Hidden without profiles; left alone
    /// while an edit is waiting to be written.
    pub fn show(&self, mouse: Option<&Mouse>) {
        let profiles = mouse.filter(|m| m.state == "connected").map_or(&[][..], |m| m.profiles.as_slice());
        self.group.set_visible(!profiles.is_empty());
        let Some(m) = mouse.filter(|_| !profiles.is_empty()) else { return };
        if self.pending.borrow().is_some() {
            return;
        }
        let Some(profile) = profiles.iter().find(|p| p.number == m.active_profile) else { return };
        let (min, max, step) = m.dpi_range;
        if step > 0 {
            self.range.set(DpiRange { min, max, step });
        }
        self.syncing.set(true);
        if self.profile_names.n_items() as usize != profiles.len() {
            let names: Vec<String> = profiles.iter().map(|p| format!("Profile {}", p.number)).collect();
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            self.profile_names.splice(0, self.profile_names.n_items(), &names);
        }
        self.profile_row.set_selected(u32::from(m.active_profile.saturating_sub(1)));
        self.active.set(m.active_profile);
        *self.edit.borrow_mut() = Some(ProfileEdit {
            report_rate_ms: profile.report_rate_ms,
            default_index: profile.default_index,
            dpis: profile.dpis.clone(),
        });
        self.syncing.set(false);
        self.sync_rows();
    }

    /// Show the edit in the rows: one visible row per step.
    fn sync_rows(&self) {
        let Some(edit) = self.edit.borrow().clone() else { return };
        let range = self.range.get();
        self.syncing.set(true);
        self.rate_row.set_selected(edit.rate_position() as u32);
        for (i, step) in self.steps.iter().enumerate() {
            let Some(&dpi) = edit.dpis.get(i) else {
                step.row.set_visible(false);
                continue;
            };
            step.row.set_range(f64::from(range.min), f64::from(range.max));
            step.row.adjustment().set_step_increment(f64::from(range.step));
            step.row.set_value(f64::from(dpi));
            step.default.set_active(i == usize::from(edit.default_index));
            step.remove.set_sensitive(edit.dpis.len() > 1);
            step.row.set_visible(true);
        }
        self.add_row.set_sensitive(edit.dpis.len() < MAX_STEPS);
        self.syncing.set(false);
    }

    /// Apply a change to the edit, show it, and restart the save timer.
    fn change(self: &Rc<Self>, f: impl FnOnce(&mut ProfileEdit, DpiRange)) {
        let range = self.range.get();
        if let Some(edit) = self.edit.borrow_mut().as_mut() {
            f(edit, range);
        }
        self.sync_rows();
        if let Some(id) = self.pending.borrow_mut().take() {
            id.remove();
        }
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(SAVE_DELAY, move || {
            if let Some(s) = weak.upgrade() {
                s.pending.borrow_mut().take();
                s.send();
            }
        });
        *self.pending.borrow_mut() = Some(id);
    }

    /// Write a waiting edit at once (before switching profiles).
    fn save_now(&self) {
        if let Some(id) = self.pending.borrow_mut().take() {
            id.remove();
            self.send();
        }
    }

    fn send(&self) {
        let Some(edit) = self.edit.borrow().clone() else { return };
        if let Some(f) = self.on_save.borrow().as_ref() {
            f((self.active.get(), edit));
        }
    }
}
