use super::{SUBJECT, commits, into_previous};
use crate::rebase::testing::{line_of, with_line, with_pushed};
use crate::testing::{commit, context, lines, modified, plan, question, state};
use model::contract::{Answer, Answers, Danger, FollowUp, Outcome, Query, Step, Target};
use model::state::RepositoryState;

/// `A→B→C→D→E`, `E` checked out: digits `1`, `2`, `3`, `4`, `5`.
fn line_from_b() -> model::rebase::LineToHead {
    line_of(
        "1",
        &[
            ('2', "Add the login page\n\nWith a form.\n"),
            ('3', "Fix a typo\n"),
            ('4', "Style the form\n"),
            ('5', "Add the logout\n"),
        ],
    )
}

fn at_e() -> RepositoryState {
    let mut state = state();
    state.head.commit = Some(commit('5'));
    state
}

fn selected(digits: &str) -> Target {
    Target::Commits(digits.chars().map(commit).collect())
}

fn answered(subject: &str) -> Answers {
    with_pushed(
        with_line(Answers::new(), line_from_b()).with(SUBJECT, Answer::Text(subject.to_owned())),
        &[],
    )
}

#[test]
fn squash_commits__b_c_and_d_become_one_commit_selected_under_e() {
    let context = context(at_e(), selected("432"));
    let plan = plan(commits(&context, &answered("Add the login page")));
    let Some(Step::Write { contents, .. }) = plan.steps.first() else {
        panic!("the message is written first")
    };
    assert_eq!(
        String::from_utf8_lossy(contents),
        "Add the login page\n\nWith a form.\n\nFix a typo\n\nStyle the form\n"
    );
    let Some(Step::Write { contents, .. }) = plan.steps.get(1) else {
        panic!("then the todo list")
    };
    let todo = String::from_utf8_lossy(contents);
    let words: Vec<&str> = todo
        .lines()
        .map(|line| line.split(' ').next().unwrap_or_default())
        .collect();
    assert_eq!(words, ["pick", "exec", "fixup", "fixup", "pick"]);
    assert!(lines(&plan).last().unwrap().contains("rebase -i --autostash"));
    assert_eq!(plan.title.as_str(), "Squash 3 commits");
    assert!(plan.after.contains(&FollowUp::SelectBeforeHead(1)));
    let Danger::Confirm(note) = &plan.danger else {
        panic!("it asks first")
    };
    assert_eq!(
        note.as_str(),
        "2222222, 3333333, 4444444 become one commit, with the changes of them all. The commit after them is made \
         again on top of it."
    );
}

#[test]
fn squash_commits__is_refused_while_a_bisect_is_under_way() {
    let mut state = at_e();
    state.bisect = Some(model::bisect::Bisect::default());
    let Outcome::Refuse(refusal) = commits(&context(state, selected("432")), &answered("Add the login page")) else {
        panic!("refused")
    };
    assert_eq!(refusal.message.as_str(), "End the bisect first.");
}

#[test]
fn squash_commits__b_and_d_without_c_are_refused() {
    let context = context(at_e(), selected("24"));
    let Outcome::Refuse(refusal) = commits(&context, &answered("Squashed")) else {
        panic!("refused")
    };
    assert!(
        refusal
            .message
            .as_str()
            .starts_with("Only commits that follow one another")
    );
}

#[test]
fn squash_commits__asks_for_the_subject_first_with_that_of_the_oldest() {
    let context = context(at_e(), selected("32"));
    let asked = question(commits(&context, &with_line(Answers::new(), line_from_b())));
    assert_eq!(asked.key, SUBJECT);
    assert_eq!(asked.title.as_str(), "Squash 2 commits");
    assert_eq!(asked.default, Some(Answer::Text(String::from("Add the login page"))));
}

#[test]
fn squash_commits__sets_aside_what_is_not_committed_and_says_so() {
    let mut state = at_e();
    state.working_tree.unstaged.push(modified("notes.txt"));
    let plan = plan(commits(&context(state, selected("23")), &answered("Squashed")));
    let Danger::Confirm(note) = &plan.danger else {
        panic!("it asks first")
    };
    assert!(note.as_str().ends_with(
        "The 2 commits after them are made again on top of it. Changes that are not committed are set aside and put \
         back."
    ));
}

#[test]
fn squash_commits__reads_again_the_line_of_an_older_commit_selected() {
    let context = context(at_e(), selected("32"));
    let from_c = line_of("2", &[('3', "Fix a typo\n"), ('4', "Style\n"), ('5', "Logout\n")]);
    let outcome = commits(&context, &with_line(Answers::new(), from_c));
    assert_eq!(
        outcome,
        Outcome::Read(crate::rebase::LINE, Query::Line { commit: commit('2') })
    );
}

#[test]
fn squash_into_previous__on_head_squashes_it_with_its_parent() {
    let context = context(at_e(), Target::Commit(commit('5')));
    let parents = line_of("4", &[('5', "Add the logout\n")]);
    let answers = with_pushed(
        with_line(
            Answers::new(),
            line_of("3", &[('4', "Style\n"), ('5', "Add the logout\n")]),
        )
        .with(super::into_previous::PARENTS, Answer::Line(parents))
        .with(SUBJECT, Answer::Text(String::from("Style"))),
        &[],
    );
    let plan = plan(into_previous(&context, &answers));
    assert_eq!(plan.title.as_str(), "Squash 2 commits");
    assert!(plan.after.contains(&FollowUp::SelectBeforeHead(0)));
}

#[test]
fn squash_into_previous__refuses_the_first_commit() {
    let context = context(at_e(), Target::Commit(commit('1')));
    let answers = Answers::new().with(
        super::into_previous::PARENTS,
        Answer::Line(line_of("", &[('1', "First\n")])),
    );
    assert!(matches!(into_previous(&context, &answers), Outcome::Refuse(_)));
}

#[test]
fn squash_into_previous__refuses_a_commit_whose_parent_is_a_merge() {
    let context = context(at_e(), Target::Commit(commit('5')));
    let mut merge = line_of("3", &[]);
    merge.commits = None;
    let answers = with_line(
        Answers::new().with(
            super::into_previous::PARENTS,
            Answer::Line(line_of("4", &[('5', "Logout\n")])),
        ),
        merge,
    );
    assert!(matches!(into_previous(&context, &answers), Outcome::Refuse(_)));
}
