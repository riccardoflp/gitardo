//! The messages of the features, from stage 2: `feature/run` from the extension, `feature/ask` and `feature/confirm`
//! from the engine while one runs, `question/validate` while a name is typed, `feature/followUps` before the answer of a
//! feature that ran, and `settings/changed` (`docs/transition/02-protocol.md`, Features and Settings).

mod follow_up;
mod plan;
mod question;
mod resolutions;
mod target;

use std::collections::BTreeMap;

pub use follow_up::FollowUpJson;
pub use plan::{DangerJson, FinishedJson, OfferJson, PlanJson, StepJson, UndoJson, WentOnAfterJson};
pub use question::{
    AnswerJson, ChoiceJson, FileFilterJson, FileQuestionJson, QuestionJson, QuestionKindJson, ValidationJson,
};
pub use resolutions::{
    ConfidenceJson, FileResolutionsJson, MergeResultJson, ProposalJson, ProposalSourceJson, ResolutionChoiceJson,
    RuleCaseJson,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
pub use target::{NotACommit, TargetJson};

use crate::rpc::RequestId;

/// `feature/run`: run a feature of the catalog in a repository opened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureRunParams {
    /// The repository.
    pub repository: String,
    /// The identifier of the feature: `branch.create`.
    pub feature: String,
    /// What it was invoked on.
    pub target: TargetJson,
    /// Answers given before it asks, by the key of their question: asked again of no one.
    #[serde(default)]
    pub answers: BTreeMap<String, AnswerJson>,
    /// Whether what ran is kept for Undo; false only in the tests that check the plan alone.
    #[serde(default = "yes")]
    pub journal: bool,
    /// The values given last in this workspace for the inputs of the format of branch names (`worktreeInputs`), by the
    /// identifier of each input.
    #[serde(default)]
    pub inputs: BTreeMap<String, String>,
}

fn yes() -> bool {
    true
}

/// `feature/ask`: the engine asks a question of a feature that runs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskParams {
    /// The id of the `feature/run` it belongs to.
    pub run: RequestId,
    /// The question.
    pub question: QuestionJson,
}

/// The answer of `feature/ask`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskResult {
    /// The answer; none when the user backed out.
    pub answer: Option<AnswerJson>,
}

/// `feature/confirm`: the engine asks whether to run a plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmParams {
    /// The id of the `feature/run` it belongs to.
    pub run: RequestId,
    /// The plan.
    pub plan: PlanJson,
}

/// The answer of `feature/confirm`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmResult {
    /// Whether to run it.
    pub run: bool,
}

/// `question/validate`: whether the text typed so far is an answer the question takes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateParams {
    /// The repository.
    pub repository: String,
    /// What the text must be, as the question gave it.
    pub validation: ValidationJson,
    /// The text so far.
    pub text: String,
}

/// The answer of `question/validate`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateResult {
    /// The sentence under the box; none when the text is fine.
    pub problem: Option<String>,
}

/// `feature/followUps`: what the extension carries out once a feature ran, sent right before its answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowUpsParams {
    /// The id of the `feature/run`.
    pub run: RequestId,
    /// The follow-ups, in their order.
    pub follow_ups: Vec<FollowUpJson>,
}

/// `settings/changed`: every setting, whenever one changes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsParams {
    /// The settings, by the desktop's names.
    pub settings: Map<String, Value>,
}
