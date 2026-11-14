//! `diff.discard-file`: takes the file of a Diff page back to its staged version, or deletes it when it is untracked,
//! from its title button **Discard changes**.

use model::contract::{Answers, Context, Danger, Outcome, Plan, Refusal, Target, Text, Undo};

use super::change::with_file;
use super::stage_file::CHOOSE_A_FILE;

/// The warning of changes to a tracked file that are not staged.
const UNSTAGED_WARNING: &str = "Unstaged changes to tracked files will be lost.";
/// The warning of an untracked file.
const UNTRACKED_WARNING: &str = "One untracked file will be deleted from disk.";
/// Said when the file has no change in the working tree.
const NOTHING_TO_DISCARD: &str = "There is nothing to discard.";

/// Discards the changes of the file that are not staged, after a warning: `git restore -- <file>` for a tracked file,
/// whose staged changes stay, `git clean -f -- <file>` for an untracked one.
///
/// Feature: diff.discard-file
#[must_use]
pub fn discard_file(context: &Context, _answers: &Answers) -> Outcome {
    let Target::File(path) = &context.target else {
        return Outcome::Refuse(Refusal::saying(CHOOSE_A_FILE));
    };
    let tree = &context.repository.working_tree;
    let folder = context.repository.folder.clone();
    let (step, warning) = if tree.untracked.iter().any(|untracked| untracked == path) {
        (with_file(folder, &["clean", "-f"], path), UNTRACKED_WARNING)
    } else if tree.unstaged.iter().any(|change| change.path == *path) {
        (with_file(folder, &["restore"], path), UNSTAGED_WARNING)
    } else {
        return Outcome::Refuse(Refusal::saying(NOTHING_TO_DISCARD));
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Discard the changes of {path}")),
        steps: vec![step],
        danger: Danger::Destructive(Text::from(warning)),
        undo: Undo::Impossible(Text::from("What was not staged is not kept anywhere.")),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::discard_file;
    use crate::testing::{context, lines, modified, plan, state};
    use model::contract::{Answers, Danger, Outcome, Target};

    fn target(path: &str) -> Target {
        Target::File(path.to_owned())
    }

    #[test]
    fn diff_discard_file__a_tracked_file_partly_staged_is_restored_after_the_warning_and_its_staged_changes_stay() {
        let mut state = state();
        state.working_tree.staged.push(modified("src/app.rs"));
        state.working_tree.unstaged.push(modified("src/app.rs"));
        let discarded = plan(discard_file(&context(state, target("src/app.rs")), &Answers::new()));
        // `git restore` with no `--staged` and no `--source` takes the file back to the index, which keeps what is staged.
        assert_eq!(lines(&discarded), ["git restore -- src/app.rs"]);
        assert_eq!(
            discarded.danger,
            Danger::Destructive("Unstaged changes to tracked files will be lost.".into())
        );
    }

    #[test]
    fn diff_discard_file__an_untracked_file_is_cleaned() {
        let mut state = state();
        state.working_tree.untracked.push(String::from("notes.txt"));
        let discarded = plan(discard_file(&context(state, target("notes.txt")), &Answers::new()));
        assert_eq!(lines(&discarded), ["git clean -f -- notes.txt"]);
        assert_eq!(
            discarded.danger,
            Danger::Destructive("One untracked file will be deleted from disk.".into())
        );
    }

    #[test]
    fn refuses_a_file_with_nothing_to_discard() {
        let Outcome::Refuse(refusal) = discard_file(&context(state(), target("src/app.rs")), &Answers::new()) else {
            panic!("there is nothing to discard");
        };
        assert_eq!(refusal.message.as_str(), "There is nothing to discard.");
    }
}
