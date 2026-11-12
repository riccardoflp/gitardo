//! What an operation run from the window shows: each question it asks, as a dialog of the window; the plan it is about
//! to run, with its commands, when the engine shows it first (`preview.confirm`); and what is said once it ended, with
//! **Undo** when it can be taken back.

#[cfg(test)]
mod tests;

use model::contract::{Answer, Answers, FeatureId, Plan, Question, QuestionKind, Step, Target};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{
    Button, ButtonKind, Choice, ConfirmAnswer, ConfirmStyle, Confirmation, Dialog, DialogButton, DialogRole, Icon,
    InputBox, Label, Message, Rect, Severity, TextKind, Tone, Ui,
};

use crate::intent::Intent;
use crate::text::feature as words;

/// What is asked of an operation, or answered to it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FeatureIntent {
    /// Run an operation of the catalog on what it was invoked on.
    Run(FeatureId, Target),
    /// Run an operation with some of its questions answered already, as an entry of a menu that names the answer does:
    /// **Pull (rebase)** among the ways to pull.
    RunWith(FeatureId, Target, Answers),
    /// The text of the question shown was typed.
    Typed(String),
    /// A choice of the question shown was picked, or ticked or unticked when it takes many.
    Picked(String),
    /// The question shown was answered; none when it was cancelled, and the operation runs nothing.
    Answer(Option<Answer>),
    /// The plan shown is to run, or not.
    Confirm(bool),
    /// Put away what was said once the operation ended.
    Dismiss,
}

/// A question of an operation, as it is being answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuestionState<'a> {
    /// The question.
    pub question: &'a Question,
    /// What is typed, for a question of text.
    pub typed: &'a str,
    /// The choices picked, for a pick: one at most, or any number when it takes many.
    pub picked: &'a [String],
}

/// A plan the engine shows before it runs (`preview.confirm`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfirmState<'a> {
    /// The plan.
    pub plan: &'a Plan,
}

/// What is said once an operation ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaidState<'a> {
    /// The sentence.
    pub text: &'a str,
    /// Whether it says the operation failed or was refused.
    pub is_error: bool,
    /// Whether **Undo** is offered beside it (`undo.last`).
    pub can_undo: bool,
}

/// A question of an operation over the window: its title, what it says will happen, and the box, the choices or the
/// button that answers it.
pub fn question_view(state: &QuestionState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    match &state.question.kind {
        QuestionKind::Confirm { accept } => confirm_question(state.question, accept.as_str(), ui),
        QuestionKind::Text { .. } | QuestionKind::Pick { .. } | QuestionKind::PickMany { .. } => {
            answer_dialog(state, ui)
        }
    }
}

/// A question that is answered yes or no.
fn confirm_question(question: &Question, accept: &str, ui: &mut Ui<'_>) -> Vec<Intent> {
    let confirmation = Confirmation {
        title: question.title.as_str(),
        message: question.detail.as_ref().map_or("", |detail| detail.as_str()),
        commands: &[],
        confirm: accept,
        style: ConfirmStyle::Normal,
    };
    match ui.confirm(("question", question.key.0), &confirmation) {
        Some(ConfirmAnswer::Confirmed) => vec![answered(Some(Answer::Confirmed))],
        Some(ConfirmAnswer::Cancelled) => vec![answered(None)],
        None => Vec::new(),
    }
}

/// A question answered with text typed or with choices.
fn answer_dialog(state: &QuestionState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let theme = *ui.theme();
    let gap = theme.sizes.spacing.medium;
    let question = state.question;
    let detail = question.detail.as_ref().map(model::contract::Text::as_str);
    let detail_height = detail.map_or(0.0, |detail| paragraph_height(ui, detail) + gap);
    let problem = problem_of(state);
    let can_answer = can_answer(state, problem.as_deref());
    let body_height = detail_height + answer_height(state, ui);
    let buttons = [
        DialogButton {
            label: words::CANCEL,
            role: DialogRole::Cancel,
        },
        DialogButton {
            label: question.title.as_str(),
            role: if can_answer {
                DialogRole::Primary
            } else {
                DialogRole::Secondary
            },
        },
    ];
    let dialog = Dialog {
        title: question.title.as_str(),
        body_height,
        buttons: &buttons,
    };
    let mut intents = Vec::new();
    let response = ui.dialog(("question", question.key.0), &dialog, |ui, body| {
        let mut area = Area::with_gap(body.rect, gap);
        if let Some(detail) = detail {
            let rect = area.top(detail_height - gap);
            let _ = ui.paragraph(rect, detail, (TextKind::Body, Tone::Muted));
        }
        intents.extend(answer_body(
            state,
            problem.as_deref(),
            ui,
            (area.rest(), body.is_opening),
        ));
    });
    if response.is_cancelled {
        intents.push(answered(None));
    } else if response.pressed.is_some() && can_answer {
        intents.push(answered(Some(answer_of(state))));
    }
    intents
}

/// The box or the choices of a question.
fn answer_body(
    state: &QuestionState<'_>,
    problem: Option<&str>,
    ui: &mut Ui<'_>,
    (rect, is_opening): (Rect, bool),
) -> Vec<Intent> {
    let theme = *ui.theme();
    let mut intents = Vec::new();
    match &state.question.kind {
        QuestionKind::Text { placeholder, .. } => {
            let placeholder = placeholder.as_ref().map_or("", |placeholder| placeholder.as_str());
            let mut input = InputBox::new(state.typed, placeholder);
            if let Some(problem) = problem {
                input = input.with_message(Message {
                    text: problem,
                    severity: Severity::Error,
                });
            }
            if is_opening {
                let id = ui.id("answer");
                ui.request_focus(id);
            }
            let field = Area::new(rect).top(theme.sizes.control_height);
            if let Some(text) = ui.input_box("answer", field, &input).changed_text {
                intents.push(Intent::Feature(FeatureIntent::Typed(text)));
            }
        }
        QuestionKind::Pick { choices } | QuestionKind::PickMany { choices } => {
            let is_many = matches!(state.question.kind, QuestionKind::PickMany { .. });
            let mut area = Area::new(rect);
            for choice in choices {
                let is_on = state.picked.contains(&choice.identifier);
                let label = choice.label.as_str();
                let widget = if is_many {
                    Choice::check_box(label, is_on)
                } else {
                    Choice::radio(label, is_on)
                };
                let row = area.top(theme.sizes.row_height);
                if ui.choice(("choice", choice.identifier.as_str()), row, &widget).clicked {
                    intents.push(Intent::Feature(FeatureIntent::Picked(choice.identifier.clone())));
                }
                if let Some(description) = &choice.description {
                    let line = inset(
                        area.top(theme.sizes.row_height),
                        Insets::symmetric(theme.sizes.icon + theme.sizes.spacing.medium, 0.0),
                    );
                    let _ = ui.label(
                        line,
                        &Label::new(description.as_str())
                            .kind(TextKind::Detail)
                            .tone(Tone::Muted),
                    );
                }
            }
        }
        QuestionKind::Confirm { .. } => {}
    }
    intents
}

/// The height of the box or of the choices of a question.
fn answer_height(state: &QuestionState<'_>, ui: &Ui<'_>) -> f32 {
    let sizes = ui.theme().sizes;
    match &state.question.kind {
        QuestionKind::Text { .. } => sizes.control_height + sizes.row_height,
        QuestionKind::Pick { choices } | QuestionKind::PickMany { choices } => {
            let rows: usize = choices
                .iter()
                .map(|choice| 1 + usize::from(choice.description.is_some()))
                .sum();
            crate::line::rows_of(rows) * sizes.row_height
        }
        QuestionKind::Confirm { .. } => 0.0,
    }
}

/// The height of a paragraph of the body of a dialog.
fn paragraph_height(ui: &Ui<'_>, text: &str) -> f32 {
    let style = ui.text_style(TextKind::Body);
    let lines = ui.fonts().wrap(text, style, ui.dialog_body_width()).len();
    crate::line::rows_of(lines) * ui.line_height(style)
}

/// What is wrong with the text typed, said under the box while it is typed.
fn problem_of(state: &QuestionState<'_>) -> Option<String> {
    match &state.question.kind {
        QuestionKind::Text { validation, .. } => validation
            .problem(state.typed)
            .map(|problem| problem.as_str().to_owned()),
        _ => None,
    }
}

/// Whether the question can be answered as it stands: text with nothing wrong, or a choice picked.
fn can_answer(state: &QuestionState<'_>, problem: Option<&str>) -> bool {
    match &state.question.kind {
        QuestionKind::Text { .. } => problem.is_none(),
        QuestionKind::Pick { .. } => !state.picked.is_empty(),
        QuestionKind::PickMany { .. } | QuestionKind::Confirm { .. } => true,
    }
}

/// The answer the question has as it stands.
fn answer_of(state: &QuestionState<'_>) -> Answer {
    match &state.question.kind {
        QuestionKind::Text { .. } => Answer::Text(state.typed.to_owned()),
        QuestionKind::Pick { .. } => Answer::Choice(state.picked.first().cloned().unwrap_or_default()),
        QuestionKind::PickMany { .. } => Answer::Choices(state.picked.to_vec()),
        QuestionKind::Confirm { .. } => Answer::Confirmed,
    }
}

/// The intent of an answer.
fn answered(answer: Option<Answer>) -> Intent {
    Intent::Feature(FeatureIntent::Answer(answer))
}

/// The lines of a plan as the dialog shows them: each step as the log will have it, and under a step that takes
/// something on its standard input, that input, indented.
#[must_use]
pub fn plan_lines(plan: &Plan) -> Vec<String> {
    let mut lines = Vec::new();
    for step in &plan.steps {
        lines.push(step.log_line());
        if let Step::Git { input: Some(input), .. } = step {
            lines.extend(String::from_utf8_lossy(input).lines().map(|line| format!("  {line}")));
        }
    }
    lines
}

/// The plan an operation is about to run, with its commands exactly as the log will have them, the reason when it is
/// destructive, and **Run** and **Cancel**; **Run** is pressed by `Enter` only when the plan is not destructive.
///
/// Feature: preview.confirm
pub fn confirm_view(state: &ConfirmState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let lines = plan_lines(state.plan);
    let commands: Vec<&str> = lines.iter().map(String::as_str).collect();
    let title = state.plan.title.as_str();
    let (reason, style) = match &state.plan.danger {
        model::contract::Danger::Destructive(reason) => (Some(reason.as_str()), ConfirmStyle::Danger),
        model::contract::Danger::Confirm(reason) => (Some(reason.as_str()), ConfirmStyle::Normal),
        model::contract::Danger::None => (None, ConfirmStyle::Normal),
    };
    // A reason that only says the title again is not said twice.
    let message = match reason.filter(|reason| reason.trim_end_matches('.') != title) {
        Some(reason) => words::warning(reason),
        None => words::THESE_COMMANDS_RUN.to_owned(),
    };
    let confirmation = Confirmation {
        title,
        message: &message,
        commands: &commands,
        confirm: words::RUN,
        style,
    };
    match ui.confirm("plan", &confirmation) {
        Some(ConfirmAnswer::Confirmed) => vec![Intent::Feature(FeatureIntent::Confirm(true))],
        Some(ConfirmAnswer::Cancelled) => vec![Intent::Feature(FeatureIntent::Confirm(false))],
        None => Vec::new(),
    }
}

/// What was said once an operation ended, across the top of what is under the header, with **Undo** when it can be
/// taken back and a button that puts it away.
pub fn said_view(state: &SaidState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    ui.fill_rect(rect, theme.colors.card_background);
    let mut parts = Area::with_gap(
        inset(rect, Insets::symmetric(spacing.large + spacing.small, 0.0)),
        spacing.medium,
    );
    let (icon, tone) = if state.is_error {
        (Icon::Error, Tone::Error)
    } else {
        (Icon::Info, Tone::Accent)
    };
    ui.icon_in(parts.left(theme.sizes.icon), icon, tone);
    let mut intents = Vec::new();
    let dismiss = Button::secondary(words::DISMISS).kind(ButtonKind::Outline);
    let (width, height) = ui.button_size(&dismiss);
    if ui
        .button(
            "dismiss said",
            place(parts.right(width), (width, height), (Align::Start, Align::Center)),
            &dismiss,
        )
        .clicked
    {
        intents.push(Intent::Feature(FeatureIntent::Dismiss));
    }
    if state.can_undo {
        let undo = Button::secondary(words::UNDO).kind(ButtonKind::Outline);
        let (width, height) = ui.button_size(&undo);
        if ui
            .button(
                "undo said",
                place(parts.right(width), (width, height), (Align::Start, Align::Center)),
                &undo,
            )
            .clicked
        {
            intents.push(Intent::Feature(FeatureIntent::Run(
                FeatureId("undo.last"),
                Target::Head,
            )));
        }
    }
    let _ = ui.label(parts.rest(), &Label::new(state.text));
    intents
}
