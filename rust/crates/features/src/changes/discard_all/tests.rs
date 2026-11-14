use super::discard_all;
use crate::testing::{context, lines, modified, plan, state};
use model::contract::{Answers, Danger, Outcome, Plan, Target};
use model::state::{ConflictKind, ConflictedFile, Operation, RepositoryState};

fn discarded(state: RepositoryState, target: Target) -> Plan {
    plan(discard_all(&context(state, target), &Answers::new()))
}

fn conflict(path: &str) -> ConflictedFile {
    ConflictedFile {
        path: path.to_owned(),
        kind: ConflictKind::BothModified,
    }
}

fn warning(plan: &Plan) -> &str {
    match &plan.danger {
        Danger::Destructive(warning) => warning.as_str(),
        other => panic!("discarding is destructive, not {other:?}"),
    }
}

#[test]
fn changes_discard_all__from_the_header_with_nothing_staged_restores_and_cleans_each_file() {
    let mut state = state();
    state.working_tree.unstaged.push(modified("a.txt"));
    state.working_tree.untracked.push(String::from("b.txt"));
    let plan = discarded(state, Target::Changes);
    assert_eq!(lines(&plan), ["git restore -- a.txt", "git clean -f -- b.txt"]);
    assert_eq!(
        warning(&plan),
        "Unstaged changes to tracked files will be lost. One untracked file will be deleted from disk."
    );
}

#[test]
fn changes_discard_all__from_the_header_with_a_file_staged_restores_every_file_of_the_list_at_once() {
    let mut state = state();
    state.working_tree.staged.push(modified("a.txt"));
    state.working_tree.unstaged.push(modified("b.txt"));
    state.working_tree.unstaged.push(modified("c.txt"));
    let plan = discarded(state, Target::Changes);
    assert_eq!(
        lines(&plan),
        ["git restore --source=HEAD --staged --worktree -- a.txt b.txt c.txt"]
    );
    assert_eq!(
        warning(&plan),
        "3 files go back to the last commit, staged changes included: they will be lost, and a file added since is \
         deleted from disk."
    );
}

#[test]
fn changes_discard_all__from_the_header_a_file_in_conflict_is_not_named() {
    let mut state = state();
    state.working_tree.conflicted.push(conflict("c.txt"));
    state.working_tree.staged.push(modified("a.txt"));
    state.working_tree.untracked.push(String::from("b.txt"));
    let lines = lines(&discarded(state, Target::Changes));
    assert!(lines.iter().all(|line| !line.contains("c.txt")), "{lines:?}");
}

#[test]
fn changes_discard_all__from_the_row_restores_and_cleans_the_whole_working_tree() {
    let mut state = state();
    state.working_tree.unstaged.push(modified("a.txt"));
    state.working_tree.staged.push(modified("b.txt"));
    state.working_tree.untracked.push(String::from("tmp/x.txt"));
    let plan = discarded(state, Target::Head);
    assert_eq!(
        lines(&plan),
        [
            "git restore --source=HEAD --staged --worktree -- .",
            "git clean -f -d -- ."
        ]
    );
    assert_eq!(
        warning(&plan),
        "2 files go back to the last commit, staged changes included: they will be lost, and a file added since is \
         deleted from disk. One untracked file will be deleted from disk."
    );
}

#[test]
fn changes_discard_all__from_the_row_is_refused_while_a_merge_is_stopped() {
    let mut state = state();
    state.operation = Some(Operation::Merge);
    state.working_tree.conflicted.push(conflict("a.txt"));
    let Outcome::Refuse(refusal) = discard_all(&context(state, Target::Head), &Answers::new()) else {
        panic!("refused during a merge")
    };
    assert_eq!(refusal.message.as_str(), "Abort the merge to discard all of it.");
}

#[test]
fn changes_discard_all__is_destructive_so_it_asks_even_with_the_preview_off() {
    let mut state = state();
    state.working_tree.unstaged.push(modified("a.txt"));
    assert!(matches!(
        discarded(state, Target::Changes).danger,
        Danger::Destructive(_)
    ));
}

#[test]
fn refuses_with_nothing_to_discard_and_removes_the_index_before_the_first_commit() {
    assert!(matches!(
        discard_all(&context(state(), Target::Head), &Answers::new()),
        Outcome::Refuse(_)
    ));
    let mut state = state();
    state.head.commit = None;
    state.working_tree.staged.push(modified("a.txt"));
    assert_eq!(
        lines(&discarded(state.clone(), Target::Head)),
        ["git rm -r --force --quiet -- ."]
    );
    assert_eq!(
        lines(&discarded(state, Target::Changes)),
        ["git rm -r --force --quiet -- a.txt"]
    );
}
