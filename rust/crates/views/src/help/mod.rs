//! The Ask Gitardo page (`help.page`, decision D76): a question about how to do something with Gitardo, in the user's
//! own words, and its answer drawn as the preview of Markdown draws it; the questions and the answers one under the
//! other, the box to ask at the foot.

mod conversation;
mod foot;

use model::contract::FeatureId;
use ui::editor::{Buffer, Command, Cursor};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Button, Label, Rect, TextKind, Tone, Ui};

use crate::file_view::MarkdownContent;
use crate::intent::Intent;
use crate::text::help as words;

/// The widest the conversation and the box are; a wider page keeps them in its middle.
const WIDEST: f32 = 820.0;

/// A question of the conversation, and what came of it.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct HelpExchange {
    /// The question, as it was asked.
    pub question: String,
    /// The answer as it came so far, read into Markdown, each feature it names a link.
    pub answer: MarkdownContent,
    /// Whether the first words of the answer came.
    pub has_words: bool,
    /// Why there is no answer: what the provider said, or that the question was not sent.
    pub failed: Option<String>,
}

/// What the Ask Gitardo page shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HelpState<'a> {
    /// The questions asked since the page opened, or since **New conversation**, each with its answer.
    pub exchanges: &'a [HelpExchange],
    /// Whether an AI provider is set up: without one the page says how to set one up, and its box is disabled.
    pub has_provider: bool,
    /// Whether an answer comes: the box is disabled, and **Ask** is **Stop**.
    pub is_asking: bool,
    /// The text of the box.
    pub question: &'a Buffer,
    /// The caret and the selection of the box.
    pub cursor: Cursor,
    /// A number that changes each time the box takes the keyboard.
    pub focus: u64,
    /// A number that changes each time the end of the conversation is to be brought into sight.
    pub reveal: u64,
}

/// Something asked of the Ask Gitardo page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HelpIntent {
    /// **Ask Gitardo…** in Quick search or in the **…** menu: the page shown.
    Open,
    /// **Ask Gitardo: "…"** in Quick search, or an example of the empty page: the page shown, with that question asked.
    AskAbout(String),
    /// **Ask**, or `Enter` in the box: the text of the box asked.
    Ask,
    /// What the keys and the pointer ask of the text of the box.
    Write(Vec<Command>),
    /// **Stop**: the answer that comes stops, what came of it kept.
    Stop,
    /// **New conversation**: the questions and the answers forgotten.
    NewConversation,
    /// **Try again**: the last question asked again.
    TryAgain,
    /// **Open the settings**: the settings page at `ai.provider`.
    OpenSettings,
    /// A link of an answer clicked, as the answer writes it: `show-me:` and a feature (`help.show-me`).
    OpenLink(String),
    /// The button of **Show me** of a feature named by an answer (`help.show-me`).
    ShowMe(FeatureId),
}

/// The Ask Gitardo page over the rectangle its caller clipped it to: **New conversation** at the top while there is a
/// conversation; the conversation, or, empty, what the page is for with three examples, or, with no AI provider, how
/// to set one up; and at the foot the box that grows to 6 lines with **Ask**, or **Stop** while an answer comes.
///
/// Feature: help.page
pub fn page_view(state: &HelpState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut intents = Vec::new();
    ui.region("help page", (rect, AccessRole::Main, words::PAGE), |ui| {
        let padded = inset(rect, Insets::all(spacing.large));
        let width = padded.width.min(WIDEST);
        let column = Rect::new(padded.x + (padded.width - width) * 0.5, padded.y, width, padded.height);
        let mut area = Area::with_gap(column, spacing.medium);
        let foot = area.bottom(foot::height(state, ui));
        intents.extend(foot::foot_view(state, ui, foot));
        if !state.exchanges.is_empty() {
            let top = area.top(theme.sizes.control_height);
            intents.extend(new_conversation(ui, top));
        }
        let body = area.rest();
        if !state.has_provider {
            intents.extend(no_provider(ui, body));
        } else if state.exchanges.is_empty() {
            intents.extend(empty(state, ui, body));
        } else {
            intents.extend(ui.clip(body, |ui| conversation::conversation_view(state, ui)));
        }
    });
    intents
}

/// **New conversation**, at the right of the top of the page.
fn new_conversation(ui: &mut Ui<'_>, row: Rect) -> Vec<Intent> {
    let button = Button::secondary(words::NEW_CONVERSATION).with_icon(crate::icons::ADD);
    let (width, height) = ui.button_size(&button);
    let cell = place(row, (width, height), (Align::End, Align::Center));
    if ui.button("help new conversation", cell, &button).clicked {
        return vec![Intent::Help(HelpIntent::NewConversation)];
    }
    Vec::new()
}

/// The page with no question yet: what it is for, then the three examples, each a button that asks it.
fn empty(state: &HelpState<'_>, ui: &mut Ui<'_>, body: Rect) -> Vec<Intent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let row = theme.sizes.row_height;
    let height = row + spacing.large + 3.0 * theme.sizes.control_height + 2.0 * spacing.small;
    let top = body.y + ((body.height - height) * 0.4).max(0.0);
    let line = Rect::new(body.x, top, body.width, row);
    let _ = ui.label(line, &Label::new(words::EMPTY).tone(Tone::Muted).align(Align::Center));
    let mut y = line.bottom() + spacing.large;
    let mut intents = Vec::new();
    for (index, example) in words::EXAMPLES.into_iter().enumerate() {
        let mut button = Button::secondary(example);
        if state.is_asking {
            button = button.disabled();
        }
        let (width, button_height) = ui.button_size(&button);
        let cell = place(
            Rect::new(body.x, y, body.width, button_height),
            (width.min(body.width), button_height),
            (Align::Center, Align::Start),
        );
        if ui.button(("help example", index), cell, &button).clicked {
            intents.push(Intent::Help(HelpIntent::AskAbout(example.to_owned())));
        }
        y += button_height + spacing.small;
    }
    intents
}

/// The page with no AI provider: what to set up, and **Open the settings**, which shows `ai.provider`.
fn no_provider(ui: &mut Ui<'_>, body: Rect) -> Vec<Intent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let row = theme.sizes.row_height;
    let top = body.y + ((body.height - row - spacing.large - theme.sizes.control_height) * 0.4).max(0.0);
    let line = Rect::new(body.x, top, body.width, row);
    let said = Label::new(model::help::NO_PROVIDER)
        .kind(TextKind::Body)
        .tone(Tone::Muted)
        .align(Align::Center);
    let _ = ui.label(line, &said);
    let button = Button::primary(words::OPEN_SETTINGS).with_icon(crate::icons::SETTINGS);
    let (width, height) = ui.button_size(&button);
    let cell = place(
        Rect::new(body.x, line.bottom() + spacing.large, body.width, height),
        (width, height),
        (Align::Center, Align::Start),
    );
    if ui.button("help open settings", cell, &button).clicked {
        return vec![Intent::Help(HelpIntent::OpenSettings)];
    }
    Vec::new()
}
