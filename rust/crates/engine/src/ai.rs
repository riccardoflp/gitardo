//! The language models: the AI provider of `ai.provider` over HTTP, or those of the host the engine runs under
//! ([`LanguageModel`]); the models listed (`ai.choose-model`), a question asked with its answer streamed, the draft of
//! a commit message (`commit.draft-message`), and the question about one conflict (`ai.ask-conflict`).
//!
//! Nothing here is sent unless a person asked for it: the window calls these functions only from a button, and the
//! tests send them to a server of their own on 127.0.0.1.

mod copilot;
mod wire;

use std::fmt;
use std::io::BufReader;

use http::{Cancel, Events, Http};
use model::ai::{
    AiModel, AiModels, AiProvider, AiRequest, AiSettings, AnswerError, ConflictAnswer, Key, QuestionInput, TrimInput,
};
use model::commit::{DraftInput, clean_draft, draft_message};

pub use copilot::{
    ARGUMENTS, CopilotCli, CopilotError, CopilotModel, CopilotStreams, copilot_model, find_copilot, program_extensions,
};
pub use wire::{ANTHROPIC_VERSION, MOST_TOKENS};

/// The provider asked, with its key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provider {
    /// The settings: the provider, its address, the model and the effort.
    pub settings: AiSettings,
    /// The key, from the credential store of the system or just given; none when there is none.
    pub key: Option<Key>,
}

/// How a request is sent: with what, and what cancels it.
#[derive(Clone, Copy, Debug)]
pub struct Sending<'a> {
    /// The client.
    pub http: &'a Http,
    /// What cancels the request: the token of the job that sends it.
    pub cancel: &'a Cancel,
}

/// What a question to a language model is for, so that the host says why it sends it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    /// The draft of a commit message (`commit.draft-message`).
    CommitMessage,
    /// How to resolve one conflict (`ai.ask-conflict`).
    Conflict,
}

/// Who answers the questions to a language model: the AI provider over HTTP in Gitardo Desktop
/// ([`ProviderModel`]), GitHub Copilot through its CLI ([`CopilotModel`]), or the models of the host the engine runs
/// under, which the server asks of the extension.
pub trait LanguageModel {
    /// The models it has, in its order.
    ///
    /// # Errors
    ///
    /// When they cannot be listed.
    fn models(&mut self) -> Result<Vec<AiModel>, AiError>;

    /// Asks a question of the model its settings choose, or of its first model when they choose none; each piece of
    /// the answer is given to `pieces` as it comes, before the whole answer is.
    ///
    /// # Errors
    ///
    /// When there is no model, the one chosen is not there, the question is refused or cancelled, or the answer
    /// cannot be read.
    fn ask(&mut self, purpose: Purpose, request: &AiRequest, pieces: &mut dyn FnMut(&str)) -> Result<Asked, AiError>;
}

/// The AI provider of `ai.provider`, asked over HTTP as Gitardo Desktop asks it.
#[derive(Clone, Copy, Debug)]
pub struct ProviderModel<'a> {
    /// How the requests are sent.
    pub sending: Sending<'a>,
    /// The provider, with its key.
    pub provider: &'a Provider,
}

impl LanguageModel for ProviderModel<'_> {
    fn models(&mut self) -> Result<Vec<AiModel>, AiError> {
        list_models(self.sending, self.provider)
    }

    fn ask(&mut self, _purpose: Purpose, request: &AiRequest, pieces: &mut dyn FnMut(&str)) -> Result<Asked, AiError> {
        ask(self.sending, self.provider, request, pieces)
    }
}

/// What listing the models came to when it failed: what GitHub Copilot needs first, or why, in words.
#[must_use]
pub fn failed_models(error: AiError) -> AiModels {
    match error {
        AiError::Copilot(CopilotError::Needs(need)) => AiModels::CopilotNeeds(need),
        error => AiModels::Failed(error.to_string()),
    }
}

/// What went wrong asking the AI provider.
#[derive(Debug)]
pub enum AiError {
    /// `ai.provider` is empty.
    NotSetUp,
    /// The provider needs a key and none is kept.
    NoKey,
    /// The provider refused the key.
    KeyRefused,
    /// The model of `ai.model` is not one the provider has.
    ModelMissing(String),
    /// The provider lists no model.
    NoModels,
    /// The request could not be sent, or its answer read.
    Http(http::Error),
    /// The provider answered with a status that is not 2xx: the status with its reason, and what it said.
    Status {
        /// `500 Internal Server Error`.
        line: String,
        /// The message of its answer, when it has one.
        message: String,
    },
    /// The provider stopped the answer with an error, or the model declined to answer.
    Refused(String),
    /// The answer is not what the provider's interface says.
    Unreadable(String),
    /// The answer is not a resolution (`ai.read-answer`, `ai.trim-answer`).
    Answer(AnswerError),
    /// The question was cancelled.
    Cancelled,
    /// The host the engine runs under could not ask its model, and said why in its own words.
    Host(String),
    /// GitHub Copilot could not be asked (`ai.copilot`).
    Copilot(CopilotError),
}

impl From<http::Error> for AiError {
    fn from(error: http::Error) -> AiError {
        match error {
            http::Error::Cancelled => AiError::Cancelled,
            error => AiError::Http(error),
        }
    }
}

impl fmt::Display for AiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AiError::NotSetUp => formatter.write_str("No AI provider is set up."),
            AiError::NoKey => formatter
                .write_str("No key of the AI provider is kept: choose the model that resolves conflicts to give one."),
            AiError::KeyRefused => formatter.write_str(
                "The AI provider refused the key: choose the model that resolves conflicts to give another one.",
            ),
            AiError::ModelMissing(model) => write!(
                formatter,
                "The model chosen to resolve conflicts, {model}, is not available: choose another one."
            ),
            AiError::NoModels => {
                formatter.write_str("No language model is available from the AI provider. Set one up in the settings.")
            }
            AiError::Http(error) => error.fmt(formatter),
            AiError::Status { line, message } if message.is_empty() => {
                write!(formatter, "The AI provider answered {line}.")
            }
            AiError::Status { line, message } => write!(formatter, "The AI provider answered {line}: {message}"),
            AiError::Refused(message) => write!(formatter, "The AI provider refused the request: {message}"),
            AiError::Unreadable(reason) => {
                write!(formatter, "The answer of the AI provider could not be read: {reason}")
            }
            AiError::Answer(error) => formatter.write_str(error.0.as_str()),
            AiError::Cancelled => formatter.write_str("The question was cancelled."),
            AiError::Host(said) => formatter.write_str(said),
            AiError::Copilot(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for AiError {}

/// The models the provider lists, in its order.
///
/// # Errors
///
/// When no provider is set up, it wants a key that is not kept or refuses it, or the request fails.
pub fn list_models(sending: Sending<'_>, provider: &Provider) -> Result<Vec<AiModel>, AiError> {
    let kind = http_kind(provider)?;
    let request = wire::models_request(kind, provider)?;
    let response = sending.http.send(&request, sending.cancel)?;
    if !response.head.is_success() {
        return Err(wire::status_error(&response.head, &response.body, provider, None));
    }
    wire::parse_models(kind, &response.body)
}

/// The kind of provider asked over HTTP; GitHub Copilot is asked through its CLI ([`CopilotModel`]) instead.
fn http_kind(provider: &Provider) -> Result<AiProvider, AiError> {
    match provider.settings.provider {
        None => Err(AiError::NotSetUp),
        Some(AiProvider::Copilot) => Err(AiError::Copilot(CopilotError::NotOverHttp)),
        Some(kind) => Ok(kind),
    }
}

/// What listing the models came to, as `ai.choose-model` reads it.
#[must_use]
pub fn models_answer(sending: Sending<'_>, provider: &Provider) -> AiModels {
    match list_models(sending, provider) {
        Ok(models) => AiModels::Listed(models),
        Err(AiError::NoKey | AiError::KeyRefused) => AiModels::KeyNeeded,
        Err(error) => failed_models(error),
    }
}

/// What the model answered, and which model it was.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asked {
    /// The model asked: that of `ai.model`, or the first the provider lists.
    pub model: String,
    /// Its whole answer.
    pub text: String,
}

/// Asks the model, its answer streamed: each piece is given to `pieces` as it comes, before the whole answer is.
///
/// # Errors
///
/// When the provider is not set up, the model is not there, the provider refuses, the answer cannot be read, or the
/// question is cancelled.
pub fn ask(
    sending: Sending<'_>,
    provider: &Provider,
    request: &AiRequest,
    pieces: &mut dyn FnMut(&str),
) -> Result<Asked, AiError> {
    let kind = http_kind(provider)?;
    let model = if provider.settings.model.is_empty() {
        let models = list_models(sending, provider)?;
        models.into_iter().next().ok_or(AiError::NoModels)?.id
    } else {
        provider.settings.model.clone()
    };
    let sent = wire::ask_request(kind, provider, &model, request)?;
    let mut opened = sending.http.open(&sent, sending.cancel)?;
    if !opened.head.is_success() {
        let body = opened.body.read_all()?;
        let chosen = (!provider.settings.model.is_empty()).then_some(model.as_str());
        return Err(wire::status_error(&opened.head, &body, provider, chosen));
    }
    let mut events = Events::new(BufReader::new(opened.body));
    let mut text = String::new();
    loop {
        if sending.cancel.is_cancelled() {
            return Err(AiError::Cancelled);
        }
        let event = match events.next_event() {
            Ok(Some(event)) => event,
            Ok(None) => break,
            // A request cancelled while its answer is read fails that way: the failure is the cancelling.
            Err(_) if sending.cancel.is_cancelled() => return Err(AiError::Cancelled),
            Err(error) => return Err(AiError::Unreadable(error.to_string())),
        };
        match wire::read_event(kind, &event)? {
            wire::Read::Piece(piece) => {
                text.push_str(&piece);
                pieces(&piece);
            }
            wire::Read::Nothing => {}
            wire::Read::End => break,
        }
    }
    if sending.cancel.is_cancelled() {
        return Err(AiError::Cancelled);
    }
    Ok(Asked { model, text })
}

/// The question about one conflict, and what its answer is checked against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictRequest {
    /// The question (`ai.conflict-question`).
    pub question: String,
    /// The lines sent as `<before>`.
    pub before: Vec<String>,
    /// The lines sent as `<after>`.
    pub after: Vec<String>,
    /// Whether a side of the conflict had lines.
    pub has_side_lines: bool,
}

impl ConflictRequest {
    /// The question about a conflict of a file (`ai.conflict-question`), with the lines sent around it.
    #[must_use]
    pub fn of(input: &QuestionInput<'_>) -> ConflictRequest {
        let around = model::ai::lines_around(input);
        ConflictRequest {
            question: model::ai::conflict_question(input),
            before: around.before.iter().map(|line| (*line).to_owned()).collect(),
            after: around.after.iter().map(|line| (*line).to_owned()).collect(),
            has_side_lines: !input.conflict.current.is_empty() || !input.conflict.incoming.is_empty(),
        }
    }
}

/// What the model proposed for a conflict, and which model it was.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictReply {
    /// The model asked.
    pub model: String,
    /// Its resolution, read and trimmed.
    pub answer: ConflictAnswer,
}

/// Sends the question about one conflict and returns its answer: the pieces as they come, then the resolution read
/// (`ai.read-answer`) without the lines around the conflict it wrote again (`ai.trim-answer`).
///
/// Feature: ai.ask-conflict
///
/// # Errors
///
/// As [`LanguageModel::ask`], and when the answer is not a resolution.
pub fn ask_conflict(
    model: &mut dyn LanguageModel,
    request: &ConflictRequest,
    pieces: &mut dyn FnMut(&str),
) -> Result<ConflictReply, AiError> {
    let asked = model.ask(
        Purpose::Conflict,
        &AiRequest {
            prompt: request.question.clone(),
        },
        pieces,
    )?;
    let answer = model::ai::read_answer(&asked.text).map_err(AiError::Answer)?;
    let before: Vec<&str> = request.before.iter().map(String::as_str).collect();
    let after: Vec<&str> = request.after.iter().map(String::as_str).collect();
    let trimmed = model::ai::trim_answer(
        answer,
        &TrimInput {
            before: &before,
            after: &after,
            has_side_lines: request.has_side_lines,
        },
    )
    .map_err(AiError::Answer)?;
    Ok(ConflictReply {
        model: asked.model,
        answer: trimmed,
    })
}

/// Asks the model for the message of a commit from the changes (`commit.draft-message`), the pieces of its answer as
/// they come, and gives the answer cleaned as the message box takes it.
///
/// # Errors
///
/// As [`LanguageModel::ask`].
pub fn draft(
    model: &mut dyn LanguageModel,
    input: &DraftInput<'_>,
    pieces: &mut dyn FnMut(&str),
) -> Result<String, AiError> {
    let asked = model.ask(Purpose::CommitMessage, &draft_message(input), pieces)?;
    Ok(clean_draft(&asked.text))
}

/// Whether a provider cannot be asked without a key.
#[must_use]
pub fn needs_key(provider: AiProvider) -> bool {
    match provider {
        AiProvider::Anthropic => true,
        // A server of the chat completions on the user's own machine often wants none, and GitHub Copilot is signed
        // in to with its CLI, which Gitardo sends no key.
        AiProvider::OpenAiCompatible | AiProvider::Copilot => false,
    }
}

#[cfg(test)]
mod tests;
