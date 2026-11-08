//! The Change dialog: pick what one mouse button does, from mouse buttons,
//! a captured keyboard shortcut, a macro stored on the mouse (board "Change
//! dialog · G9 · After"), media keys, mouse functions or Disabled.
//! The button rows, the callouts and the mouse drawing all open it.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use adw::prelude::*;
use gtk::{gdk, glib};

use crate::{
    button_actions::{
        MEDIA_KEYS, MOUSE_BUTTONS, SPECIALS, describe, is_modifier, modifier_bits, shortcut_label, unshifted, usage,
    },
    service::ButtonAction,
};

pub type Handler<T> = RefCell<Option<Box<dyn Fn(T)>>>;

/// One choice in the dialog: a fixed action, the captured shortcut, or the
/// macro picked in the drop-down.
enum Choice {
    Fixed(ButtonAction),
    Shortcut,
    Macro,
}

/// The macros a button can play: (sector, title), in the picker's order.
pub type MacroChoices = Vec<(u8, String)>;

pub struct ChangeDialog {
    pub dialog: adw::Dialog,
    apply: gtk::Button,
    choices: Vec<(gtk::CheckButton, Choice)>,
    shortcut_row: adw::ActionRow,
    shortcut_check: gtk::CheckButton,
    /// (modifier bits, HID usage).
    shortcut: Cell<Option<(u8, u8)>>,
    capturing: Cell<bool>,
    macro_check: Option<gtk::CheckButton>,
    macro_picker: Option<gtk::DropDown>,
    macro_titles: gtk::StringList,
    /// Sectors in the picker's order; "New Macro…" comes after them.
    macro_sectors: RefCell<Vec<u8>>,
    /// The picker's last real choice, to go back to after "New Macro…".
    macro_selected: Cell<u32>,
    on_new_macro: Handler<()>,
    on_apply: Handler<ButtonAction>,
    /// What the button does now: applying it again would only wear the flash.
    current: ButtonAction,
}

fn fixed(kind: &str, value: u16) -> ButtonAction {
    ButtonAction { kind: kind.into(), value, modifiers: 0 }
}

impl ChangeDialog {
    /// `macros` is `None` on a service without macros: no Macro group then.
    pub fn new(name: &str, current: &ButtonAction, macros: Option<MacroChoices>) -> Rc<Self> {
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
        let macro_titles = gtk::StringList::new(&[]);
        let macro_group = adw::PreferencesGroup::builder().title("Macro").build();
        let (macro_check, macro_picker) = if macros.is_some() {
            let (row, check) = option(&macro_group, "Macro", Choice::Macro);
            row.set_subtitle("Stored on the mouse");
            let picker = gtk::DropDown::builder().model(&macro_titles).valign(gtk::Align::Center).build();
            picker.update_property(&[gtk::accessible::Property::Label("Macro to play")]);
            row.add_suffix(&picker);
            (Some(check), Some(picker))
        } else {
            (None, None)
        };
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
        for g in [&mouse, &keyboard] {
            page.add(g);
        }
        if macros.is_some() {
            page.add(&macro_group);
        }
        for g in [&media, &function, &other] {
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
        let dialog = adw::Dialog::builder().title(name).content_width(420).content_height(600).child(&toolbar).build();

        let d = Rc::new(ChangeDialog {
            dialog,
            apply,
            choices,
            shortcut_row,
            shortcut_check,
            shortcut: Cell::new(None),
            capturing: Cell::new(false),
            macro_check,
            macro_picker,
            macro_titles,
            macro_sectors: RefCell::new(Vec::new()),
            macro_selected: Cell::new(0),
            on_new_macro: RefCell::new(None),
            on_apply: RefCell::new(None),
            current: current.clone(),
        });
        d.set_macros(macros.unwrap_or_default(), None);
        d.select(current);
        if let Some(picker) = &d.macro_picker {
            let weak = Rc::downgrade(&d);
            picker.connect_selected_notify(move |p| {
                let Some(d) = weak.upgrade() else { return };
                let count = d.macro_sectors.borrow().len() as u32;
                if p.selected() == count {
                    // "New Macro…": make one, then come back with it chosen.
                    p.set_selected(d.macro_selected.get().min(count.saturating_sub(1)));
                    if let Some(f) = d.on_new_macro.borrow().as_ref() {
                        f(());
                    }
                    return;
                }
                d.macro_selected.set(p.selected());
                if let Some(check) = &d.macro_check {
                    check.set_active(true);
                }
                d.update_apply();
            });
        }

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

    pub fn connect_apply(&self, f: impl Fn(ButtonAction) + 'static) {
        *self.on_apply.borrow_mut() = Some(Box::new(f));
    }

    /// `f` runs when "New Macro…" is picked; answer with [`ChangeDialog::set_macros`].
    pub fn connect_new_macro(&self, f: impl Fn(()) + 'static) {
        *self.on_new_macro.borrow_mut() = Some(Box::new(f));
    }

    /// The macros the picker offers, then "New Macro…"; `select` chooses one
    /// (a macro just made) and the Macro option with it.
    pub fn set_macros(&self, macros: MacroChoices, select: Option<u8>) {
        let Some(picker) = &self.macro_picker else { return };
        let mut titles: Vec<String> = macros.iter().map(|(_, t)| t.clone()).collect();
        titles.push("New Macro…".into());
        let titles: Vec<&str> = titles.iter().map(String::as_str).collect();
        *self.macro_sectors.borrow_mut() = macros.iter().map(|&(sector, _)| sector).collect();
        self.macro_titles.splice(0, self.macro_titles.n_items(), &titles);
        let index = select.and_then(|s| macros.iter().position(|&(sector, _)| sector == s));
        picker.set_selected(index.unwrap_or(0) as u32);
        self.macro_selected.set(index.unwrap_or(0) as u32);
        if index.is_some()
            && let Some(check) = &self.macro_check
        {
            check.set_active(true);
        }
        self.update_apply();
    }

    /// Start from what the button does now.
    fn select(&self, current: &ButtonAction) {
        if current.kind == "key" {
            self.shortcut.set(Some((current.modifiers, current.value as u8)));
            self.shortcut_row.set_subtitle(&describe(current));
            self.shortcut_check.set_active(true);
        } else if current.kind == "macro" {
            self.shortcut_row.set_subtitle("Not set");
            let index = self.macro_sectors.borrow().iter().position(|&s| u16::from(s) == current.value);
            if let (Some(index), Some(picker), Some(check)) = (index, &self.macro_picker, &self.macro_check) {
                picker.set_selected(index as u32);
                self.macro_selected.set(index as u32);
                check.set_active(true);
            }
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
            Choice::Macro => {
                let index = self.macro_picker.as_ref()?.selected() as usize;
                let sector = *self.macro_sectors.borrow().get(index)?;
                Some(ButtonAction { kind: "macro".into(), value: u16::from(sector), modifiers: 0 })
            }
        }
    }

    fn update_apply(&self) {
        self.apply.set_sensitive(self.chosen().is_some_and(|a| a != self.current));
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
        let Some(usage) = usage(unshifted(ctrl, keyval, keycode)) else {
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
