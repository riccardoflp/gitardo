use super::commit;
use crate::rebase::testing::{line_of, with_line, with_pushed};
use crate::testing::{commit as id, context, lines, modified, plan, state};
use model::contract::{Answers, Danger, FollowUp, Outcome, Step, Target, Text, Undo, UndoRecord};

fn answers(line: model::rebase::LineToHead, pushed: &[&str]) -> Answers {
    with_pushed(with_line(Answers::new(), line), pushed)
}

#[test]
fn split_commit__the_last_commit_is_reset_away_and_its_message_goes_to_the_draft() {
    let context = context(state(), Target::Commit(id('a')));
    let plan = plan(commit(
        &context,
        &answers(line_of("9", &[('a', "Add the parser\n\nWith tests.\n")]), &[]),
    ));
    assert_eq!(lines(&plan), ["git reset --mixed HEAD~"]);
    assert_eq!(plan.undo, Undo::Journal(UndoRecord::SplitLast { commit: id('a') }));
    assert!(plan.after.contains(&FollowUp::FillCommitMessage(String::from(
        "Add the parser\n\nWith tests."
    ))));
    assert!(plan.after.contains(&FollowUp::Say(Text::from(
        "The changes of \"Add the parser\" are in the working tree, unstaged: commit them in as many commits as you \
         like."
    ))));
}

#[test]
fn split_commit__an_older_commit_stops_a_rebase_and_writes_gitardo_split() {
    let mut state = state();
    state.working_tree.unstaged.push(modified("notes.txt"));
    let context = context(state, Target::Commit(id('b')));
    let line = line_of("9", &[('b', "Add the parser\n"), ('a', "Use it\n")]);
    let plan = plan(commit(&context, &answers(line, &["origin/main"])));
    let all = lines(&plan);
    assert!(
        all.get(1)
            .unwrap()
            .ends_with(&format!("rebase -i --autostash {}", id('9')))
    );
    assert_eq!(all.get(2).map(String::as_str), Some("git reset --mixed HEAD~"));
    let Some(Step::Write { path, contents }) = plan.steps.last() else {
        panic!("the split is written last")
    };
    assert!(path.ends_with("gitardo-split"));
    assert_eq!(String::from_utf8_lossy(contents), format!("{}\n", id('b')));
    let Danger::Confirm(note) = &plan.danger else {
        panic!("it asks first")
    };
    assert!(note.as_str().starts_with("bbbbbbb is pushed already, on origin/main"));
    assert!(note.as_str().contains(
        "The rebase stops at bbbbbbb with its changes taken out of it, unstaged: commit them in as many commits as \
         you like, then Continue, and the commit after it is made again on top."
    ));
    assert!(
        note.as_str()
            .ends_with("Changes that are not committed are set aside until the end.")
    );
}

#[test]
fn split_commit__is_refused_while_a_bisect_is_under_way() {
    let mut state = state();
    state.bisect = Some(model::bisect::Bisect::default());
    let context = context(state, Target::Commit(id('a')));
    let Outcome::Refuse(refusal) = commit(&context, &Answers::new()) else {
        panic!("refused")
    };
    assert_eq!(refusal.message.as_str(), "End the bisect first.");
}

#[test]
fn split_commit__refuses_a_merge_and_the_first_commit() {
    let context = context(state(), Target::Commit(id('a')));
    let mut merge = line_of("89", &[]);
    merge.commits = None;
    let Outcome::Refuse(refusal) = commit(&context, &answers(merge, &[])) else {
        panic!("refused")
    };
    assert_eq!(refusal.message.as_str(), "aaaaaaa is a merge: it cannot be split.");
    let Outcome::Refuse(refusal) = commit(&context, &answers(line_of("", &[('a', "First\n")]), &[])) else {
        panic!("refused")
    };
    assert!(refusal.message.as_str().starts_with("aaaaaaa is the first commit"));
}
