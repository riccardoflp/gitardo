//! The [`Asker`] of a feature run for the extension: each question is `feature/ask` and each plan to confirm is
//! `feature/confirm`, sent on the same connection and waited for (`docs/transition/02-protocol.md`, How the Asker is
//! translated). The answers given in `feature/run` answer their questions without asking.

use std::collections::BTreeMap;

use engine::Asker;
use git::CancelToken;
use model::contract::{Answer, Plan, Question};
use protocol::feature::{AnswerJson, AskParams, AskResult, ConfirmParams, ConfirmResult, PlanJson, QuestionJson};
use protocol::messages::methods;
use protocol::rpc::RequestId;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::session::Session;

/// Asks the extension the questions of one `feature/run`.
pub struct HostAsker<'a> {
    /// Who sends the requests and gets their answers.
    session: &'a Session,
    /// The id of the `feature/run`.
    run: RequestId,
    /// Answers given before, by the key of their question; each is taken once.
    given: BTreeMap<String, AnswerJson>,
    /// The token of the `feature/run`: cancelling it backs out of the question waited for.
    cancel: CancelToken,
}

impl<'a> HostAsker<'a> {
    /// An asker for the `feature/run` of this id.
    pub fn new(session: &'a Session, run: RequestId, given: BTreeMap<String, AnswerJson>, cancel: CancelToken) -> Self {
        HostAsker {
            session,
            run,
            given,
            cancel,
        }
    }

    /// Sends a request to the extension and reads its answer; none when it was not answered.
    fn request<T: DeserializeOwned>(&self, method: &str, params: &impl Serialize) -> Option<T> {
        let params = serde_json::to_value(params).ok()?;
        let answer: Value = self
            .session
            .asked()
            .ask(|message| self.session.send(message), method, params, &self.cancel)
            .ok()?;
        serde_json::from_value(answer).ok()
    }
}

impl Asker for HostAsker<'_> {
    fn ask(&mut self, question: &Question) -> Option<Answer> {
        if let Some(given) = self.given.remove(question.key.0) {
            return Some(given.into());
        }
        let params = AskParams {
            run: self.run.clone(),
            question: QuestionJson::from(question),
        };
        let AskResult { answer } = self.request(methods::FEATURE_ASK, &params)?;
        answer.map(Answer::from)
    }

    /// The text of the clipboard of VS Code, which the extension reads itself before it runs a feature that may need
    /// it, and gives as the text answered under `clipboard`; none when it gave none.
    fn clipboard(&mut self) -> Option<String> {
        match self.given.remove(features::patch::CLIPBOARD.0) {
            Some(AnswerJson::Text { text }) => Some(text),
            _ => None, // Any other answer is no text of the clipboard.
        }
    }

    fn confirm(&mut self, plan: &Plan) -> bool {
        let params = ConfirmParams {
            run: self.run.clone(),
            plan: PlanJson::from(plan),
        };
        self.request::<ConfirmResult>(methods::FEATURE_CONFIRM, &params)
            .is_some_and(|answer| answer.run)
    }
}
