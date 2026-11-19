//! `worktree-setup.bring-into-worktree` and `worktree-setup.bring-into-other-worktrees`: what new worktrees get, given
//! to worktrees that are there already, without touching what they have.

use std::path::{Path, PathBuf};

use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, Outcome, Plan, Query, Refusal, Step, Target, Text, Undo,
};
use model::state::Worktree;
use model::worktree_setup::{LinkAndCopyInput, SetupPreview, SetupSettings, TakeRequest, link_and_copy_steps};

use super::files::{patterns_of, settings_given};

/// The key of what the patterns take for each worktree.
const TAKES: AnswerKey = AnswerKey("worktree-setup-bring-takes");

/// What is said when no pattern is in effect.
const NOTHING_YET: &str = "New worktrees get nothing yet: choose what they get first.";

/// How the plans are undone.
const NOT_UNDONE: &str = "Not taken back by Undo: what was made is in the log, and can be deleted.";

/// Gives a worktree that is there already the links and the copies new worktrees get, from the folder the views
/// show, with what it has from its commit; no commands run.
///
/// Feature: worktree-setup.bring-into-worktree
#[must_use]
pub fn bring_into_worktree(context: &Context, answers: &Answers) -> Outcome {
    let Target::Worktree(folder) = &context.target else {
        return Outcome::Refuse(Refusal::saying("Choose the worktree to bring ignored files into."));
    };
    if same_folder(folder, &context.repository.folder) {
        return Outcome::Refuse(Refusal::saying("This is the checkout it comes from."));
    }
    let worktree = context
        .repository
        .worktrees
        .iter()
        .find(|worktree| same_folder(&worktree.folder, folder));
    let Some(worktree) = worktree.filter(|worktree| !worktree.is_missing) else {
        return Outcome::Refuse(Refusal::saying(&format!("{} is not there.", folder.display())));
    };
    let name = folder.file_name().map_or_else(
        || folder.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    bring(
        context,
        answers,
        &[worktree],
        &format!("Bring ignored files into {name}"),
    )
}

/// Gives every other worktree of the repository that is not missing the links and the copies new worktrees get, one
/// worktree after the other; a failure in one does not stop the next.
///
/// Feature: worktree-setup.bring-into-other-worktrees
#[must_use]
pub fn bring_into_other_worktrees(context: &Context, answers: &Answers) -> Outcome {
    let others: Vec<&Worktree> = context
        .repository
        .worktrees
        .iter()
        .filter(|worktree| !worktree.is_current && !worktree.is_missing)
        .filter(|worktree| !same_folder(&worktree.folder, &context.repository.folder))
        .collect();
    if others.is_empty() {
        return Outcome::Refuse(Refusal::saying("There are no other worktrees to bring it into."));
    }
    let title = format!("Bring ignored files into {} worktrees", others.len());
    bring(context, answers, &others, &title)
}

/// Whether two folders are the same, as git and the system may spell them.
fn same_folder(one: &Path, other: &Path) -> bool {
    let plain = |folder: &Path| {
        let text = folder.to_string_lossy().replace('\\', "/");
        let text = text.trim_end_matches('/');
        if cfg!(windows) {
            text.to_lowercase()
        } else {
            text.to_owned()
        }
    };
    plain(one) == plain(other)
}

/// The plan that links and copies into each worktree in turn.
fn bring(context: &Context, answers: &Answers, worktrees: &[&Worktree], title: &str) -> Outcome {
    let settings = match settings_given(context, answers) {
        Ok(settings) => settings,
        Err(outcome) => return *outcome,
    };
    let patterns = patterns_of(&settings);
    if patterns.link.is_empty() && patterns.copy.is_empty() {
        return Outcome::Refuse(Refusal::saying(NOTHING_YET));
    }
    let Some(Answer::SetupTakes(takes)) = answers.get(TAKES) else {
        let requests = worktrees
            .iter()
            .map(|worktree| TakeRequest {
                folder: context.repository.folder.clone(),
                patterns: patterns.clone(),
                commit: worktree.commit.as_ref().map(|commit| commit.as_str().to_owned()),
            })
            .collect();
        return Outcome::Read(TAKES, Query::SetupTakes(requests));
    };
    let (steps, note) = steps_for(&settings, &context.repository.folder, worktrees, takes);
    Outcome::Run(Plan {
        title: Text::from(title),
        steps,
        danger: Danger::Confirm(Text::from(note)),
        undo: Undo::Impossible(Text::from(NOT_UNDONE)),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

/// The steps for each worktree, and the note the plan is confirmed with: the patterns and where from.
fn steps_for(
    settings: &SetupSettings,
    source: &Path,
    worktrees: &[&Worktree],
    takes: &[SetupPreview],
) -> (Vec<Step>, String) {
    let empty = SetupPreview::default();
    let mut steps = Vec::new();
    let mut note = String::new();
    for (index, worktree) in worktrees.iter().enumerate() {
        let folder: &PathBuf = &worktree.folder;
        let made = link_and_copy_steps(&LinkAndCopyInput {
            settings,
            source,
            worktree: folder,
            take: takes.get(index).unwrap_or(&empty),
        });
        if note.is_empty()
            && let Some(said) = made.note
        {
            note = said.strip_prefix("Then ").map_or(said.clone(), ToOwned::to_owned);
        }
        steps.extend(made.steps);
    }
    (steps, note)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use model::contract::{Answer, Answers, Outcome, Step, Target};
    use model::state::Worktree;
    use model::worktree_setup::SetupPreview;

    use super::{TAKES, bring_into_other_worktrees, bring_into_worktree};
    use crate::testing::{context, lines, plan, state};
    use crate::worktree_setup::files::SETTINGS_FILES;

    fn worktree(folder: &str, is_missing: bool) -> Worktree {
        Worktree {
            folder: PathBuf::from(folder),
            commit: None,
            branch: None,
            is_main: false,
            is_current: false,
            is_missing,
            locked: None,
            uncommitted: None,
        }
    }

    fn with_worktrees(target: Target, worktrees: Vec<Worktree>) -> model::contract::Context {
        let mut repository = state();
        repository.worktrees = worktrees;
        context(repository, target)
    }

    fn settings(text: &str) -> Answers {
        Answers::new().with(
            SETTINGS_FILES,
            Answer::Files(vec![Some(text.as_bytes().to_vec()), None]),
        )
    }

    #[test]
    fn worktree_setup_bring_into_worktree__with_no_pattern_it_is_refused() {
        let context = with_worktrees(
            Target::Worktree(PathBuf::from("feature")),
            vec![worktree("feature", false)],
        );
        let Outcome::Refuse(refusal) = bring_into_worktree(&context, &settings("{}")) else {
            panic!("expected a refusal");
        };
        assert_eq!(
            refusal.message.as_str(),
            "New worktrees get nothing yet: choose what they get first."
        );
    }

    #[test]
    fn worktree_setup_bring_into_other_worktrees__the_missing_worktree_gets_no_steps() {
        let worktrees = vec![worktree("one", false), worktree("gone", true), worktree("two", false)];
        let context = with_worktrees(Target::Head, worktrees);
        let take = SetupPreview {
            linked: vec![String::from("node_modules")],
            ..SetupPreview::default()
        };
        let answers = settings(r#"{"worktrees.linkFolders": ["node_modules/"]}"#)
            .with(TAKES, Answer::SetupTakes(vec![take.clone(), take]));
        let plan = plan(bring_into_other_worktrees(&context, &answers));
        assert_eq!(plan.title.as_str(), "Bring ignored files into 2 worktrees");
        assert_eq!(lines(&plan), ["symlink node_modules", "symlink node_modules"]);
        let into: Vec<_> = plan
            .steps
            .iter()
            .filter_map(|step| {
                if let Step::Link { from, .. } = step {
                    from.parent().map(PathBuf::from)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(into, [PathBuf::from("one"), PathBuf::from("two")]);
    }
}
