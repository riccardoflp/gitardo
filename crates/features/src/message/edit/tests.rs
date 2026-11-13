use super::{BEFORE_HEAD, EDITED, LINE, MESSAGE, OLD, PUSHED, edit};
use crate::testing::{commit, context, lines, modified, plan, question, state};
use model::commit::Commit;
use model::contract::{Answer, Answers, Danger, FollowUp, Outcome, Query, Target};
use model::{CommitId, Timestamp};

fn row(id: CommitId, parent: CommitId) -> Commit {
    Commit {
        id,
        parents: vec![parent],
        author_name: String::new(),
        author_email: String::new(),
        author_date: Timestamp(0),
        subject: String::new(),
    }
}

fn typed(message: &str) -> Answers {
    Answers::new()
        .with(OLD, Answer::Message(String::from("Old words")))
        .with(MESSAGE, Answer::Text(message.to_owned()))
}

#[test]
fn message_edit__the_last_commit_is_amended_with_the_message_on_standard_input() {
    let context = context(state(), Target::Commit(commit('a')));
    assert!(matches!(
        edit(&context, &Answers::new()),
        Outcome::Read(OLD, Query::Message { .. })
    ));
    let asked = question(edit(
        &context,
        &Answers::new().with(OLD, Answer::Message(String::from("Old words"))),
    ));
    assert_eq!(asked.default, Some(Answer::Text(String::from("Old words"))));
    let answers = typed("  New words\n").with(PUSHED, Answer::Names(Vec::new()));
    let plan = plan(edit(&context, &answers));
    assert_eq!(lines(&plan), ["git commit --amend --only --allow-empty -F -"]);
    let Some(model::contract::Step::Git { input, .. }) = plan.steps.first() else {
        panic!("one git step")
    };
    assert_eq!(input.as_deref(), Some(&b"New words\n"[..]));
    assert_eq!(plan.danger, Danger::None);
}

#[test]
fn message_edit__an_older_commit_is_reworded_by_one_rebase_with_autostash() {
    let mut state = state();
    state.head.commit = Some(commit('d'));
    state.working_tree.unstaged.push(modified("a.txt"));
    let context = context(state, Target::Commit(commit('b')));
    let line = vec![row(commit('d'), commit('c')), row(commit('c'), commit('b'))];
    let answers = typed("New words")
        .with(BEFORE_HEAD, Answer::Flag(true))
        .with(EDITED, Answer::Commits(vec![row(commit('b'), commit('a'))]))
        .with(LINE, Answer::Commits(line))
        .with(PUSHED, Answer::Names(Vec::new()));
    let plan = plan(edit(&context, &answers));
    let lines = lines(&plan);
    assert_eq!(lines.len(), 4, "{lines:?}");
    assert!(lines[2].starts_with("git -c \"sequence.editor=cp "), "{}", lines[2]);
    assert!(lines[2].ends_with(&format!("-c core.editor=true rebase -i --autostash {}", commit('a'))));
    let Some(model::contract::Step::Write { contents, .. }) = plan.steps.get(1) else {
        panic!("the todo list is written")
    };
    let todo = String::from_utf8_lossy(contents);
    assert!(todo.starts_with(&format!("pick {}\nexec git commit --amend -F", commit('b'))));
    assert!(todo.ends_with(&format!("pick {}\npick {}\n", commit('c'), commit('d'))));
    assert_eq!(plan.after, [FollowUp::SelectBeforeHead(2)]);
    let Danger::Confirm(note) = plan.danger else {
        panic!("a rewrite asks first")
    };
    assert_eq!(
        note.as_str(),
        "bbbbbbb gets the new message, and the 2 commits after it are made again on top of it, with new hashes.\n\n\
         Changes that are not committed are set aside and put back."
    );
}

#[test]
fn message_edit__a_commit_on_origin_main_asks_first_and_says_it_will_be_force_pushed() {
    let context = context(state(), Target::Commit(commit('a')));
    let answers = typed("New words").with(PUSHED, Answer::Names(vec![String::from("origin/main")]));
    let Danger::Confirm(note) = plan(edit(&context, &answers)).danger else {
        panic!("a pushed commit asks first")
    };
    assert!(
        note.as_str()
            .starts_with("aaaaaaa is pushed already, on origin/main: the branch will have to be force pushed"),
        "{note}"
    );
}

#[test]
fn message_edit__an_empty_message_is_refused() {
    let context = context(state(), Target::Commit(commit('a')));
    let Outcome::Refuse(refusal) = edit(&context, &typed("  \n")) else {
        panic!("an empty message is refused")
    };
    assert_eq!(refusal.message.as_str(), "Enter a message.");
}

#[test]
fn message_edit__a_commit_off_the_line_before_head_is_refused_before_asking() {
    let mut state = state();
    state.head.commit = Some(commit('d'));
    let context = context(state, Target::Commit(commit('b')));
    let answers = Answers::new()
        .with(OLD, Answer::Message(String::from("Old words")))
        .with(BEFORE_HEAD, Answer::Flag(false));
    assert!(matches!(edit(&context, &answers), Outcome::Refuse(_)));
}
