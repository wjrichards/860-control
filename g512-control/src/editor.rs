//! The key-map editor page: the drawn keyboard, selection, painting, and
//! which map (Custom or Gaming) is being edited.

use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

use adw::prelude::*;
use gtk::gdk;

use crate::{
    layout::{self, GROUPS, KEYS},
    service::Keys,
};

/// Pixels per key.
const UNIT: f64 = 34.0;
/// Space between drawn keys, in pixels.
const GAP: i32 = 3;

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

pub struct Editor {
    pub page: adw::NavigationPage,
    custom_toggle: gtk::ToggleButton,
    gaming_toggle: gtk::ToggleButton,
    /// (usage, button, spoken key name), in layout order.
    keys: Vec<(u8, gtk::ToggleButton, &'static str)>,
    color_button: gtk::ColorDialogButton,
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
        let target = gtk::Box::builder().css_classes(["linked"]).halign(gtk::Align::Center).build();
        target.append(&custom_toggle);
        target.append(&gaming_toggle);
        target.update_property(&[gtk::accessible::Property::Label("Map being edited")]);

        let groups = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .max_children_per_line(8)
            .column_spacing(6)
            .row_spacing(6)
            .homogeneous(false)
            .halign(gtk::Align::Center)
            .build();

        let fixed = gtk::Fixed::new();
        fixed.add_css_class("keygrid");
        fixed.set_size_request(
            (f64::from(layout::WIDTH) / 4.0 * UNIT) as i32,
            (f64::from(layout::HEIGHT) / 4.0 * UNIT) as i32,
        );
        let mut keys = Vec::new();
        for key in KEYS {
            let px = |q: u16| f64::from(q) / 4.0 * UNIT;
            let (w, h) = (px(key.w) as i32 - GAP, px(key.h) as i32 - GAP);
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
            widget.set_size_request(w, h);
            fixed.put(&widget, px(key.x), px(key.y));
        }
        let scroller = gtk::ScrolledWindow::builder()
            .child(&fixed)
            .vscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_width(true)
            .propagate_natural_height(true)
            .halign(gtk::Align::Center)
            .build();

        let color_button = gtk::ColorDialogButton::new(Some(
            gtk::ColorDialog::builder().title("Key Colour").with_alpha(false).build(),
        ));
        color_button.set_rgba(&gdk::RGBA::new(1.0, 0.125, 0.0, 1.0));
        color_button.update_property(&[gtk::accessible::Property::Label("Paint colour")]);
        let paint = gtk::Button::builder().label("Paint Selected").css_classes(["suggested-action"]).build();
        let off = gtk::Button::builder().label("Turn Off").build();
        let clear = gtk::Button::builder().label("Clear Selection").build();
        let actions = gtk::Box::builder().spacing(6).halign(gtk::Align::Center).build();
        actions.append(&color_button);
        actions.append(&paint);
        actions.append(&off);
        actions.append(&clear);

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(18)
            .margin_top(18)
            .margin_bottom(18)
            .margin_start(12)
            .margin_end(12)
            .build();
        content.append(&target);
        content.append(&groups);
        content.append(&scroller);
        content.append(&actions);

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&adw::HeaderBar::new());
        toolbar.set_content(Some(&gtk::ScrolledWindow::builder().child(&content).build()));
        let page = adw::NavigationPage::builder().title("Key Maps").tag("keymaps").child(&toolbar).build();

        let editor = Rc::new(Editor {
            page,
            custom_toggle,
            gaming_toggle,
            keys,
            color_button,
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
        editor.connect_drag_select(&fixed);
        editor.redraw();
        editor
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
             .keygrid button.key:checked { outline: 3px solid @accent_color; outline-offset: -3px; }\n\
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
    fn connect_drag_select(self: &Rc<Self>, fixed: &gtk::Fixed) {
        let drag = gtk::GestureDrag::new();
        drag.set_propagation_phase(gtk::PropagationPhase::Capture);
        let start: Rc<Cell<DragStart>> = Rc::new(Cell::new(None));
        let active = Rc::new(Cell::new(false));
        let to_quarters = |px: f64| px / UNIT * 4.0;
        {
            let (start, active) = (start.clone(), active.clone());
            drag.connect_drag_begin(move |_, x, y| {
                start.set(Some((x, y, layout::key_at(to_quarters(x), to_quarters(y)))));
                active.set(false);
            });
        }
        let w = Rc::downgrade(self);
        drag.connect_drag_update(move |gesture, dx, dy| {
            let (Some(editor), Some((x, y, first))) = (w.upgrade(), start.get()) else { return };
            let here = layout::key_at(to_quarters(x + dx), to_quarters(y + dy));
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
        fixed.add_controller(drag);
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
    fn map_round_trip() {
        let keys: Keys = vec![(0x04, 1, 2, 3), (0xe0, 4, 5, 6)];
        assert_eq!(to_keys(&to_map(&keys)), keys);
    }
}
