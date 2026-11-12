//! Input boxes: one line of text the person types, with a caret that blinks, a selection made with Shift and the
//! arrows or by dragging, a word selected by a double click, the clipboard, undo and redo of its own, a placeholder
//! when empty, and a message under it when what is typed is not valid. Text longer than the box scrolls across to keep
//! the caret in sight.

mod keys;
mod line;
mod words;

use std::hash::Hash;
use std::time::Duration;

pub use line::{EditableLine, Extent, Movement, Selecting};

use render::{Border, Point, Rect, TextRun, TextStyle};

use crate::frame::{TextKind, Ui};
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
    /// What the box shows while it is empty.
    pub placeholder: &'text str,
    /// The message under the box, if any.
    pub message: Option<Message<'text>>,
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
        let style = self.text_style(TextKind::Body);
        let inner = self.inner_rect(rect);
        let before = (state.line.text().to_owned(), state.line.selection(), state.line.caret());
        let mut is_submitted = false;
        for event in events {
            match event {
                WidgetEvent::Pressed {
                    button: PointerButton::Primary,
                    position,
                    modifiers,
                    click_count,
                } => {
                    let at = self.index_at(&state, (style, inner), position.x);
                    match click_count {
                        1 => state.line.place_caret(at, selecting(modifiers.shift())),
                        2 => state.line.select_word_at(at),
                        _ => state.line.select_all(),
                    }
                }
                WidgetEvent::Dragged { position, .. } => {
                    let at = self.index_at(&state, (style, inner), position.x);
                    state.line.place_caret(at, Selecting::Extend);
                }
                WidgetEvent::Key(key) => {
                    if keys::is_used(key) {
                        self.claim_key(id, key);
                    }
                    let outcome = keys::apply_key(&mut state.line, key);
                    is_submitted |= outcome.is_submitted;
                    if let Some(copied) = outcome.copied {
                        self.copy_to_clipboard(copied);
                    }
                }
                WidgetEvent::Text(text) | WidgetEvent::Paste(text) => {
                    let _ = state.line.insert(&text);
                }
                _ => {}
            }
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
        let _ = self.memory().input_boxes.insert(id, state);
        InputBoxResponse {
            response,
            changed_text,
            is_submitted,
        }
    }

    /// The part of an input box the text is drawn in.
    fn inner_rect(&self, rect: Rect) -> Rect {
        let padding = self.theme().sizes.spacing.medium;
        Rect::new(
            rect.x + padding,
            rect.y,
            (rect.width - 2.0 * padding).max(0.0),
            rect.height,
        )
    }

    /// The position in the text nearest to a place across the window.
    fn index_at(&self, state: &InputBoxState, (style, inner): (TextStyle, Rect), x: f32) -> usize {
        let text = state.line.text();
        let wanted = x - inner.x + state.scroll;
        let boundaries: Vec<usize> = text.char_indices().map(|(at, _)| at).chain([text.len()]).collect();
        let after = boundaries.partition_point(|at| self.prefix_width(text, *at, style) < wanted);
        let Some(&next) = boundaries.get(after) else {
            return text.len();
        };
        let Some(&previous) = after.checked_sub(1).and_then(|before| boundaries.get(before)) else {
            return next;
        };
        let to_previous = wanted - self.prefix_width(text, previous, style);
        let to_next = self.prefix_width(text, next, style) - wanted;
        if to_previous <= to_next { previous } else { next }
    }

    /// The width of the text before a position.
    fn prefix_width(&self, text: &str, position: usize, style: TextStyle) -> f32 {
        self.measure(text.get(..position).unwrap_or_default(), style)
    }

    /// Scrolls the text across the least that keeps the caret in sight, and no further than its end.
    fn keep_caret_in_sight(&self, state: &mut InputBoxState, (style, inner): (TextStyle, Rect)) {
        let text = state.line.text();
        let caret = self.prefix_width(text, state.line.caret(), style);
        let whole = self.measure(text, style);
        let caret_width = self.theme().sizes.border;
        let mut scroll = state.scroll;
        if caret - scroll < 0.0 {
            scroll = caret;
        } else if caret + caret_width - scroll > inner.width {
            scroll = caret + caret_width - inner.width;
        }
        state.scroll = scroll.min((whole + caret_width - inner.width).max(0.0)).max(0.0);
    }

    /// Draws an input box: its background and border, the selection, the text or the placeholder, the caret, and the
    /// message under it.
    fn draw_input_box(&mut self, rect: Rect, input: &InputBox<'_>, state: &InputBoxState, response: &Response) {
        let theme = *self.theme();
        let colors = theme.colors;
        let radius = theme.sizes.radius.medium;
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
        let style = self.text_style(TextKind::Body);
        let inner = self.inner_rect(rect);
        let line_height = self.line_height(style);
        let line = place(inner, (inner.width, line_height), (Align::Start, Align::Center));
        let text = state.line.text();
        let origin = Point::new(inner.x - state.scroll, line.y);
        self.clip(inner, |ui| {
            let selection = state.line.selection();
            if !selection.is_empty() {
                let start = ui.prefix_width(text, selection.start, style);
                let end = ui.prefix_width(text, selection.end, style);
                let shown = Rect::new(origin.x + start, line.y, end - start, line_height);
                ui.fill_rect(shown, colors.text_selection);
            }
            let (shown, color) = if text.is_empty() {
                (input.placeholder, colors.input_placeholder)
            } else {
                (text, colors.input_foreground)
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

    /// Draws the caret while it is in the visible half of its blink, and asks for the frame where it turns.
    fn draw_caret(&mut self, state: &InputBoxState, (origin, height): (Point, f32), inner: Rect) {
        let since = self.now().saturating_sub(state.changed_at);
        let phases = crate::number::steps(since.as_millis(), BLINK.as_millis());
        let next = state.changed_at + BLINK * u32::try_from(phases + 1).unwrap_or(u32::MAX);
        self.request_frame_at(next);
        let x = origin.x + self.prefix_width(state.line.text(), state.line.caret(), self.text_style(TextKind::Body));
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
