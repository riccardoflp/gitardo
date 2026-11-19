//! What runs the plans of Gitardo Desktop: preview, steps, the log, the undo journal, activity and follow-ups.
//!
//! The engine is the only crate that changes the disk or the repository. It calls a feature with what is known so
//! far, asks its questions through an [`Asker`], answers its queries by reading git, shows its plan first when
//! [`needs_confirmation`] says so, and runs the plan's steps one by one, writing each to the [`CommandLog`] before it
//! starts.

#![forbid(unsafe_code)]

mod activity;
pub mod ai;
mod clock;
mod drive;
mod error;
pub mod github;
mod held;
mod history;
mod host;
pub mod hosting;
pub mod log;
mod preview;
mod queries;
mod readings;
mod secrets;
mod setup;
mod steps;
mod tasks;
pub mod timeline;
pub mod undo;

pub use activity::{Activities, ActivityEntry, ActivityGuard, ActivityReport, EndedActivity};
pub use clock::{Clock, FixedClock};
pub use drive::{Asker, Engine, FindOperation, Finished};
pub use error::EngineError;
pub use host::Host;
pub use log::{CommandLog, LogEntry, LogKind, LogLevel};
pub use preview::needs_confirmation;
pub use secrets::{GITHUB_TOKEN, MemorySecrets, SecretStore, SystemSecrets};
pub use steps::StepFailure;
pub use tasks::{TaskEvent, TaskRunner, TaskSteps, TaskTalk, TasksFailure};
