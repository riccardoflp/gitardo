//! Input boxes: one line of text the person types, with a caret that blinks, a selection made with Shift and the
//! arrows or by dragging, a word selected by a double click, the clipboard, undo and redo of its own, a placeholder
//! when empty, and a message under it when what is typed is not valid. Text longer than the box scrolls across to keep
//! the caret in sight. A box may have an icon before its text and the keys that reach it after, as Quick search has.

mod keys;
mod line;
mod words;

use std::hash::Hash;
use std::time::Duration;

pub use line::{EditableLine, Extent, Movement, Selecting};

use render::{Border, Icon, Point, Rect, TextRun, TextStyle};

use crate::access::{AccessRole, Description};
use crate::frame::{TextKind, Ui};
use crate::id::WidgetId;
use crate::input::{Cursor, PointerButton};
use crate::interaction::{Response, Sense, WidgetEvent};
use crate::layer::Layer;
use crate::layout::{Align, place};
use crate::memory::InputBoxState;
use crate::widget::Tone;

/// How long the caret shows, and then how long it hides, while it blinks.
const BLINK: Duration = Duration::from_millis(530);

/// How a message under an input box speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Severity {
    /// What is typed cannot be used.
    Error,
    /// What is typed can be used, with a warning.
    Warning,
    /// Something to know about what is typed.
    Info,
}

/// A message under an input box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Message<'text> {
    /// What it says.
    pub text: &'text str,
    /// How it speaks.
    pub severity: Severity,
}

/// An input box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputBox<'text> {
    /// The text the view holds. When it changes from what the view gave before, the box takes it, so that a view can
    /// clear or fill the box; otherwise the box keeps what is being typed.
    pub text: &'text str,
    /// What the box shows while it is empty, which is also its name for the screen reader.
    pub placeholder: &'text str,
    /// The message under the box, if any.
    pub message: Option<Message<'text>>,
    /// The icon before the text, such as the magnifying glass of a box that searches.
    pub icon: Option<Icon>,
    /// The keys that bring the keyboard to the box, shown at its right end, such as `Ctrl K`.
    pub key_hint: Option<&'text str>,
    /// Whether what is typed is hidden, as a token is: a dot stands for each character, the screen reader is told the
    /// dots, and nothing is copied from it.
    pub is_hidden: bool,
}

impl<'text> InputBox<'text> {
    /// A box holding a text, with what it shows while it is empty, and no message, icon or keys.
    #[must_use]
    pub fn new(text: &'text str, placeholder: &'text str) -> InputBox<'text> {
        InputBox {
            text,
            placeholder,
            message: None,
            icon: None,
            key_hint: None,
            is_hidden: false,
        }
    }

    /// The same box with a message under it.
    #[must_use]
    pub fn with_message(self, message: Message<'text>) -> InputBox<'text> {
        InputBox {
            message: Some(message),
            ..self
        }
    }

    /// The same box with an icon before its text.
    #[must_use]
    pub fn with_icon(self, icon: Icon) -> InputBox<'text> {
        InputBox {
            icon: Some(icon),
            ..self
        }
    }

    /// The same box with the keys that reach it at its right end.
    #[must_use]
    pub fn with_key_hint(self, keys: &'text str) -> InputBox<'text> {
        InputBox {
            key_hint: Some(keys),
            ..self
        }
    }

    /// The same box hiding what is typed in it.
    #[must_use]
    pub fn hidden(self) -> InputBox<'text> {
        InputBox {
            is_hidden: true,
            ..self
        }
    }
}

/// What a box that hides what is typed shows for each character.
const HIDDEN: char = '\u{2022}';

/// What a box shows of its text: the text, or a dot for each character when it hides it.
fn shown_text(state: &InputBoxState) -> std::borrow::Cow<'_, str> {
    let text = state.line.text();
    if state.is_hidden {
        std::borrow::Cow::Owned(text.chars().map(|_| HIDDEN).collect())
    } else {
        std::borrow::Cow::Borrowed(text)
    }
}

/// What an input box tells its view.
#[derive(Clone, Debug, PartialEq)]
pub struct InputBoxResponse {
    /// What happened to the box as a whole.
    pub response: Response,
    /// The text, when it changed in this frame.
    pub changed_text: Option<String>,
    /// Whether Enter was pressed in it.
    pub is_submitted: bool,
}

impl Ui<'_> {
    /// Declares an input box over a rectangle and says what was typed in it.
    pub fn input_box(&mut self, key: impl Hash, rect: Rect, input: &InputBox<'_>) -> InputBoxResponse {
        let id = self.id(key);
        let (response, events) = self.interact_with_events(id, rect, Sense::CLICK | Sense::DRAG | Sense::FOCUS);
        let mut state = self.memory().input_boxes.remove(&id).unwrap_or_default();
        take_given_text(&mut state, input.text);
        state.is_hidden = input.is_hidden;
        let style = self.text_style(TextKind::Body);
        let inner = self.inner_rect(rect, input);
        let before = (state.line.text().to_owned(), state.line.selection(), state.line.caret());
        let mut is_submitted = false;
        for event in events {
            is_submitted |= self.input_box_event(id, &mut state, (style, inner), event);
        }
        let is_moved =
            (state.line.text(), state.line.selection(), state.line.caret()) != (before.0.as_str(), before.1, before.2);
        if is_moved {
            state.changed_at = self.now();
        }
        let changed_text = (state.line.text() != before.0).then(|| state.line.text().to_owned());
        self.keep_caret_in_sight(&mut state, (style, inner));
        if response.hovered {
            self.set_cursor(Cursor::Text);
        }
        self.draw_input_box(rect, input, &state, &response);
        let role = if input.icon == Some(Icon::Search) {
            AccessRole::SearchInput
        } else {
            AccessRole::TextInput
        };
        let shown = shown_text(&state);
        let description = Description::new(role, input.placeholder).value(&shown);
        self.access_node(id, rect, &description);
        let _ = self.memory().input_boxes.insert(id, state);
        InputBoxResponse {
            response,
            changed_text,
            is_submitted,
        }
    }

    /// Gives the keyboard to an input box with all its text selected, as `Ctrl+F` does to a search box. Called before
    /// the box is declared, the box is drawn so in the same frame.
    pub fn focus_input_box(&mut self, id: WidgetId) {
        let now = self.now();
        if let Some(state) = self.memory().input_boxes.get_mut(&id) {
            state.line.select_all();
            state.changed_at = now;
        }
        self.request_focus(id);
    }

    /// Gives the keyboard to an input box with all its text selected, once for each number: the box that finds in a
    /// page takes it each time `Ctrl+F` is pressed, and leaves it to a click elsewhere in between.
    pub fn focus_input_box_once(&mut self, id: WidgetId, serial: u64) {
        if self.memory().focus_serials.insert(id, serial) != Some(serial) {
            self.focus_input_box(id);
        }
    }

    /// Applies an event to an input box: a press places the caret or selects, a drag selects, a key edits or moves,
    /// and typed or pasted text goes in. Says whether Enter was pressed.
    fn input_box_event(
        &mut self,
        id: WidgetId,
        state: &mut InputBoxState,
        (style, inner): (TextStyle, Rect),
        event: WidgetEvent,
    ) -> bool {
        match event {
            WidgetEvent::Pressed {
                button: PointerButton::Primary,
                position,
                modifiers,
                click_count,
            } => {
                let at = self.index_at(state, (style, inner), position.x);
                match click_count {
                    1 => state.line.place_caret(at, selecting(modifiers.shift())),
                    2 => state.line.select_word_at(at),
                    _ => state.line.select_all(),
                }
            }
            WidgetEvent::Dragged { position, .. } => {
                let at = self.index_at(state, (style, inner), position.x);
                state.line.place_caret(at, Selecting::Extend);
            }
            WidgetEvent::Key(key) => {
                if keys::is_used(key) {
                    self.claim_key(id, key);
                }
                let outcome = keys::apply_key(&mut state.line, key);
                if let Some(copied) = outcome.copied.filter(|_| !state.is_hidden) {
                    self.copy_to_clipboard(copied);
                }
                return outcome.is_submitted;
            }
            WidgetEvent::Text(text) | WidgetEvent::Paste(text) => {
                let _ = state.line.insert(&text);
            }
            // The other events do not change an input box.
            _ => {}
        }
        false
    }

    /// The part of an input box the text is drawn in: inside its padding, after its icon and before its keys.
    fn inner_rect(&self, rect: Rect, input: &InputBox<'_>) -> Rect {
        let theme = self.theme();
        let padding = theme.sizes.spacing.large;
        let icon = input
            .icon
            .map_or(0.0, |_| theme.sizes.icon + theme.sizes.spacing.medium);
        let keys = input
            .key_hint
            .map_or(0.0, |keys| self.key_hint_width(keys) + theme.sizes.spacing.medium);
        Rect::new(
            rect.x + padding + icon,
            rect.y,
            (rect.width - 2.0 * padding - icon - keys).max(0.0),
            rect.height,
        )
    }

    /// The width of the box of the keys that reach an input box, or that do what a button does.
    pub(crate) fn key_hint_width(&self, keys: &str) -> f32 {
        let theme = self.theme();
        self.measure(keys, self.text_style(TextKind::Small))
            + 2.0 * (theme.sizes.spacing.small + theme.sizes.spacing.tiny)
    }

    /// The position in the text nearest to a place across the window.
    fn index_at(&self, state: &InputBoxState, (style, inner): (TextStyle, Rect), x: f32) -> usize {
        let text = state.line.text();
        let wanted = x - inner.x + state.scroll;
        let boundaries: Vec<usize> = text.char_indices().map(|(at, _)| at).chain([text.len()]).collect();
        let after = boundaries.partition_point(|at| self.prefix_width(state, *at, style) < wanted);
        let Some(&next) = boundaries.get(after) else {
            return text.len();
        };
        let Some(&previous) = after.checked_sub(1).and_then(|before| boundaries.get(before)) else {
            return next;
        };
        let to_previous = wanted - self.prefix_width(state, previous, style);
        let to_next = self.prefix_width(state, next, style) - wanted;
        if to_previous <= to_next { previous } else { next }
    }

    /// The width of what the box shows of its text before a position: the dots of the characters before it, in a box
    /// that hides what is typed.
    fn prefix_width(&self, state: &InputBoxState, position: usize, style: TextStyle) -> f32 {
        let before = state.line.text().get(..position).unwrap_or_default();
        if state.is_hidden {
            let dots: String = before.chars().map(|_| HIDDEN).collect();
            self.measure(&dots, style)
        } else {
            self.measure(before, style)
        }
    }

    /// Scrolls the text across the least that keeps the caret in sight, and no further than its end.
    fn keep_caret_in_sight(&self, state: &mut InputBoxState, (style, inner): (TextStyle, Rect)) {
        let caret = self.prefix_width(state, state.line.caret(), style);
        let whole = self.prefix_width(state, state.line.text().len(), style);
        let caret_width = self.theme().sizes.border;
        let mut scroll = state.scroll;
        if caret - scroll < 0.0 {
            scroll = caret;
        } else if caret + caret_width - scroll > inner.width {
            scroll = caret + caret_width - inner.width;
        }
        state.scroll = scroll.min((whole + caret_width - inner.width).max(0.0)).max(0.0);
    }

    /// Draws an input box: its background and border, its icon and keys, the selection, the text or the placeholder,
    /// the caret, and the message under it.
    fn draw_input_box(&mut self, rect: Rect, input: &InputBox<'_>, state: &InputBoxState, response: &Response) {
        let theme = *self.theme();
        let colors = theme.colors;
        let radius = theme.sizes.radius.large;
        self.fill_rounded_rect(rect, radius, colors.input_background);
        let edge = match (input.message.map(|message| message.severity), response.has_focus) {
            (Some(severity), _) => severity_tone(severity).color(&theme),
            (None, true) => colors.focus_ring,
            (None, false) => colors.input_border,
        };
        self.border(
            rect,
            Border {
                width: theme.sizes.border,
                radius,
                color: edge,
            },
        );
        self.input_ends(rect, input);
        let style = self.text_style(TextKind::Body);
        let inner = self.inner_rect(rect, input);
        let line_height = self.line_height(style);
        let line = place(inner, (inner.width, line_height), (Align::Start, Align::Center));
        let text = shown_text(state);
        let origin = Point::new(inner.x - state.scroll, line.y);
        self.clip(inner, |ui| {
            let selection = state.line.selection();
            if !selection.is_empty() {
                let start = ui.prefix_width(state, selection.start, style);
                let end = ui.prefix_width(state, selection.end, style);
                let shown = Rect::new(origin.x + start, line.y, end - start, line_height);
                ui.fill_rect(shown, colors.text_selection);
            }
            let (shown, color) = if text.is_empty() {
                (input.placeholder, colors.input_placeholder)
            } else {
                (text.as_ref(), colors.input_foreground)
            };
            let run = TextRun {
                text: shown,
                style,
                color,
                origin,
                max_width: None,
            };
            let _ = ui.text(&run);
        });
        if response.has_focus && self.is_window_focused() {
            self.draw_caret(state, (origin, line_height), inner);
        }
        if let Some(message) = input.message {
            self.input_message(rect, message);
        }
    }

    /// Draws the icon at the start of an input box and the keys that reach it at its end.
    fn input_ends(&mut self, rect: Rect, input: &InputBox<'_>) {
        let theme = *self.theme();
        let spacing = theme.sizes.spacing;
        let muted = theme.colors.muted_foreground;
        if let Some(icon) = input.icon {
            let side = theme.sizes.icon;
            let at = place(rect, (side, side), (Align::Start, Align::Center));
            self.icon(icon, Point::new(rect.x + spacing.large, at.y), side, muted);
        }
        let Some(keys) = input.key_hint else {
            return;
        };
        let size = (self.key_hint_width(keys), rect.height);
        let right = Rect::new(rect.x, rect.y, rect.width - spacing.medium, rect.height);
        self.key_hint(place(right, size, (Align::End, Align::Center)), keys);
    }

    /// Draws the box of some keys, such as `Ctrl K`, at the start of a rectangle and in its middle down.
    pub(crate) fn key_hint(&mut self, rect: Rect, keys: &str) {
        let theme = *self.theme();
        let spacing = theme.sizes.spacing;
        let style = self.text_style(TextKind::Small);
        let size = (self.key_hint_width(keys), self.line_height(style) + 2.0 * spacing.tiny);
        let hint = place(rect, size, (Align::Start, Align::Center));
        let border = Border {
            width: theme.sizes.border,
            radius: theme.sizes.radius.small,
            color: theme.colors.border,
        };
        self.border(hint, border);
        let run = TextRun {
            text: keys,
            style,
            color: theme.colors.muted_foreground,
            origin: Point::new(hint.x + spacing.small + spacing.tiny, hint.y + spacing.tiny),
            max_width: None,
        };
        let _ = self.text(&run);
    }

    /// Draws the caret while it is in the visible half of its blink, and asks for the frame where it turns.
    fn draw_caret(&mut self, state: &InputBoxState, (origin, height): (Point, f32), inner: Rect) {
        let since = self.now().saturating_sub(state.changed_at);
        let phases = crate::number::steps(since.as_millis(), BLINK.as_millis());
        let next = state.changed_at + BLINK * u32::try_from(phases + 1).unwrap_or(u32::MAX);
        self.request_frame_at(next);
        let x = origin.x + self.prefix_width(state, state.line.caret(), self.text_style(TextKind::Body));
        let caret = Rect::new(x, origin.y, self.theme().sizes.border, height);
        self.set_caret(caret);
        if phases.is_multiple_of(2) {
            let color = self.theme().colors.input_foreground;
            self.clip(inner.inset(-self.theme().sizes.border), |ui| ui.fill_rect(caret, color));
        }
    }

    /// Draws the message under an input box, over what is below it.
    fn input_message(&mut self, rect: Rect, message: Message<'_>) {
        let theme = *self.theme();
        let padding = theme.sizes.spacing.small + theme.sizes.spacing.tiny;
        let style = self.text_style(TextKind::Body);
        let width = rect.width;
        let lines = self.fonts().wrap(message.text, style, width - 2.0 * padding).len();
        let height = crate::number::float_of_count(lines) * self.line_height(style) + 2.0 * padding;
        let shown = Rect::new(rect.x, rect.bottom() - theme.sizes.border, width, height);
        let color = severity_tone(message.severity).color(&theme);
        self.layer(Layer::Popup, |ui| {
            let radius = theme.sizes.radius.small;
            ui.fill_rounded_rect(shown, radius, theme.colors.menu_background);
            ui.fill_rounded_rect(shown, radius, color.with_alpha(MESSAGE_TINT));
            ui.border(
                shown,
                Border {
                    width: theme.sizes.border,
                    radius,
                    color,
                },
            );
            let text = Rect::new(
                shown.x + padding,
                shown.y + padding,
                width - 2.0 * padding,
                height - 2.0 * padding,
            );
            let _ = ui.paragraph(text, message.text, (TextKind::Body, Tone::Normal));
        });
    }
}

/// How opaque the tint of a message is over the background of menus, out of 255.
const MESSAGE_TINT: u8 = 40;

/// The tone a message speaks in.
fn severity_tone(severity: Severity) -> Tone {
    match severity {
        Severity::Error => Tone::Error,
        Severity::Warning => Tone::Warning,
        Severity::Info => Tone::Accent,
    }
}

/// Whether Shift extends the selection.
fn selecting(is_shift: bool) -> Selecting {
    if is_shift { Selecting::Extend } else { Selecting::Move }
}

/// Takes the text the view gives when it differs from what it gave before, so that the view can clear or fill the box
/// while what is typed is kept otherwise.
fn take_given_text(state: &mut InputBoxState, given: &str) {
    if state.given.as_deref() != Some(given) {
        if state.line.text() != given {
            state.line = EditableLine::new(given);
            state.scroll = 0.0;
        }
        state.given = Some(given.to_owned());
    }
}
