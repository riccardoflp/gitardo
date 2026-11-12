use super::{BRANCH_REFLOG, FILES_MATCH, HEAD_REFLOG, JOURNAL, last};
use crate::testing::{commit, context, lines, modified, plan, state};
use model::contract::{
    Answer, Answers, Danger, JournalEntry, Outcome, Query, ReflogEntry, Target, Text, Undo, UndoRecord,
};

fn entry(digit: char, subject: &str) -> ReflogEntry {
    ReflogEntry {
        commit: commit(digit),
        subject: subject.to_owned(),
    }
}

/// No entry in the journal: what Undo takes back is in the reflogs.
fn no_journal() -> Answers {
    Answers::new().with(JOURNAL, Answer::Journal(None))
}

fn reflogs(head: Vec<ReflogEntry>, branch: Vec<ReflogEntry>) -> Answers {
    no_journal()
        .with(HEAD_REFLOG, Answer::Reflog(head))
        .with(BRANCH_REFLOG, Answer::Reflog(branch))
}

#[test]
fn undo_last__reads_the_reflog_of_head_then_of_the_branch() {
    let context = context(state(), Target::Head);
    let Outcome::Read(key, Query::Reflog { reference, count }) = last(&context, &no_journal()) else {
        panic!("the reflog of HEAD is read first")
    };
    assert_eq!((key, reference.as_str(), count), (HEAD_REFLOG, "HEAD", 2));
    let head = no_journal().with(
        HEAD_REFLOG,
        Answer::Reflog(vec![entry('b', "commit: Fix"), entry('a', "commit (initial): Add")]),
    );
    let Outcome::Read(key, Query::Reflog { reference, .. }) = last(&context, &head) else {
        panic!("the reflog of the branch is read next")
    };
    assert_eq!((key, reference.as_str()), (BRANCH_REFLOG, "refs/heads/main"));
}

#[test]
fn undo_last__resets_a_commit_softly_so_its_changes_stay_staged() {
    let entries = vec![
        entry('b', "commit: Fix the parser"),
        entry('a', "commit (initial): Add"),
    ];
    let plan = plan(last(
        &context(state(), Target::Head),
        &reflogs(entries.clone(), entries),
    ));
    assert_eq!(lines(&plan), [format!("git reset --soft {}", commit('a'))]);
    assert_eq!(plan.title.as_str(), "Undo the commit \"Fix the parser\"");
    assert!(matches!(plan.danger, Danger::Destructive(_)));
}

#[test]
fn undo_last__takes_a_rebase_back_to_where_the_branch_reflog_had_it() {
    let head = vec![
        entry('d', "rebase (finish): returning to refs/heads/main"),
        entry('c', "rebase (pick): Fix"),
    ];
    let branch = vec![
        entry('d', "rebase (finish): refs/heads/main onto e"),
        entry('a', "commit: Before"),
    ];
    let plan = plan(last(&context(state(), Target::Head), &reflogs(head, branch)));
    assert_eq!(lines(&plan), [format!("git reset --keep {}", commit('a'))]);
}

#[test]
fn undo_last__checks_out_again_what_was_checked_out_before() {
    let head = vec![
        entry('b', "checkout: moving from main to feature"),
        entry('a', "commit: Fix"),
    ];
    let plan = plan(last(&context(state(), Target::Head), &reflogs(head, Vec::new())));
    assert_eq!(lines(&plan), ["git checkout main"]);
    let detached = vec![
        entry('b', &format!("checkout: moving from {} to main", commit('a'))),
        entry('a', "commit: Fix"),
    ];
    let plan = super::super::last(&context(state(), Target::Head), &reflogs(detached, Vec::new()));
    assert_eq!(
        lines(&crate::testing::plan(plan)),
        [format!("git checkout --detach {}", commit('a'))]
    );
}

#[test]
fn undo_last__resets_mixed_when_the_files_already_are_as_before() {
    let mut state = state();
    state.working_tree.staged.push(modified("a.txt"));
    let context = context(state, Target::Head);
    let entries = vec![entry('a', "reset: moving to aaaa"), entry('b', "commit: Fix")];
    let Outcome::Read(key, Query::FilesMatch { commit: asked }) =
        last(&context, &reflogs(entries.clone(), entries.clone()))
    else {
        panic!("with changes not committed, whether the files match is read")
    };
    assert_eq!((key, asked), (FILES_MATCH, commit('b')));
    let matching = reflogs(entries.clone(), entries.clone()).with(FILES_MATCH, Answer::Flag(true));
    assert_eq!(
        lines(&plan(last(&context, &matching))),
        [format!("git reset --mixed {}", commit('b'))]
    );
    let differing = reflogs(entries.clone(), entries).with(FILES_MATCH, Answer::Flag(false));
    let keep = plan(last(&context, &differing));
    assert_eq!(lines(&keep), [format!("git reset --keep {}", commit('b'))]);
    assert_eq!(keep.on_failure.len(), 1);
}

#[test]
fn undo_last__refuses_when_there_is_nothing_to_undo() {
    let context = context(state(), Target::Head);
    for head in [Vec::new(), vec![entry('a', "commit (initial): Add")]] {
        let answers = reflogs(head.clone(), head);
        let Outcome::Refuse(refusal) = last(&context, &answers) else {
            panic!("nothing to undo is refused")
        };
        assert_eq!(refusal.message.as_str(), "Nothing to undo");
    }
}

fn journal(record: UndoRecord) -> Answers {
    let entry = JournalEntry {
        number: 7,
        title: Text::from("Cherry-pick 3 commits"),
        record,
        head: state().head,
    };
    Answers::new().with(JOURNAL, Answer::Journal(Some(entry)))
}

fn journal_lines(record: UndoRecord) -> Vec<String> {
    let plan = plan(last(&context(state(), Target::Head), &journal(record)));
    assert_eq!(plan.undo, Undo::TakesBack(7));
    // Undo always shows what it does first; the undo of a stash dropped is confirmed rather than warned of
    // (`stash.undo-drop`).
    assert!(!matches!(plan.danger, Danger::None));
    lines(&plan)
}

#[test]
fn undo_last__reads_the_journal_before_the_reflogs() {
    let Outcome::Read(key, Query::Journal) = last(&context(state(), Target::Head), &Answers::new()) else {
        panic!("the journal is read first")
    };
    assert_eq!(key, JOURNAL);
}

#[test]
fn undo_last__creates_a_deleted_branch_again_at_its_commit() {
    let record = UndoRecord::BranchDeleted {
        name: String::from("feature"),
        commit: commit('b'),
    };
    assert_eq!(
        journal_lines(record.clone()),
        [format!("git branch feature {}", commit('b'))]
    );
    let plan = plan(last(&context(state(), Target::Head), &journal(record)));
    let Danger::Destructive(warning) = plan.danger else {
        panic!("undo is destructive")
    };
    assert!(warning.as_str().contains("does not get its upstream back"), "{warning}");
}

#[test]
fn undo_last__stores_a_dropped_stash_again_with_its_message() {
    let record = UndoRecord::StashDropped {
        message: String::from("On main: try this"),
        commit: commit('c'),
    };
    assert_eq!(
        journal_lines(record),
        [format!("git stash store -m \"On main: try this\" {}", commit('c'))]
    );
}

#[test]
fn undo_last__takes_a_series_of_cherry_picks_back_at_once() {
    let record = UndoRecord::Series {
        head_before: commit('a'),
    };
    assert_eq!(journal_lines(record), [format!("git reset --keep {}", commit('a'))]);
}

#[test]
fn undo_last__resets_a_squash_merge_with_merge() {
    assert_eq!(journal_lines(UndoRecord::SquashMerge), ["git reset --merge"]);
}

#[test]
fn undo_last__marks_resolved_conflicts_again_in_every_file() {
    let record = UndoRecord::ConflictsResolved {
        files: vec![String::from("a.txt"), String::from("pages/[id].tsx")],
    };
    assert_eq!(
        journal_lines(record),
        ["git checkout --merge -- a.txt :(literal)pages/[id].tsx"]
    );
}
