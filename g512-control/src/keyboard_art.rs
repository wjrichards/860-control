//! The keyboard illustration on Keyboard › Lighting, Keyboard › Games and the
//! narrow sidebar list: lit keys as solid tiles in their colour, unlit keys
//! as outlines, in a rounded case, on a card that takes the page's free
//! height. It is a button: clicking it, or Enter while focused, opens Key
//! Maps. Moving effects play on it while animations are on.

use std::{cell::RefCell, rc::Rc};

use gtk::prelude::*;

use crate::{
    art::{self, Rgb, Shown},
    art_area::{ArtArea, Ticker, is_dark, rounded_rect, set_rgb},
    editor::color_name,
    layout::KEYS,
    motion::{self, Loop},
    service::{Keys, LightingState},
};

/// The board's proportions: a 694 x 246 frame around a 658 x 210 drawing.
const CARD_W: f64 = 694.0;
const CARD_H: f64 = 246.0;
const INSET: f64 = 18.0;
/// Pixels per quarter key, and the drawing's own padding (as on the board).
const UNIT: f64 = 7.0;
const PAD: f64 = 14.0;

#[derive(Default)]
struct Drawn {
    /// In `KEYS` order: a lit key's colour and level, or `None` for an outline.
    keys: Vec<Option<(Rgb, f64)>>,
    /// The case and outline colour; `None` draws them dim white.
    main: Option<Rgb>,
}

/// What the keyboard shows, kept so each frame can be drawn from it.
struct Scene {
    lights_on: bool,
    gaming_shown: bool,
    state: LightingState,
    custom: Keys,
    gaming: Keys,
}

impl Scene {
    fn shown(&self) -> Shown<'_> {
        art::shown(self.lights_on, self.gaming_shown, &self.state, &self.custom, &self.gaming)
    }

    /// The frame `t` seconds into the effect's loop, or the still frame.
    fn frame(&self, t: Option<f64>) -> Drawn {
        let shown = self.shown();
        let (l, p) = match (art::shown_loop(&shown), t) {
            (Some(l), Some(t)) => (Some(l), motion::phase(l, self.state.speed, t)),
            _ => (None, 0.0),
        };
        let cycle = if l == Some(Loop::Cycle) { motion::cycle_degrees(p) } else { 0.0 };
        let level = |key: &_| l.map_or(1.0, |l| motion::key_level(l, p, key));
        Drawn { keys: art::key_colours(&shown, &level, cycle), main: art::main_colour(&shown, cycle) }
    }
}

pub struct KeyboardArt {
    /// The card: the drawing, and the hint under it when there is one.
    pub widget: gtk::Box,
    button: gtk::Button,
    hint: gtk::Label,
    area: ArtArea,
    scene: Rc<RefCell<Option<Scene>>>,
    ticker: Rc<Ticker>,
}

impl KeyboardArt {
    /// A card whose drawing takes the page's free height, with `hint` under
    /// it (none when empty).
    pub fn new(hint: &str) -> Rc<Self> {
        let drawn: Rc<RefCell<Drawn>> = Rc::default();
        let d = drawn.clone();
        let area =
            ArtArea::filling(CARD_W / CARD_H, CARD_W as i32, 1.0, Box::new(move |cr, w, h| draw(cr, w, h, &d.borrow())));
        let button =
            gtk::Button::builder().child(&area).vexpand(true).css_classes(["flat", "device-art"]).build();
        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .vexpand(true)
            .css_classes(["drawing-card"])
            .build();
        widget.append(&button);
        let hint = gtk::Label::builder()
            .label(hint)
            .wrap(true)
            .visible(!hint.is_empty())
            .css_classes(["dim-label", "caption"])
            .build();
        widget.append(&hint);
        let scene: Rc<RefCell<Option<Scene>>> = Rc::default();
        let s = scene.clone();
        let ticker = Ticker::new(&area, move |t| {
            if let Some(scene) = s.borrow().as_ref() {
                *drawn.borrow_mut() = scene.frame(t);
            }
        });
        Rc::new(KeyboardArt { widget, button, hint, area, scene, ticker })
    }

    /// The hint under the drawing (a shorter one fits a narrow window).
    pub fn set_hint(&self, hint: &str) {
        self.hint.set_label(hint);
    }

    /// `f` runs when the illustration is clicked or activated.
    pub fn connect_activate(&self, f: impl Fn() + 'static) {
        self.button.connect_clicked(move |_| f());
    }

    /// Show what the keyboard is showing.
    pub fn show(&self, lights_on: bool, gaming_shown: bool, state: &LightingState, custom: &Keys, gaming: &Keys) {
        let scene =
            Scene { lights_on, gaming_shown, state: state.clone(), custom: custom.clone(), gaming: gaming.clone() };
        let shown = scene.shown();
        self.button.update_property(&[gtk::accessible::Property::Label(&format!(
            "G512 keyboard, {}. Open Key Maps",
            describe(&shown, gaming_shown)
        ))]);
        let moving = art::shown_loop(&shown).is_some();
        *self.scene.borrow_mut() = Some(scene);
        self.ticker.set_wanted(moving);
        self.area.queue_draw();
    }

    /// Show the Gaming map, whatever the keyboard is showing (the Games page).
    pub fn show_gaming(&self, gaming: &Keys) {
        let state = LightingState { effect: "Static".into(), color: (0, 0, 0), speed: 0, brightness: 100 };
        self.show(true, true, &state, &Keys::new(), gaming);
    }
}

/// The spoken description, e.g. "Ripple, green" or "lights off".
fn describe(shown: &Shown, gaming_shown: bool) -> String {
    match shown {
        Shown::Dark => "lights off".into(),
        Shown::Colour { effect, colour } => format!("{effect}, {}", color_name(*colour)),
        Shown::Rainbow { effect } => (*effect).to_string(),
        Shown::Map(_) if gaming_shown => "Gaming map".into(),
        Shown::Map(_) => "Custom map".into(),
    }
}

fn draw(cr: &gtk::cairo::Context, w: f64, h: f64, drawn: &Drawn) {
    let s = (w / CARD_W).min(h / CARD_H);
    let (ox, oy) = ((w - CARD_W * s) / 2.0, (h - CARD_H * s) / 2.0);
    cr.translate(ox, oy);
    cr.scale(s, s);
    // The card is the widget's background (`.drawing-card`); it follows the style.
    let card = art::card(is_dark());
    cr.translate(INSET, INSET);
    let line = drawn.main.map(|c| art::drawing_colour(c, card));
    let (line_rgb, case_alpha, outline_alpha) = match line {
        Some(c) => (c, 1.0, 0.25),
        None => (art::dim_ink(card), 0.3, 0.21),
    };
    // The case.
    let (dw, dh) = (90.0 * UNIT + 2.0 * PAD, 26.0 * UNIT + 2.0 * PAD);
    rounded_rect(cr, 2.5, 2.5, dw - 5.0, dh - 5.0, 16.0);
    set_rgb(cr, line_rgb, case_alpha);
    cr.set_line_width(3.0);
    let _ = cr.stroke();
    // The keys.
    cr.set_line_width(1.5);
    for (key, fill) in KEYS.iter().zip(&drawn.keys) {
        let x = PAD + f64::from(key.x) * UNIT + 2.0;
        let y = PAD + f64::from(key.y) * UNIT + 2.0;
        let (kw, kh) = (f64::from(key.w) * UNIT - 4.0, f64::from(key.h) * UNIT - 4.0);
        match fill {
            Some((colour, level)) => {
                rounded_rect(cr, x, y, kw, kh, 3.5);
                set_rgb(cr, art::drawing_colour(*colour, card), *level);
                let _ = cr.fill();
            }
            None => {
                rounded_rect(cr, x + 0.75, y + 0.75, kw - 1.5, kh - 1.5, 3.0);
                set_rgb(cr, line_rgb, outline_alpha);
                let _ = cr.stroke();
            }
        }
    }
}
