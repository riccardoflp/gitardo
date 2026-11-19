//! The conversation of the Ask Gitardo page: each question at the right in a bubble, its answer under it drawn as the
//! preview of Markdown draws it, *Looking for the features…* until its first words, and a failure in the colors of an
//! error with **Try again**; scrolled, the end brought into sight when a question is asked.

use std::ops::Range;

use ui::layout::{Align, place};
use ui::{AccessRole, Border, Button, Description, Label, Point, Rect, TextKind, Tone, Ui};

use super::{HelpExchange, HelpIntent, HelpState};
use crate::file_view::{LaidMarkdown, draw_markdown, lay_out_markdown};
use crate::intent::Intent;
use crate::text::help as words;

/// The widest a bubble of a question is, as a part of the conversation.
const BUBBLE_PART: f32 = 0.75;
/// The key of the scroll area.
const SCROLL: &str = "help conversation";

/// A question and its answer laid out in the width of the conversation.
struct LaidExchange<'a> {
    /// The question and its answer.
    exchange: &'a HelpExchange,
    /// The lines of the question, as bytes of it.
    question_lines: Vec<Range<usize>>,
    /// The size of the bubble of the question.
    bubble: (f32, f32),
    /// The answer, once its first words came.
    answer: Option<LaidMarkdown>,
    /// Whether *Looking for the features…* shows in place of the answer.
    is_looking: bool,
    /// The height of the box of the failure, when there is one.
    failure: Option<f32>,
    /// Whether **Try again** is under the failure: the last question failed.
    can_try_again: bool,
}

/// The conversation over the rectangle its caller clipped it to, scrolled, its links and its buttons asked of the page.
pub(super) fn conversation_view(state: &HelpState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let width = (rect.width - ui.theme().sizes.scrollbar - ui.theme().sizes.spacing.small).max(1.0);
    let last = state.exchanges.len().saturating_sub(1);
    let laid: Vec<LaidExchange<'_>> = state
        .exchanges
        .iter()
        .enumerate()
        .map(|(index, exchange)| lay_out(exchange, (index == last, state.is_asking), ui, width))
        .collect();
    let gap = ui.theme().sizes.spacing.huge;
    let total: f32 = laid.iter().map(|exchange| height(exchange, ui)).sum::<f32>()
        + gap * crate::number::float_of_count(laid.len().saturating_sub(1));
    let _ = ui.scroll_to_once(SCROLL, state.reveal, (total - rect.height).max(0.0));
    let mut intents = Vec::new();
    ui.region(SCROLL, (rect, AccessRole::Region, words::CONVERSATION), |ui| {
        let (_, asked) = ui.scroll_area(SCROLL, rect, (rect.width, total), |ui, content| {
            let mut asked = Vec::new();
            let mut top = content.y;
            for (index, exchange) in laid.iter().enumerate() {
                asked.extend(draw(exchange, index, ui, Point::new(content.x, top), width));
                top += height(exchange, ui) + gap;
            }
            asked
        });
        intents.extend(asked);
    });
    intents
}

/// A question and its answer laid out in a width: the bubble at most three quarters of it wide.
fn lay_out<'a>(
    exchange: &'a HelpExchange,
    (is_last, is_asking): (bool, bool),
    ui: &Ui<'_>,
    width: f32,
) -> LaidExchange<'a> {
    let spacing = ui.theme().sizes.spacing;
    let style = ui.text_style(TextKind::Body);
    let text_width = (width * BUBBLE_PART - 2.0 * spacing.large).max(1.0);
    let question_lines = ui.fonts().wrap(&exchange.question, style, text_width);
    let widest = question_lines
        .iter()
        .map(|line| ui.measure(exchange.question.get(line.clone()).unwrap_or_default(), style))
        .fold(0.0_f32, f32::max);
    let lines = crate::number::float_of_count(question_lines.len().max(1));
    let bubble = (
        widest.min(text_width) + 2.0 * spacing.large,
        lines * ui.line_height(style) + 2.0 * spacing.medium,
    );
    let answer = exchange
        .has_words
        .then(|| lay_out_markdown(&exchange.answer, ui, width));
    let failure = exchange.failed.as_ref().map(|failed| {
        let count = ui
            .fonts()
            .wrap(failed, style, (width - 2.0 * spacing.large).max(1.0))
            .len()
            .max(1);
        crate::number::float_of_count(count) * ui.line_height(style) + 2.0 * spacing.medium
    });
    LaidExchange {
        exchange,
        question_lines,
        bubble,
        is_looking: is_last && is_asking && !exchange.has_words,
        answer,
        failure,
        can_try_again: is_last && !is_asking && exchange.failed.is_some(),
    }
}

/// The height a question and its answer take.
fn height(laid: &LaidExchange<'_>, ui: &Ui<'_>) -> f32 {
    let sizes = ui.theme().sizes;
    let mut height = laid.bubble.1;
    if let Some(answer) = &laid.answer {
        height += sizes.spacing.medium + answer.height();
        if !laid.exchange.buttons.is_empty() {
            height += sizes.spacing.medium + super::show_me::row_height(&laid.exchange.buttons, ui);
        }
    } else if laid.is_looking {
        height += sizes.spacing.medium + sizes.row_height;
    }
    if let Some(failure) = laid.failure {
        height += sizes.spacing.medium + failure;
    }
    if laid.can_try_again {
        height += sizes.spacing.small + sizes.control_height;
    }
    height
}

/// Draws a question and its answer from the top left of their place, and gives what their links and buttons ask.
fn draw(laid: &LaidExchange<'_>, index: usize, ui: &mut Ui<'_>, at: Point, width: f32) -> Vec<Intent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let bubble = Rect::new(at.x + width - laid.bubble.0, at.y, laid.bubble.0, laid.bubble.1);
    draw_bubble(laid, index, ui, bubble);
    let mut top = bubble.bottom() + spacing.medium;
    let mut intents = Vec::new();
    if let Some(answer) = &laid.answer {
        let clicked = draw_markdown(answer, ("help answer", index), ui, Point::new(at.x, top));
        intents.extend(clicked.into_iter().map(|link| Intent::Help(HelpIntent::OpenLink(link))));
        top += answer.height() + spacing.medium;
        let buttons = &laid.exchange.buttons;
        if !buttons.is_empty() {
            let row = Rect::new(at.x, top, width, super::show_me::row_height(buttons, ui));
            intents.extend(super::show_me::buttons_view(buttons, index, ui, row));
            top = row.bottom() + spacing.medium;
        }
    } else if laid.is_looking {
        let line = Rect::new(at.x, top, width, theme.sizes.row_height);
        let _ = ui.label(line, &Label::new(words::LOOKING).tone(Tone::Muted));
        top += theme.sizes.row_height + spacing.medium;
    }
    if let (Some(failed), Some(failure)) = (&laid.exchange.failed, laid.failure) {
        let frame = Rect::new(at.x, top, width, failure);
        draw_failure(ui, frame, failed);
        top = frame.bottom() + spacing.small;
    }
    if laid.can_try_again {
        let button = Button::secondary(words::TRY_AGAIN).with_icon(crate::icons::READ_AGAIN);
        let (button_width, button_height) = ui.button_size(&button);
        let cell = place(
            Rect::new(at.x, top, width, theme.sizes.control_height),
            (button_width, button_height),
            (Align::Start, Align::Center),
        );
        if ui.button(("help try again", index), cell, &button).clicked {
            intents.push(Intent::Help(HelpIntent::TryAgain));
        }
    }
    intents
}

/// The bubble of a question, at the right, with its lines in it.
fn draw_bubble(laid: &LaidExchange<'_>, index: usize, ui: &mut Ui<'_>, bubble: Rect) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    ui.fill_rounded_rect(bubble, theme.sizes.radius.large, theme.colors.list_inactive_selection);
    let style = ui.text_style(TextKind::Body);
    let line_height = ui.line_height(style);
    let mut y = bubble.y + spacing.medium;
    for line in &laid.question_lines {
        let text = laid.exchange.question.get(line.clone()).unwrap_or_default();
        let rect = Rect::new(
            bubble.x + spacing.large,
            y,
            bubble.width - 2.0 * spacing.large,
            line_height,
        );
        let _ = ui.text(&ui::TextRun {
            text,
            style,
            color: theme.colors.foreground,
            origin: Point::new(rect.x, rect.y),
            max_width: Some(rect.width),
        });
        y += line_height;
    }
    let id = ui.id(("help question", index));
    ui.describe(
        id,
        bubble,
        &Description::new(AccessRole::Label, &laid.exchange.question),
    );
}

/// A failure in the colors of an error: what the provider said, in a frame.
fn draw_failure(ui: &mut Ui<'_>, frame: Rect, failed: &str) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    ui.fill_rounded_rect(frame, theme.sizes.radius.medium, theme.colors.error.with_alpha(28));
    ui.border(
        frame,
        Border {
            width: theme.sizes.border,
            color: theme.colors.error,
            radius: theme.sizes.radius.medium,
        },
    );
    let text = Rect::new(
        frame.x + spacing.large,
        frame.y + spacing.medium,
        frame.width - 2.0 * spacing.large,
        frame.height - 2.0 * spacing.medium,
    );
    let _ = ui.paragraph(text, failed, (TextKind::Body, Tone::Error));
}
