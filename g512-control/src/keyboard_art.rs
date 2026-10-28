//! The keyboard illustration at the top of the Keyboard view: lit keys as
//! solid tiles in their colour, unlit keys as outlines, in a rounded case.
//! It is a button: clicking it, or Enter while focused, opens Key Maps.

use std::{cell::RefCell, rc::Rc};

use gtk::prelude::*;

use crate::{
    art::{self, CARD, Rgb, Shown},
    art_area::{ArtArea, rounded_rect, set_rgb},
    editor::color_name,
    layout::KEYS,
    service::{Keys, LightingState},
};

/// The board's proportions: a 694 x 246 card around a 658 x 210 drawing.
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

pub struct KeyboardArt {
    pub widget: gtk::Box,
    button: gtk::Button,
    area: ArtArea,
    drawn: Rc<RefCell<Drawn>>,
}

impl KeyboardArt {
    pub fn new() -> Rc<Self> {
        let drawn: Rc<RefCell<Drawn>> = Rc::default();
        let d = drawn.clone();
        let area = ArtArea::new(CARD_W / CARD_H, CARD_W as i32, Box::new(move |cr, w, h| draw(cr, w, h, &d.borrow())));
        let button = gtk::Button::builder().child(&area).css_classes(["flat", "device-art"]).build();
        let caption = gtk::Label::builder()
            .label("Click the keyboard to edit Key Maps")
            .css_classes(["dim-label", "caption"])
            .build();
        let widget = gtk::Box::new(gtk::Orientation::Vertical, 8);
        widget.append(&button);
        widget.append(&caption);
        Rc::new(KeyboardArt { widget, button, area, drawn })
    }

    /// `f` runs when the illustration is clicked or activated.
    pub fn connect_activate(&self, f: impl Fn() + 'static) {
        self.button.connect_clicked(move |_| f());
    }

    /// Show what the keyboard is showing.
    pub fn show(&self, lights_on: bool, gaming_shown: bool, state: &LightingState, custom: &Keys, gaming: &Keys) {
        let shown = art::shown(lights_on, gaming_shown, state, custom, gaming);
        *self.drawn.borrow_mut() = Drawn { keys: art::key_colours(&shown, &|_| 1.0), main: art::main_colour(&shown) };
        self.button.update_property(&[gtk::accessible::Property::Label(&format!(
            "G512 keyboard, {}. Open Key Maps",
            describe(&shown, gaming_shown)
        ))]);
        self.area.queue_draw();
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
    // The card.
    rounded_rect(cr, 0.0, 0.0, CARD_W, CARD_H, 12.0);
    set_rgb(cr, CARD, 1.0);
    let _ = cr.fill();
    cr.translate(INSET, INSET);
    let line = drawn.main.map(|c| art::drawing_colour(c, CARD));
    let (line_rgb, case_alpha, outline_alpha) = match line {
        Some(c) => (c, 1.0, 0.25),
        None => ((255, 255, 255), 0.3, 0.21),
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
                set_rgb(cr, art::drawing_colour(*colour, CARD), *level);
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
