//! The macro editor (boards "Macro editor · Copy line", "· recording",
//! "· editing steps" and the three "error" boards): a dialog to name a macro,
//! choose how it plays, record keys, edit the steps, and save it to the
//! mouse or delete it.
//!
//! Recording takes key events in this dialog only, so nothing outside the
//! app is listened to; mouse clicks aren't recorded (a click on Stop would
//! be). Mouse-button and media-key steps come from Add Step.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use crate::{
    button_actions::{MEDIA_KEYS, MOUSE_BUTTONS, is_modifier, modifier_bits, unshifted, usage},
    change_dialog::Handler,
    macro_steps::{self, PLAY_MODES, Recorder, Row, Target},
    service::{MacroProblem, MacroStep},
};

/// A new pause from Add Step.
const NEW_PAUSE_MS: u16 = 50;

/// What the editor starts from.
pub struct MacroDraft {
    /// `None` for a new macro.
    pub sector: Option<u8>,
    pub name: String,
    pub mode: String,
    pub steps: Vec<MacroStep>,
    /// Where it is used ("G9 in profile 4"); `None` for a new macro.
    pub used_by: Option<String>,
    /// How many buttons play it, for the delete confirmation.
    pub users: usize,
}

/// A save request: (sector or `None` for new, name, play mode, steps).
pub type SaveRequest = (Option<u8>, String, String, Vec<MacroStep>);

/// What an Add Step or Move menu item does, given its `u16` parameter.
type MenuAction = Box<dyn Fn(&MacroEditor, Option<u16>)>;

pub struct MacroEditor {
    pub dialog: adw::Dialog,
    save: gtk::Button,
    banner: adw::Banner,
    name_row: adw::EntryRow,
    plays_row: adw::ComboRow,
    steps_group: adw::PreferencesGroup,
    record: gtk::Button,
    record_icon: gtk::Image,
    record_label: gtk::Label,
    list: gtk::ListBox,
    delete: gtk::Button,
    draft_sector: Option<u8>,
    used_by: Option<String>,
    users: usize,
    rows: RefCell<Vec<Row>>,
    /// While recording: what has been heard, and when Record was pressed.
    recording: RefCell<Option<(Recorder, Instant)>>,
    tick: RefCell<Option<glib::SourceId>>,
    /// The row whose key is being changed by pressing a new one.
    capturing: Cell<Option<usize>>,
    connected: Cell<bool>,
    problem: Cell<Option<MacroProblem>>,
    on_save: Handler<SaveRequest>,
    on_delete: Handler<u8>,
    /// Row widgets call back into the editor; they hold it weakly.
    me: std::rc::Weak<MacroEditor>,
}

impl MacroEditor {
    pub fn new(draft: MacroDraft) -> Rc<Self> {
        let cancel = gtk::Button::with_label("Cancel");
        let save = gtk::Button::builder().label("Save").css_classes(["suggested-action"]).build();
        let header = adw::HeaderBar::builder().show_start_title_buttons(false).show_end_title_buttons(false).build();
        header.pack_start(&cancel);
        header.pack_end(&save);
        let banner = adw::Banner::new("");

        let name_row = adw::EntryRow::builder().title("Name").text(&draft.name).build();
        let mode_labels: Vec<&str> = PLAY_MODES.iter().map(|&(_, label)| label).collect();
        let plays_row =
            adw::ComboRow::builder().title("Plays").model(&gtk::StringList::new(&mode_labels)).build();
        plays_row.set_selected(PLAY_MODES.iter().position(|&(m, _)| m == draft.mode).unwrap_or(0) as u32);
        let settings = adw::PreferencesGroup::new();
        settings.add(&name_row);
        settings.add(&plays_row);
        if let Some(used_by) = &draft.used_by {
            let used_row = adw::ActionRow::builder().title("Used by").build();
            used_row.add_suffix(
                &gtk::Label::builder()
                    .label(used_by)
                    .css_classes(["dim-label"])
                    .ellipsize(gtk::pango::EllipsizeMode::End)
                    .build(),
            );
            settings.add(&used_row);
        }

        // The dot is red while idle, like the boards' Record button.
        let record_icon = gtk::Image::builder().icon_name("media-record-symbolic").css_classes(["error"]).build();
        let record_label = gtk::Label::new(Some("Record"));
        let record_content = gtk::Box::builder().spacing(6).build();
        record_content.append(&record_icon);
        record_content.append(&record_label);
        let record = gtk::Button::builder().child(&record_content).valign(gtk::Align::Center).build();
        let steps_group = adw::PreferencesGroup::builder().title("Steps").header_suffix(&record).build();
        let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).css_classes(["boxed-list"]).build();
        steps_group.add(&list);

        let delete = gtk::Button::builder()
            .label("Delete Macro")
            .halign(gtk::Align::Center)
            .css_classes(["destructive-action"])
            .visible(draft.sector.is_some())
            .build();
        let delete_group = adw::PreferencesGroup::new();
        delete_group.add(&delete);

        let page = adw::PreferencesPage::new();
        page.add(&settings);
        page.add(&steps_group);
        page.add(&delete_group);
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.add_top_bar(&banner);
        toolbar.set_content(Some(&page));
        let title = if draft.sector.is_some() && !draft.name.is_empty() { draft.name.clone() } else { "New Macro".into() };
        let dialog = adw::Dialog::builder().title(&title).content_width(560).content_height(680).child(&toolbar).build();

        let editor = Rc::new_cyclic(|me| MacroEditor {
            dialog,
            save,
            banner,
            name_row,
            plays_row,
            steps_group,
            record,
            record_icon,
            record_label,
            list,
            delete,
            draft_sector: draft.sector,
            used_by: draft.used_by,
            users: draft.users,
            rows: RefCell::new(macro_steps::rows(&draft.steps)),
            recording: RefCell::new(None),
            tick: RefCell::new(None),
            capturing: Cell::new(None),
            connected: Cell::new(true),
            problem: Cell::new(None),
            on_save: RefCell::new(None),
            on_delete: RefCell::new(None),
            me: me.clone(),
        });
        editor.wire(&cancel);
        editor.render();
        editor
    }

    /// `on_save` gets the macro to write; answer with [`MacroEditor::show_problem`]
    /// or close the dialog.
    pub fn connect_save(&self, f: impl Fn(SaveRequest) + 'static) {
        *self.on_save.borrow_mut() = Some(Box::new(f));
    }

    /// `on_delete` gets the sector, once the user has confirmed.
    pub fn connect_delete(&self, f: impl Fn(u8) + 'static) {
        *self.on_delete.borrow_mut() = Some(Box::new(f));
    }

    /// Whether the mouse is there to save to; the editor keeps its steps either way.
    pub fn set_connected(&self, connected: bool) {
        if self.connected.replace(connected) != connected {
            if connected {
                self.problem.set(None);
            }
            self.render();
        }
    }

    /// The service refused the save; the steps stay.
    pub fn show_problem(&self, problem: MacroProblem) {
        self.problem.set(Some(problem));
        self.render();
    }

    /// For screenshots against the boards (`G512_CONTROL_MACRO_STATE`):
    /// `recording` (with Home, a pause and Shift+End heard), `capture` (the
    /// first step waiting for a new key), `write-failed`, `memory-full` or
    /// `delete` (the confirmation).
    pub fn debug_state(self: &Rc<Self>, state: &str) {
        match state {
            "recording" => {
                self.start_recording();
                if let Some((recorder, _)) = self.recording.borrow_mut().as_mut() {
                    recorder.press(0, 0, 0x4a);
                    recorder.release(80, 0x4a);
                    recorder.press(392, 2, 0x4d);
                    recorder.release(450, 0x4d);
                }
                self.render();
            }
            "capture" => self.capture(0),
            "write-failed" => self.show_problem(MacroProblem::WriteFailed),
            "memory-full" => self.show_problem(MacroProblem::MemoryFull),
            "delete" => self.confirm_delete(),
            _ => {}
        }
    }

    fn wire(self: &Rc<Self>, cancel: &gtk::Button) {
        let weak = Rc::downgrade(self);
        cancel.connect_clicked(move |_| {
            if let Some(e) = weak.upgrade() {
                e.stop_recording(false);
                e.dialog.close();
            }
        });
        let weak = Rc::downgrade(self);
        self.save.connect_clicked(move |_| {
            if let Some(e) = weak.upgrade() {
                e.request_save();
            }
        });
        let weak = Rc::downgrade(self);
        self.banner.connect_button_clicked(move |_| {
            if let Some(e) = weak.upgrade() {
                e.request_save();
            }
        });
        let weak = Rc::downgrade(self);
        self.record.connect_clicked(move |_| {
            let Some(e) = weak.upgrade() else { return };
            if e.recording.borrow().is_some() { e.stop_recording(true) } else { e.start_recording() }
        });
        let weak = Rc::downgrade(self);
        self.plays_row.connect_selected_notify(move |_| {
            if let Some(e) = weak.upgrade() {
                e.render();
            }
        });
        let weak = Rc::downgrade(self);
        self.delete.connect_clicked(move |_| {
            if let Some(e) = weak.upgrade() {
                e.confirm_delete();
            }
        });

        // Keys: recording, or the new key for a step.
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(self);
        keys.connect_key_pressed(move |ctrl, keyval, keycode, state| {
            let Some(e) = weak.upgrade() else { return glib::Propagation::Proceed };
            e.key(ctrl, keyval, keycode, state, true)
        });
        let weak = Rc::downgrade(self);
        keys.connect_key_released(move |ctrl, keyval, keycode, state| {
            if let Some(e) = weak.upgrade() {
                e.key(ctrl, keyval, keycode, state, false);
            }
        });
        self.dialog.add_controller(keys);

        // Add Step's menu: Key…, a mouse button, a media key, Pause.
        let actions = gio::SimpleActionGroup::new();
        let add = |name: &str, param: Option<&glib::VariantTy>, f: MenuAction| {
            let action = gio::SimpleAction::new(name, param);
            let weak = Rc::downgrade(self);
            action.connect_activate(move |_, value| {
                if let Some(e) = weak.upgrade() {
                    f(&e, value.and_then(|v| v.get::<u16>()));
                }
            });
            actions.add_action(&action);
        };
        add(
            "add-key",
            None,
            Box::new(|e, _| {
                let index = e.push(Row::Tap(Target::Key { modifiers: 0, usage: 0x04 }));
                e.capture(index);
            }),
        );
        add(
            "add-button",
            Some(glib::VariantTy::UINT16),
            Box::new(|e, v| {
                e.push(Row::Tap(Target::Button(v.unwrap_or(1))));
            }),
        );
        add(
            "add-media",
            Some(glib::VariantTy::UINT16),
            Box::new(|e, v| {
                e.push(Row::Tap(Target::Media(v.unwrap_or(0x00cd))));
            }),
        );
        add(
            "add-pause",
            None,
            Box::new(|e, _| {
                e.push(Row::Pause(NEW_PAUSE_MS));
            }),
        );
        add(
            "move",
            Some(glib::VariantTy::UINT16),
            Box::new(|e, v| {
                // The value packs (index << 1) | down.
                let v = usize::from(v.unwrap_or(0));
                let (index, down) = (v >> 1, v & 1 == 1);
                let to = if down { index + 1 } else { index.wrapping_sub(1) };
                e.move_row(index, to);
            }),
        );
        self.dialog.insert_action_group("editor", Some(&actions));
    }

    /// Append a row; returns its index.
    fn push(&self, row: Row) -> usize {
        let index = {
            let mut rows = self.rows.borrow_mut();
            rows.push(row);
            rows.len() - 1
        };
        self.render();
        index
    }

    fn move_row(&self, from: usize, to: usize) {
        {
            let mut rows = self.rows.borrow_mut();
            if from >= rows.len() || to >= rows.len() || from == to {
                return;
            }
            let row = rows.remove(from);
            rows.insert(to, row);
        }
        self.render();
    }

    fn remove_row(&self, index: usize) {
        if index < self.rows.borrow().len() {
            self.rows.borrow_mut().remove(index);
            self.capturing.set(None);
            self.render();
        }
    }

    fn set_row(&self, index: usize, row: Row) {
        if let Some(r) = self.rows.borrow_mut().get_mut(index) {
            *r = row;
        }
        self.render();
    }

    /// The next key pressed becomes row `index`'s key (Esc cancels).
    fn capture(&self, index: usize) {
        self.capturing.set(Some(index));
        self.render();
        self.dialog.grab_focus();
    }

    fn mode(&self) -> &'static str {
        PLAY_MODES.get(self.plays_row.selected() as usize).map_or("once", |&(m, _)| m)
    }

    fn steps(&self) -> Vec<MacroStep> {
        macro_steps::steps(&self.rows.borrow())
    }

    fn start_recording(self: &Rc<Self>) {
        self.capturing.set(None);
        self.problem.set(None);
        *self.recording.borrow_mut() = Some((Recorder::default(), Instant::now()));
        let weak = Rc::downgrade(self);
        let tick = glib::timeout_add_local(Duration::from_millis(250), move || {
            let Some(e) = weak.upgrade() else { return glib::ControlFlow::Break };
            e.render_recording();
            glib::ControlFlow::Continue
        });
        if let Some(old) = self.tick.replace(Some(tick)) {
            old.remove();
        }
        self.render();
        self.dialog.grab_focus();
    }

    /// Stop recording; `keep` adds what was heard after the existing steps.
    fn stop_recording(&self, keep: bool) {
        if let Some(tick) = self.tick.take() {
            tick.remove();
        }
        let Some((recorder, _)) = self.recording.take() else { return };
        if keep {
            let heard = macro_steps::rows(&recorder.finish());
            self.rows.borrow_mut().extend(heard);
        }
        self.render();
    }

    fn key(
        &self,
        ctrl: &gtk::EventControllerKey,
        keyval: gdk::Key,
        keycode: u32,
        state: gdk::ModifierType,
        pressed: bool,
    ) -> glib::Propagation {
        if self.recording.borrow().is_some() {
            // While recording every key is the macro's, Esc included.
            if !is_modifier(keyval)
                && let Some(usage) = usage(unshifted(ctrl, keyval, keycode))
            {
                let time = ctrl.current_event_time();
                if let Some((recorder, _)) = self.recording.borrow_mut().as_mut() {
                    if pressed { recorder.press(time, modifier_bits(state), usage) } else { recorder.release(time, usage) }
                }
                self.render();
            }
            return glib::Propagation::Stop;
        }
        let Some(index) = self.capturing.get() else { return glib::Propagation::Proceed };
        if !pressed || is_modifier(keyval) {
            return glib::Propagation::Stop;
        }
        let modifiers = modifier_bits(state);
        if keyval == gdk::Key::Escape && modifiers == 0 {
            self.capturing.set(None);
            self.render();
            return glib::Propagation::Stop;
        }
        if let Some(usage) = usage(unshifted(ctrl, keyval, keycode)) {
            self.capturing.set(None);
            let old = self.rows.borrow().get(index).copied();
            let target = Target::Key { modifiers, usage };
            let row = match old {
                Some(Row::Down(_)) => Row::Down(target),
                Some(Row::Up(_)) => Row::Up(target),
                _ => Row::Tap(target),
            };
            self.set_row(index, row);
        }
        glib::Propagation::Stop
    }

    fn request_save(&self) {
        if !self.save.is_sensitive() && self.problem.get() != Some(MacroProblem::WriteFailed) {
            return;
        }
        self.problem.set(None);
        self.render();
        let request = (self.draft_sector, self.name_row.text().trim().to_string(), self.mode().to_string(), self.steps());
        if let Some(f) = self.on_save.borrow().as_ref() {
            f(request);
        }
    }

    fn confirm_delete(self: &Rc<Self>) {
        let Some(sector) = self.draft_sector else { return };
        let name = self.name_row.text();
        let name = if name.is_empty() { format!("Macro in slot {sector}") } else { name.to_string() };
        let body = match (&self.used_by, self.users) {
            (Some(used_by), 1) => format!(
                "{used_by} plays this macro. After it's deleted, that button does nothing until you choose \
                 something else for it."
            ),
            (Some(used_by), n) if n > 1 => format!(
                "{used_by} play this macro. After it's deleted, those buttons do nothing until you choose \
                 something else for them."
            ),
            _ => "No button plays it.".to_string(),
        };
        let alert = adw::AlertDialog::new(Some(&format!("Delete “{name}”?")), Some(&body));
        alert.add_responses(&[("cancel", "Cancel"), ("delete", "Delete")]);
        alert.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
        alert.set_default_response(Some("cancel"));
        alert.set_close_response("cancel");
        let weak = Rc::downgrade(self);
        alert.connect_response(Some("delete"), move |_, _| {
            let Some(e) = weak.upgrade() else { return };
            if let Some(f) = e.on_delete.borrow().as_ref() {
                f(sector);
            }
        });
        alert.present(Some(&self.dialog));
    }

    /// Only what changes every tick while recording: the timer and the last row.
    fn render_recording(&self) {
        let recording = self.recording.borrow();
        let Some((recorder, started)) = recording.as_ref() else { return };
        let secs = started.elapsed().as_secs();
        self.record_label.set_label(&format!("Stop · {}:{:02}", secs / 60, secs % 60));
        let mut steps = self.steps();
        steps.extend(recorder.steps.iter().copied());
        let percent = macro_steps::slot_percent(self.mode(), &steps);
        if let Some(row) = self.list.last_child().and_downcast::<adw::ActionRow>() {
            row.set_subtitle(&format!("uses {percent}% so far"));
        }
    }

    /// Everything from the current state: rows, caption, banner, buttons.
    fn render(&self) {
        let recording = self.recording.borrow().is_some();
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        let rows = self.rows.borrow().clone();
        for (index, row) in rows.iter().enumerate() {
            self.list.append(&self.row_widget(index, *row, rows.len(), recording));
        }
        // What has been heard so far, dimmed until Stop.
        let heard = self.recording.borrow().as_ref().map(|(r, _)| macro_steps::rows(&r.steps)).unwrap_or_default();
        for (index, row) in heard.iter().enumerate() {
            self.list.append(&self.row_widget(rows.len() + index, *row, rows.len() + heard.len(), true));
        }
        if recording {
            let waiting = adw::ActionRow::builder().title("Waiting for the next key…").css_classes(["dim-label"]).build();
            waiting.add_prefix(&gtk::Image::builder().icon_name("media-record-symbolic").css_classes(["error"]).build());
            self.list.append(&waiting);
        } else {
            self.list.append(&self.add_step_row());
        }

        let mode = self.mode();
        let steps = self.steps();
        let fits = macro_steps::fits(mode, &steps);
        let percent = macro_steps::slot_percent(mode, &steps);
        let count = macro_steps::step_count(rows.len());
        self.steps_group.set_description(Some(&if recording {
            "Recording. Press keys in this window; pauses between them are kept.".to_string()
        } else if fits {
            format!("{count} · uses {percent}% of its slot · drag to reorder")
        } else {
            format!("{count} · needs {percent}% of its slot · drag to reorder")
        }));

        if recording {
            self.record_icon.set_icon_name(Some("media-playback-stop-symbolic"));
            self.record_icon.remove_css_class("error");
            self.record.add_css_class("destructive-action");
            self.render_recording();
        } else {
            self.record_icon.set_icon_name(Some("media-record-symbolic"));
            self.record_icon.add_css_class("error");
            self.record_label.set_label("Record");
            self.record.remove_css_class("destructive-action");
        }

        let connected = self.connected.get();
        let (banner, button) = if !connected {
            (Some("Mouse disconnected. Plug it back in to save.".to_string()), None)
        } else if !fits {
            (Some(format!("This macro needs {percent}% of a slot. Remove some steps to save it.")), None)
        } else {
            match self.problem.get() {
                Some(MacroProblem::WriteFailed) => (
                    Some("Couldn't save to the mouse. Nothing on it changed, and your steps are still here.".into()),
                    Some("Try Again"),
                ),
                Some(MacroProblem::MemoryFull) => {
                    (Some("No free macro memory. Delete a macro you don't use.".into()), None)
                }
                Some(MacroProblem::TooLong) => {
                    (Some("This macro is too long for one slot. Remove some steps to save it.".into()), None)
                }
                None => (None, None),
            }
        };
        self.banner.set_revealed(banner.is_some());
        if let Some(title) = banner {
            self.banner.set_title(&title);
        }
        self.banner.set_button_label(button);

        self.name_row.set_sensitive(!recording);
        self.plays_row.set_sensitive(!recording);
        self.save.set_sensitive(connected && fits && !recording && !rows.is_empty());
        self.delete.set_sensitive(connected && !recording);
    }

    fn row_widget(&self, index: usize, row: Row, count: usize, recording: bool) -> adw::ActionRow {
        let capturing = self.capturing.get() == Some(index);
        let subtitle = if capturing { "Press the new key (Esc cancels)" } else { row.subtitle() };
        let widget = adw::ActionRow::builder().title(row.title()).subtitle(subtitle).sensitive(!recording).build();

        // Move: a menu for the keyboard, and the handle to drag.
        let menu = gio::Menu::new();
        let packed = |down: bool| ((index << 1) | usize::from(down)) as u16;
        if index > 0 {
            menu.append(Some("Move Up"), Some(&format!("editor.move(uint16 {})", packed(false))));
        }
        if index + 1 < count {
            menu.append(Some("Move Down"), Some(&format!("editor.move(uint16 {})", packed(true))));
        }
        let handle = gtk::MenuButton::builder()
            .icon_name("list-drag-handle-symbolic")
            .menu_model(&menu)
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .tooltip_text("Move step")
            .build();
        handle.update_property(&[gtk::accessible::Property::Label("Move step")]);
        let drag = gtk::DragSource::new();
        drag.set_actions(gdk::DragAction::MOVE);
        drag.connect_prepare(move |_, _, _| Some(gdk::ContentProvider::for_value(&(index as u32).to_value())));
        handle.add_controller(drag);
        widget.add_prefix(&handle);
        let drop = gtk::DropTarget::new(u32::static_type(), gdk::DragAction::MOVE);
        let weak = self.self_weak();
        drop.connect_drop(move |_, value, _, _| {
            let (Some(e), Ok(from)) = (weak.upgrade(), value.get::<u32>()) else { return false };
            e.move_row(from as usize, index);
            true
        });
        widget.add_controller(drop);

        match row {
            Row::Tap(target) | Row::Down(target) | Row::Up(target) => {
                widget.add_suffix(&self.target_button(index, row, target, capturing));
            }
            Row::Pause(ms) => {
                let spin = gtk::SpinButton::with_range(1.0, f64::from(u16::MAX), 10.0);
                spin.set_value(f64::from(ms));
                spin.set_valign(gtk::Align::Center);
                spin.update_property(&[gtk::accessible::Property::Label("Pause in milliseconds")]);
                let weak = self.self_weak();
                spin.connect_value_changed(move |s| {
                    if let Some(e) = weak.upgrade()
                        && let Some(r) = e.rows.borrow_mut().get_mut(index)
                    {
                        *r = Row::Pause(s.value() as u16);
                    }
                    if let Some(e) = weak.upgrade() {
                        e.render_caption();
                    }
                });
                widget.add_suffix(&spin);
                widget.add_suffix(&gtk::Label::builder().label("ms").css_classes(["dim-label"]).build());
            }
            Row::WaitRelease => {}
        }

        let remove = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .tooltip_text("Delete step")
            .build();
        remove.update_property(&[gtk::accessible::Property::Label("Delete step")]);
        let weak = self.self_weak();
        remove.connect_clicked(move |_| {
            if let Some(e) = weak.upgrade() {
                e.remove_row(index);
            }
        });
        widget.add_suffix(&remove);
        widget
    }

    /// The keycap: a key is changed by pressing a new one; a mouse button or
    /// media key is picked from a menu.
    fn target_button(&self, index: usize, row: Row, target: Target, capturing: bool) -> gtk::Widget {
        let label = if capturing { "…".to_string() } else { target.label() };
        let with = move |t: Target| match row {
            Row::Down(_) => Row::Down(t),
            Row::Up(_) => Row::Up(t),
            _ => Row::Tap(t),
        };
        match target {
            Target::Key { .. } => {
                let button = gtk::Button::builder().label(&label).valign(gtk::Align::Center).build();
                button.update_property(&[gtk::accessible::Property::Description("Press to change the key")]);
                let weak = self.self_weak();
                button.connect_clicked(move |_| {
                    if let Some(e) = weak.upgrade() {
                        e.capture(index);
                    }
                });
                button.upcast()
            }
            Target::Button(_) | Target::Media(_) => {
                let choices: Vec<(Target, &str)> = match target {
                    Target::Button(_) => MOUSE_BUTTONS.iter().map(|&(v, n)| (Target::Button(v), n)).collect(),
                    _ => MEDIA_KEYS.iter().map(|&(v, n)| (Target::Media(v), n)).collect(),
                };
                let list = gtk::Box::builder().orientation(gtk::Orientation::Vertical).build();
                let popover = gtk::Popover::builder().child(&list).build();
                for (choice, name) in choices {
                    let item = gtk::Button::builder().label(name).css_classes(["flat"]).build();
                    let weak = self.self_weak();
                    let popover = popover.clone();
                    item.connect_clicked(move |_| {
                        popover.popdown();
                        if let Some(e) = weak.upgrade() {
                            e.set_row(index, with(choice));
                        }
                    });
                    list.append(&item);
                }
                gtk::MenuButton::builder().label(&label).popover(&popover).valign(gtk::Align::Center).build().upcast()
            }
        }
    }

    fn add_step_row(&self) -> gtk::ListBoxRow {
        let menu = gio::Menu::new();
        menu.append(Some("Key…"), Some("editor.add-key"));
        let buttons = gio::Menu::new();
        for (value, name) in MOUSE_BUTTONS {
            buttons.append(Some(name), Some(&format!("editor.add-button(uint16 {value})")));
        }
        menu.append_submenu(Some("Mouse Button"), &buttons);
        let media = gio::Menu::new();
        for (value, name) in MEDIA_KEYS {
            media.append(Some(name), Some(&format!("editor.add-media(uint16 {value})")));
        }
        menu.append_submenu(Some("Media Key"), &media);
        menu.append(Some("Pause"), Some("editor.add-pause"));
        let content = adw::ButtonContent::builder().icon_name("list-add-symbolic").label("Add Step").build();
        let button = gtk::MenuButton::builder()
            .child(&content)
            .menu_model(&menu)
            .direction(gtk::ArrowType::Up)
            .css_classes(["flat"])
            .hexpand(true)
            .build();
        gtk::ListBoxRow::builder().child(&button).activatable(false).build()
    }

    /// The steps caption alone, while a pause is being typed.
    fn render_caption(&self) {
        let mode = self.mode();
        let steps = self.steps();
        let percent = macro_steps::slot_percent(mode, &steps);
        let count = macro_steps::step_count(self.rows.borrow().len());
        let verb = if macro_steps::fits(mode, &steps) { "uses" } else { "needs" };
        self.steps_group.set_description(Some(&format!("{count} · {verb} {percent}% of its slot · drag to reorder")));
    }

    fn self_weak(&self) -> std::rc::Weak<MacroEditor> {
        self.me.clone()
    }
}
