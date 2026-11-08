//! The Mouse › Macros page (boards "Mouse › Macros · 1280/860/360", "· empty"
//! and "· memory full"): the macros stored on the mouse, how many of its
//! slots they use, and New Macro. With none yet, a status page instead.
//!
//! All ten rows are built once and shown or hidden; rows added later would
//! not reach the accessibility tree.

use std::{cell::RefCell, rc::Rc};

use adw::prelude::*;

use crate::{
    change_dialog::Handler,
    macro_steps,
    service::{Mouse, MouseMacro},
};

/// The most macros the mouse holds: one per slot, sectors 6 to `0x0f`.
const MAX_MACROS: usize = 10;
/// The page's column is no wider than this, like GNOME Settings pages.
const MAX_WIDTH: i32 = 640;

pub struct MacrosPage {
    pub widget: adw::Clamp,
    stack: gtk::Stack,
    header: gtk::Box,
    new_button: gtk::Button,
    rows: Vec<adw::ActionRow>,
    used: gtk::Label,
    free: gtk::Label,
    level: gtk::LevelBar,
    note: gtk::Label,
    empty_button: gtk::Button,
    macros: RefCell<Vec<MouseMacro>>,
    /// `None` opens a new macro, `Some(sector)` an existing one.
    on_open: Handler<Option<u8>>,
}

impl MacrosPage {
    pub fn new() -> Rc<Self> {
        let title = gtk::Label::builder().label("On the Mouse").xalign(0.0).css_classes(["heading"]).build();
        let description = gtk::Label::builder()
            .label("They work on other computers too, even with this app closed")
            .xalign(0.0)
            .wrap(true)
            .css_classes(["dim-label", "caption"])
            .build();
        let labels = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(2).hexpand(true).build();
        labels.append(&title);
        labels.append(&description);
        let new_button = gtk::Button::builder().valign(gtk::Align::End).halign(gtk::Align::Start).build();
        new_button.set_child(Some(&adw::ButtonContent::builder().icon_name("list-add-symbolic").label("New Macro").build()));
        let header = gtk::Box::builder().spacing(12).build();
        header.append(&labels);
        header.append(&new_button);

        let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).css_classes(["boxed-list"]).build();
        let rows: Vec<adw::ActionRow> = (0..MAX_MACROS)
            .map(|_| {
                let row = adw::ActionRow::builder().activatable(true).subtitle_lines(0).visible(false).build();
                row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                list.append(&row);
                row
            })
            .collect();
        let macros_group = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(12).build();
        macros_group.append(&header);
        macros_group.append(&list);

        let used = gtk::Label::builder().xalign(0.0).hexpand(true).build();
        let free = gtk::Label::builder().css_classes(["dim-label"]).build();
        let meter_labels = gtk::Box::builder().spacing(12).build();
        meter_labels.append(&used);
        meter_labels.append(&free);
        let level = gtk::LevelBar::builder().min_value(0.0).max_value(10.0).build();
        // Full is a fact here, not a warning: no offset colours.
        level.remove_offset_value(Some(gtk::LEVEL_BAR_OFFSET_LOW));
        level.remove_offset_value(Some(gtk::LEVEL_BAR_OFFSET_HIGH));
        level.remove_offset_value(Some(gtk::LEVEL_BAR_OFFSET_FULL));
        let note = gtk::Label::builder().xalign(0.0).wrap(true).css_classes(["dim-label", "caption"]).build();
        let meter = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();
        meter.append(&meter_labels);
        meter.append(&level);
        meter.append(&note);
        let memory_row = adw::PreferencesRow::builder().activatable(false).child(&meter).build();
        let memory_group = adw::PreferencesGroup::builder().title("Mouse Memory").build();
        memory_group.add(&memory_row);

        let filled = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(24).build();
        filled.append(&macros_group);
        filled.append(&memory_group);

        let empty_button = gtk::Button::builder()
            .label("New Macro")
            .halign(gtk::Align::Center)
            .css_classes(["pill", "suggested-action"])
            .build();
        let empty = adw::StatusPage::builder()
            .icon_name("g512-macros-symbolic")
            .title("No Macros Yet")
            .description(
                "Record keys and pauses, then put the macro on a mouse button. \
                 Macros are stored on the mouse, so they work anywhere.",
            )
            .child(&empty_button)
            .vexpand(true)
            .build();

        let stack = gtk::Stack::builder().vhomogeneous(false).build();
        stack.add_named(&filled, Some("macros"));
        stack.add_named(&empty, Some("empty"));
        let widget = adw::Clamp::builder().maximum_size(MAX_WIDTH).tightening_threshold(MAX_WIDTH).child(&stack).build();

        let page = Rc::new(MacrosPage {
            widget,
            stack,
            header,
            new_button,
            rows,
            used,
            free,
            level,
            note,
            empty_button,
            macros: RefCell::new(Vec::new()),
            on_open: RefCell::new(None),
        });
        for button in [&page.new_button, &page.empty_button] {
            let weak = Rc::downgrade(&page);
            button.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.open(None);
                }
            });
        }
        for (i, row) in page.rows.iter().enumerate() {
            let weak = Rc::downgrade(&page);
            row.connect_activated(move |_| {
                let Some(p) = weak.upgrade() else { return };
                let sector = p.macros.borrow().get(i).map(|m| m.sector);
                if sector.is_some() {
                    p.open(sector);
                }
            });
        }
        page
    }

    fn open(&self, sector: Option<u8>) {
        if let Some(f) = self.on_open.borrow().as_ref() {
            f(sector);
        }
    }

    /// `on_open(None)` makes a new macro; `on_open(Some(sector))` edits one.
    pub fn connect_open(&self, f: impl Fn(Option<u8>) + 'static) {
        *self.on_open.borrow_mut() = Some(Box::new(f));
    }

    /// Under 550 sp, New Macro goes under the group's title.
    pub fn set_narrow(&self, narrow: bool) {
        self.header.set_orientation(if narrow { gtk::Orientation::Vertical } else { gtk::Orientation::Horizontal });
    }

    /// The service's view of the mouse. Returns whether it offers macros
    /// (the Macros row is shown only then).
    pub fn show(&self, mouse: Option<&Mouse>) -> bool {
        let Some((mouse, macros)) = mouse.and_then(|m| Some((m, m.macros.as_ref()?))) else { return false };
        self.stack.set_visible_child_name(if macros.is_empty() { "empty" } else { "macros" });
        for (i, row) in self.rows.iter().enumerate() {
            match macros.get(i) {
                Some(m) => {
                    row.set_title(&gtk::glib::markup_escape_text(&macro_steps::title(m)));
                    row.set_subtitle(&gtk::glib::markup_escape_text(&macro_steps::summary(mouse, m)));
                    row.set_visible(true);
                }
                None => row.set_visible(false),
            }
        }
        let (used, total) = mouse.macro_sectors;
        let full = total > 0 && used >= total;
        self.used.set_label(&format!("{used} of {total} macro slots used"));
        self.free.set_label(&if full { "Full".to_string() } else { format!("{} free", total.saturating_sub(used)) });
        self.level.set_max_value(f64::from(total.max(1)));
        self.level.set_value(f64::from(used));
        self.note.set_label(if full {
            "Delete a macro you don't use to make room for a new one."
        } else {
            "Each macro takes one slot. The editor shows how much of it a macro uses."
        });
        self.new_button.set_sensitive(!full);
        *self.macros.borrow_mut() = macros.clone();
        true
    }
}
