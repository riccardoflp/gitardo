//! The contract every feature follows (`docs/04-feature-contract.md`): what it is given, what it asks, what it reads,
//! and the plan it returns for the engine to run.

mod answers;
mod context;
mod outcome;
mod plan;
mod question;
mod words;

pub use answers::{Answer, AnswerKey, Answers, ReflogEntry};
pub use context::{CommandPreview, Context, FeatureSettings, Target};
pub use outcome::{FeatureId, Operation, Outcome, Query, Refusal};
pub use plan::{Danger, FollowUp, Plan, Step, Undo};
pub use question::{Choice, Question, QuestionKind, Validation};
pub use words::Text;
