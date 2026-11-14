//! `compare.take-file`: a file of the working tree and the index made what the second side of a comparison has.

use model::changes::pathspec;
use model::compare::{ComparisonPart, Side};
use model::contract::{Answers, Context, Danger, Outcome, Plan, Refusal, Step, Target, Text, Undo};
use model::state::ChangeKind;

/// Takes the version of the second side of a comparison into the working tree and the index with
/// `git checkout <commit> -- <file>`, or, for a file the second side does not have, deletes it with
/// `git rm -f --ignore-unmatch -- <file>`. It asks first, and warns when the file has changes not committed, which are
/// lost.
///
/// Feature: compare.take-file
#[must_use]
pub fn take_file(context: &Context, _answers: &Answers) -> Outcome {
    let Target::Comparison(target) = &context.target else {
        return Outcome::Refuse(Refusal::saying("Choose a file of a comparison."));
    };
    let ComparisonPart::File(file) = &target.part else {
        return Outcome::Refuse(Refusal::saying("Choose a file of a comparison."));
    };
    let Side::Commit(commit) = &target.second.side else {
        return Outcome::Refuse(Refusal::saying(
            "The second side is the working tree: its files are there already.",
        ));
    };
    let second = &target.second.name;
    let path = &file.path;
    let named = pathspec(path).to_string_lossy().into_owned();
    let folder = context.repository.folder.clone();
    let (title, note, step) = if file.kind == ChangeKind::Deleted {
        (
            format!("Delete {path}, as in {second}"),
            format!("{second} has no such file, so it is deleted from the working tree and the index."),
            Step::git(folder, ["rm", "-f", "--ignore-unmatch", "--", &named]),
        )
    } else {
        (
            format!("Take {path} from {second}"),
            format!("The file in the working tree and the index becomes the one of {second}."),
            Step::git(folder, ["checkout", commit.as_str(), "--", &named]),
        )
    };
    let working_tree = &context.repository.working_tree;
    let is_changed = working_tree
        .staged
        .iter()
        .chain(&working_tree.unstaged)
        .any(|change| change.path == *path)
        || working_tree
            .conflicted
            .iter()
            .any(|conflicted| conflicted.path == *path);
    let danger = if is_changed {
        Danger::Destructive(Text::from(format!(
            "{note} Its changes that are not committed are lost."
        )))
    } else {
        Danger::Confirm(Text::from(note))
    };
    Outcome::Run(Plan {
        title: Text::from(title),
        steps: vec![step],
        danger,
        undo: Undo::Impossible(Text::from(
            "No reference moves, and the changes that were not committed are kept nowhere.",
        )),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::compare::{CompareSide, ComparisonPart};
    use model::contract::{Answers, Danger, Outcome, Target};
    use model::state::{ChangeKind, FileChange};

    use super::take_file;
    use crate::compare::testing::{branch, compared};
    use crate::testing::{commit, context, lines, modified, plan, state};

    /// The context of the arrow of a file of `main ↔ feature`, changed in a way.
    fn on_file(path: &str, kind: ChangeKind, state: model::state::RepositoryState) -> model::contract::Context {
        let file = FileChange {
            path: path.to_owned(),
            original_path: None,
            kind,
        };
        let target = compared(branch("main", 'a'), branch("feature", 'b'), ComparisonPart::File(file));
        context(state, Target::Comparison(Box::new(target)))
    }

    #[test]
    fn compare_take_file__runs_git_checkout_of_the_second_side_after_asking() {
        let plan = plan(take_file(
            &on_file("src/app.rs", ChangeKind::Modified, state()),
            &Answers::new(),
        ));
        assert_eq!(lines(&plan), [format!("git checkout {} -- src/app.rs", commit('b'))]);
        assert_eq!(plan.title.as_str(), "Take src/app.rs from feature");
        assert!(matches!(plan.danger, Danger::Confirm(_)));
    }

    #[test]
    fn compare_take_file__a_file_the_second_side_deleted_is_removed() {
        let plan = plan(take_file(
            &on_file("old.rs", ChangeKind::Deleted, state()),
            &Answers::new(),
        ));
        assert_eq!(lines(&plan), ["git rm -f --ignore-unmatch -- old.rs"]);
    }

    #[test]
    fn compare_take_file__a_file_with_changes_not_committed_warns_they_are_lost() {
        let mut state = state();
        state.working_tree.unstaged.push(modified("src/app.rs"));
        let plan = plan(take_file(
            &on_file("src/app.rs", ChangeKind::Modified, state),
            &Answers::new(),
        ));
        let Danger::Destructive(warning) = plan.danger else {
            panic!("a warning")
        };
        assert!(
            warning
                .as_str()
                .ends_with("Its changes that are not committed are lost.")
        );
    }

    #[test]
    fn compare_take_file__refused_when_the_second_side_is_the_working_tree() {
        let file = FileChange {
            path: String::from("a.txt"),
            original_path: None,
            kind: ChangeKind::Modified,
        };
        let target = compared(
            branch("main", 'a'),
            CompareSide::working_tree(),
            ComparisonPart::File(file),
        );
        let context = context(state(), Target::Comparison(Box::new(target)));
        assert!(matches!(take_file(&context, &Answers::new()), Outcome::Refuse(_)));
    }
}
