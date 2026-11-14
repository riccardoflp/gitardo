//! The AI provider of `ai.provider`, over HTTP: the models it lists (`ai.choose-model`), a question asked with its
//! answer streamed (`commit.draft-message`), and the question about one conflict (`ai.ask-conflict`).
//!
//! Nothing here is sent unless a person asked for it: the window calls these functions only from a button, and the
//! tests send them to a server of their own on 127.0.0.1.

mod wire;

use std::fmt;
use std::io::BufReader;

use http::{Cancel, Events, Http};
use model::ai::{AiModel, AiModels, AiProvider, AiRequest, AiSettings, AnswerError, ConflictAnswer, Key, TrimInput};

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
    let kind = provider.settings.provider.ok_or(AiError::NotSetUp)?;
    let request = wire::models_request(kind, provider)?;
    let response = sending.http.send(&request, sending.cancel)?;
    if !response.head.is_success() {
        return Err(wire::status_error(&response.head, &response.body, provider, None));
    }
    wire::parse_models(kind, &response.body)
}

/// What listing the models came to, as `ai.choose-model` reads it.
#[must_use]
pub fn models_answer(sending: Sending<'_>, provider: &Provider) -> AiModels {
    match list_models(sending, provider) {
        Ok(models) => AiModels::Listed(models),
        Err(AiError::NoKey | AiError::KeyRefused) => AiModels::KeyNeeded,
        Err(error) => AiModels::Failed(error.to_string()),
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
    let kind = provider.settings.provider.ok_or(AiError::NotSetUp)?;
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
/// As [`ask`], and when the answer is not a resolution.
pub fn ask_conflict(
    sending: Sending<'_>,
    provider: &Provider,
    request: &ConflictRequest,
    pieces: &mut dyn FnMut(&str),
) -> Result<ConflictReply, AiError> {
    let asked = ask(
        sending,
        provider,
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

/// Whether a provider cannot be asked without a key.
#[must_use]
pub fn needs_key(provider: AiProvider) -> bool {
    match provider {
        AiProvider::Anthropic => true,
        // A server of the chat completions on the user's own machine often wants none.
        AiProvider::OpenAiCompatible => false,
    }
}

#[cfg(test)]
mod tests;
