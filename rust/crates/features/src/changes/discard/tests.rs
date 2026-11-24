use super::discard;
use crate::testing::{context, lines, modified, plan, renamed, state};
use model::contract::{Answers, Danger, Outcome, Plan, Target};
use model::state::RepositoryState;

fn discarded(state: RepositoryState, path: &str) -> Plan {
    plan(discard(&context(state, Target::File(path.to_owned())), &Answers::new()))
}

fn warning(plan: &Plan) -> &str {
    match &plan.danger {
        Danger::Destructive(warning) => warning.as_str(),
        other => panic!("discarding is destructive, not {other:?}"),
    }
}

#[test]
fn changes_discard__a_modified_tracked_file_is_restored_after_the_warning() {
    let mut state = state();
    state.working_tree.unstaged.push(modified("a.txt"));
    let plan = discarded(state, "a.txt");
    assert_eq!(lines(&plan), ["git restore -- a.txt"]);
    assert_eq!(warning(&plan), "Unstaged changes to tracked files will be lost.");
}

#[test]
fn changes_discard__an_untracked_file_is_cleaned() {
    let mut state = state();
    state.working_tree.untracked.push(String::from("new.txt"));
    let plan = discarded(state, "new.txt");
    assert_eq!(lines(&plan), ["git clean -f -- new.txt"]);
    assert_eq!(warning(&plan), "One untracked file will be deleted from disk.");
}

#[test]
fn changes_discard__a_file_partly_staged_goes_back_to_head_in_the_index_and_on_disk() {
    let mut state = state();
    state.working_tree.staged.push(modified("a.txt"));
    state.working_tree.unstaged.push(modified("a.txt"));
    let plan = discarded(state, "a.txt");
    assert_eq!(lines(&plan), ["git restore --source=HEAD --staged --worktree -- a.txt"]);
    assert_eq!(
        warning(&plan),
        "One file goes back to the last commit, staged changes included: they will be lost, and a file added since \
         is deleted from disk."
    );
}

#[test]
fn changes_discard__a_staged_rename_names_both_names() {
    let mut state = state();
    state.working_tree.staged.push(renamed("b", "a"));
    let plan = discarded(state, "b");
    assert_eq!(lines(&plan), ["git restore --source=HEAD --staged --worktree -- b a"]);
}

#[test]
fn changes_discard__is_destructive_so_it_asks_even_with_the_preview_off() {
    // With `commandPreview` off the engine still shows a destructive plan first (`engine::needs_confirmation`).
    let mut state = state();
    state.working_tree.unstaged.push(modified("a.txt"));
    let plan = discarded(state, "a.txt");
    assert!(matches!(plan.danger, Danger::Destructive(_)));
}

#[test]
fn refuses_a_file_with_no_change_and_deletes_a_staged_file_before_the_first_commit() {
    assert!(matches!(
        discard(&context(state(), Target::File(String::from("a.txt"))), &Answers::new()),
        Outcome::Refuse(_)
    ));
    let mut state = state();
    state.head.commit = None;
    state.working_tree.staged.push(modified("a.txt"));
    assert_eq!(
        lines(&discarded(state, "a.txt")),
        ["git rm -r --force --quiet -- a.txt"]
    );
}

#[test]
fn changes_discard__the_files_ticked_are_discarded_at_once_with_one_warning_that_counts_them() {
    let mut state = state();
    state.working_tree.unstaged.push(modified("a.txt"));
    state.working_tree.unstaged.push(modified("b.txt"));
    state.working_tree.unstaged.push(modified("c.txt"));
    state.working_tree.untracked.push(String::from("new.txt"));
    let target = Target::Files(vec![
        String::from("a.txt"),
        String::from("b.txt"),
        String::from("new.txt"),
    ]);
    let plan = plan(discard(&context(state, target), &Answers::new()));
    assert_eq!(lines(&plan), ["git restore -- a.txt b.txt", "git clean -f -- new.txt"]);
    assert_eq!(
        warning(&plan),
        "Unstaged changes to tracked files will be lost. One untracked file will be deleted from disk."
    );
    assert_eq!(plan.title.as_str(), "Discard the changes of 3 files");
}

#[test]
fn changes_discard__before_the_first_commit_the_warning_says_the_staged_file_is_deleted_from_disk() {
    let mut state = state();
    state.head.commit = None;
    state.working_tree.staged.push(modified("a.txt"));
    assert_eq!(
        warning(&discarded(state, "a.txt")),
        "One staged file will be deleted from disk."
    );
}
