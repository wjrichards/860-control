//! A drawing area that keeps its aspect ratio: height follows width, so an
//! illustration fits a wide window and a phone-width column alike.

use gtk::{glib, prelude::*, subclass::prelude::*};

/// Draws onto the area: (context, width, height).
pub type DrawFn = Box<dyn Fn(&gtk::cairo::Context, f64, f64)>;

mod imp {
    use std::cell::{Cell, RefCell};

    use super::*;

    #[derive(Default)]
    pub struct ArtArea {
        pub aspect: Cell<f64>,
        pub natural_width: Cell<i32>,
        pub draw: RefCell<Option<DrawFn>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ArtArea {
        const NAME: &'static str = "G512ArtArea";
        type Type = super::ArtArea;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_accessible_role(gtk::AccessibleRole::Img);
        }
    }

    impl ObjectImpl for ArtArea {}

    impl WidgetImpl for ArtArea {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let natural = self.natural_width.get();
            match orientation {
                gtk::Orientation::Horizontal => (natural.min(200), natural, -1, -1),
                _ => {
                    let width = if for_size < 0 { natural } else { for_size };
                    let height = (f64::from(width) / self.aspect.get().max(0.1)).round() as i32;
                    (height, height, -1, -1)
                }
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let (w, h) = (widget.width() as f32, widget.height() as f32);
            if w <= 0.0 || h <= 0.0 {
                return;
            }
            let cr = snapshot.append_cairo(&gtk::graphene::Rect::new(0.0, 0.0, w, h));
            if let Some(draw) = self.draw.borrow().as_ref() {
                draw(&cr, f64::from(w), f64::from(h));
            }
        }
    }
}

glib::wrapper! {
    pub struct ArtArea(ObjectSubclass<imp::ArtArea>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ArtArea {
    /// An area `aspect` (width / height) wide for its height, preferring
    /// `natural_width` pixels across.
    pub fn new(aspect: f64, natural_width: i32, draw: DrawFn) -> Self {
        let area: Self = glib::Object::new();
        let imp = area.imp();
        imp.aspect.set(aspect);
        imp.natural_width.set(natural_width);
        *imp.draw.borrow_mut() = Some(draw);
        area
    }
}

/// A rounded rectangle path, for cards, cases and key caps.
pub fn rounded_rect(cr: &gtk::cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let r = r.min(w / 2.0).min(h / 2.0);
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.arc(x + r, y + h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
    cr.arc(x + r, y + r, r, std::f64::consts::PI, 1.5 * std::f64::consts::PI);
    cr.close_path();
}

/// Set a Cairo source colour from an (r, g, b) triple and an alpha.
pub fn set_rgb(cr: &gtk::cairo::Context, (r, g, b): (u8, u8, u8), alpha: f64) {
    cr.set_source_rgba(f64::from(r) / 255.0, f64::from(g) / 255.0, f64::from(b) / 255.0, alpha);
}
