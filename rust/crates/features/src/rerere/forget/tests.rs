use model::contract::{Answers, Danger, Outcome, Target};
use model::rerere::ResolvedAsBefore;
use model::state::RepositoryState;

use super::{forget, warning};
use crate::testing::{context, lines, plan, state};

/// A repository in which git resolved `m.txt` as before, staged or not.
fn resolved(is_staged: bool) -> RepositoryState {
    RepositoryState {
        resolved_as_before: vec![ResolvedAsBefore {
            path: String::from("m.txt"),
            is_staged,
        }],
        ..state()
    }
}

#[test]
fn rerere_forget__on_m_resolved_as_before_it_asks_with_the_warning_then_runs_forget_and_checkout_merge() {
    let ran = plan(forget(
        &context(resolved(false), Target::File(String::from("m.txt"))),
        &Answers::new(),
    ));
    assert_eq!(
        lines(&ran),
        ["git rerere forget m.txt", "git checkout --merge -- m.txt"]
    );
    assert_eq!(ran.danger, Danger::Destructive(warning("m.txt").into()));
}

#[test]
fn rerere_forget__on_m_that_git_staged_it_runs_the_same_two_commands() {
    let ran = plan(forget(
        &context(resolved(true), Target::File(String::from("m.txt"))),
        &Answers::new(),
    ));
    assert_eq!(lines(&ran).len(), 2);
    let other = context(resolved(true), Target::File(String::from("n.txt")));
    assert!(matches!(forget(&other, &Answers::new()), Outcome::Refuse(_)));
}
