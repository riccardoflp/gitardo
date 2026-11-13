//! `conflicts.mark-resolved`: tells git that a file's conflicts are resolved, by staging it.

use model::changes::pathspec;
use model::contract::{Answers, Context, Danger, Outcome, Plan, Step, Text, Undo};

use super::{is_in_conflict, refuse, target_file};

/// Stages a file in conflict as it is in the working tree: `git add -- <file>`. It does not look into the file: what
/// is there is what is staged, markers included.
///
/// Feature: conflicts.mark-resolved
#[must_use]
pub fn mark_resolved(context: &Context, _answers: &Answers) -> Outcome {
    let Some(path) = target_file(context) else {
        return refuse("Choose a file in conflict first.");
    };
    if !is_in_conflict(context, path) {
        return refuse(&format!("{path} is not in conflict."));
    }
    Outcome::Run(Plan {
        title: Text::from(format!("Stage {path}")),
        steps: vec![staging(context.repository.folder.clone(), path)],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Staging leaves nothing in the reflog.")),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

/// The step that tells git a file's conflicts are resolved: `git add -- <file>`, the file named as itself. The Merge
/// page stages through it too (`merge-view.mark-resolved`).
pub(crate) fn staging(folder: std::path::PathBuf, path: &str) -> Step {
    Step::git(folder, [std::ffi::OsString::from("add"), "--".into(), pathspec(path)])
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::mark_resolved;
    use crate::testing::{context, lines, plan, state};
    use model::contract::{Answers, Outcome, Target};
    use model::state::{ConflictKind, ConflictedFile};

    fn with_conflict(path: &str) -> model::state::RepositoryState {
        let mut state = state();
        state.working_tree.conflicted.push(ConflictedFile {
            path: path.to_owned(),
            kind: ConflictKind::BothModified,
        });
        state
    }

    #[test]
    fn conflicts_mark_resolved__stages_the_file_with_git_add() {
        let context = context(with_conflict("src/app.rs"), Target::File(String::from("src/app.rs")));
        let plan = plan(mark_resolved(&context, &Answers::new()));
        assert_eq!(lines(&plan), ["git add -- src/app.rs"]);
        assert_eq!(plan.title.as_str(), "Stage src/app.rs");
    }

    #[test]
    fn conflicts_mark_resolved__a_name_that_is_a_pattern_is_passed_literally() {
        let context = context(
            with_conflict("pages/[id].tsx"),
            Target::File(String::from("pages/[id].tsx")),
        );
        assert_eq!(
            lines(&plan(mark_resolved(&context, &Answers::new()))),
            ["git add -- :(literal)pages/[id].tsx"]
        );
    }

    #[test]
    fn a_file_not_in_conflict_is_refused() {
        let context = context(state(), Target::File(String::from("a.txt")));
        assert!(matches!(mark_resolved(&context, &Answers::new()), Outcome::Refuse(_)));
    }
}
