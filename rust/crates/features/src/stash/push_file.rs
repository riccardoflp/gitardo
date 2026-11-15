//! `stash.push-file`: sets aside every change to one file, or to the files ticked, staged or not.

use model::changes::pathspec;
use model::contract::{Answers, Context, Outcome, Refusal, Step, Target};

use super::push::MESSAGE;
use super::{message_question, push_plan, refuse_before_first_commit};

/// Asks an optional message, then stashes the files it was invoked on with `git stash push --include-untracked --
/// <file>…`: a renamed file with both its names, a name git would read as a pattern as `:(literal)<name>`.
///
/// Feature: stash.push-file
#[must_use]
pub fn push_file(context: &Context, answers: &Answers) -> Outcome {
    if let Some(refusal) = refuse_before_first_commit(context) {
        return refusal;
    }
    let Target::Files(files) = &context.target else {
        return Outcome::Refuse(Refusal::saying("Pick the files to stash first."));
    };
    // The old name of a staged rename whose new name is there too is the same file, which takes it along below.
    let staged = &context.repository.working_tree.staged;
    let files: Vec<&String> = files
        .iter()
        .filter(|file| {
            !staged
                .iter()
                .any(|change| change.original_path.as_ref() == Some(*file) && files.contains(&change.path))
        })
        .collect();
    let (Some(first), false) = (files.first(), files.is_empty()) else {
        return Outcome::Refuse(Refusal::saying("Pick the files to stash first."));
    };
    let Some(message) = answers.text(MESSAGE) else {
        let title = if files.len() == 1 {
            format!("Stash `{first}`")
        } else {
            format!("Stash {} files", files.len())
        };
        return Outcome::Ask(message_question(
            title,
            "Optional message. Every change to the file is stashed, staged or not.",
        ));
    };
    let mut paths = vec![String::from("--")];
    let mut unstaged_first = Vec::new();
    for file in &files {
        let renamed_from = staged
            .iter()
            .find(|change| change.path == **file)
            .and_then(|change| change.original_path.as_ref());
        if let Some(original) = renamed_from {
            // Git takes no path that is not in the index: the old name of a staged rename is put back in the index
            // first, deleted in the working tree, so that the stash takes both (open question Q123).
            let original = pathspec(original);
            let step = ["reset", "--quiet", "--"].map(std::ffi::OsString::from);
            unstaged_first.push(Step::git(
                context.repository.folder.clone(),
                step.into_iter().chain([original.clone()]),
            ));
            paths.push(original.to_string_lossy().into_owned());
        }
        paths.push(pathspec(file).to_string_lossy().into_owned());
    }
    let title = match files.len() {
        1 => format!("Stash {first}"),
        count => format!("Stash {count} files"),
    };
    let mut plan = push_plan(context, &title, (&["--include-untracked"], message), &paths);
    unstaged_first.append(&mut plan.steps);
    plan.steps = unstaged_first;
    Outcome::Run(plan)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::push_file;
    use crate::stash::MESSAGE;
    use crate::testing::{context, lines, plan, question, state};
    use model::contract::{Answer, Answers, Target};
    use model::state::{ChangeKind, FileChange};

    fn stashed(files: &[&str], state: model::state::RepositoryState) -> Vec<String> {
        let target = Target::Files(files.iter().map(|file| (*file).to_owned()).collect());
        let answers = Answers::new().with(MESSAGE, Answer::Text(String::new()));
        lines(&plan(push_file(&context(state, target), &answers)))
    }

    #[test]
    fn stash_push_file__stashes_that_file_only() {
        assert_eq!(
            stashed(&["src/app.rs"], state()),
            ["git stash push --include-untracked -- src/app.rs"]
        );
        let target = Target::Files(vec![String::from("src/app.rs")]);
        let asked = question(push_file(&context(state(), target), &Answers::new()));
        assert_eq!(asked.title.as_str(), "Stash `src/app.rs`");
    }

    #[test]
    fn stash_push_file__an_untracked_file_is_taken_too() {
        assert_eq!(
            stashed(&["notes.txt"], state()),
            ["git stash push --include-untracked -- notes.txt"]
        );
    }

    #[test]
    fn stash_push_file__two_files_ticked_are_stashed() {
        assert_eq!(
            stashed(&["a.txt", "b.txt"], state()),
            ["git stash push --include-untracked -- a.txt b.txt"]
        );
    }

    #[test]
    fn stash_push_file__a_name_that_is_a_pattern_is_given_literally() {
        assert_eq!(
            stashed(&["pages/[id].tsx"], state()),
            ["git stash push --include-untracked -- :(literal)pages/[id].tsx"]
        );
    }

    #[test]
    fn stash_push_file__a_staged_rename_is_given_with_both_names() {
        let mut state = state();
        state.working_tree.staged.push(FileChange {
            path: String::from("new.txt"),
            original_path: Some(String::from("old.txt")),
            kind: ChangeKind::Renamed,
        });
        assert_eq!(
            stashed(&["new.txt"], state.clone()),
            [
                "git reset --quiet -- old.txt",
                "git stash push --include-untracked -- old.txt new.txt"
            ]
        );
        // A host that lists the rename as one row may send both its names: the old one is not given twice.
        assert_eq!(
            stashed(&["new.txt", "old.txt"], state),
            [
                "git reset --quiet -- old.txt",
                "git stash push --include-untracked -- old.txt new.txt"
            ]
        );
    }
}
