//! Macros as the editor and the Macros page show them: the service's steps
//! (a key going down, then up) grouped into rows ("Press and release · Key ·
//! Home"), how much of its slot a macro takes, and the words for how it plays
//! and which buttons use it.

use crate::{
    button_actions::{MEDIA_KEYS, MOUSE_BUTTONS, button_name, shortcut_label},
    service::{MacroStep, Mouse, MouseMacro},
};

/// Bytes in one macro slot before its CRC (a 256-byte sector, spike 10).
pub const SLOT_BYTES: usize = 254;

/// What a key-like step acts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// A HID keyboard usage with modifier bits.
    Key { modifiers: u8, usage: u8 },
    /// A mouse-button mask.
    Button(u16),
    /// A consumer (media) usage.
    Media(u16),
}

/// One row of the editor's steps list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    /// Down then straight up: "Press and release".
    Tap(Target),
    /// Down, with other steps before it comes up: "Hold down".
    Down(Target),
    /// "Let go".
    Up(Target),
    /// "Pause", in milliseconds.
    Pause(u16),
    /// "Wait for release": the rest plays when the button is let go.
    WaitRelease,
}

impl Target {
    fn from_step(step: &MacroStep) -> Option<(Target, bool)> {
        let down = step.kind.ends_with("-down");
        let target = match step.kind {
            "key-down" | "key-up" => Target::Key { modifiers: step.modifiers, usage: step.value as u8 },
            "button-down" | "button-up" => Target::Button(step.value),
            "media-down" | "media-up" => Target::Media(step.value),
            _ => return None,
        };
        Some((target, down))
    }

    fn step(self, down: bool) -> MacroStep {
        let (kind, value, modifiers) = match self {
            Target::Key { modifiers, usage } => (if down { "key-down" } else { "key-up" }, u16::from(usage), modifiers),
            Target::Button(mask) => (if down { "button-down" } else { "button-up" }, mask, 0),
            Target::Media(usage) => (if down { "media-down" } else { "media-up" }, usage, 0),
        };
        MacroStep::new(kind, value, modifiers).expect("a known kind")
    }

    /// "Key", "Mouse button" or "Media key", for the row's subtitle.
    pub fn kind_label(self) -> &'static str {
        match self {
            Target::Key { .. } => "Key",
            Target::Button(_) => "Mouse button",
            Target::Media(_) => "Media key",
        }
    }

    /// What goes in the keycap: "Shift+End", "Left", "Volume up".
    pub fn label(self) -> String {
        match self {
            Target::Key { modifiers, usage } => shortcut_label(modifiers, usage),
            Target::Button(mask) => MOUSE_BUTTONS
                .iter()
                .find(|(v, _)| *v == mask)
                .map_or_else(|| format!("Buttons {mask:#04x}"), |(_, name)| name.trim_end_matches(" click").to_string()),
            Target::Media(usage) => {
                MEDIA_KEYS.iter().find(|(v, _)| *v == usage).map_or_else(|| format!("{usage:#06x}"), |(_, n)| n.to_string())
            }
        }
    }
}

impl Row {
    /// "Press and release", "Hold down", "Let go", "Pause", "Wait for release".
    pub fn title(self) -> &'static str {
        match self {
            Row::Tap(_) => "Press and release",
            Row::Down(_) => "Hold down",
            Row::Up(_) => "Let go",
            Row::Pause(_) => "Pause",
            Row::WaitRelease => "Wait for release",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Row::Tap(t) | Row::Down(t) | Row::Up(t) => t.kind_label(),
            Row::Pause(_) => "Wait before the next step",
            Row::WaitRelease => "The rest plays when the button is let go",
        }
    }
}

/// The service's steps as editor rows: a down followed straight away by
/// its own up is one "Press and release" row.
pub fn rows(steps: &[MacroStep]) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut i = 0;
    while i < steps.len() {
        let step = &steps[i];
        match (step.kind, Target::from_step(step)) {
            ("pause", _) => rows.push(Row::Pause(step.value)),
            ("wait-release", _) => rows.push(Row::WaitRelease),
            (_, Some((target, true))) if steps.get(i + 1).and_then(Target::from_step) == Some((target, false)) => {
                rows.push(Row::Tap(target));
                i += 1;
            }
            (_, Some((target, true))) => rows.push(Row::Down(target)),
            (_, Some((target, false))) => rows.push(Row::Up(target)),
            (_, None) => {}
        }
        i += 1;
    }
    rows
}

/// Editor rows back to the service's steps.
pub fn steps(rows: &[Row]) -> Vec<MacroStep> {
    rows.iter()
        .flat_map(|row| match *row {
            Row::Tap(t) => vec![t.step(true), t.step(false)],
            Row::Down(t) => vec![t.step(true)],
            Row::Up(t) => vec![t.step(false)],
            Row::Pause(ms) => vec![MacroStep::new("pause", ms, 0).expect("a known kind")],
            Row::WaitRelease => vec![MacroStep::new("wait-release", 0, 0).expect("a known kind")],
        })
        .collect()
}

/// Bytes the macro takes on the mouse, with its repeat command and end
/// command: 3 per step, 1 for wait-for-release (macro format 1).
pub fn encoded_size(mode: &str, steps: &[MacroStep]) -> usize {
    let repeat = usize::from(mode != "once");
    repeat + steps.iter().map(|s| if s.kind == "wait-release" { 1 } else { 3 }).sum::<usize>() + 1
}

/// How much of one slot the macro takes, in whole percent, rounded.
pub fn slot_percent(mode: &str, steps: &[MacroStep]) -> u32 {
    let size = encoded_size(mode, steps);
    ((size * 100 + SLOT_BYTES / 2) / SLOT_BYTES) as u32
}

/// Whether the macro fits in one slot.
pub fn fits(mode: &str, steps: &[MacroStep]) -> bool {
    encoded_size(mode, steps) <= SLOT_BYTES
}

/// The play modes the editor offers, as (bus name, label); the three that
/// spike 10 confirmed on the mouse.
pub const PLAY_MODES: [(&str, &str); 3] =
    [("once", "Once"), ("held", "Repeats while held"), ("toggle", "Repeats until pressed again")];

/// "Plays once", "Repeats while held", "Repeats until pressed again".
pub fn plays(mode: &str) -> &'static str {
    match mode {
        "held" => "Repeats while held",
        "toggle" => "Repeats until pressed again",
        _ => "Plays once",
    }
}

/// A macro's title: its name, or "Macro in slot N" when it has none.
pub fn title(m: &MouseMacro) -> String {
    if m.name.is_empty() { format!("Macro in slot {}", m.sector) } else { m.name.clone() }
}

/// Every (profile, button index) that plays the macro in `sector`.
pub fn users(mouse: &Mouse, sector: u8) -> Vec<(u8, usize)> {
    let mut users = Vec::new();
    for (profile, actions) in &mouse.buttons {
        for (index, action) in actions.iter().enumerate() {
            if action.kind == "macro" && action.value == u16::from(sector) {
                users.push((*profile, index));
            }
        }
    }
    users
}

/// "G9 in profile 4", "DPI Up in profiles 1 and 2", "Not on a button".
pub fn used_by(users: &[(u8, usize)]) -> String {
    let mut buttons: Vec<(usize, Vec<u8>)> = Vec::new();
    for &(profile, index) in users {
        match buttons.iter_mut().find(|(i, _)| *i == index) {
            Some((_, profiles)) => profiles.push(profile),
            None => buttons.push((index, vec![profile])),
        }
    }
    if buttons.is_empty() {
        return "Not on a button".into();
    }
    let parts: Vec<String> = buttons
        .into_iter()
        .map(|(index, profiles)| {
            let numbers: Vec<String> = profiles.iter().map(u8::to_string).collect();
            let list = match numbers.as_slice() {
                [one] => format!("profile {one}"),
                [rest @ .., last] => format!("profiles {} and {last}", rest.join(", ")),
                [] => unreachable!(),
            };
            format!("{} in {list}", button_name(index))
        })
        .collect();
    parts.join(", ")
}

/// "1 step", "5 steps".
pub fn step_count(rows: usize) -> String {
    if rows == 1 { "1 step".into() } else { format!("{rows} steps") }
}

/// A row's subtitle on the Macros page: "5 steps · Plays once · G9 in profile 4",
/// with "Made on another computer" for a macro without a name.
pub fn summary(mouse: &Mouse, m: &MouseMacro) -> String {
    let mut parts = vec![step_count(rows(&m.steps).len()), plays(&m.mode).into(), used_by(&users(mouse, m.sector))];
    if m.name.is_empty() {
        parts.push("Made on another computer".into());
    }
    parts.join(" · ")
}

/// Builds steps from key events while recording: each key going down and
/// coming up, and the time between keys as a pause. Time held between a
/// key's down and up isn't kept, so a quick press stays one "Press and
/// release" row; auto-repeat is ignored. Times are the events' own, in ms.
#[derive(Debug, Default)]
pub struct Recorder {
    pub steps: Vec<MacroStep>,
    /// When the last key event happened.
    last: Option<u32>,
    /// Keys down now, with the modifiers they went down with.
    down: Vec<(u8, u8)>,
}

impl Recorder {
    pub fn press(&mut self, time: u32, modifiers: u8, usage: u8) {
        if self.down.iter().any(|&(_, u)| u == usage) {
            return;
        }
        if let Some(last) = self.last {
            let gap = time.saturating_sub(last).min(u32::from(u16::MAX)) as u16;
            if gap > 0 {
                self.steps.push(MacroStep::new("pause", gap, 0).expect("a known kind"));
            }
        }
        self.steps.push(MacroStep::new("key-down", u16::from(usage), modifiers).expect("a known kind"));
        self.down.push((modifiers, usage));
        self.last = Some(time);
    }

    pub fn release(&mut self, time: u32, usage: u8) {
        let Some(i) = self.down.iter().position(|&(_, u)| u == usage) else { return };
        let (modifiers, _) = self.down.remove(i);
        self.steps.push(MacroStep::new("key-up", u16::from(usage), modifiers).expect("a known kind"));
        self.last = Some(time);
    }

    /// Stop: keys still down are let go, so the macro never leaves one held.
    pub fn finish(mut self) -> Vec<MacroStep> {
        for (modifiers, usage) in std::mem::take(&mut self.down) {
            self.steps.push(MacroStep::new("key-up", u16::from(usage), modifiers).expect("a known kind"));
        }
        self.steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(kind: &str, value: u16, modifiers: u8) -> MacroStep {
        MacroStep::new(kind, value, modifiers).unwrap()
    }

    fn copy_line() -> Vec<MacroStep> {
        vec![
            step("key-down", 0x4a, 0),
            step("key-up", 0x4a, 0),
            step("pause", 50, 0),
            step("key-down", 0x4d, 2),
            step("key-up", 0x4d, 2),
            step("pause", 50, 0),
            step("key-down", 0x06, 1),
            step("key-up", 0x06, 1),
        ]
    }

    #[test]
    fn copy_line_is_five_rows() {
        let r = rows(&copy_line());
        assert_eq!(r.len(), 5);
        assert_eq!(r[0], Row::Tap(Target::Key { modifiers: 0, usage: 0x4a }));
        assert_eq!((r[2].title(), r[2].subtitle()), ("Press and release", "Key"));
        assert_eq!(r[1], Row::Pause(50));
        assert_eq!(steps(&r), copy_line());
        // 8 three-byte steps and the end command: 25 bytes, 10% of a slot (spike 10).
        assert_eq!((encoded_size("once", &copy_line()), slot_percent("once", &copy_line())), (25, 10));
    }

    #[test]
    fn held_keys_are_two_rows() {
        let shift = Target::Key { modifiers: 0, usage: 0xe1 };
        let s = [
            step("key-down", 0xe1, 0),
            step("button-down", 1, 0),
            step("button-up", 1, 0),
            step("key-up", 0xe1, 0),
            step("wait-release", 0, 0),
        ];
        let r = rows(&s);
        assert_eq!(r, [Row::Down(shift), Row::Tap(Target::Button(1)), Row::Up(shift), Row::WaitRelease]);
        assert_eq!((r[0].title(), r[2].title()), ("Hold down", "Let go"));
        assert_eq!(steps(&r), s);
        assert_eq!(Target::Button(1).label(), "Left");
        assert_eq!(Target::Media(0x00e9).label(), "Volume up");
        assert_eq!(Target::Key { modifiers: 2, usage: 0x4d }.label(), "Shift+End");
    }

    #[test]
    fn slot_limits() {
        // 42 taps: 252 bytes and the end; with a repeat command, 254: exactly full.
        let taps: Vec<MacroStep> = (0..42).flat_map(|_| [step("key-down", 4, 0), step("key-up", 4, 0)]).collect();
        assert!(fits("held", &taps) && slot_percent("held", &taps) == 100);
        let more: Vec<MacroStep> = taps.iter().copied().chain([step("pause", 5, 0)]).collect();
        assert!(!fits("once", &more));
        assert_eq!(slot_percent("once", &more), 101);
    }

    #[test]
    fn recording_keeps_the_gaps_between_keys() {
        // Home at 1000 ms (held 80 ms), Shift+End 312 ms later, auto-repeat, then Ctrl+C held at Stop.
        let mut r = Recorder::default();
        r.press(1000, 0, 0x4a);
        r.release(1080, 0x4a);
        r.press(1392, 2, 0x4d);
        r.press(1420, 2, 0x4d);
        r.release(1450, 0x4d);
        r.release(1460, 0x4d);
        r.press(1500, 1, 0x06);
        let steps = r.finish();
        let shown = rows(&steps);
        assert_eq!(
            shown,
            [
                Row::Tap(Target::Key { modifiers: 0, usage: 0x4a }),
                Row::Pause(312),
                Row::Tap(Target::Key { modifiers: 2, usage: 0x4d }),
                Row::Pause(50),
                Row::Tap(Target::Key { modifiers: 1, usage: 0x06 }),
            ]
        );
        // A long wait is capped at the longest pause the mouse takes.
        let mut r = Recorder::default();
        r.press(0, 0, 4);
        r.release(10, 4);
        r.press(100_000, 0, 5);
        assert_eq!(rows(&r.finish())[1], Row::Pause(65535));
    }

    #[test]
    fn who_uses_it() {
        assert_eq!(used_by(&[]), "Not on a button");
        assert_eq!(used_by(&[(4, 8)]), "G9 in profile 4");
        assert_eq!(used_by(&[(1, 7), (2, 7)]), "DPI Up in profiles 1 and 2");
        assert_eq!(used_by(&[(1, 7), (2, 7), (3, 7), (4, 8)]), "DPI Up in profiles 1, 2 and 3, G9 in profile 4");
        assert_eq!((step_count(1), step_count(5)), ("1 step".to_string(), "5 steps".to_string()));
        assert_eq!(plays("toggle"), "Repeats until pressed again");
    }
}
