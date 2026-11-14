//! The box that finds in a File page (`file-view.find`): it finds as it is typed, says **3 of 12** or **No results**,
//! and has the switches of match case, whole word and regular expressions, the arrows that go from match to match, and
//! the button that closes it.

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{Button, ButtonKind, Choice, Icon, InputBox, Key, Label, Message, NamedKey, Severity, TextKind, Tone, Ui};

use super::{FilePageIntent, FilePageState, FindOptions, page_intent};
use crate::intent::Intent;
use crate::text::file_view as words;

/// How wide the box is, at most.
const BOX_WIDTH: f32 = 280.0;

/// The bar of the box that finds, over the rectangle its caller clipped it to.
///
/// Feature: file-view.find
pub fn find_view(state: &FilePageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let Some(find) = state.find else {
        return Vec::new();
    };
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let inner = inset(rect, Insets::symmetric(spacing.large + spacing.small, spacing.medium));
    let mut parts = Area::with_gap(inner, spacing.medium);
    let mut asked = Vec::new();
    let input_id = ui.id(("find box", state.id));
    ui.focus_input_box_once(input_id, find.focus);
    let mut input = InputBox::new(find.typed, words::FIND_PLACEHOLDER).with_icon(Icon::Search);
    if find.is_invalid {
        input = input.with_message(Message {
            text: words::INVALID_PATTERN,
            severity: Severity::Error,
        });
    }
    let field = parts.left(BOX_WIDTH.min(inner.width * 0.5));
    let answer = ui.input_box(("find box", state.id), field, &input);
    if let Some(text) = answer.changed_text {
        asked.push(FilePageIntent::FindTyped(text));
    }
    if answer.is_submitted {
        let is_shift = ui
            .key_presses()
            .iter()
            .any(|key| key.key == Key::Named(NamedKey::Enter) && key.modifiers.shift());
        asked.push(if is_shift {
            FilePageIntent::FindPrevious
        } else {
            FilePageIntent::FindNext
        });
    }
    let said = match (find.marks.len(), find.current) {
        _ if find.typed.is_empty() || find.is_invalid => String::new(),
        (0, _) => words::NO_RESULTS.to_owned(),
        (count, current) => words::match_place(current.map_or(0, |current| current + 1), count),
    };
    let said_width = ui.measure(&said, ui.text_style(TextKind::Detail)).max(80.0);
    let _ = ui.label(
        parts.left(said_width),
        &Label::new(&said).kind(TextKind::Detail).tone(Tone::Muted),
    );
    asked.extend(buttons(state, ui, &mut parts));
    asked.extend(switches(state, find.options, ui, &mut parts));
    asked.into_iter().map(|intent| page_intent(state, intent)).collect()
}

/// The arrows that go from match to match, from the left, and the button that closes the box, at the right end.
fn buttons(state: &FilePageState<'_>, ui: &mut Ui<'_>, parts: &mut Area) -> Vec<FilePageIntent> {
    let side = ui.theme().sizes.control_height;
    let mut asked = Vec::new();
    for (icon, tooltip, intent) in [
        (Icon::ArrowUp, words::PREVIOUS_MATCH, FilePageIntent::FindPrevious),
        (Icon::ArrowDown, words::NEXT_MATCH, FilePageIntent::FindNext),
    ] {
        let button = Button::icon(icon, tooltip).kind(ButtonKind::Icon);
        let cell = place(parts.left(side), (side, side), (Align::Start, Align::Center));
        if ui.button(("find arrow", state.id, tooltip), cell, &button).clicked {
            asked.push(intent);
        }
    }
    let close = Button::icon(Icon::Close, words::CLOSE_FIND).kind(ButtonKind::Icon);
    let cell = place(parts.right(side), (side, side), (Align::Start, Align::Center));
    if ui.button(("find close", state.id), cell, &close).clicked {
        asked.push(FilePageIntent::CloseFind);
    }
    asked
}

/// The switches of match case, whole word and regular expressions, from the left.
fn switches(state: &FilePageState<'_>, options: FindOptions, ui: &mut Ui<'_>, parts: &mut Area) -> Vec<FilePageIntent> {
    let switches = [
        (words::MATCH_CASE, options.match_case),
        (words::WHOLE_WORD, options.whole_word),
        (words::REGULAR_EXPRESSION, options.is_regex),
    ];
    let mut asked = Vec::new();
    for (index, (label, is_on)) in switches.into_iter().enumerate() {
        let width = ui.measure(label, ui.text_style(TextKind::Body)) + 2.0 * ui.theme().sizes.control_height;
        let rect = parts.left(width);
        if ui
            .choice(("find switch", state.id, index), rect, &Choice::check_box(label, is_on))
            .clicked
        {
            let mut changed = options;
            match index {
                0 => changed.match_case = !is_on,
                1 => changed.whole_word = !is_on,
                _ => changed.is_regex = !is_on,
            }
            asked.push(FilePageIntent::FindOptions(changed));
        }
    }
    asked
}
