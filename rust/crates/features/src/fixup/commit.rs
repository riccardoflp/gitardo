//! `fixup.commit`: folds the staged changes, or all the changes when nothing is staged, into an earlier commit.

use model::contract::{AnswerKey, Answers, Context, Danger, Outcome, Plan, Query, Step, Target, Text, Undo};

use crate::rebase::{refuse, refuse_under_way, stack};

/// The parent of the commit, or none for the first commit.
pub const BASE: AnswerKey = AnswerKey("base");

/// Adds the staged changes, or all of them when nothing is staged, to the commit it was invoked on: a `fixup!` commit
/// of them, then `git rebase -i --autosquash --autostash`, whose own todo list git takes as it prepares it, which moves
/// the `fixup!` commit under its target. What is not staged is set aside meanwhile. The other branches on the commits
/// made again move with it by `--update-refs`, as `rebase.update-refs` decides.
///
/// Feature: fixup.commit
#[must_use]
pub fn commit(context: &Context, answers: &Answers) -> Outcome {
    let Target::Commit(commit) = &context.target else {
        return refuse("Choose a commit first.");
    };
    if let Some(refusal) = refuse_under_way(context) {
        return refusal;
    }
    let working_tree = &context.repository.working_tree;
    if working_tree.staged.is_empty() && working_tree.unstaged.is_empty() && working_tree.untracked.is_empty() {
        return refuse("There are no changes to add to that commit.");
    }
    let Some(base) = answers.commit(BASE) else {
        return Outcome::Read(
            BASE,
            Query::Revision {
                revision: format!("{commit}^"),
            },
        );
    };
    let parent = format!("{commit}^");
    let tip = stack::checked_out_tip(context);
    let decided = match stack::decide(context, answers, (base.map(|_| parent.as_str()), &tip)) {
        Ok(decided) => decided,
        Err(read) => return *read,
    };
    let folder = context.repository.folder.clone();
    let mut steps = Vec::new();
    let what = if working_tree.staged.is_empty() {
        steps.push(Step::git(folder.clone(), ["add", "--all"]));
        "all changes"
    } else {
        "staged changes"
    };
    steps.push(Step::git(folder.clone(), ["commit", &format!("--fixup={commit}")]));
    let upstream = base.map_or_else(|| String::from("--root"), |_| parent.clone());
    let mut arguments = vec![
        "-c",
        "sequence.editor=true",
        "rebase",
        "-i",
        "--autosquash",
        "--autostash",
    ];
    arguments.extend(decided.option);
    arguments.push(upstream.as_str());
    steps.push(Step::git(folder, arguments));
    let short = commit.short();
    let plan = Plan {
        title: Text::from(format!("Add {what} to {short}")),
        steps,
        danger: Danger::Confirm(Text::from(format!(
            "This rewrites {short} and every commit after it. If the changes do not fit there, the rebase stops with \
             conflicts."
        ))),
        undo: Undo::ByReflog,
        after: Vec::new(),
        on_failure: Vec::new(),
    };
    Outcome::Run(stack::with_todo_stack(plan, answers, &decided))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{BASE, commit};
    use crate::testing::{commit as id, context, lines, modified, plan, state};
    use model::contract::{Answer, Answers, Outcome, Target};

    fn based(parent: Option<char>) -> Answers {
        crate::rebase::stack::no_stack(Answers::new().with(BASE, Answer::Commit(parent.map(id))))
    }

    #[test]
    fn fixup_commit__commits_the_staged_changes_as_a_fixup_then_rebases() {
        let mut state = state();
        state.working_tree.staged.push(modified("a.txt"));
        state.working_tree.unstaged.push(modified("b.txt"));
        let plan = plan(commit(&context(state, Target::Commit(id('b'))), &based(Some('c'))));
        let hash = id('b');
        assert_eq!(
            lines(&plan),
            [
                format!("git commit --fixup={hash}"),
                format!("git -c sequence.editor=true rebase -i --autosquash --autostash {hash}^"),
            ]
        );
        assert_eq!(plan.title.as_str(), "Add staged changes to bbbbbbb");
    }

    #[test]
    fn fixup_commit__with_nothing_staged_adds_all_first() {
        let mut state = state();
        state.working_tree.unstaged.push(modified("b.txt"));
        let plan = plan(commit(&context(state, Target::Commit(id('b'))), &based(Some('c'))));
        assert_eq!(lines(&plan).first().map(String::as_str), Some("git add --all"));
        assert_eq!(plan.title.as_str(), "Add all changes to bbbbbbb");
    }

    #[test]
    fn fixup_commit__on_the_first_commit_rebases_from_the_root() {
        let mut state = state();
        state.working_tree.unstaged.push(modified("b.txt"));
        let plan = plan(commit(&context(state, Target::Commit(id('b'))), &based(None)));
        assert!(lines(&plan).last().unwrap().ends_with("--autostash --root"));
    }

    #[test]
    fn fixup_commit__is_refused_while_a_bisect_is_under_way() {
        let mut state = state();
        state.working_tree.staged.push(modified("a.txt"));
        state.bisect = Some(model::bisect::Bisect::default());
        let Outcome::Refuse(refusal) = commit(&context(state, Target::Commit(id('b'))), &based(Some('c'))) else {
            panic!("refused")
        };
        assert_eq!(refusal.message.as_str(), "End the bisect first.");
    }

    #[test]
    fn fixup_commit__refuses_with_no_change() {
        let Outcome::Refuse(refusal) = commit(&context(state(), Target::Commit(id('b'))), &Answers::new()) else {
            panic!("refused")
        };
        assert_eq!(refusal.message.as_str(), "There are no changes to add to that commit.");
    }
}
