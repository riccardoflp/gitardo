//! The options of the next commit (`commit.options`): each repository keeps its own while the window is open, from
//! where they start (as git's configuration signs, and as `commit.signOff` says) to the commit that uses them, after
//! which they are back where they start.

use std::path::Path;

use model::commit::CommitOptions;
use model::contract::{Answer, Answers};
use views::changes::{CommitOptionsState, OptionsIntent};

use super::{Effect, State};

/// Where the options of a commit start: signed as git's configuration says, **Add Signed-off-by** as `commit.signOff`
/// says, no co-author, the hooks run.
fn starts(state: &State) -> CommitOptions {
    CommitOptions {
        sign_off: state.settings.commit_sign_off,
        ..CommitOptions::default()
    }
}

/// The options in force for the next commit of a repository.
pub(super) fn in_force(state: &State, folder: &Path) -> CommitOptions {
    state
        .changes
        .options
        .get(folder)
        .cloned()
        .unwrap_or_else(|| starts(state))
}

/// Something asked of the options of the next commit of the repository shown.
pub fn reduce(state: &mut State, intent: OptionsIntent) -> Vec<Effect> {
    let Some(folder) = state.shown_folder().cloned() else {
        return Vec::new();
    };
    let mut options = in_force(state, &folder);
    match intent {
        OptionsIntent::Sign(is_signed) => options.sign = Some(is_signed),
        OptionsIntent::SignOff(is_signed_off) => options.sign_off = is_signed_off,
        OptionsIntent::SkipHooks(skips) => options.skip_hooks = skips,
        OptionsIntent::RemoveCoAuthor(index) => {
            if index < options.co_authors.len() {
                let _removed = options.co_authors.remove(index);
            }
        }
        OptionsIntent::ChooseCoAuthors => return super::co_authors::open(state),
    }
    let _before = state.changes.options.insert(folder, options);
    Vec::new()
}

/// The answers a commit of the repository shown is run with, which say its options; none while every option is where
/// it starts.
#[must_use]
pub fn answers(state: &State) -> Answers {
    let Some(folder) = state.shown_folder() else {
        return Answers::new();
    };
    let options = in_force(state, folder);
    let mut answers = Answers::new();
    if let Some(is_signed) = options.sign {
        answers.insert(features::commit::SIGN, Answer::Flag(is_signed));
    }
    if options.sign_off {
        answers.insert(features::commit::SIGN_OFF, Answer::Flag(true));
    }
    if options.skip_hooks {
        answers.insert(features::commit::SKIP_HOOKS, Answer::Flag(true));
    }
    if !options.co_authors.is_empty() {
        let people = options
            .co_authors
            .iter()
            .map(|person| format!("{} <{}>", person.name, person.email));
        answers.insert(features::commit::CO_AUTHORS, Answer::Choices(people.collect()));
    }
    answers
}

/// A commit of a repository was made: its options are back where they start.
pub fn made(state: &mut State, folder: &Path) {
    let _before = state.changes.options.remove(folder);
}

/// What the button of the options and the line under the commit box of a repository are drawn from.
#[must_use]
pub fn options_state(state: &State, folder: &Path, read: &model::state::RepositoryState) -> CommitOptionsState {
    let identity = &read.identity;
    let sign_off_line = format!(
        "Signed-off-by: {} <{}>",
        identity.name.as_deref().unwrap_or_default(),
        identity.email.as_deref().unwrap_or_default()
    );
    CommitOptionsState {
        options: in_force(state, folder),
        starts: starts(state),
        signing: read.signing.clone(),
        sign_off_line,
        is_running: state.is_operation_running,
    }
}
