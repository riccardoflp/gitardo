//! `rebase.onto`: makes the commits of the branch checked out again on top of a commit, a branch or a remote branch.

use model::branch::{RefKind, RefNames, ref_argument};
use model::contract::{Answers, Context, Danger, Outcome, Plan, Step, Target, Text, Undo};

use super::{current_name, refuse};

/// Rebases what is checked out onto the commit, branch, remote branch or tag it was invoked on: `git rebase <onto>`,
/// a reference named in full when another kind has its short name. When it stops on a conflict, the repository is left
/// as git leaves it.
///
/// Feature: rebase.onto
#[must_use]
pub fn onto(context: &Context, _answers: &Answers) -> Outcome {
    if !context.repository.has_commit() {
        return refuse("There is no commit checked out to rebase.");
    }
    let names = RefNames::of(&context.repository);
    let (argument, said) = match &context.target {
        Target::Commit(commit) => (commit.to_string(), commit.short().to_owned()),
        Target::Branch(name) => (ref_argument(RefKind::Branch, name, &names), name.clone()),
        Target::RemoteBranch(name) => (ref_argument(RefKind::RemoteBranch, name, &names), name.clone()),
        Target::Tag(name) => (ref_argument(RefKind::Tag, name, &names), name.clone()),
        Target::Head
        | Target::Commits(_)
        | Target::Stash(_)
        | Target::Remote(_)
        | Target::Files(_)
        | Target::File(_)
        | Target::Changes
        | Target::DiffChange(_)
        | Target::Comparison(_)
        | Target::MergeResult(_)
        | Target::Resolutions(_)
        | Target::FileSave(_)
        | Target::FileAt(_) => return refuse("Choose a commit or a branch to rebase onto."),
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Rebase {} onto {said}", current_name(context))),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["rebase", argument.as_str()],
        )],
        danger: Danger::None,
        undo: Undo::ByReflog,
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::onto;
    use crate::testing::{branch, commit, context, lines, plan, state, tag};
    use model::Timestamp;
    use model::contract::{Answers, Outcome, Target, Undo};
    use model::state::RemoteBranch;

    #[test]
    fn rebase_onto__a_remote_branch_by_its_short_name() {
        let mut state = state();
        state.remote_branches.push(RemoteBranch {
            name: String::from("origin/main"),
            remote: String::from("origin"),
            commit: commit('b'),
            date: Timestamp(0),
        });
        let plan = plan(onto(
            &context(state, Target::RemoteBranch(String::from("origin/main"))),
            &Answers::new(),
        ));
        assert_eq!(lines(&plan), ["git rebase origin/main"]);
        assert_eq!(plan.title.as_str(), "Rebase main onto origin/main");
        assert_eq!(plan.undo, Undo::ByReflog);
    }

    #[test]
    fn rebase_onto__a_commit_by_its_full_hash() {
        let plan = plan(onto(&context(state(), Target::Commit(commit('b'))), &Answers::new()));
        assert_eq!(lines(&plan), [format!("git rebase {}", commit('b'))]);
        assert_eq!(plan.title.as_str(), "Rebase main onto bbbbbbb");
    }

    #[test]
    fn rebase_onto__a_branch_named_in_full_when_a_tag_has_its_name() {
        let mut state = state();
        state.branches.push(branch("release", 'b', false));
        state.tags.push(tag("release", 'c'));
        let plan = plan(onto(
            &context(state, Target::Branch(String::from("release"))),
            &Answers::new(),
        ));
        assert_eq!(lines(&plan), ["git rebase refs/heads/release"]);
        assert!(matches!(
            onto(
                &context(super::super::testing::empty(), Target::Commit(commit('b'))),
                &Answers::new()
            ),
            Outcome::Refuse(_)
        ));
    }
}
