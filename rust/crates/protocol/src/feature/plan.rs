//! The JSON of a plan and of how a feature ended: [`PlanJson`], [`StepJson`], [`FollowUpJson`] and [`FinishedJson`],
//! written by the engine only.

use std::path::Path;

use engine::{Finished, StepFailure};
use model::contract::{Danger, Plan, Step, Text, Undo};

use super::follow_up::FollowUpJson;
use serde::{Deserialize, Serialize};

/// A plan: what a feature runs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanJson {
    /// What it does, as the log and the confirmation say it.
    pub title: String,
    /// Its steps, in order.
    pub steps: Vec<StepJson>,
    /// Whether it is confirmed first, and why.
    pub danger: DangerJson,
    /// How Undo takes it back.
    pub undo: UndoJson,
    /// What the extension does once it ran, of the follow-ups that are the extension's.
    pub after: Vec<FollowUpJson>,
}

/// A step of a plan, with `line`, the words the log writes for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum StepJson {
    /// git, with its arguments.
    Git {
        /// The arguments.
        arguments: Vec<String>,
        /// The folder it runs in.
        directory: Option<String>,
        /// What is written on its standard input, when it is text.
        input: Option<String>,
        /// The line of the log.
        line: String,
    },
    /// A file written.
    Write {
        /// The file.
        path: String,
        /// The line of the log.
        line: String,
    },
    /// A link to a folder.
    Link {
        /// The folder linked to.
        from: String,
        /// The link.
        to: String,
        /// The line of the log.
        line: String,
    },
    /// A file or a folder copied.
    Copy {
        /// What is copied.
        from: String,
        /// Where.
        to: String,
        /// The line of the log.
        line: String,
    },
    /// A link removed.
    Unlink {
        /// The link.
        path: String,
        /// The line of the log.
        line: String,
    },
    /// A file or a folder deleted.
    Delete {
        /// What is deleted.
        path: String,
        /// The line of the log.
        line: String,
    },
    /// A command of the settings run.
    Run {
        /// The command.
        command: String,
        /// The folder it runs in.
        directory: String,
        /// The line of the log.
        line: String,
    },
    /// A command of the settings run as a task.
    Task {
        /// The command.
        command: String,
        /// The folder it runs in.
        directory: String,
        /// The line of the log.
        line: String,
    },
    /// A folder made.
    MakeFolder {
        /// The folder.
        path: String,
        /// The line of the log.
        line: String,
    },
    /// A folder deleted when it is empty.
    DeleteEmptyFolder {
        /// The folder.
        path: String,
        /// The line of the log.
        line: String,
    },
    /// A warning said with the steps.
    Warn {
        /// The warning.
        text: String,
        /// The line of the log.
        line: String,
    },
}

/// Whether a plan is confirmed first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DangerJson {
    /// It runs as it is asked.
    None,
    /// It is confirmed first.
    Confirm {
        /// Why.
        reason: String,
    },
    /// It loses work, and is confirmed first.
    Destructive {
        /// What is lost.
        reason: String,
    },
}

/// How Undo takes a plan back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum UndoJson {
    /// By the reflog.
    ByReflog,
    /// It cannot.
    Impossible {
        /// Why.
        reason: String,
    },
    /// By what the journal of the engine keeps.
    Journal,
    /// It is itself the Undo of an operation.
    TakesBack {
        /// The number of that operation in the journal.
        number: u64,
    },
}

fn text_of(text: &Text) -> String {
    text.as_str().to_owned()
}

fn path_of(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

impl From<&Step> for StepJson {
    fn from(step: &Step) -> StepJson {
        let line = step.log_line();
        match step {
            Step::Git {
                arguments,
                directory,
                input,
            } => git_step(arguments, directory, input.as_deref(), line),
            Step::Write { path, .. } => StepJson::Write {
                path: path_of(path),
                line,
            },
            Step::Link { from, to } => StepJson::Link {
                from: path_of(from),
                to: path_of(to),
                line,
            },
            Step::Copy { from, to } => StepJson::Copy {
                from: path_of(from),
                to: path_of(to),
                line,
            },
            Step::Unlink { path } => StepJson::Unlink {
                path: path_of(path),
                line,
            },
            Step::Delete { path } => StepJson::Delete {
                path: path_of(path),
                line,
            },
            Step::Run { command, directory } => StepJson::Run {
                command: command.clone(),
                directory: path_of(directory),
                line,
            },
            Step::Task { command, directory, .. } => StepJson::Task {
                command: command.clone(),
                directory: path_of(directory),
                line,
            },
            Step::MakeFolder { path } => StepJson::MakeFolder {
                path: path_of(path),
                line,
            },
            Step::DeleteEmptyFolder { path } => StepJson::DeleteEmptyFolder {
                path: path_of(path),
                line,
            },
            Step::Warn { words } => StepJson::Warn {
                text: words.clone(),
                line,
            },
        }
    }
}

/// The JSON of a step of git.
fn git_step(arguments: &[std::ffi::OsString], directory: &Path, input: Option<&[u8]>, line: String) -> StepJson {
    StepJson::Git {
        arguments: arguments
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect(),
        directory: Some(path_of(directory)),
        input: input.and_then(|bytes| String::from_utf8(bytes.to_vec()).ok()),
        line,
    }
}

impl From<&Danger> for DangerJson {
    fn from(danger: &Danger) -> DangerJson {
        match danger {
            Danger::None => DangerJson::None,
            Danger::Confirm(reason) => DangerJson::Confirm {
                reason: text_of(reason),
            },
            Danger::Destructive(reason) => DangerJson::Destructive {
                reason: text_of(reason),
            },
        }
    }
}

impl From<&Undo> for UndoJson {
    fn from(undo: &Undo) -> UndoJson {
        match undo {
            Undo::ByReflog => UndoJson::ByReflog,
            Undo::Impossible(reason) => UndoJson::Impossible {
                reason: text_of(reason),
            },
            Undo::Journal(_) => UndoJson::Journal,
            Undo::TakesBack(number) => UndoJson::TakesBack { number: *number },
        }
    }
}

impl From<&Plan> for PlanJson {
    fn from(plan: &Plan) -> PlanJson {
        PlanJson {
            title: text_of(&plan.title),
            steps: plan.steps.iter().map(StepJson::from).collect(),
            danger: (&plan.danger).into(),
            undo: (&plan.undo).into(),
            after: FollowUpJson::all_of(&plan.after),
        }
    }
}

/// A link or a copy that failed while the steps after it went on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WentOnAfterJson {
    /// The line of the step that failed.
    pub failed: String,
    /// What the system said.
    pub said: String,
}

impl From<&StepFailure> for WentOnAfterJson {
    fn from(failure: &StepFailure) -> WentOnAfterJson {
        WentOnAfterJson {
            failed: failure.failed.clone(),
            said: failure.said.clone(),
        }
    }
}

/// A feature offered in place of one that refused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferJson {
    /// The feature.
    pub feature: String,
    /// The words of its button.
    pub label: String,
}

/// How a feature ended: the answer of `feature/run`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum FinishedJson {
    /// Its plan ran.
    Ran {
        /// The plan.
        plan: PlanJson,
        /// The lines of the steps that ran.
        steps: Vec<String>,
        /// The links and copies that failed while the others went on.
        went_on_after: Vec<WentOnAfterJson>,
        /// The steps handed over to run apart, in VS Code's terminal.
        waiting: Vec<StepJson>,
    },
    /// It refused, and said why.
    Refused {
        /// Why.
        message: String,
        /// What is offered in its place.
        offer: Option<OfferJson>,
    },
    /// The user backed out of a question.
    BackedOut,
    /// The plan was shown and not run.
    NotConfirmed {
        /// The plan.
        plan: PlanJson,
    },
}

impl From<&Finished> for FinishedJson {
    fn from(finished: &Finished) -> FinishedJson {
        match finished {
            Finished::Ran {
                plan,
                steps,
                went_on_after,
                waiting,
                ..
            } => FinishedJson::Ran {
                plan: plan.into(),
                steps: steps.clone(),
                went_on_after: went_on_after.iter().map(WentOnAfterJson::from).collect(),
                waiting: waiting.iter().map(StepJson::from).collect(),
            },
            Finished::Refused(refusal) => FinishedJson::Refused {
                message: text_of(&refusal.message),
                offer: refusal.offer.as_ref().map(|(feature, label)| OfferJson {
                    feature: feature.0.to_owned(),
                    label: text_of(label),
                }),
            },
            Finished::BackedOut => FinishedJson::BackedOut,
            Finished::NotConfirmed(plan) => FinishedJson::NotConfirmed { plan: plan.into() },
        }
    }
}
