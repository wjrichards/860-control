//! The mouse illustration at the top of the Mouse view: the user's G502
//! HERO ink drawings, top view and 3/4 side view, in the lighting colour,
//! with the DPI LEDs and G logo lit and glowing, and a numbered badge on
//! each programmable button, in a card with the callouts. Clicking a button
//! on the drawing reports it like its callout.

use std::{cell::RefCell, rc::Rc};

use gtk::prelude::*;

use crate::{
    art::{self, CARD, Rgb, Shown},
    art_area::{ArtArea, rounded_rect, set_rgb},
    callouts::Callouts,
    change_dialog::Handler,
    editor::color_name,
    mouse_ink::{self, Ink},
    service::ButtonAction,
};

/// The compact board's layout, in its own pixels: the top view 300 px tall
/// at (16, 20), the side view 270 px wide at (196, 46).
const AREA_W: f64 = 482.0;
const AREA_H: f64 = 340.0;
const TOP_AT: (f64, f64) = (16.0, 20.0);
const TOP_H: f64 = 300.0;
const SIDE_AT: (f64, f64) = (196.0, 46.0);
const SIDE_W: f64 = 270.0;
const BADGE_R: f64 = 8.0;
/// Badges: `color.accent-strong` (#1D8092), white numbers at 4.61:1.
const BADGE: Rgb = (0x1d, 0x80, 0x92);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Top,
    Side,
}

/// A programmable button on the drawing.
pub struct Hotspot {
    /// The number on its badge and callout.
    pub badge: u8,
    /// Its index in the service's button list (`MouseButtons`).
    pub button: u8,
    pub view: View,
    /// Its area in the SVG's units: x, y, width, height. Badges sit at the centre.
    pub rect: (f64, f64, f64, f64),
}

/// Smallest first, so the wheel's tilts and G9 win over the buttons around them.
pub const HOTSPOTS: [Hotspot; 11] = [
    Hotspot { badge: 6, button: 8, view: View::Top, rect: (211.7, 346.8, 41.6, 52.2) },
    Hotspot { badge: 4, button: 10, view: View::Top, rect: (188.0, 166.0, 16.0, 60.0) },
    Hotspot { badge: 5, button: 9, view: View::Top, rect: (256.0, 166.0, 16.0, 60.0) },
    Hotspot { badge: 3, button: 2, view: View::Top, rect: (205.0, 162.0, 50.0, 118.0) },
    Hotspot { badge: 7, button: 7, view: View::Top, rect: (62.0, 128.0, 40.0, 100.0) },
    Hotspot { badge: 8, button: 6, view: View::Top, rect: (54.0, 236.0, 42.0, 108.0) },
    Hotspot { badge: 1, button: 0, view: View::Top, rect: (106.0, 120.0, 88.0, 240.0) },
    Hotspot { badge: 2, button: 1, view: View::Top, rect: (264.0, 120.0, 98.0, 240.0) },
    Hotspot { badge: 9, button: 5, view: View::Side, rect: (276.0, 150.0, 124.0, 60.0) },
    Hotspot { badge: 10, button: 3, view: View::Side, rect: (328.0, 96.0, 134.0, 42.0) },
    Hotspot { badge: 11, button: 4, view: View::Side, rect: (466.0, 88.0, 106.0, 28.0) },
];

/// Where each view lands in the area's own pixels: (x, y, scale).
fn placement(view: View, top: &Ink, side: &Ink) -> (f64, f64, f64) {
    match view {
        View::Top => (TOP_AT.0, TOP_AT.1, TOP_H / top.height),
        View::Side => (SIDE_AT.0, SIDE_AT.1, SIDE_W / side.width),
    }
}

/// The badge centre of `spot`, in the area's own pixels.
fn badge_centre(spot: &Hotspot, top: &Ink, side: &Ink) -> (f64, f64) {
    let (ox, oy, s) = placement(spot.view, top, side);
    let (x, y, w, h) = spot.rect;
    (ox + (x + w / 2.0) * s, oy + (y + h / 2.0) * s)
}

/// The button under a point in the area's own pixels.
pub fn hit(x: f64, y: f64, top: &Ink, side: &Ink) -> Option<&'static Hotspot> {
    HOTSPOTS.iter().find(|spot| {
        let (ox, oy, s) = placement(spot.view, top, side);
        let (rx, ry, rw, rh) = spot.rect;
        let (px, py) = ((x - ox) / s, (y - oy) / s);
        px >= rx && px <= rx + rw && py >= ry && py <= ry + rh
    })
}

#[derive(Default)]
struct State {
    /// The lighting colour; `None` while the mouse's lights are off.
    colour: Option<Rgb>,
    /// LED, logo and glow brightness, 0 to 1 (animated by effects).
    level: f64,
    /// Draw the numbered badges (the mouse's buttons are known).
    badges: bool,
    /// The service button index under the pointer.
    hover: Option<u8>,
}

pub struct MouseArt {
    /// The card: the drawing and its hint, with the callouts beside or below.
    pub widget: gtk::Box,
    area: ArtArea,
    hint: gtk::Label,
    callouts: Rc<Callouts>,
    state: Rc<RefCell<State>>,
    inks: Rc<(Ink, Ink)>,
    on_button: Rc<Handler<usize>>,
}

impl MouseArt {
    pub fn new() -> Rc<Self> {
        let inks = Rc::new((
            mouse_ink::parse(mouse_ink::TOP_SVG).expect("the top-view SVG parses (tested)"),
            mouse_ink::parse(mouse_ink::SIDE_SVG).expect("the side-view SVG parses (tested)"),
        ));
        let state: Rc<RefCell<State>> = Rc::default();
        let (s, i) = (state.clone(), inks.clone());
        let area =
            ArtArea::new(AREA_W / AREA_H, AREA_W as i32, Box::new(move |cr, w, h| draw(cr, w, h, &s.borrow(), &i)));
        area.set_hexpand(true);
        // The hint sits in the room under the side view, as on the board; the
        // narrow board has none.
        let hint = gtk::Label::builder()
            .label("Click a button or its chip to change what it does. Left click stays fixed.")
            .wrap(true)
            .xalign(0.0)
            .max_width_chars(28)
            .halign(gtk::Align::End)
            .valign(gtk::Align::Center)
            .margin_top(60)
            .css_classes(["dim-label", "caption"])
            .build();
        let drawing = gtk::Overlay::builder().child(&area).valign(gtk::Align::Start).build();
        drawing.add_overlay(&hint);
        let callouts = Callouts::new();
        let widget = gtk::Box::builder().spacing(16).css_classes(["mouse-card"]).build();
        widget.append(&drawing);
        widget.append(&callouts.grid);
        let on_button: Rc<Handler<usize>> = Rc::default();
        let f = on_button.clone();
        callouts.connect_open(move |index| {
            if let Some(f) = f.borrow().as_ref() {
                f(index);
            }
        });
        let art = Rc::new(MouseArt { widget, area, hint, callouts, state, inks, on_button });
        art.connect_pointer();
        art.set_compact(true);
        art
    }

    /// `f` gets the service index of a button clicked on the drawing or
    /// activated in the callouts.
    pub fn connect_button(&self, f: impl Fn(usize) + 'static) {
        *self.on_button.borrow_mut() = Some(Box::new(f));
    }

    /// Callouts beside the drawing (`compact`), or below it in a narrow window.
    pub fn set_compact(&self, compact: bool) {
        let orientation = if compact { gtk::Orientation::Horizontal } else { gtk::Orientation::Vertical };
        self.widget.set_orientation(orientation);
        self.hint.set_visible(compact);
        self.callouts.arrange(compact);
    }

    /// Show what the mouse is showing, and the active profile's actions
    /// (`None` while they're unknown: no badges and no callouts).
    pub fn show(&self, shown: &Shown, actions: Option<&[ButtonAction]>) {
        let badges = actions.is_some();
        self.callouts.show(actions);
        let colour = art::main_colour(shown);
        {
            let mut st = self.state.borrow_mut();
            st.colour = colour;
            st.level = if colour.is_some() { 1.0 } else { 0.0 };
            st.badges = badges;
        }
        let description = match shown {
            Shown::Dark => "lights off".into(),
            Shown::Colour { effect, colour } => format!("{effect}, {}", color_name(*colour)),
            Shown::Rainbow { effect } => (*effect).to_string(),
            Shown::Map(_) => "key map".into(),
        };
        self.area.update_property(&[gtk::accessible::Property::Label(&format!("G502 HERO mouse, {description}"))]);
        self.area.queue_draw();
    }

    /// Area pixels from widget pixels (the drawing is centred and scaled).
    fn to_area(&self, x: f64, y: f64) -> (f64, f64) {
        let (w, h) = (f64::from(self.area.width()), f64::from(self.area.height()));
        let s = (w / AREA_W).min(h / AREA_H);
        ((x - (w - AREA_W * s) / 2.0) / s, (y - (h - AREA_H * s) / 2.0) / s)
    }

    fn connect_pointer(self: &Rc<Self>) {
        let motion = gtk::EventControllerMotion::new();
        let weak = Rc::downgrade(self);
        motion.connect_motion(move |_, x, y| {
            let Some(art) = weak.upgrade() else { return };
            let (ax, ay) = art.to_area(x, y);
            let over = hit(ax, ay, &art.inks.0, &art.inks.1).map(|s| s.button);
            if art.state.borrow().hover != over {
                art.state.borrow_mut().hover = over;
                art.area.set_cursor_from_name(over.filter(|&b| b != 0).map(|_| "pointer"));
                art.area.queue_draw();
            }
        });
        let weak = Rc::downgrade(self);
        motion.connect_leave(move |_| {
            if let Some(art) = weak.upgrade() {
                art.state.borrow_mut().hover = None;
                art.area.queue_draw();
            }
        });
        self.area.add_controller(motion);
        let click = gtk::GestureClick::new();
        let weak = Rc::downgrade(self);
        click.connect_released(move |_, _, x, y| {
            let Some(art) = weak.upgrade() else { return };
            if !art.state.borrow().badges {
                return;
            }
            let (ax, ay) = art.to_area(x, y);
            if let Some(spot) = hit(ax, ay, &art.inks.0, &art.inks.1).filter(|s| s.button != 0)
                && let Some(f) = art.on_button.borrow().as_ref()
            {
                f(usize::from(spot.button));
            }
        });
        self.area.add_controller(click);
    }
}

/// The G cut into the top-view logo disc, in the SVG's units.
fn trace_g_top(cr: &gtk::cairo::Context) {
    cr.new_sub_path();
    cr.arc_negative(161.6, 518.2, 15.0, -std::f64::consts::FRAC_PI_4, -2.0 * std::f64::consts::PI);
    cr.line_to(163.2, 518.2);
}

/// The same G, flattened and tilted onto the side view's logo.
fn trace_g_side(cr: &gtk::cairo::Context) {
    let m = cr.matrix();
    cr.translate(546.0, 24.0);
    cr.rotate(14f64.to_radians());
    cr.scale(1.2, 0.5);
    cr.new_sub_path();
    cr.arc_negative(0.0, 0.0, 15.0, -std::f64::consts::FRAC_PI_4, -2.0 * std::f64::consts::PI);
    cr.line_to(1.6, 0.0);
    cr.set_matrix(m);
}

fn draw(cr: &gtk::cairo::Context, w: f64, h: f64, st: &State, inks: &(Ink, Ink)) {
    let s = (w / AREA_W).min(h / AREA_H);
    cr.translate((w - AREA_W * s) / 2.0, (h - AREA_H * s) / 2.0);
    cr.scale(s, s);
    let line = st.colour.map(|c| art::drawing_colour(c, CARD));
    let (line_rgb, line_alpha) = line.map_or(((255, 255, 255), 0.35), |c| (c, 1.0));
    let (top, side) = inks;
    for (view, ink, trace_g) in
        [(View::Top, top, trace_g_top as fn(&gtk::cairo::Context)), (View::Side, side, trace_g_side)]
    {
        let (ox, oy, k) = placement(view, top, side);
        let m = cr.matrix();
        cr.translate(ox, oy);
        cr.scale(k, k);
        // Glow, outlines (filled shapes), then LEDs and logo.
        if let Some(c) = line {
            for p in &ink.glow {
                mouse_ink::trace(cr, p);
                set_rgb(cr, c, 0.2 * st.level);
                let _ = cr.fill();
            }
        }
        for p in &ink.outline {
            mouse_ink::trace(cr, p);
        }
        set_rgb(cr, line_rgb, line_alpha);
        let _ = cr.fill();
        let lit = line.map_or(((255, 255, 255), 0.2), |c| (c, st.level.max(0.15)));
        for p in ink.leds.iter().chain(&ink.logo) {
            mouse_ink::trace(cr, p);
        }
        set_rgb(cr, lit.0, lit.1);
        let _ = cr.fill();
        trace_g(cr);
        set_rgb(cr, CARD, 1.0);
        cr.set_line_width(if view == View::Top { 6.5 } else { 7.0 });
        cr.set_line_cap(gtk::cairo::LineCap::Round);
        let _ = cr.stroke();
        // The hovered button.
        if let Some(spot) = HOTSPOTS.iter().find(|s| s.view == view && Some(s.button) == st.hover && s.button != 0) {
            let (x, y, bw, bh) = spot.rect;
            rounded_rect(cr, x, y, bw, bh, 6.0);
            set_rgb(cr, line_rgb, 0.35);
            let _ = cr.fill();
        }
        cr.set_matrix(m);
    }
    if st.badges {
        cr.select_font_face("Sans", gtk::cairo::FontSlant::Normal, gtk::cairo::FontWeight::Bold);
        cr.set_font_size(9.0);
        for spot in &HOTSPOTS {
            let (cx, cy) = badge_centre(spot, top, side);
            cr.new_sub_path();
            cr.arc(cx, cy, BADGE_R + 1.5, 0.0, std::f64::consts::TAU);
            set_rgb(cr, CARD, 1.0);
            let _ = cr.fill();
            cr.arc(cx, cy, BADGE_R, 0.0, std::f64::consts::TAU);
            set_rgb(cr, BADGE, 1.0);
            let _ = cr.fill();
            let label = spot.badge.to_string();
            if let Ok(ext) = cr.text_extents(&label) {
                cr.move_to(cx - ext.width() / 2.0 - ext.x_bearing(), cy - ext.height() / 2.0 - ext.y_bearing());
                set_rgb(cr, (255, 255, 255), 1.0);
                let _ = cr.show_text(&label);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_badge_hits_its_own_button() {
        let top = mouse_ink::parse(mouse_ink::TOP_SVG).unwrap();
        let side = mouse_ink::parse(mouse_ink::SIDE_SVG).unwrap();
        for spot in &HOTSPOTS {
            let (x, y) = badge_centre(spot, &top, &side);
            assert_eq!(hit(x, y, &top, &side).map(|s| s.badge), Some(spot.badge), "badge {}", spot.badge);
        }
        assert!(hit(5.0, 5.0, &top, &side).is_none(), "the card's corner is no button");
    }

    #[test]
    fn badges_and_buttons_are_one_to_one() {
        let mut badges: Vec<u8> = HOTSPOTS.iter().map(|s| s.badge).collect();
        let mut buttons: Vec<u8> = HOTSPOTS.iter().map(|s| s.button).collect();
        badges.sort_unstable();
        buttons.sort_unstable();
        assert_eq!(badges, (1..=11).collect::<Vec<_>>());
        assert_eq!(buttons, (0..=10).collect::<Vec<_>>());
    }

    #[test]
    fn badges_stay_on_the_card() {
        let top = mouse_ink::parse(mouse_ink::TOP_SVG).unwrap();
        let side = mouse_ink::parse(mouse_ink::SIDE_SVG).unwrap();
        for spot in &HOTSPOTS {
            let (x, y) = badge_centre(spot, &top, &side);
            assert!(x > BADGE_R && x < AREA_W - BADGE_R && y > BADGE_R && y < AREA_H - BADGE_R, "badge {}", spot.badge);
        }
    }
}
