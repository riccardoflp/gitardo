//! Milestone M31, Ask Gitardo in the Chat of VS Code (`read/help.ask`, version 1.11): the two requests of a question
//! asked of the model chosen in the Chat with `host/languageModel`, the second under the id of the read so that the
//! extension writes its text in the Chat as it comes, and the answer read with the guide of the extension. Nothing of a
//! repository is read or sent, and the read takes none, so it is not logged, as `read/file-view.mermaid` is not.

use std::sync::Arc;

use engine::ai::{AiError, Asked, LanguageModel, Purpose};
use engine::help::{HelpRequest, said};
use git::CancelToken;
use model::ai::{AiModel, AiRequest, AiSettings};
use model::help::{App, Guide, guide, read_chosen};
use protocol::help::{FeatureTitleJson, HelpAnswerJson, HelpAskParams};
use protocol::rpc::{RequestId, ResponseError};
use serde_json::Value;

use crate::host::HostQuestions;
use crate::session::{Session, json};

/// The model chosen in the Chat, asked the two requests of a question: the first with no read, since the Chat shows
/// nothing of the cards it chooses, the second under the id of the read and with the titles of the cards chosen, which
/// the extension writes in place of each `[[id]]` as the answer comes.
struct AskedInTheChat {
    /// The questions of the first request.
    choosing: HostQuestions,
    /// The questions of the second request.
    answering: HostQuestions,
    /// The guide of the extension, whose cards the first answer chooses.
    guide: Guide,
    /// Whether the first request was asked.
    has_chosen: bool,
}

impl LanguageModel for AskedInTheChat {
    fn models(&mut self) -> Result<Vec<AiModel>, AiError> {
        self.choosing.models()
    }

    fn ask(&mut self, purpose: Purpose, request: &AiRequest, pieces: &mut dyn FnMut(&str)) -> Result<Asked, AiError> {
        if self.has_chosen {
            return self.answering.ask(purpose, request, pieces);
        }
        self.has_chosen = true;
        let asked = self.choosing.ask(purpose, request, pieces)?;
        let titles = read_chosen(&asked.text, &self.guide)
            .iter()
            .filter_map(|id| self.guide.card(id.0))
            .map(FeatureTitleJson::from)
            .collect();
        self.answering.naming(titles);
        Ok(asked)
    }
}

/// The answer of `read/help.ask`: the question asked of the model chosen in the Chat, with no effort, as the Chat asks
/// it, and its answer read with the guide of the extension; a model that fails or refuses is a `failed` answer, in the
/// host's words.
///
/// # Errors
///
/// When the read is cancelled.
pub fn ask(
    session: &Arc<Session>,
    id: &RequestId,
    params: &HelpAskParams,
    cancel: &CancelToken,
) -> Result<Value, ResponseError> {
    let ai = AiSettings {
        model: params.model.clone().unwrap_or_default(),
        ..AiSettings::default()
    };
    let questions = |read: Option<RequestId>| HostQuestions::new(Arc::clone(session), cancel.clone(), read, ai.clone());
    let mut model = AskedInTheChat {
        choosing: questions(None),
        answering: questions(Some(id.clone())),
        guide: guide(App::Extension),
        has_chosen: false,
    };
    let request = HelpRequest {
        app: App::Extension,
        input: params.input(),
    };
    match engine::help::ask(&mut model, &request, &mut |_| {}) {
        Ok(answer) => json(&HelpAnswerJson::from(&answer)),
        Err(AiError::Cancelled) => Err(crate::ai::cancelled()),
        Err(error) => json(&HelpAnswerJson::Failed { message: said(&error) }),
    }
}
