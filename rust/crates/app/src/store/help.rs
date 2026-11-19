//! The Ask Gitardo page (`help.page`, decision D76): the conversation of the window, each question asked of the AI
//! provider with the ones before it and their answers, the answer read as it comes; forgotten when the page closes.

use engine::help::HelpRequest;
use model::help::{App, Exchange, HelpInput, guide, read_answer};
use ui::editor::{Cursor, Document, apply};
use views::Intent;
use views::file_view::MarkdownContent;
use views::help::{HelpExchange, HelpIntent, HelpState};
use views::page::Page;
use views::search::{QuickChoice, SearchIntent};
use views::settings::SettingsAnchor;

use super::message::{Effect, Message};
use super::state::State;
use crate::work::ai::{AiQuestion, AiReply, AiResult, AiWork, AskId};

/// The conversation of the Ask Gitardo page, and its box.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Help {
    /// The questions asked, each with its answer as the page draws it.
    pub exchanges: Vec<HelpExchange>,
    /// The text of each answer as the model wrote it, sent with the questions after it.
    pub answers: Vec<String>,
    /// The question whose answer comes.
    pub asking: Option<AskId>,
    /// The text of the box.
    pub document: Document,
    /// The caret and the selection of the box.
    pub cursor: Cursor,
    /// A number that changes each time the box takes the keyboard.
    pub focus: u64,
    /// A number that changes each time the end of the conversation is brought into sight.
    pub reveal: u64,
}

/// What the page shows.
#[must_use]
pub fn page_state(state: &State) -> HelpState<'_> {
    let help = &state.help;
    HelpState {
        exchanges: &help.exchanges,
        has_provider: state.settings.features.ai.provider.is_some(),
        is_asking: help.asking.is_some(),
        question: help.document.buffer(),
        cursor: help.cursor,
        focus: help.focus,
        reveal: help.reveal,
    }
}

/// What the page asked.
pub fn reduce(state: &mut State, intent: HelpIntent) -> Vec<Effect> {
    match intent {
        HelpIntent::Open => {
            open(state);
            Vec::new()
        }
        HelpIntent::AskAbout(question) => {
            open(state);
            ask(state, question)
        }
        HelpIntent::Ask => {
            let question = state.help.document.buffer().text().trim().to_owned();
            if question.is_empty() || state.help.asking.is_some() {
                return Vec::new();
            }
            state.help.document = Document::default();
            state.help.cursor = Cursor::default();
            ask(state, question)
        }
        HelpIntent::Write(commands) => {
            let help = &mut state.help;
            for command in &commands {
                let _changes = apply(&mut help.document, &mut help.cursor, command);
            }
            Vec::new()
        }
        HelpIntent::Stop => stop(state),
        HelpIntent::NewConversation => {
            let effects = stop(state);
            state.help.exchanges.clear();
            state.help.answers.clear();
            effects
        }
        HelpIntent::TryAgain => try_again(state),
        HelpIntent::OpenSettings => {
            let reveal = QuickChoice::Settings(SettingsAnchor::Setting("ai.provider"));
            super::reduce(
                state,
                Message::Intent(Intent::Search(SearchIntent::ChooseQuick(reveal))),
            )
        }
        HelpIntent::OpenLink(link) => open_link(state, &link),
        HelpIntent::ShowMe(_) => Vec::new(),
    }
}

/// Shows the page, opening it when it is not open, its box taking the keyboard.
fn open(state: &mut State) {
    super::pages::show_page(state, Page::Help);
    state.help.focus += 1;
}

/// A link of an answer: an address of the web opens in the browser; the others, the features of `help.show-me`
/// apart, go nowhere.
fn open_link(state: &mut State, link: &str) -> Vec<Effect> {
    let _ = state;
    if link.starts_with("https://") || link.starts_with("http://") {
        return vec![Effect::OpenUrl(link.to_owned())];
    }
    Vec::new()
}

/// Asks a question of the AI provider, with the questions before it that were answered: refused while another is
/// answered and with no AI provider, which the page says.
fn ask(state: &mut State, question: String) -> Vec<Effect> {
    let settings = (*state.settings.features.ai).clone();
    if state.help.asking.is_some() || settings.provider.is_none() {
        return Vec::new();
    }
    let help = &state.help;
    let earlier = help
        .exchanges
        .iter()
        .zip(&help.answers)
        .filter(|(exchange, _)| exchange.failed.is_none())
        .map(|(exchange, answer)| Exchange {
            question: exchange.question.clone(),
            answer: answer.clone(),
        })
        .collect();
    let input = HelpInput {
        question: question.clone(),
        earlier,
    };
    let id = super::review::ai::next_ask(state);
    let help = &mut state.help;
    help.exchanges.push(HelpExchange {
        question,
        ..HelpExchange::default()
    });
    help.answers.push(String::new());
    help.asking = Some(id);
    help.reveal += 1;
    vec![Effect::Ask(Box::new(AiQuestion {
        id,
        settings,
        work: AiWork::Help(HelpRequest {
            app: App::Desktop,
            input,
        }),
    }))]
}

/// **Stop**: the answer stops coming, what came of it kept, and the box is given back.
fn stop(state: &mut State) -> Vec<Effect> {
    let Some(ask) = state.help.asking.take() else {
        return Vec::new();
    };
    state.help.focus += 1;
    vec![Effect::StopAsking(vec![ask])]
}

/// **Try again**: the last question, which failed, asked again in its place.
fn try_again(state: &mut State) -> Vec<Effect> {
    let help = &mut state.help;
    if help.asking.is_some() || help.exchanges.last().is_none_or(|exchange| exchange.failed.is_none()) {
        return Vec::new();
    }
    let Some(failed) = help.exchanges.pop() else {
        return Vec::new();
    };
    let _answer = help.answers.pop();
    ask(state, failed.question)
}

/// Forgets the conversation, the page being closed: the answer that comes stops.
pub fn forget(state: &mut State) -> Vec<Effect> {
    let effects = stop(state);
    state.help = Help::default();
    effects
}

/// Whether a message of the AI provider is about the question of the page.
#[must_use]
pub fn is_answering(state: &State, message: &Message) -> bool {
    match message {
        Message::AiPiece { ask, .. } | Message::AiAnswered { ask, .. } => state.help.asking == Some(*ask),
        _ => false,
    }
}

/// What the AI provider said of the question of the page: its answer so far, or what it came to.
pub fn reply(state: &mut State, message: Message) -> Vec<Effect> {
    match message {
        Message::AiPiece { text, .. } => {
            show_answer(state, &text);
            if let Some(answer) = state.help.answers.last_mut() {
                *answer = text;
            }
        }
        Message::AiAnswered { result, .. } => answered(state, result),
        _ => {}
    }
    Vec::new()
}

/// What the question came to: its answer read, or why there is none, under it; the box is given back.
fn answered(state: &mut State, result: AiResult) {
    state.help.asking = None;
    state.help.focus += 1;
    match result {
        Ok(AiReply::Help { answer, text }) => {
            if let Some(exchange) = state.help.exchanges.last_mut() {
                exchange.has_words = !answer.markdown.trim().is_empty();
                exchange.answer = content(&answer.markdown);
            }
            if let Some(kept) = state.help.answers.last_mut() {
                *kept = text;
            }
        }
        Ok(AiReply::Conflict(_) | AiReply::Draft(_)) => {}
        Err(failed) => {
            if let Some(need) = failed.need {
                state.operations.said = Some(super::draft::copilot_said(need));
            }
            if let Some(exchange) = state.help.exchanges.last_mut() {
                exchange.failed = Some(failed.text);
            }
        }
    }
}

/// The answer so far read as the page draws it, without a `[[` whose end has not come yet.
fn show_answer(state: &mut State, text: &str) {
    let answer = read_answer(text, &guide(App::Desktop));
    if let Some(exchange) = state.help.exchanges.last_mut() {
        exchange.has_words = !answer.markdown.trim().is_empty();
        exchange.answer = content(&answer.markdown);
    }
}

/// The Markdown of an answer, laid out as the preview of a file lays it out.
fn content(markdown: &str) -> MarkdownContent {
    MarkdownContent {
        blocks: model::markdown::blocks(markdown),
        ..MarkdownContent::default()
    }
}
