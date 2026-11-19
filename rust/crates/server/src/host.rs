//! Area 6 of stage 4, AI and GitHub: the host questions the engine asks the extension in place of the AI provider and
//! the credential store of Gitardo Desktop. The token of GitHub is VS Code's sign-in (`host/githubToken`), the language
//! models are VS Code's (`host/languageModels`), and each question to a model is one `host/languageModel`, whose whole
//! answer comes back at once: the prompt and the reading of the answer stay in the engine
//! (`docs/transition/02-protocol.md`, Host questions).

use std::sync::Arc;

use engine::ai::{AiError, Asked, LanguageModel, Purpose};
use git::CancelToken;
use model::ai::{AiModel, AiRequest, AiSettings};
use model::github::Token;
use protocol::ai::{
    LanguageModelParams, LanguageModelResult, LanguageModelsResult, ModelMessageJson, RoleJson, methods as ai_methods,
};
use protocol::help::FeatureTitleJson;
use protocol::reads::{GitHubTokenParams, GitHubTokenResult, methods};
use protocol::rpc::RequestId;
use serde::Serialize;
use serde_json::Value;

use crate::outgoing::NoAnswer;
use crate::session::Session;

/// What the extension says when VS Code has no language model, as it said it before the engine asked: the engine has
/// no other words for it, since the extension lists its models only when asked.
pub const NO_MODEL: &str =
    "No language model is available in VS Code. Install an extension that provides one, such as GitHub Copilot.";

/// The host questions of one request of the extension: a `feature/run`, or a read that asks a model.
pub struct HostQuestions {
    /// Who sends the questions and gets their answers.
    session: Arc<Session>,
    /// The token of the request: cancelling it cancels the question waited for.
    cancel: CancelToken,
    /// The id of the read the questions are asked for, which the extension shows the answer under; none for a feature.
    read: Option<RequestId>,
    /// The model and the effort asked, from `ai.model` and `ai.effort`; the provider is the host's.
    ai: AiSettings,
    /// The features an answer may name, with their titles, which the extension writes in their place as it comes.
    features: Vec<FeatureTitleJson>,
}

impl HostQuestions {
    /// The host questions of a request, with the settings of the model it asks.
    pub fn new(session: Arc<Session>, cancel: CancelToken, read: Option<RequestId>, ai: AiSettings) -> HostQuestions {
        HostQuestions {
            session,
            cancel,
            read,
            ai,
            features: Vec::new(),
        }
    }

    /// Sends with each question the features its answer may name, with their titles.
    pub fn naming(&mut self, features: Vec<FeatureTitleJson>) {
        self.features = features;
    }

    /// Sends a question to the extension and waits for its answer.
    fn request(&self, method: &str, params: &impl Serialize) -> Result<Value, NoAnswer> {
        let params = serde_json::to_value(params).map_err(|error| NoAnswer::Failed(error.to_string()))?;
        let session = &self.session;
        session
            .asked()
            .ask(|message| session.send(message), method, params, &self.cancel)
    }
}

impl engine::Host for HostQuestions {
    /// VS Code's token for github.com, when the extension answers `host/githubToken`; none when it does not, gives
    /// none, cannot answer, or the user backed out of signing in.
    fn github_token(&mut self, ask: bool) -> Option<Token> {
        if !self.session.host().github_token {
            return None;
        }
        let params = GitHubTokenParams {
            host: String::from("github.com"),
            ask,
        };
        let answer = self.request(methods::HOST_GITHUB_TOKEN, &params).ok()?;
        serde_json::from_value::<GitHubTokenResult>(answer)
            .ok()
            .and_then(|result| result.token)
            .filter(|token| !token.is_empty())
            .map(Token)
    }

    /// VS Code's language models, in its order: with none, the words of [`NO_MODEL`].
    fn language_models(&mut self) -> Result<Vec<AiModel>, AiError> {
        let answer = self
            .request(ai_methods::HOST_LANGUAGE_MODELS, &serde_json::json!({}))
            .map_err(no_answer)?;
        let listed: LanguageModelsResult =
            serde_json::from_value(answer).map_err(|error| AiError::Unreadable(error.to_string()))?;
        if listed.models.is_empty() {
            return Err(AiError::Host(String::from(NO_MODEL)));
        }
        Ok(listed.models.into_iter().map(AiModel::from).collect())
    }
}

impl LanguageModel for HostQuestions {
    fn models(&mut self) -> Result<Vec<AiModel>, AiError> {
        engine::Host::language_models(self)
    }

    /// One `host/languageModel`: the extension shows the answer as it comes, so the whole answer is the one piece.
    fn ask(&mut self, purpose: Purpose, request: &AiRequest, pieces: &mut dyn FnMut(&str)) -> Result<Asked, AiError> {
        let params = LanguageModelParams {
            purpose: purpose.into(),
            model: Some(self.ai.model.clone()).filter(|model| !model.is_empty()),
            effort: self.ai.effort.map(|effort| effort.setting().to_owned()),
            messages: vec![ModelMessageJson {
                role: RoleJson::User,
                text: request.prompt.clone(),
            }],
            read: self.read.clone(),
            features: self.features.clone(),
        };
        let answer = self
            .request(ai_methods::HOST_LANGUAGE_MODEL, &params)
            .map_err(no_answer)?;
        match serde_json::from_value::<LanguageModelResult>(answer) {
            Ok(LanguageModelResult::Answered { text, model }) => {
                pieces(&text);
                Ok(Asked { model, text })
            }
            Ok(LanguageModelResult::Refused { refused }) => Err(AiError::Host(refused)),
            Err(error) => Err(AiError::Unreadable(error.to_string())),
        }
    }
}

/// The error of a question the extension did not answer.
fn no_answer(no_answer: NoAnswer) -> AiError {
    match no_answer {
        NoAnswer::Cancelled => AiError::Cancelled,
        NoAnswer::Failed(message) => AiError::Host(message),
    }
}
