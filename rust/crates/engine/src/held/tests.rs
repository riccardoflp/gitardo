use std::path::{Path, PathBuf};

use model::contract::Step;
use model::worktree::TRIED_AGAIN;

use super::{Attempts, recover};

/// What a recovery did: the lines it logged, the folders it deleted, and how many times it ran the step again.
#[derive(Default)]
struct Done {
    logged: Vec<String>,
    deleted: Vec<PathBuf>,
    tries: usize,
}

/// Runs a recovery of a step that failed with `said`, where each try again says what `again` gives in turn, and git
/// lists the worktree or not.
fn recovered(
    (arguments, said): (&[&str], &str),
    again: &[Result<(), &str>],
    (is_windows, listed): (bool, bool),
) -> (Result<(), String>, Done) {
    let arguments: Vec<String> = arguments.iter().map(|word| (*word).to_owned()).collect();
    let mut logged = Vec::new();
    let mut deleted = Vec::new();
    let mut tries = 0;
    let mut answers = again.iter();
    let result = recover(
        &arguments,
        said.to_owned(),
        &mut Attempts {
            is_windows,
            again: &mut || {
                tries += 1;
                answers
                    .next()
                    .copied()
                    .unwrap_or(Err("still held"))
                    .map_err(str::to_owned)
            },
            is_listed: &mut |_| listed,
            delete: &mut |folder: &Path| {
                deleted.push(folder.to_path_buf());
                Ok(())
            },
            wait: &mut |_| {},
            note: &mut |step: &Step| logged.push(step.log_line()),
        },
    );
    (result, Done { logged, deleted, tries })
}

const HELD_MOVE: &str = "fatal: failed to move 'C:/trees/a' to 'C:/trees/b': Permission denied\n";
const MOVE: [&str; 4] = ["worktree", "move", "C:/trees/a", "C:/trees/b"];
const REMOVE: [&str; 3] = ["worktree", "remove", "C:/trees/feature/login"];
const HELD_REMOVE: &str = "error: failed to delete 'C:/trees/feature/login': Directory not empty\n";

#[test]
fn worktree_recover_held_folder__a_move_that_fails_twice_then_succeeds_ends_as_a_success() {
    let (result, done) = recovered((&MOVE, HELD_MOVE), &[Err(HELD_MOVE), Ok(())], (true, true));
    assert_eq!(result, Ok(()));
    assert_eq!(done.tries, 2);
    let tried_again: Vec<&String> = done.logged.iter().filter(|line| line.ends_with(TRIED_AGAIN)).collect();
    assert_eq!(tried_again.len(), 2, "{:?}", done.logged);
    assert!(tried_again[0].starts_with("fatal: failed to move"), "{:?}", done.logged);
    assert_eq!(done.logged.last().unwrap(), "git worktree move C:/trees/a C:/trees/b");
}

#[test]
fn worktree_recover_held_folder__a_removal_git_forgot_deletes_the_folder_and_goes_on() {
    let (result, done) = recovered((&REMOVE, HELD_REMOVE), &[], (true, false));
    assert_eq!(result, Ok(()), "the plan goes on with its next step, as git branch -d");
    assert_eq!(done.deleted, [PathBuf::from("C:/trees/feature/login")]);
    assert!(
        done.logged.contains(&String::from("delete C:/trees/feature/login")),
        "{:?}",
        done.logged
    );
    assert_eq!(done.tries, 0);
}

#[test]
fn worktree_recover_held_folder__a_removal_git_still_lists_stops_with_its_message() {
    let (result, done) = recovered((&REMOVE, HELD_REMOVE), &[], (true, true));
    assert_eq!(result, Err(HELD_REMOVE.to_owned()));
    assert_eq!(done.deleted, Vec::<PathBuf>::new());
    assert_eq!(done.logged, Vec::<String>::new());
}

#[test]
fn worktree_recover_held_folder__on_macos_and_linux_nothing_is_tried_again() {
    let (result, done) = recovered((&MOVE, HELD_MOVE), &[Ok(())], (false, false));
    assert_eq!(result, Err(HELD_MOVE.to_owned()));
    assert_eq!(done.tries, 0);
    let (result, done) = recovered((&REMOVE, HELD_REMOVE), &[], (false, false));
    assert_eq!(result, Err(HELD_REMOVE.to_owned()));
    assert_eq!(done.deleted, Vec::<PathBuf>::new());
}

#[test]
fn a_move_that_never_succeeds_shows_the_last_failure_after_twenty_tries() {
    let (result, done) = recovered((&MOVE, HELD_MOVE), &[], (true, true));
    assert_eq!(result, Err(String::from("still held")));
    assert_eq!(done.tries, 1, "the last failure no longer says the folder is held");
    let last = "fatal: failed to move 'a' to 'b': Device or resource busy";
    let held: Vec<Result<(), &str>> = vec![Err(last); 30];
    let (result, done) = recovered((&MOVE, HELD_MOVE), &held, (true, true));
    assert_eq!(result, Err(last.to_owned()));
    assert_eq!(done.tries, 19);
}

#[test]
fn submodule_deinitialize__a_folder_windows_holds_is_tried_again_until_git_empties_it() {
    let held = "error: could not remove 'lib': Permission denied\n";
    let deinit = ["-C", "vendor", "submodule", "deinit", "--", "lib"];
    let (result, done) = recovered((&deinit, held), &[Err(held), Ok(())], (true, true));
    assert_eq!(result, Ok(()));
    assert_eq!(done.tries, 2);
    assert_eq!(done.logged.last().unwrap(), "git -C vendor submodule deinit -- lib");
    let (result, done) = recovered((&deinit, held), &[Ok(())], (false, true));
    assert_eq!(result, Err(held.to_owned()), "only Windows holds folders so");
    assert_eq!(done.tries, 0);
}

#[test]
fn submodule_remove__git_rm_on_a_folder_windows_holds_is_tried_again() {
    let held = "error: unable to remove 'lib': Device or resource busy\n";
    let (result, done) = recovered((&["rm", "--force", "--", "lib"], held), &[Ok(())], (true, true));
    assert_eq!(result, Ok(()));
    assert_eq!(done.tries, 1);
    let other = "fatal: pathspec 'lib' did not match any files\n";
    let (result, _) = recovered((&["rm", "--force", "--", "lib"], other), &[Ok(())], (true, true));
    assert_eq!(result, Err(other.to_owned()), "a failure of its own is the operation's");
}
