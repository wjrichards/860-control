//! The Key Maps keyboard: drawn keys placed by the layout's quarter units,
//! scaled down to fit the width they're given, but not below a floor. Below
//! the floor the grid keeps that size and its scroller scrolls.

use std::cell::{Cell, RefCell};

use gtk::{gdk, glib, prelude::*, subclass::prelude::*};

use crate::layout;

/// Pixels per key at full size.
pub const UNIT: f64 = 34.0;
/// Space between drawn keys, in pixels.
const GAP: f64 = 3.0;

/// A key's place in the layout's quarter units: (x, y, w, h).
pub type Place = (u16, u16, u16, u16);

/// The grid's full-size width and height in pixels.
pub fn full_size() -> (f64, f64) {
    (f64::from(layout::WIDTH) / 4.0 * UNIT, f64::from(layout::HEIGHT) / 4.0 * UNIT)
}

/// The scale the keys are drawn at for `width` pixels: full size when there
/// is room, never under `floor`.
pub fn scale_for(width: f64, floor: f64) -> f64 {
    (width / full_size().0).clamp(floor.min(1.0), 1.0)
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct KeyGrid {
        /// Each key widget with its place.
        pub keys: RefCell<Vec<(gtk::Widget, Place)>>,
        pub floor: Cell<f64>,
        /// The scale and left offset of the last allocation, for hit testing.
        pub scale: Cell<f64>,
        pub offset: Cell<f64>,
        /// Key labels follow the scale through this provider.
        pub css: RefCell<Option<gtk::CssProvider>>,
        pub css_scale: Cell<f64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for KeyGrid {
        const NAME: &'static str = "G512KeyGrid";
        type Type = super::KeyGrid;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for KeyGrid {
        fn dispose(&self) {
            for (key, _) in self.keys.take() {
                key.unparent();
            }
            if let (Some(css), Some(display)) = (self.css.take(), gdk::Display::default()) {
                gtk::style_context_remove_provider_for_display(&display, &css);
            }
        }
    }

    impl WidgetImpl for KeyGrid {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let (w, h) = full_size();
            let floor = self.floor.get();
            match orientation {
                gtk::Orientation::Horizontal => ((w * floor).ceil() as i32, w.ceil() as i32, -1, -1),
                _ => {
                    let scale = if for_size < 0 { 1.0 } else { scale_for(f64::from(for_size), floor) };
                    let height = (h * scale).ceil() as i32;
                    (height, height, -1, -1)
                }
            }
        }

        fn size_allocate(&self, width: i32, _height: i32, _baseline: i32) {
            let scale = scale_for(f64::from(width), self.floor.get());
            let offset = ((f64::from(width) - full_size().0 * scale) / 2.0).max(0.0).floor();
            self.scale.set(scale);
            self.offset.set(offset);
            let px = |q: u16| f64::from(q) / 4.0 * UNIT * scale;
            for (key, (x, y, w, h)) in self.keys.borrow().iter() {
                // Ask first, as GTK expects. A label a little wider than its key
                // widens the key; the labels shrink with the keys.
                let (min_w, ..) = key.measure(gtk::Orientation::Horizontal, -1);
                let (min_h, ..) = key.measure(gtk::Orientation::Vertical, -1);
                let width = ((px(*w) - GAP).round() as i32).max(min_w);
                let height = ((px(*h) - GAP).round() as i32).max(min_h);
                let at = gtk::graphene::Point::new((offset + px(*x).round()) as f32, px(*y).round() as f32);
                key.allocate(width, height, -1, Some(gsk_translate(at)));
            }
            // Smaller keys get smaller labels; changing them now would resize
            // the grid mid-allocation, so it waits for idle.
            let rounded = (scale * 20.0).round() / 20.0;
            if (rounded - self.css_scale.get()).abs() > f64::EPSILON {
                self.css_scale.set(rounded);
                let weak = self.obj().downgrade();
                glib::idle_add_local_once(move || {
                    if let Some(grid) = weak.upgrade() {
                        grid.load_label_size(rounded);
                    }
                });
            }
        }
    }

    fn gsk_translate(at: gtk::graphene::Point) -> gtk::gsk::Transform {
        gtk::gsk::Transform::new().translate(&at)
    }
}

glib::wrapper! {
    pub struct KeyGrid(ObjectSubclass<imp::KeyGrid>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for KeyGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyGrid {
    pub fn new() -> Self {
        let grid: Self = glib::Object::new();
        grid.add_css_class("keygrid");
        let imp = grid.imp();
        imp.floor.set(1.0);
        imp.scale.set(1.0);
        imp.css_scale.set(1.0);
        let css = gtk::CssProvider::new();
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(&display, &css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        }
        *imp.css.borrow_mut() = Some(css);
        grid
    }

    /// Place `key` at (x, y), w × h quarter units.
    pub fn put(&self, key: &impl IsA<gtk::Widget>, place: Place) {
        key.set_parent(self);
        self.imp().keys.borrow_mut().push((key.clone().upcast(), place));
    }

    /// The least scale before the grid stops shrinking (and scrolls): 1.0
    /// keeps the keys full size.
    pub fn set_floor(&self, floor: f64) {
        self.imp().floor.set(floor.clamp(0.1, 1.0));
        self.queue_resize();
    }

    /// A point in the grid's pixels, in the layout's quarter units.
    pub fn to_quarters(&self, x: f64, y: f64) -> (f64, f64) {
        let imp = self.imp();
        let unit = UNIT * imp.scale.get();
        ((x - imp.offset.get()) / unit * 4.0, y / unit * 4.0)
    }

    fn load_label_size(&self, scale: f64) {
        if let Some(css) = self.imp().css.borrow().as_ref() {
            css.load_from_string(&format!(".keygrid {{ font-size: {}%; }}", (scale * 100.0).round()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_fit_the_width_down_to_the_floor() {
        let (full, _) = full_size();
        assert_eq!(scale_for(full * 2.0, 0.6), 1.0);
        assert!((scale_for(full * 0.8, 0.6) - 0.8).abs() < 1e-9);
        assert_eq!(scale_for(full * 0.3, 0.6), 0.6);
        // A floor of 1.0 keeps full size: the grid scrolls instead.
        assert_eq!(scale_for(full * 0.5, 1.0), 1.0);
    }
}
