//! The JSON of what version 1.11 adds for milestone M31 (Ask Gitardo, D76): `read/help.ask`, a question of the Chat of
//! VS Code answered with the guide of the extension, and the titles of the features a `help` question may name, sent
//! with `host/languageModel` so that the Chat writes them as the answer comes.

use model::help::{Exchange, GuideCard, HelpAnswer, HelpInput};
use serde::{Deserialize, Serialize};

/// The names of the methods of the milestone.
pub mod methods {
    /// A question to Ask Gitardo, asked in the Chat (`help.ask`).
    pub const ASK: &str = "read/help.ask";
}

/// A question asked earlier in the Chat, with its answer as the Chat wrote it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeJson {
    /// The question, in the user's words.
    pub question: String,
    /// The answer.
    pub answer: String,
}

/// The parameters of `read/help.ask`, which takes no `repository`: nothing of one is read or sent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelpAskParams {
    /// The question, in the user's words.
    pub question: String,
    /// The questions asked before it in the Chat and their answers, oldest first.
    #[serde(default)]
    pub earlier: Vec<ExchangeJson>,
    /// The id of the model chosen in the Chat for the question; none for VS Code's first.
    #[serde(default)]
    pub model: Option<String>,
}

impl HelpAskParams {
    /// What the question is asked from.
    #[must_use]
    pub fn input(&self) -> HelpInput {
        HelpInput {
            question: self.question.clone(),
            earlier: self
                .earlier
                .iter()
                .map(|exchange| Exchange {
                    question: exchange.question.clone(),
                    answer: exchange.answer.clone(),
                })
                .collect(),
        }
    }
}

/// A feature an answer may name as `[[` its identifier `]]`, with the title the Chat writes in its place.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureTitleJson {
    /// The identifier of its card.
    pub id: String,
    /// The title of its card.
    pub title: String,
}

impl From<&GuideCard> for FeatureTitleJson {
    fn from(card: &GuideCard) -> FeatureTitleJson {
        FeatureTitleJson {
            id: card.id.0.to_owned(),
            title: card.title.to_owned(),
        }
    }
}

/// The answer of `read/help.ask`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum HelpAnswerJson {
    /// The answer, read with the guide of the extension.
    Answered {
        /// Its Markdown, each feature it names written as the title of its card in bold.
        markdown: String,
        /// The identifiers of the features it names, each once, in the order first named.
        named: Vec<String>,
        /// The name of the model that wrote it.
        model: String,
    },
    /// No answer: the model failed or refused.
    Failed {
        /// Why, in the host's words.
        message: String,
    },
}

impl From<&HelpAnswer> for HelpAnswerJson {
    fn from(answer: &HelpAnswer) -> HelpAnswerJson {
        HelpAnswerJson::Answered {
            markdown: answer.markdown.clone(),
            named: answer.named.iter().map(|id| id.0.to_owned()).collect(),
            model: answer.model.clone(),
        }
    }
}
