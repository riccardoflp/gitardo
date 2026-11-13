use super::{FILES, LEFT, MESSAGE, continue_split};
use crate::testing::{commit, context, lines, modified, plan, question, renamed, state};
use model::contract::{Answer, Answers, Outcome, Target};
use model::rebase::{RebaseStop, StopKind};
use model::state::{Operation, RepositoryState};

fn splitting() -> RepositoryState {
    let mut state = state();
    state.operation = Some(Operation::Rebase);
    state.rebase_stop = Some(RebaseStop {
        kind: StopKind::Split,
        commit: commit('b'),
        subject: String::from("Add the parser"),
    });
    state
}

fn files(paths: &[&str]) -> Answers {
    Answers::new().with(
        FILES,
        Answer::Names(paths.iter().map(|path| (*path).to_owned()).collect()),
    )
}

#[test]
fn split_continue__with_everything_committed_continues_the_rebase() {
    let plan = plan(continue_split(
        &context(splitting(), Target::Head),
        &files(&["a.txt", "b.txt"]),
    ));
    assert_eq!(lines(&plan), ["git -c core.editor=true rebase --continue"]);
}

#[test]
fn split_continue__a_file_of_it_left_is_committed_with_its_message_then_continued() {
    let mut state = splitting();
    state.working_tree.unstaged.push(modified("b.txt"));
    let context = context(state, Target::Head);
    let asked = question(continue_split(&context, &files(&["a.txt", "b.txt"])));
    assert_eq!(
        asked.title.as_str(),
        "\"Add the parser\" is not all committed yet: 1 file of it is still changed."
    );
    let answers = files(&["a.txt", "b.txt"])
        .with(LEFT, Answer::Choice(String::from("commit")))
        .with(MESSAGE, Answer::Messages(vec![String::from("Add the parser\n")]));
    assert_eq!(
        lines(&plan(continue_split(&context, &answers))),
        [
            "git add --all -- b.txt",
            "git commit --quiet -m \"Add the parser\"",
            "git -c core.editor=true rebase --continue",
        ]
    );
}

#[test]
fn split_continue__another_file_modified_is_refused_by_its_name() {
    let mut state = splitting();
    state.working_tree.unstaged.push(modified("other.txt"));
    state.working_tree.staged.push(renamed("new.txt", "a.txt"));
    let Outcome::Refuse(refusal) = continue_split(&context(state, Target::Head), &files(&["a.txt"])) else {
        panic!("refused")
    };
    assert_eq!(
        refusal.message.as_str(),
        "other.txt has changes that are not committed and are not part of the split: commit or stash them, then \
         continue."
    );
}

#[test]
fn split_continue__an_untracked_file_does_not_keep_it_from_continuing() {
    let mut state = splitting();
    state.working_tree.untracked.push(String::from("notes.txt"));
    let plan = plan(continue_split(&context(state, Target::Head), &files(&["a.txt"])));
    assert_eq!(lines(&plan), ["git -c core.editor=true rebase --continue"]);
}
