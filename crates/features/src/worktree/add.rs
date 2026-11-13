//! The plan that makes a worktree, shared by the operations that make one: refused when its folder exists by then,
//! `git worktree add`, what the setup of new worktrees adds, then what is said and offered.

use std::path::{Path, PathBuf};

use model::contract::{
    AnswerKey, Answers, Context, Danger, FeatureId, FollowUp, OfferedRun, Outcome, Plan, Query, Refusal, Step, Target,
    Text, Undo, WorktreeOpenIn,
};

use super::REMOVE_TO_UNDO;
use super::setup::{NewWorktree, Setup, setup_steps};

/// Whether the folder of the new worktree exists by the time it runs.
const EXISTS: AnswerKey = AnswerKey("folder exists");

/// A worktree to make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Addition {
    /// The title of the plan.
    pub(super) title: String,
    /// What `git worktree add` is given after `add`, the folder included.
    pub(super) arguments: Vec<String>,
    /// Its folder.
    pub(super) folder: PathBuf,
    /// What it checks out, as git is given it.
    pub(super) start: String,
}

/// The plan that makes the worktree, or what comes first: the read of whether its folder exists, the refusal when it
/// does, or what the setup reads.
pub(super) fn add(context: &Context, answers: &Answers, addition: Addition) -> Result<(Plan, Setup), Box<Outcome>> {
    let Some(exists) = answers.flag(EXISTS) else {
        return Err(Box::new(Outcome::Read(
            EXISTS,
            Query::PathExists {
                path: addition.folder.clone(),
            },
        )));
    };
    if exists {
        return Err(Box::new(Outcome::Refuse(Refusal::saying(&format!(
            "`{}` already exists.",
            addition.folder.display()
        )))));
    }
    let worktree = NewWorktree {
        folder: &addition.folder,
        commit: &addition.start,
    };
    let setup = setup_steps(context, answers, &worktree)?;
    let mut arguments = vec![String::from("worktree"), String::from("add")];
    arguments.extend(addition.arguments);
    let mut steps = vec![Step::git(context.repository.folder.clone(), arguments)];
    steps.extend(setup.steps.iter().cloned());
    let plan = Plan {
        title: Text::from(addition.title),
        steps,
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(REMOVE_TO_UNDO)),
        after: Vec::new(),
        on_failure: Vec::new(),
    };
    Ok((plan, setup))
}

/// What is said and offered once a worktree was made: **Worktree created in `<folder>`**, then **Open in new window**,
/// **Open in external editor** while `editor.external` is set, and **Open in terminal**, the one `worktrees.openIn`
/// names first; and, while nothing is set for new worktrees, **Choose What It Gets…** the first time. They follow the
/// commands of the setup, since they come only once the plan succeeded; when a command fails, opening is offered all
/// the same.
pub(super) fn created(context: &Context, folder: &Path, setup: &Setup) -> (Vec<FollowUp>, Vec<FollowUp>) {
    let mut after = vec![FollowUp::Say(Text::from(format!(
        "Worktree created in `{}`",
        folder.display()
    )))];
    after.extend(setup.said.iter().map(|said| FollowUp::Say(Text::from(said.as_str()))));
    let opens = opens(context, folder);
    after.extend(opens.iter().cloned());
    if !setup.is_set && !context.settings.worktrees.setup_offered {
        after.push(offer(
            "Choose What It Gets\u{2026}",
            "worktree-setup.page",
            Target::Head,
        ));
    }
    let on_failure = if setup.has_commands { opens } else { Vec::new() };
    (after, on_failure)
}

/// The offers that open a new worktree, the one `worktrees.openIn` names first.
fn opens(context: &Context, folder: &Path) -> Vec<FollowUp> {
    let target = || Target::Worktree(folder.to_path_buf());
    let window = offer("Open in new window", "worktree.open-in-new-window", target());
    let editor = (!context.settings.external_editor.trim().is_empty())
        .then(|| offer("Open in external editor", "worktree.open-in-editor", target()));
    let terminal = offer("Open in terminal", "worktree.open-in-terminal", target());
    let mut offers = match context.settings.worktrees.open_in {
        WorktreeOpenIn::Window => vec![Some(window), editor, Some(terminal)],
        WorktreeOpenIn::Editor => vec![editor, Some(window), Some(terminal)],
        WorktreeOpenIn::Terminal => vec![Some(terminal), Some(window), editor],
    };
    offers.retain(Option::is_some);
    offers.into_iter().flatten().collect()
}

/// An operation offered with its button.
fn offer(label: &str, feature: &'static str, target: Target) -> FollowUp {
    FollowUp::OfferRun(Box::new(OfferedRun {
        label: Text::from(label),
        feature: FeatureId(feature),
        target,
        answers: Answers::new(),
        when_said: Vec::new(),
    }))
}

/// The plan, with what it says and offers once it ran.
pub(super) fn with_offers(context: &Context, (mut plan, setup): (Plan, Setup), folder: &Path) -> Plan {
    let (after, on_failure) = created(context, folder, &setup);
    plan.after = after;
    plan.on_failure = on_failure;
    plan
}

/// The answers of a worktree whose folder is not there by the time it runs, in a repository that sets nothing for new
/// worktrees: neither of its settings files is there.
#[cfg(test)]
pub(super) fn not_there() -> Answers {
    use model::contract::Answer;
    Answers::new().with(EXISTS, Answer::Flag(false)).with(
        crate::worktree_setup::files::SETTINGS_FILES,
        Answer::Files(vec![None, None]),
    )
}

/// The answers of a worktree whose folder exists by the time it runs.
#[cfg(test)]
pub(super) fn there() -> Answers {
    Answers::new().with(EXISTS, model::contract::Answer::Flag(true))
}
