//! The Mouse Buttons group: what each button of the active profile does,
//! and a dialog to change it. Left click can't be changed.
//!
//! All rows are built once and shown or hidden as the mouse needs; rows
//! added later would not reach the accessibility tree.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use adw::prelude::*;
use gtk::{gdk, glib};

use crate::{
    button_actions::{
        MEDIA_KEYS, MOUSE_BUTTONS, SPECIALS, button_name, describe, is_modifier, modifier_bits, shortcut_label, usage,
    },
    service::{ButtonAction, Mouse},
};

/// Room for the most buttons a profile can hold.
const MAX_BUTTONS: usize = 16;

type Handler<T> = RefCell<Option<Box<dyn Fn(T)>>>;

struct ButtonRow {
    row: adw::ActionRow,
    /// `None` for left click.
    change: Option<gtk::Button>,
}

pub struct ButtonSection {
    pub group: adw::PreferencesGroup,
    rows: Vec<ButtonRow>,
    actions: RefCell<Vec<ButtonAction>>,
    profile: Cell<u8>,
    on_set: Handler<(u8, u8, ButtonAction)>,
}

impl ButtonSection {
    pub fn new() -> Rc<Self> {
        let group = adw::PreferencesGroup::builder().title("Mouse Buttons").visible(false).build();
        let rows: Vec<ButtonRow> = (0..MAX_BUTTONS)
            .map(|i| {
                let row = adw::ActionRow::builder().title(button_name(i)).visible(false).build();
                let change = (i > 0).then(|| {
                    let b = gtk::Button::builder().label("Change").valign(gtk::Align::Center).build();
                    b.update_property(&[gtk::accessible::Property::Label(&format!("Change {}", button_name(i)))]);
                    row.add_suffix(&b);
                    row.set_activatable_widget(Some(&b));
                    b
                });
                group.add(&row);
                ButtonRow { row, change }
            })
            .collect();
        let section = Rc::new(ButtonSection {
            group,
            rows,
            actions: RefCell::new(Vec::new()),
            profile: Cell::new(0),
            on_set: RefCell::new(None),
        });
        for (i, r) in section.rows.iter().enumerate() {
            let Some(change) = &r.change else { continue };
            let weak = Rc::downgrade(&section);
            change.connect_clicked(move |_| {
                if let Some(s) = weak.upgrade() {
                    s.open_dialog(i);
                }
            });
        }
        section
    }

    /// `on_set` changes one button: (profile, button, action).
    pub fn connect(&self, on_set: impl Fn((u8, u8, ButtonAction)) + 'static) {
        *self.on_set.borrow_mut() = Some(Box::new(on_set));
    }

    /// The service's view of the mouse: the active profile's buttons.
    pub fn show(&self, mouse: Option<&Mouse>) {
        let active = mouse
            .filter(|m| m.state == "connected")
            .and_then(|m| m.buttons.iter().find(|(number, _)| *number == m.active_profile));
        self.group.set_visible(active.is_some());
        let Some((number, actions)) = active else { return };
        self.group.set_description(Some(&format!("Profile {number}, stored on the mouse")));
        self.profile.set(*number);
        for (i, r) in self.rows.iter().enumerate() {
            match actions.get(i) {
                Some(a) => {
                    r.row.set_subtitle(&describe(a));
                    r.row.set_visible(true);
                }
                None => r.row.set_visible(false),
            }
        }
        *self.actions.borrow_mut() = actions.clone();
    }

    fn open_dialog(self: &Rc<Self>, index: usize) {
        let Some(current) = self.actions.borrow().get(index).cloned() else { return };
        let dialog = ChangeDialog::new(&button_name(index), &current);
        let weak = Rc::downgrade(self);
        let profile = self.profile.get();
        dialog.connect_apply(move |action| {
            let Some(s) = weak.upgrade() else { return };
            if let Some(f) = s.on_set.borrow().as_ref() {
                f((profile, index as u8, action));
            }
        });
        dialog.dialog.present(Some(&self.group));
    }
}

/// One choice in the dialog: a fixed action, or the captured shortcut.
enum Choice {
    Fixed(ButtonAction),
    Shortcut,
}

struct ChangeDialog {
    dialog: adw::Dialog,
    apply: gtk::Button,
    choices: Vec<(gtk::CheckButton, Choice)>,
    shortcut_row: adw::ActionRow,
    shortcut_check: gtk::CheckButton,
    /// (modifier bits, HID usage).
    shortcut: Cell<Option<(u8, u8)>>,
    capturing: Cell<bool>,
    on_apply: Handler<ButtonAction>,
}

fn fixed(kind: &str, value: u16) -> ButtonAction {
    ButtonAction { kind: kind.into(), value, modifiers: 0 }
}

impl ChangeDialog {
    fn new(name: &str, current: &ButtonAction) -> Rc<Self> {
        let page = adw::PreferencesPage::new();
        let mut choices = Vec::new();
        let mut first: Option<gtk::CheckButton> = None;
        let mut option = |group: &adw::PreferencesGroup, title: &str, choice: Choice| {
            let check = gtk::CheckButton::new();
            check.set_group(first.as_ref());
            first.get_or_insert_with(|| check.clone());
            let row = adw::ActionRow::builder().title(title).activatable_widget(&check).build();
            row.add_prefix(&check);
            group.add(&row);
            choices.push((check.clone(), choice));
            (row, check)
        };

        let mouse = adw::PreferencesGroup::builder().title("Mouse Button").build();
        for (value, label) in MOUSE_BUTTONS {
            option(&mouse, label, Choice::Fixed(fixed("mouse", value)));
        }
        let keyboard = adw::PreferencesGroup::builder().title("Keyboard").build();
        let (shortcut_row, shortcut_check) = option(&keyboard, "Shortcut", Choice::Shortcut);
        let set = gtk::Button::builder().label("Set Shortcut").valign(gtk::Align::Center).build();
        shortcut_row.add_suffix(&set);
        let media = adw::PreferencesGroup::builder().title("Media Key").build();
        for (value, label) in MEDIA_KEYS {
            option(&media, label, Choice::Fixed(fixed("media", value)));
        }
        let function = adw::PreferencesGroup::builder().title("Mouse Function").build();
        for (value, label) in SPECIALS {
            option(&function, label, Choice::Fixed(fixed("special", value)));
        }
        let other = adw::PreferencesGroup::builder().title("Other").build();
        option(&other, "Disabled", Choice::Fixed(fixed("disabled", 0)));
        for g in [&mouse, &keyboard, &media, &function, &other] {
            page.add(g);
        }

        let cancel = gtk::Button::with_label("Cancel");
        let apply = gtk::Button::builder().label("Apply").css_classes(["suggested-action"]).sensitive(false).build();
        let header = adw::HeaderBar::builder().show_start_title_buttons(false).show_end_title_buttons(false).build();
        header.pack_start(&cancel);
        header.pack_end(&apply);
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&page));
        let dialog = adw::Dialog::builder()
            .title(name)
            .content_width(420)
            .content_height(600)
            .child(&toolbar)
            .build();

        let d = Rc::new(ChangeDialog {
            dialog,
            apply,
            choices,
            shortcut_row,
            shortcut_check,
            shortcut: Cell::new(None),
            capturing: Cell::new(false),
            on_apply: RefCell::new(None),
        });
        d.select(current);

        let weak = Rc::downgrade(&d);
        cancel.connect_clicked(move |_| {
            if let Some(d) = weak.upgrade() {
                d.dialog.close();
            }
        });
        let weak = Rc::downgrade(&d);
        d.apply.connect_clicked(move |_| {
            let Some(d) = weak.upgrade() else { return };
            if let Some(action) = d.chosen() {
                if let Some(f) = d.on_apply.borrow().as_ref() {
                    f(action);
                }
                d.dialog.close();
            }
        });
        for (check, _) in &d.choices {
            let weak = Rc::downgrade(&d);
            check.connect_toggled(move |_| {
                if let Some(d) = weak.upgrade() {
                    d.update_apply();
                }
            });
        }
        let weak = Rc::downgrade(&d);
        set.connect_clicked(move |_| {
            if let Some(d) = weak.upgrade() {
                d.capturing.set(true);
                d.shortcut_row.set_subtitle("Press the keys now (Esc cancels)");
                d.dialog.grab_focus();
            }
        });
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(&d);
        keys.connect_key_pressed(move |ctrl, keyval, keycode, state| {
            let Some(d) = weak.upgrade().filter(|d| d.capturing.get()) else { return glib::Propagation::Proceed };
            d.key_pressed(ctrl, keyval, keycode, state);
            glib::Propagation::Stop
        });
        d.dialog.add_controller(keys);
        // The dialog's handlers hold weak references; keep it alive while open.
        let keep = d.clone();
        d.dialog.connect_closed(move |_| {
            let _ = &keep;
        });
        d
    }

    fn connect_apply(&self, f: impl Fn(ButtonAction) + 'static) {
        *self.on_apply.borrow_mut() = Some(Box::new(f));
    }

    /// Start from what the button does now.
    fn select(&self, current: &ButtonAction) {
        if current.kind == "key" {
            self.shortcut.set(Some((current.modifiers, current.value as u8)));
            self.shortcut_row.set_subtitle(&describe(current));
            self.shortcut_check.set_active(true);
        } else {
            self.shortcut_row.set_subtitle("Not set");
            let found = self.choices.iter().find(|(_, c)| matches!(c, Choice::Fixed(a) if a == current));
            if let Some((check, _)) = found {
                check.set_active(true);
            }
        }
        self.update_apply();
    }

    fn chosen(&self) -> Option<ButtonAction> {
        let (_, choice) = self.choices.iter().find(|(check, _)| check.is_active())?;
        match choice {
            Choice::Fixed(a) => Some(a.clone()),
            Choice::Shortcut => {
                let (modifiers, usage) = self.shortcut.get()?;
                Some(ButtonAction { kind: "key".into(), value: u16::from(usage), modifiers })
            }
        }
    }

    fn update_apply(&self) {
        self.apply.set_sensitive(self.chosen().is_some());
    }

    fn key_pressed(&self, ctrl: &gtk::EventControllerKey, keyval: gdk::Key, keycode: u32, state: gdk::ModifierType) {
        if is_modifier(keyval) {
            return;
        }
        let modifiers = modifier_bits(state);
        if keyval == gdk::Key::Escape && modifiers == 0 {
            self.capturing.set(false);
            match self.shortcut.get() {
                Some((m, u)) => self.shortcut_row.set_subtitle(&shortcut_label(m, u)),
                None => self.shortcut_row.set_subtitle("Not set"),
            }
            return;
        }
        // The key itself, without Shift: Shift+1 is 1, not !.
        let base = ctrl
            .current_event()
            .and_then(|e| e.downcast::<gdk::KeyEvent>().ok())
            .and_then(|e| e.display()?.translate_key(keycode, gdk::ModifierType::empty(), e.layout() as i32))
            .map_or(keyval, |(k, ..)| k);
        let Some(usage) = usage(base) else {
            self.shortcut_row.set_subtitle("The mouse can't send that key; try another (Esc cancels)");
            return;
        };
        self.capturing.set(false);
        self.shortcut.set(Some((modifiers, usage)));
        self.shortcut_row.set_subtitle(&shortcut_label(modifiers, usage));
        self.shortcut_check.set_active(true);
        self.update_apply();
    }
}
