//! The Keyboard › Key Maps page: the drawn keyboard on a card that takes the
//! page's free height, selection, painting, and which map (Custom or Gaming)
//! is being edited.

use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

use adw::prelude::*;
use gtk::gdk;

use crate::{
    key_grid::KeyGrid,
    layout::{self, GROUPS, KEYS},
    service::Keys,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapName {
    Custom,
    Gaming,
}

impl MapName {
    pub fn as_str(self) -> &'static str {
        match self {
            MapName::Custom => "custom",
            MapName::Gaming => "gaming",
        }
    }
}

type Map = BTreeMap<u8, (u8, u8, u8)>;
type EditHandler = Box<dyn Fn(MapName, Keys)>;
/// Where a drag started, in pixels, and the key under it.
type DragStart = Option<(f64, f64, Option<u8>)>;

fn to_map(keys: &Keys) -> Map {
    keys.iter().map(|&(k, r, g, b)| (k, (r, g, b))).collect()
}

fn to_keys(map: &Map) -> Keys {
    map.iter().map(|(&k, &(r, g, b))| (k, r, g, b)).collect()
}

/// A rough colour name for screen readers: "off", "white", "dim red"...
pub fn color_name((r, g, b): (u8, u8, u8)) -> String {
    let (r, g, b) = (f64::from(r) / 255.0, f64::from(g) / 255.0, f64::from(b) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    if max < 0.07 {
        return "off".into();
    }
    let sat = (max - min) / max;
    let base = if sat < 0.2 {
        if max > 0.75 { "white" } else { "grey" }
    } else {
        let d = max - min;
        let hue = if max == r {
            60.0 * (((g - b) / d).rem_euclid(6.0))
        } else if max == g {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        match hue {
            h if !(15.0..345.0).contains(&h) => "red",
            h if h < 45.0 => "orange",
            h if h < 70.0 => "yellow",
            h if h < 165.0 => "green",
            h if h < 200.0 => "cyan",
            h if h < 255.0 => "blue",
            h if h < 290.0 => "purple",
            _ => "pink",
        }
    };
    if max < 0.55 && base != "grey" { format!("dim {base}") } else { base.into() }
}

/// The hint under the keys, with how many are selected. `scrolls`: the
/// window is too narrow for the whole grid, which scrolls sideways.
fn selection_hint(selected: usize, scrolls: bool) -> String {
    let keys = if selected == 1 { "key" } else { "keys" };
    let how = if scrolls {
        "Scroll sideways to reach the rest of the keyboard."
    } else {
        "Click keys to select them, or drag across several."
    };
    format!("{how} {selected} {keys} selected.")
}

pub struct Editor {
    /// The page's content, for the Key Maps page.
    pub widget: gtk::Box,
    custom_toggle: gtk::ToggleButton,
    gaming_toggle: gtk::ToggleButton,
    /// (usage, button, spoken key name), in layout order.
    keys: Vec<(u8, gtk::ToggleButton, &'static str)>,
    color_button: gtk::ColorDialogButton,
    hint: gtk::Label,
    grid: KeyGrid,
    /// The grid's sideways scrolling, for the hint.
    scroll: gtk::Adjustment,
    /// "Shown while a game runs: Gaming", left out of a narrow window.
    shown_note: gtk::Label,
    css: gtk::CssProvider,
    custom: RefCell<Map>,
    gaming: RefCell<Map>,
    on_edit: RefCell<Option<EditHandler>>,
}

impl Editor {
    pub fn new() -> Rc<Self> {
        let css = gtk::CssProvider::new();
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(&display, &css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        }

        let custom_toggle = gtk::ToggleButton::builder().label("Custom").active(true).build();
        let gaming_toggle = gtk::ToggleButton::builder().label("Gaming").group(&custom_toggle).build();
        let target = gtk::Box::builder().css_classes(["linked"]).build();
        target.append(&custom_toggle);
        target.append(&gaming_toggle);
        target.update_property(&[gtk::accessible::Property::Label("Map being edited")]);
        let editing = gtk::Label::builder().label("Editing").css_classes(["dim-label"]).build();
        let shown = gtk::Label::builder()
            .label("Shown while a game runs: Gaming")
            .hexpand(true)
            .xalign(1.0)
            .wrap(true)
            .css_classes(["dim-label", "caption"])
            .build();
        let top = gtk::Box::builder().spacing(12).build();
        top.append(&editing);
        top.append(&target);
        top.append(&shown);

        let groups = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .max_children_per_line(8)
            .column_spacing(8)
            .row_spacing(8)
            .homogeneous(false)
            .halign(gtk::Align::Start)
            .build();

        let grid = KeyGrid::new();
        let mut keys = Vec::new();
        for key in KEYS {
            let widget: gtk::Widget = match key.usage {
                Some(usage) => {
                    let button = gtk::ToggleButton::builder()
                        .label(key.label)
                        .css_classes(["key", &format!("key-{usage:02x}")])
                        .build();
                    // The label child would name the button "W"; the name set in
                    // `redraw` ("W, red") must win.
                    button.reset_relation(gtk::AccessibleRelation::LabelledBy);
                    keys.push((usage, button.clone(), key.name));
                    button.upcast()
                }
                // Fn has no light of its own: drawn, not selectable.
                None => {
                    let label = gtk::Label::builder().label(key.label).css_classes(["key-fixed", "dim-label"]).build();
                    label.upcast()
                }
            };
            grid.put(&widget, (key.x, key.y, key.w, key.h));
        }
        // The keys shrink to fit the width down to a floor (`set_floor`);
        // below it, and in a narrow window, the grid scrolls sideways.
        let scroller = gtk::ScrolledWindow::builder()
            .child(&grid)
            .vscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_width(true)
            .propagate_natural_height(true)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .vexpand(true)
            .build();
        let hint = gtk::Label::builder().label(selection_hint(0, false)).wrap(true).css_classes(["dim-label", "caption"]).build();
        let card = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(16)
            .vexpand(true)
            .css_classes(["drawing-card"])
            .build();
        card.append(&scroller);
        card.append(&hint);

        let color_button = gtk::ColorDialogButton::new(Some(
            gtk::ColorDialog::builder().title("Key Colour").with_alpha(false).build(),
        ));
        color_button.set_rgba(&gdk::RGBA::new(1.0, 0.125, 0.0, 1.0));
        color_button.update_property(&[gtk::accessible::Property::Label("Paint colour")]);
        // Keeps its swatch size where the actions wrap.
        color_button.set_halign(gtk::Align::Start);
        let paint = gtk::Button::builder().label("Paint Selected").css_classes(["suggested-action"]).build();
        let off = gtk::Button::builder().label("Turn Off").build();
        let clear = gtk::Button::builder().label("Clear Selection").build();
        // Wraps onto more lines in a narrow window.
        let actions = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .max_children_per_line(4)
            .column_spacing(8)
            .row_spacing(8)
            .halign(gtk::Align::Start)
            .build();
        actions.append(&color_button);
        actions.append(&paint);
        actions.append(&off);
        actions.append(&clear);

        let select = gtk::Label::builder().label("Select").xalign(0.0).css_classes(["heading"]).build();
        let widget = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(20).vexpand(true).build();
        widget.append(&top);
        widget.append(&card);
        widget.append(&select);
        widget.append(&groups);
        widget.append(&actions);

        let editor = Rc::new(Editor {
            widget,
            custom_toggle,
            gaming_toggle,
            keys,
            color_button,
            hint,
            grid: grid.clone(),
            scroll: scroller.hadjustment(),
            shown_note: shown.clone(),
            css,
            custom: RefCell::new(Map::new()),
            gaming: RefCell::new(Map::new()),
            on_edit: RefCell::new(None),
        });

        for &(name, usages) in GROUPS {
            let button = gtk::Button::builder().label(name).css_classes(["pill"]).build();
            let w = Rc::downgrade(&editor);
            button.connect_clicked(move |_| {
                if let Some(e) = w.upgrade() {
                    e.select(usages);
                }
            });
            groups.append(&button);
        }
        let w = Rc::downgrade(&editor);
        paint.connect_clicked(move |_| {
            if let Some(e) = w.upgrade() {
                let c = e.color_button.rgba();
                let ch = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                e.paint(Some((ch(c.red()), ch(c.green()), ch(c.blue()))));
            }
        });
        let w = Rc::downgrade(&editor);
        off.connect_clicked(move |_| {
            if let Some(e) = w.upgrade() {
                e.paint(None);
            }
        });
        let w = Rc::downgrade(&editor);
        clear.connect_clicked(move |_| {
            if let Some(e) = w.upgrade() {
                e.keys.iter().for_each(|(_, b, _)| b.set_active(false));
            }
        });
        // Redraw from whichever toggle became active: when Gaming is clicked,
        // Custom's "toggled" fires before Gaming is active.
        for toggle in [&editor.custom_toggle, &editor.gaming_toggle] {
            let w = Rc::downgrade(&editor);
            toggle.connect_toggled(move |t| {
                if let Some(e) = w.upgrade().filter(|_| t.is_active()) {
                    e.redraw();
                }
            });
        }
        for (_, button, _) in &editor.keys {
            let w = Rc::downgrade(&editor);
            button.connect_toggled(move |_| {
                if let Some(e) = w.upgrade() {
                    e.update_hint();
                }
            });
        }
        for property in ["upper", "page-size"] {
            let w = Rc::downgrade(&editor);
            editor.scroll.connect_notify_local(Some(property), move |_, _| {
                if let Some(e) = w.upgrade() {
                    e.update_hint();
                }
            });
        }
        editor.connect_drag_select(&grid);
        editor.redraw();
        editor
    }

    /// In a narrow window the note beside the Editing toggle would wrap onto
    /// several lines, so it's left out. (Wider, Key Maps has the sidebar
    /// folded away, which leaves room.)
    pub fn set_compact(&self, compact: bool) {
        self.shown_note.set_visible(!compact);
    }

    /// The least scale the keys shrink to before the grid scrolls: 1.0 in a
    /// narrow window, where the keys keep their size.
    pub fn set_key_floor(&self, floor: f64) {
        self.grid.set_floor(floor);
    }

    fn update_hint(&self) {
        let selected = self.keys.iter().filter(|(_, b, _)| b.is_active()).count();
        let scrolls = self.scroll.upper() > self.scroll.page_size() + 0.5;
        let text = selection_hint(selected, scrolls);
        if self.hint.label() != text {
            self.hint.set_label(&text);
        }
    }

    /// Called with each whole-map edit, to send to the service.
    pub fn connect_edit(&self, f: impl Fn(MapName, Keys) + 'static) {
        *self.on_edit.borrow_mut() = Some(Box::new(f));
    }

    pub fn target(&self) -> MapName {
        if self.gaming_toggle.is_active() { MapName::Gaming } else { MapName::Custom }
    }

    pub fn set_target(&self, name: MapName) {
        match name {
            MapName::Custom => self.custom_toggle.set_active(true),
            MapName::Gaming => self.gaming_toggle.set_active(true),
        }
    }

    /// The service's maps, after a refresh.
    pub fn set_maps(&self, custom: &Keys, gaming: &Keys) {
        let (custom, gaming) = (to_map(custom), to_map(gaming));
        if *self.custom.borrow() == custom && *self.gaming.borrow() == gaming {
            return;
        }
        *self.custom.borrow_mut() = custom;
        *self.gaming.borrow_mut() = gaming;
        self.redraw();
    }

    fn map(&self) -> &RefCell<Map> {
        match self.target() {
            MapName::Custom => &self.custom,
            MapName::Gaming => &self.gaming,
        }
    }

    fn select(&self, usages: &[u8]) {
        for (usage, button, _) in &self.keys {
            if usages.contains(usage) {
                button.set_active(true);
            }
        }
    }

    /// Paint the selected keys with `color`, or turn them off.
    fn paint(&self, color: Option<(u8, u8, u8)>) {
        let selected: Vec<u8> = self.keys.iter().filter(|(_, b, _)| b.is_active()).map(|(u, _, _)| *u).collect();
        if selected.is_empty() {
            return;
        }
        {
            let mut map = self.map().borrow_mut();
            for usage in selected {
                match color {
                    Some(c) => map.insert(usage, c),
                    None => map.remove(&usage),
                };
            }
        }
        self.redraw();
        let keys = to_keys(&self.map().borrow());
        if let Some(f) = self.on_edit.borrow().as_ref() {
            f(self.target(), keys);
        }
    }

    /// Recolour every key from the edited map, and update spoken names.
    fn redraw(&self) {
        let map = self.map().borrow();
        let mut css = String::from(
            ".keygrid button.key { padding: 0; min-height: 0; min-width: 0; border-radius: 6px; font-size: 0.8em; }\n\
             .keygrid button.key:checked { outline: 3px solid white; outline-offset: -3px; box-shadow: inset 0 0 0 4px rgb(0 0 0 / 60%); }\n\
             .keygrid .key-fixed { font-size: 0.8em; }\n",
        );
        for (usage, button, name) in &self.keys {
            let color = map.get(usage).copied();
            if let Some((r, g, b)) = color {
                let light = 0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b) > 140.0;
                let text = if light { "#000000" } else { "#ffffff" };
                css.push_str(&format!(
                    ".keygrid button.key.key-{usage:02x} {{ background: #{r:02x}{g:02x}{b:02x}; color: {text}; }}\n"
                ));
            }
            let spoken = color.map_or_else(|| "off".to_string(), color_name);
            button.update_property(&[gtk::accessible::Property::Label(&format!("{name}, {spoken}"))]);
        }
        self.css.load_from_string(&css);
    }

    /// Dragging across keys selects every key passed over.
    fn connect_drag_select(self: &Rc<Self>, grid: &KeyGrid) {
        let drag = gtk::GestureDrag::new();
        drag.set_propagation_phase(gtk::PropagationPhase::Capture);
        let start: Rc<Cell<DragStart>> = Rc::new(Cell::new(None));
        let active = Rc::new(Cell::new(false));
        {
            let (start, active, grid) = (start.clone(), active.clone(), grid.downgrade());
            drag.connect_drag_begin(move |_, x, y| {
                let Some(grid) = grid.upgrade() else { return };
                let (qx, qy) = grid.to_quarters(x, y);
                start.set(Some((x, y, layout::key_at(qx, qy))));
                active.set(false);
            });
        }
        let grid_weak = grid.downgrade();
        let w = Rc::downgrade(self);
        drag.connect_drag_update(move |gesture, dx, dy| {
            let (Some(editor), Some(grid), Some((x, y, first))) = (w.upgrade(), grid_weak.upgrade(), start.get()) else {
                return;
            };
            let (qx, qy) = grid.to_quarters(x + dx, y + dy);
            let here = layout::key_at(qx, qy);
            if !active.get() {
                if here == first {
                    return; // still on the first key: leave it to the button's own click
                }
                active.set(true);
                // Stop the first key's click from toggling it back on release.
                gesture.set_state(gtk::EventSequenceState::Claimed);
                if let Some(u) = first {
                    editor.select(&[u]);
                }
            }
            if let Some(u) = here {
                editor.select(&[u]);
            }
        });
        grid.add_controller(drag);
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_remove_provider_for_display(&display, &self.css);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_names() {
        assert_eq!(color_name((0, 0, 0)), "off");
        assert_eq!(color_name((255, 255, 255)), "white");
        assert_eq!(color_name((0x20, 0x20, 0x20)), "grey");
        assert_eq!(color_name((255, 0, 0)), "red");
        assert_eq!(color_name((255, 0x20, 0)), "red");
        assert_eq!(color_name((255, 120, 0)), "orange");
        assert_eq!(color_name((0, 255, 0)), "green");
        assert_eq!(color_name((0, 0, 255)), "blue");
        assert_eq!(color_name((0x80, 0, 0)), "dim red");
    }

    #[test]
    fn selection_hint_counts() {
        assert!(selection_hint(0, false).ends_with("0 keys selected."));
        assert!(selection_hint(1, false).ends_with("1 key selected."));
        assert!(selection_hint(4, true).ends_with("4 keys selected."));
        assert!(selection_hint(4, true).starts_with("Scroll sideways"));
    }

    #[test]
    fn map_round_trip() {
        let keys: Keys = vec![(0x04, 1, 2, 3), (0xe0, 4, 5, 6)];
        assert_eq!(to_keys(&to_map(&keys)), keys);
    }
}
