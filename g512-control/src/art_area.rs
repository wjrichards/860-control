//! A drawing area that keeps its aspect ratio: height follows width, so an
//! illustration fits a wide window and a phone-width column alike. Plus the
//! ticker that runs an illustration's effect loop on the frame clock.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

use gtk::{glib, prelude::*, subclass::prelude::*};

/// Draws onto the area: (context, width, height).
pub type DrawFn = Box<dyn Fn(&gtk::cairo::Context, f64, f64)>;

mod imp {
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

/// Frames drawn by effect loops, for the reduced-motion check
/// (`G512_CONTROL_TICK_LOG` prints it).
pub static TICKS: AtomicU64 = AtomicU64::new(0);

/// Runs an effect loop on an area's frame clock. `on_frame(Some(seconds))`
/// runs every frame while the area is mapped, a loop is wanted and the
/// system's animations are on; `on_frame(None)` sets the still frame
/// whenever they aren't.
pub struct Ticker {
    area: ArtArea,
    wanted: Cell<bool>,
    id: RefCell<Option<gtk::TickCallbackId>>,
    on_frame: Rc<dyn Fn(Option<f64>)>,
}

impl Ticker {
    pub fn new(area: &ArtArea, on_frame: impl Fn(Option<f64>) + 'static) -> Rc<Self> {
        let ticker = Rc::new(Ticker {
            area: area.clone(),
            wanted: Cell::new(false),
            id: RefCell::new(None),
            on_frame: Rc::new(on_frame),
        });
        let weak = Rc::downgrade(&ticker);
        area.connect_map(move |_| {
            if let Some(t) = weak.upgrade() {
                t.update();
            }
        });
        let weak = Rc::downgrade(&ticker);
        area.connect_unmap(move |_| {
            if let Some(t) = weak.upgrade() {
                t.update();
            }
        });
        if let Some(settings) = gtk::Settings::default() {
            let weak = Rc::downgrade(&ticker);
            settings.connect_gtk_enable_animations_notify(move |_| {
                if let Some(t) = weak.upgrade() {
                    t.update();
                }
            });
        }
        ticker
    }

    /// Whether the shown effect moves.
    pub fn set_wanted(&self, wanted: bool) {
        self.wanted.set(wanted);
        self.update();
    }

    fn update(&self) {
        let animations = gtk::Settings::default().is_some_and(|s| s.is_gtk_enable_animations());
        let run = self.wanted.get() && self.area.is_mapped() && animations;
        let running = self.id.borrow().is_some();
        if run && !running {
            let on_frame = self.on_frame.clone();
            let id = self.area.add_tick_callback(move |area, clock| {
                TICKS.fetch_add(1, Ordering::Relaxed);
                on_frame(Some(clock.frame_time() as f64 / 1e6));
                area.queue_draw();
                glib::ControlFlow::Continue
            });
            *self.id.borrow_mut() = Some(id);
        } else if !run {
            if let Some(id) = self.id.borrow_mut().take() {
                id.remove();
            }
            (self.on_frame)(None);
            self.area.queue_draw();
        }
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
