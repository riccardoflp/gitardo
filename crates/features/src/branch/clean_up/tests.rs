use std::path::PathBuf;

use super::{BRANCHES, MERGED, clean_up};
use crate::testing::{branch, context, lines, plan, question, state};

fn context_of(state: RepositoryState) -> Context {
    context(state, Target::Head)
}
use model::contract::{Answer, Answers, Context, Danger, Outcome, Query, QuestionKind, Target};
use model::state::{Branch, RepositoryState, Upstream, Worktree};

/// A branch whose upstream was deleted from the remote.
fn gone(name: &str) -> Branch {
    let mut gone = branch(name, 'c', false);
    gone.upstream = Some(Upstream {
        reference: format!("refs/remotes/origin/{name}"),
        remote: String::from("origin"),
        remote_branch: format!("refs/heads/{name}"),
        ahead: 0,
        behind: 0,
        is_gone: true,
    });
    gone
}

fn worktree(folder: &str, branch: &str, uncommitted: usize) -> Worktree {
    Worktree {
        folder: PathBuf::from(folder),
        commit: None,
        branch: Some(format!("refs/heads/{branch}")),
        is_main: false,
        is_current: false,
        is_missing: false,
        uncommitted: Some(uncommitted),
    }
}

/// `main` checked out, `a` merged and `b` whose upstream is gone.
fn repository() -> RepositoryState {
    let mut state = state();
    state.branches.push(branch("a", 'b', false));
    state.branches.push(gone("b"));
    state
}

fn merged(names: &[&str]) -> Answers {
    Answers::new().with(
        MERGED,
        Answer::Names(names.iter().map(|name| (*name).to_owned()).collect()),
    )
}

fn ticked(answers: Answers, names: &[&str]) -> Answers {
    answers.with(
        BRANCHES,
        Answer::Choices(names.iter().map(|name| (*name).to_owned()).collect()),
    )
}

/// The choices of the pick, each with its description, and those ticked to start with.
fn listed(context: &Context, answers: &Answers) -> (Vec<(String, String)>, Vec<String>) {
    let asked = question(clean_up(context, answers));
    let QuestionKind::PickMany { choices } = asked.kind else {
        panic!("the branches are ticked")
    };
    let Some(Answer::Choices(ticked)) = asked.default else {
        panic!("some are ticked to start with")
    };
    let choices = choices
        .into_iter()
        .map(|choice| {
            let description = choice.description.map_or_default(|text| text.to_string());
            (choice.identifier, description)
        })
        .collect();
    (choices, ticked)
}

#[test]
fn branch_clean_up__lists_merged_and_gone_branches_ticking_only_the_merged() {
    let context = context(repository(), Target::Head);
    assert_eq!(
        clean_up(&context, &Answers::new()),
        Outcome::Read(
            MERGED,
            Query::MergedBranches {
                into: String::from("HEAD")
            }
        )
    );
    let (choices, ticked) = listed(&context, &merged(&["main", "a"]));
    assert_eq!(
        choices,
        [
            (String::from("a"), String::from("merged into main")),
            (String::from("b"), String::from("upstream is gone, not merged"))
        ]
    );
    assert_eq!(ticked, ["a"]);
}

#[test]
fn branch_clean_up__deletes_merged_with_d_and_the_others_with_capital_d_after_a_warning() {
    let context = context(repository(), Target::Head);
    let plan = plan(clean_up(&context, &ticked(merged(&["main", "a"]), &["a", "b"])));
    assert_eq!(lines(&plan), ["git branch -d a", "git branch -D b"]);
    assert_eq!(plan.title.as_str(), "Delete 2 branches");
    assert!(matches!(&plan.danger, Danger::Destructive(reason) if reason.as_str().contains(": b.")));
}

#[test]
fn branch_clean_up__removes_the_worktree_of_a_merged_branch_before_deleting_it() {
    let mut state = repository();
    state.worktrees.push(worktree("app-a", "a", 0));
    let context = context(state, Target::Head);
    let answers = merged(&["main", "a"]);
    let (choices, ticked_first) = listed(&context, &answers);
    assert_eq!(
        choices.first().map(|(_, why)| why.as_str()),
        Some("merged into main, with its worktree app-a")
    );
    assert_eq!(ticked_first, ["a"]);
    let plan = plan(clean_up(&context, &ticked(answers, &["a"])));
    assert_eq!(lines(&plan), ["git worktree remove app-a", "git branch -d a"]);
    assert_eq!(plan.title.as_str(), "Delete 1 branch and 1 worktree");
    assert!(matches!(plan.danger, Danger::Confirm(_)));
}

#[test]
fn branch_clean_up__a_worktree_with_files_not_committed_is_unticked_and_forced_with_a_warning() {
    let mut state = repository();
    state.worktrees.push(worktree("app-a", "a", 2));
    let context = context(state, Target::Head);
    let answers = merged(&["main", "a"]);
    let (choices, ticked_first) = listed(&context, &answers);
    assert_eq!(
        choices.first().map(|(_, why)| why.as_str()),
        Some("merged into main, with its worktree app-a, 2 files not committed")
    );
    assert_eq!(ticked_first, Vec::<String>::new());
    let plan = plan(clean_up(&context, &ticked(answers, &["a"])));
    assert_eq!(lines(&plan), ["git worktree remove --force app-a", "git branch -d a"]);
    assert!(matches!(&plan.danger, Danger::Destructive(reason) if reason.as_str().contains("app-a")));
}

#[test]
fn branch_clean_up__never_lists_the_branch_checked_out_nor_the_one_of_the_main_worktree() {
    let mut state = repository();
    state.branches.push(branch("trunk", 'a', false));
    let mut main = worktree("app", "trunk", 0);
    main.is_main = true;
    let mut current = worktree("app-x", "main", 0);
    current.is_current = true;
    state.worktrees = vec![main, current];
    let context = context(state, Target::Head);
    let (choices, _) = listed(&context, &merged(&["main", "trunk", "a"]));
    let names: Vec<&str> = choices.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["a", "b"]);
}

#[test]
fn branch_clean_up__refuses_when_there_is_nothing_to_clean_up() {
    let context = context(state(), Target::Head);
    assert!(matches!(
        clean_up(&context, &merged(&["main"])),
        Outcome::Refuse(refusal)
            if refusal.message.as_str() == "No local branches are merged or have lost their upstream."
    ));
    let context = context_of(repository());
    assert!(matches!(
        clean_up(&context, &ticked(merged(&["a"]), &[])),
        Outcome::Refuse(refusal) if refusal.message.as_str() == "No branches selected."
    ));
}
