//! The mouse illustration at the top of the Mouse view: the user's G502
//! HERO ink drawings, top view and 3/4 side view, in the lighting colour,
//! with the DPI LEDs and G logo lit and glowing, and a numbered badge on
//! each programmable button, in a card with the callouts. Clicking a button
//! on the drawing reports it like its callout.
//!
//! Breathe and Cycle play on the LEDs, logo and glow while animations are
//! on. Hovering a button, or its callout, fills it and sends a ring out
//! once; applying a change gives it a confirm pulse.

use std::{cell::RefCell, rc::Rc};

use adw::prelude::*;

use crate::{
    art::{self, CARD, Rgb, Shown},
    art_area::{ArtArea, Ticker, rounded_rect, set_rgb},
    callouts::Callouts,
    change_dialog::Handler,
    editor::color_name,
    motion::{self, Loop, ease_out_cubic},
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
/// The hover fill and ring, and the confirm pulse (D5, the `04 Motion` board).
const FILL_MS: u32 = 250;
const RING_MS: u32 = 600;
const CONFIRM_S: f64 = 1.85;

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
    /// The lighting colour before Cycle turns it; `None` while the lights are off.
    base: Option<Rgb>,
    /// The effect's loop and speed, if it moves.
    effect: Option<(Loop, u8)>,
    /// This frame's colour, and its LED, logo and glow level, 0 to 1.
    colour: Option<Rgb>,
    level: f64,
    /// Draw the numbered badges (the mouse's buttons are known).
    badges: bool,
    /// The button the pointer or a callout points at.
    target: Option<u8>,
    /// The button drawn highlighted (it stays while its fill fades), its
    /// fill, 0 to 1, and its ring's progress, 0 to 1 (1 is done).
    hover: Option<u8>,
    fill: f64,
    ring: f64,
    /// The button just changed, and seconds into its confirm pulse.
    confirm: Option<u8>,
    confirm_t: f64,
}

impl State {
    /// Set the frame `t` seconds into the effect's loop, or the still frame.
    fn frame(&mut self, t: Option<f64>) {
        let (colour, level) = match (self.effect, t) {
            (Some((l, speed)), Some(t)) => {
                let p = motion::phase(l, speed, t);
                match l {
                    Loop::Cycle => (self.base.map(|_| art::hue(motion::cycle_degrees(p))), 1.0),
                    Loop::Breathe => (self.base, motion::breathe_level(p)),
                    Loop::Ripple | Loop::Wave => (self.base, 1.0),
                }
            }
            _ => (self.base, 1.0),
        };
        self.colour = colour;
        self.level = if colour.is_some() { level } else { 0.0 };
    }
}

/// An animation of `0..=to` over `ms` that sets a state field and redraws.
fn animation(
    area: &ArtArea,
    state: &Rc<RefCell<State>>,
    to: f64,
    ms: u32,
    set: fn(&mut State, f64),
) -> adw::TimedAnimation {
    let (s, a) = (state.clone(), area.clone());
    let target = adw::CallbackAnimationTarget::new(move |v| {
        set(&mut s.borrow_mut(), v);
        a.queue_draw();
    });
    adw::TimedAnimation::builder().widget(area).value_from(0.0).value_to(to).duration(ms).target(&target).build()
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
    ticker: Rc<Ticker>,
    fill: adw::TimedAnimation,
    ring: adw::TimedAnimation,
    confirm: adw::TimedAnimation,
}

impl MouseArt {
    pub fn new() -> Rc<Self> {
        let inks = Rc::new((
            mouse_ink::parse(mouse_ink::TOP_SVG).expect("the top-view SVG parses (tested)"),
            mouse_ink::parse(mouse_ink::SIDE_SVG).expect("the side-view SVG parses (tested)"),
        ));
        // No ring or pulse until something starts one.
        let state = Rc::new(RefCell::new(State { ring: 1.0, confirm_t: CONFIRM_S, ..State::default() }));
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
        let s = state.clone();
        let ticker = Ticker::new(&area, move |t| s.borrow_mut().frame(t));
        let fill = animation(&area, &state, 1.0, FILL_MS, |st, v| st.fill = v);
        fill.set_easing(adw::Easing::EaseOutCubic);
        let ring = animation(&area, &state, 1.0, RING_MS, |st, v| st.ring = v);
        let confirm = animation(&area, &state, CONFIRM_S, (CONFIRM_S * 1000.0) as u32, |st, v| st.confirm_t = v);
        let art =
            Rc::new(MouseArt { widget, area, hint, callouts, state, inks, on_button, ticker, fill, ring, confirm });
        let weak = Rc::downgrade(&art);
        art.callouts.connect_highlight(move |index| {
            if let Some(art) = weak.upgrade() {
                art.highlight(index.map(|i| i as u8));
            }
        });
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

    /// Show what the mouse is showing at its effect `speed`, and the active
    /// profile's actions (`None` while they're unknown: no badges and no
    /// callouts).
    pub fn show(&self, shown: &Shown, speed: u8, actions: Option<&[ButtonAction]>) {
        self.callouts.show(actions);
        let effect = art::shown_loop(shown).map(|l| (l, speed));
        {
            let mut st = self.state.borrow_mut();
            st.base = art::main_colour(shown, 0.0);
            st.effect = effect;
            st.badges = actions.is_some();
            st.frame(None);
        }
        self.ticker.set_wanted(effect.is_some());
        let description = match shown {
            Shown::Dark => "lights off".into(),
            Shown::Colour { effect, colour } => format!("{effect}, {}", color_name(*colour)),
            Shown::Rainbow { effect } => (*effect).to_string(),
            Shown::Map(_) => "key map".into(),
        };
        self.area.update_property(&[gtk::accessible::Property::Label(&format!("G502 HERO mouse, {description}"))]);
        self.area.queue_draw();
    }

    /// Highlight a button (`None` ends it): the fill comes in over 250 ms
    /// with one ring out from it, and fades out when the highlight ends.
    /// Left Click can't be changed, so it isn't highlighted.
    fn highlight(&self, button: Option<u8>) {
        let button = button.filter(|&b| b != 0);
        let fill = {
            let mut st = self.state.borrow_mut();
            if st.target == button {
                return;
            }
            st.target = button;
            if button.is_some() {
                st.hover = button;
            }
            st.fill
        };
        // The animations call back into the state: no borrow is held here.
        if button.is_some() {
            self.fill.set_value_from(0.0);
            self.fill.set_value_to(1.0);
            self.ring.play();
        } else {
            self.fill.set_value_from(fill);
            self.fill.set_value_to(0.0);
        }
        self.fill.play();
    }

    /// The confirm pulse on a button whose change was applied: a white
    /// flash and a ring, the fill held for a second, then a fade.
    pub fn confirm(&self, button: usize) {
        self.state.borrow_mut().confirm = Some(button as u8);
        self.confirm.play();
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
            let over = hit(ax, ay, &art.inks.0, &art.inks.1).map(|s| s.button).filter(|&b| b != 0);
            art.area.set_cursor_from_name(over.map(|_| "pointer"));
            art.highlight(over);
        });
        let weak = Rc::downgrade(self);
        motion.connect_leave(move |_| {
            if let Some(art) = weak.upgrade() {
                art.highlight(None);
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

/// A button's fill at `fill` (0 to 1), and a ring of (radius, opacity)
/// around its centre, in the view's SVG units.
fn pulse(cr: &gtk::cairo::Context, spot: &Hotspot, colour: Rgb, fill: f64, ring: Option<(f64, f64)>) {
    let (x, y, w, h) = spot.rect;
    if fill > 0.01 {
        rounded_rect(cr, x, y, w, h, 6.0);
        set_rgb(cr, colour, 0.45 * fill.min(1.0));
        let _ = cr.fill();
    }
    if let Some((r, opacity)) = ring {
        cr.new_sub_path();
        cr.arc(x + w / 2.0, y + h / 2.0, r, 0.0, std::f64::consts::TAU);
        set_rgb(cr, colour, opacity);
        cr.set_line_width(5.0);
        let _ = cr.stroke();
    }
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
        // The highlighted button, and the confirm pulse.
        let spot_of = |button: Option<u8>| HOTSPOTS.iter().find(|s| s.view == view && Some(s.button) == button);
        if let Some(spot) = spot_of(st.hover) {
            let ring = (st.ring < 1.0).then(|| (30.0 + ease_out_cubic(st.ring) * 45.0, 1.0 - st.ring));
            pulse(cr, spot, line_rgb, st.fill, ring);
        }
        if let Some(spot) = spot_of(st.confirm).filter(|_| st.confirm_t < CONFIRM_S) {
            let e = st.confirm_t;
            let colour = if e < 0.25 { (255, 255, 255) } else { line_rgb };
            let fill = if e < 1.6 { 1.0 } else { 1.0 - (e - 1.6) / 0.25 };
            let ring = (e < 0.6).then(|| (30.0 + ease_out_cubic(e / 0.6) * 50.0, 1.0 - e / 0.6));
            pulse(cr, spot, colour, fill, ring);
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
