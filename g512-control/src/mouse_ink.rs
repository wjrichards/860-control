//! The G502 HERO ink drawings (`data/g502-top-ink.svg`, `g502-side-ink.svg`,
//! provided by the user) read into layers of path commands for Cairo.
//!
//! Each file has groups `Glow`, `Layer_5`, `DPI_LEDs` and `Logo`, with
//! classes `cls-1` (glow), `cls-2` (outline, drawn as filled shapes) and
//! `cls-3` (LEDs and logo). The paths use only M, L, C and Z.

pub const TOP_SVG: &str = include_str!("../data/g502-top-ink.svg");
pub const SIDE_SVG: &str = include_str!("../data/g502-side-ink.svg");

/// Shapes in a `Logo` group narrower than this are LED dashes, not the logo
/// (the side view keeps its DPI LEDs inside `Logo`).
const LED_MAX_WIDTH: f64 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cmd {
    Move(f64, f64),
    Line(f64, f64),
    Curve(f64, f64, f64, f64, f64, f64),
    Close,
}

pub type Path = Vec<Cmd>;

/// One view's drawing, in the SVG's own units.
#[derive(Debug, Default)]
pub struct Ink {
    pub width: f64,
    pub height: f64,
    pub glow: Vec<Path>,
    pub outline: Vec<Path>,
    pub leds: Vec<Path>,
    pub logo: Vec<Path>,
}

/// Parse path data with absolute M, L, C and Z commands.
pub fn parse_path(d: &str) -> Result<Path, String> {
    let mut out = Vec::new();
    // Command letters stand alone; numbers may be glued to them ("M108.4").
    let spaced: String =
        d.chars().flat_map(|c| if c.is_ascii_alphabetic() { vec![' ', c, ' '] } else { vec![c] }).collect();
    let mut tokens = spaced.split(|c: char| c.is_whitespace() || c == ',').filter(|t| !t.is_empty());
    let num = |tokens: &mut dyn Iterator<Item = &str>| -> Result<f64, String> {
        let t = tokens.next().ok_or("path ends early")?;
        t.parse::<f64>().map_err(|_| format!("bad number {t:?}"))
    };
    while let Some(cmd) = tokens.next() {
        out.push(match cmd {
            "M" => Cmd::Move(num(&mut tokens)?, num(&mut tokens)?),
            "L" => Cmd::Line(num(&mut tokens)?, num(&mut tokens)?),
            "C" => Cmd::Curve(
                num(&mut tokens)?,
                num(&mut tokens)?,
                num(&mut tokens)?,
                num(&mut tokens)?,
                num(&mut tokens)?,
                num(&mut tokens)?,
            ),
            "Z" => Cmd::Close,
            other => return Err(format!("unsupported command {other:?}")),
        });
    }
    Ok(out)
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {name}=\"");
    let start = tag.find(&key)? + key.len();
    Some(&tag[start..start + tag[start..].find('"')?])
}

fn width_of(path: &Path) -> f64 {
    let xs = path.iter().flat_map(|c| match *c {
        Cmd::Move(x, _) | Cmd::Line(x, _) => vec![x],
        Cmd::Curve(x1, _, x2, _, x, _) => vec![x1, x2, x],
        Cmd::Close => vec![],
    });
    let (lo, hi) = xs.fold((f64::MAX, f64::MIN), |(lo, hi), x| (lo.min(x), hi.max(x)));
    if lo > hi { 0.0 } else { hi - lo }
}

/// Read one ink file into its layers.
pub fn parse(svg: &str) -> Result<Ink, String> {
    let mut ink = Ink::default();
    let view_box = attr(svg, "viewBox").ok_or("no viewBox")?;
    let vb: Vec<f64> = view_box
        .split_whitespace()
        .map(|v| v.parse().map_err(|_| format!("viewBox {view_box:?}")))
        .collect::<Result<_, _>>()?;
    let [_, _, w, h] = vb[..] else { return Err(format!("viewBox {view_box:?}")) };
    (ink.width, ink.height) = (w, h);
    // Walk the tags in order, tracking the named group we're in.
    let mut groups: Vec<Option<&str>> = Vec::new();
    for raw in svg.split('<').skip(1) {
        let tag = raw.split('>').next().unwrap_or("");
        if tag.starts_with("g") && (tag.len() == 1 || tag.as_bytes()[1] == b' ') {
            groups.push(attr(tag, "id"));
        } else if tag.starts_with("/g") {
            groups.pop();
        } else if tag.starts_with("path ") {
            let d = attr(tag, "d").ok_or("path without d")?;
            let path = parse_path(d)?;
            let group = groups.iter().rev().flatten().next().copied();
            match (attr(tag, "class"), group) {
                (Some("cls-1"), _) => ink.glow.push(path),
                (Some("cls-2"), _) => ink.outline.push(path),
                (Some("cls-3"), Some("DPI_LEDs")) => ink.leds.push(path),
                (Some("cls-3"), _) if width_of(&path) < LED_MAX_WIDTH => ink.leds.push(path),
                (Some("cls-3"), _) => ink.logo.push(path),
                (class, _) => return Err(format!("unknown class {class:?}")),
            }
        }
    }
    Ok(ink)
}

/// Add `path` to the Cairo context's current path.
pub fn trace(cr: &gtk::cairo::Context, path: &Path) {
    for cmd in path {
        match *cmd {
            Cmd::Move(x, y) => cr.move_to(x, y),
            Cmd::Line(x, y) => cr.line_to(x, y),
            Cmd::Curve(x1, y1, x2, y2, x, y) => cr.curve_to(x1, y1, x2, y2, x, y),
            Cmd::Close => cr.close_path(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_files_parse_into_their_layers() {
        let top = parse(TOP_SVG).unwrap();
        assert_eq!((top.width.round(), top.height.round()), (425.0, 767.0));
        assert_eq!((top.outline.len(), top.leds.len(), top.logo.len(), top.glow.len()), (52, 3, 1, 2));
        let side = parse(SIDE_SVG).unwrap();
        assert_eq!((side.width.round(), side.height.round()), (770.0, 250.0));
        assert_eq!((side.outline.len(), side.leds.len(), side.logo.len(), side.glow.len()), (41, 3, 1, 2));
    }

    #[test]
    fn path_commands() {
        assert_eq!(
            parse_path("M1 2 L3 4 C5 6 7 8 9 10 Z").unwrap(),
            vec![Cmd::Move(1.0, 2.0), Cmd::Line(3.0, 4.0), Cmd::Curve(5.0, 6.0, 7.0, 8.0, 9.0, 10.0), Cmd::Close]
        );
        assert!(parse_path("M1 2 Q3 4 5 6").is_err(), "unsupported command");
        assert!(parse_path("M1").is_err(), "too few numbers");
    }

    #[test]
    fn unknown_class_is_an_error() {
        let svg = r#"<svg viewBox="0 0 10 10"><g id="Layer_5"><path d="M0 0 L1 1 Z" class="cls-9"/></g></svg>"#;
        assert!(parse(svg).unwrap_err().contains("unknown class"));
    }
}
