//! The callouts beside the mouse drawing: one button per mouse button, in
//! badge order, showing the badge number, the button's name and what it
//! does in the active profile. Activating one opens its Change dialog;
//! Left Click is fixed and insensitive.

use std::{cell::RefCell, rc::Rc};

use gtk::prelude::*;

use crate::{
    button_actions::{button_name, describe},
    change_dialog::Handler,
    mouse_art::HOTSPOTS,
    service::ButtonAction,
};

struct Callout {
    /// The service's button index.
    index: usize,
    badge: u8,
    button: gtk::Button,
    action: gtk::Label,
}

pub struct Callouts {
    pub grid: gtk::Grid,
    items: Vec<Callout>,
    on_open: Handler<usize>,
}

impl Callouts {
    pub fn new() -> Rc<Self> {
        let grid = gtk::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .column_homogeneous(true)
            .valign(gtk::Align::Start)
            .build();
        let mut spots: Vec<_> = HOTSPOTS.iter().collect();
        spots.sort_by_key(|s| s.badge);
        let items: Vec<Callout> = spots
            .into_iter()
            .map(|spot| {
                let index = usize::from(spot.button);
                let title = gtk::Label::builder()
                    .label(format!("{}  {}", spot.badge, button_name(index)))
                    .xalign(0.0)
                    .css_classes(["caption-heading"])
                    .build();
                let action = gtk::Label::builder()
                    .xalign(0.0)
                    .ellipsize(gtk::pango::EllipsizeMode::End)
                    .css_classes(["dim-label", "caption"])
                    .build();
                let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
                text.append(&title);
                text.append(&action);
                let button =
                    gtk::Button::builder().child(&text).halign(gtk::Align::Start).css_classes(["callout"]).build();
                if index == 0 {
                    button.set_sensitive(false);
                    button.add_css_class("callout-fixed");
                } else {
                    let what = format!("Change what {} does", button_name(index));
                    button.update_property(&[gtk::accessible::Property::Description(&what)]);
                }
                Callout { index, badge: spot.badge, button, action }
            })
            .collect();
        let callouts = Rc::new(Callouts { grid, items, on_open: RefCell::new(None) });
        for item in &callouts.items {
            let (weak, index) = (Rc::downgrade(&callouts), item.index);
            item.button.connect_clicked(move |_| {
                if let Some(c) = weak.upgrade()
                    && let Some(f) = c.on_open.borrow().as_ref()
                {
                    f(index);
                }
            });
        }
        callouts.arrange(true);
        callouts
    }

    /// `f` gets the service index of the button whose callout was activated.
    pub fn connect_open(&self, f: impl Fn(usize) + 'static) {
        *self.on_open.borrow_mut() = Some(Box::new(f));
    }

    /// Two columns: badges 1–6 down the first and 7–11 down the second
    /// (`column_major`, beside the drawing), or across in pairs (below it).
    pub fn arrange(&self, column_major: bool) {
        let rows = self.items.len().div_ceil(2);
        for (i, item) in self.items.iter().enumerate() {
            if item.button.parent().is_some() {
                self.grid.remove(&item.button);
            }
            let (column, row) = if column_major { (i / rows, i % rows) } else { (i % 2, i / 2) };
            self.grid.attach(&item.button, column as i32, row as i32, 1, 1);
        }
    }

    /// The active profile's actions, by service index; `None` hides them.
    pub fn show(&self, actions: Option<&[ButtonAction]>) {
        self.grid.set_visible(actions.is_some());
        let Some(actions) = actions else { return };
        for item in &self.items {
            let Some(action) = actions.get(item.index) else {
                item.button.set_visible(false);
                continue;
            };
            let text = if item.index == 0 { format!("{} · fixed", describe(action)) } else { describe(action) };
            item.action.set_label(&text);
            item.button.update_property(&[gtk::accessible::Property::Label(&format!(
                "{} {}, {text}",
                item.badge,
                button_name(item.index)
            ))]);
            item.button.set_visible(true);
        }
    }
}
