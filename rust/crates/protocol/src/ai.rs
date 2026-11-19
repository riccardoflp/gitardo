//! The JSON of what version 1.6 adds for area 6 of stage 4, AI and GitHub: the host questions to the language models
//! of VS Code (`host/languageModel`, `host/languageModels`), and the two reads that ask them, the draft of a commit
//! message (`read/commit.draft-message`) and the question about one conflict (`read/ai.ask-conflict`).

use engine::ai::{AiError, ConflictReply, Purpose};
use model::ai::{AiModel, Confidence};
use serde::{Deserialize, Serialize};

use crate::rpc::RequestId;

/// The names of the methods of the area.
pub mod methods {
    /// The engine asks the host's language model a question.
    pub const HOST_LANGUAGE_MODEL: &str = "host/languageModel";
    /// The engine asks which language models the host has.
    pub const HOST_LANGUAGE_MODELS: &str = "host/languageModels";
    /// The draft of a commit message, from the changes (`commit.draft-message`).
    pub const DRAFT_MESSAGE: &str = "read/commit.draft-message";
    /// The question about one conflict (`ai.ask-conflict`).
    pub const ASK_CONFLICT: &str = "read/ai.ask-conflict";
}

/// What a question to a language model is for (`Purpose` of the engine).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PurposeJson {
    /// The draft of a commit message.
    CommitMessage,
    /// How to resolve one conflict.
    Conflict,
    /// How to do something with Gitardo (`help.ask`, version 1.11).
    Help,
}

impl From<Purpose> for PurposeJson {
    fn from(purpose: Purpose) -> PurposeJson {
        match purpose {
            Purpose::CommitMessage => PurposeJson::CommitMessage,
            Purpose::Conflict => PurposeJson::Conflict,
            Purpose::Help => PurposeJson::Help,
        }
    }
}

/// Who says a message to a language model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RoleJson {
    /// The user, that is Gitardo.
    User,
    /// The model, in an answer before.
    Assistant,
}

/// A message to a language model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelMessageJson {
    /// Who says it.
    pub role: RoleJson,
    /// What it says.
    pub text: String,
}

/// The parameters of `host/languageModel` (`LanguageModelRequest`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageModelParams {
    /// What it is for, so that the host says why it sends it.
    pub purpose: PurposeJson,
    /// The id of the model asked; none for the host's first.
    pub model: Option<String>,
    /// How much the model thinks before it answers, as `ai.effort` names it; none for as the model is set up.
    pub effort: Option<String>,
    /// The messages, in their order.
    pub messages: Vec<ModelMessageJson>,
    /// The id of the extension's read it is asked for, so that the extension shows the answer as it comes where that
    /// read was asked; none when a feature asks.
    pub read: Option<RequestId>,
}

/// The answer of `host/languageModel`: the whole answer and the name of the model that wrote it, or why the host could
/// not ask, in its own words.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LanguageModelResult {
    /// The model answered.
    Answered {
        /// Its whole answer.
        text: String,
        /// The name of the model, as people know it.
        model: String,
    },
    /// The host could not ask: no model, the one asked is not there, or the model refused.
    Refused {
        /// Why, in the host's words.
        refused: String,
    },
}

/// A language model of the host.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageModelJson {
    /// Its id, as `ai.model` names it.
    pub id: String,
    /// Its name for people.
    pub name: String,
}

impl From<LanguageModelJson> for AiModel {
    fn from(model: LanguageModelJson) -> AiModel {
        AiModel {
            id: model.id,
            name: model.name,
        }
    }
}

/// The answer of `host/languageModels`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageModelsResult {
    /// The models, in the host's order.
    pub models: Vec<LanguageModelJson>,
}

/// The parameters of `read/commit.draft-message`, besides `repository`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftParams {
    /// The subjects of the last commits as the graph lists them, newest first, which the draft matches the style of.
    #[serde(default)]
    pub subjects: Vec<String>,
}

/// The answer of `read/commit.draft-message`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DraftJson {
    /// The message drafted, cleaned as the commit box takes it.
    Drafted {
        /// The message.
        message: String,
    },
    /// No message: nothing to describe, or the model could not answer.
    Failed {
        /// Why.
        message: String,
    },
}

/// A conflict as the review of the extension has it: where it is in the file, and its sides.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskedConflictJson {
    /// The line of its first marker, from 0.
    pub start: usize,
    /// The line after its last marker.
    pub end: usize,
    /// The current side's lines between the markers, without what ends them.
    pub current: Vec<String>,
    /// The incoming side's lines.
    pub incoming: Vec<String>,
    /// The base, when it is known: what the rules found the sides changed, else what git wrote between `|||||||` and
    /// `=======`.
    pub base: Option<Vec<String>>,
}

/// The parameters of `read/ai.ask-conflict`, besides `repository`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskConflictParams {
    /// The file, from the top of the working tree.
    pub path: String,
    /// Its lines, without what ends them, as the review read them.
    pub lines: Vec<String>,
    /// The conflict asked about.
    pub conflict: AskedConflictJson,
}

/// How sure the model said it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfidenceJson {
    /// High.
    High,
    /// Medium, or anything it did not say as high or low.
    Medium,
    /// Low.
    Low,
}

impl From<Confidence> for ConfidenceJson {
    fn from(confidence: Confidence) -> ConfidenceJson {
        match confidence {
            Confidence::High => ConfidenceJson::High,
            Confidence::Medium => ConfidenceJson::Medium,
            Confidence::Low => ConfidenceJson::Low,
        }
    }
}

/// The answer of `read/ai.ask-conflict`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ConflictAnswerJson {
    /// The model's resolution, read and trimmed.
    Answered {
        /// The name of the model that wrote it.
        model: String,
        /// The lines that take the place of the conflict, without what ends them.
        lines: Vec<String>,
        /// Why, in one or two sentences.
        reason: String,
        /// How sure it is.
        confidence: ConfidenceJson,
    },
    /// No resolution.
    Failed {
        /// Why.
        message: String,
        /// Whether no other conflict is to be asked about: the model chosen is not there.
        stops_the_rest: bool,
    },
}

impl ConflictAnswerJson {
    /// The JSON of what asking came to.
    #[must_use]
    pub fn of(asked: &Result<ConflictReply, AiError>) -> ConflictAnswerJson {
        match asked {
            Ok(reply) => ConflictAnswerJson::Answered {
                model: reply.model.clone(),
                lines: reply.answer.lines.clone(),
                reason: reply.answer.reason.clone(),
                confidence: reply.answer.confidence.into(),
            },
            Err(error) => ConflictAnswerJson::Failed {
                message: error.to_string(),
                stops_the_rest: matches!(error, AiError::ModelMissing(_)),
            },
        }
    }
}
