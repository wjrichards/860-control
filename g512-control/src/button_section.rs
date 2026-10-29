//! The Mouse view's Buttons group: what each button of the active profile does,
//! and the Change dialog for it. Left click can't be changed.
//!
//! All rows are built once and shown or hidden as the mouse needs; rows
//! added later would not reach the accessibility tree.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use adw::prelude::*;

use crate::{
    button_actions::{button_name, describe},
    change_dialog::{ChangeDialog, Handler},
    service::{ButtonAction, Mouse},
};

/// Room for the most buttons a profile can hold.
const MAX_BUTTONS: usize = 16;

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
        let group = adw::PreferencesGroup::builder().title("Buttons").visible(false).build();
        let rows: Vec<ButtonRow> = (0..MAX_BUTTONS)
            .map(|i| {
                let row = adw::ActionRow::builder().title(button_name(i)).visible(false).build();
                let change = (i > 0).then(|| {
                    let b = gtk::Button::builder().label("Change").valign(gtk::Align::Center).build();
                    // A button is named by its own text; say which row it changes.
                    let what = format!("Change what {} does", button_name(i));
                    b.update_property(&[gtk::accessible::Property::Description(&what)]);
                    row.add_suffix(&b);
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

    /// Open the Change dialog for button `index` of the active profile (the
    /// rows, the callouts and the mouse drawing all come here).
    pub fn open_dialog(self: &Rc<Self>, index: usize) {
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
