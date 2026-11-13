//! The contract every feature follows (`docs/04-feature-contract.md`): what it is given, what it asks, what it reads,
//! and the plan it returns for the engine to run.

mod answers;
mod conflict_results;
mod context;
mod journal;
mod outcome;
mod plan;
mod question;
mod readings;
mod results;
mod words;

pub use answers::{Answer, AnswerKey, Answers, IdentityKeys, ReflogEntry};
pub use context::{CommandPreview, Context, FeatureSettings, IdentityProfile, OpenIn, Target};
pub use journal::{JournalEntry, UndoRecord};
pub use outcome::{FeatureId, Operation, Outcome, Query, Refusal};
pub use plan::{Danger, FollowUp, Plan, Step, Undo};
pub use question::{Choice, FileFilter, FileQuestion, Question, QuestionKind, Validation};
pub use results::EmptyStop;
pub use words::Text;
