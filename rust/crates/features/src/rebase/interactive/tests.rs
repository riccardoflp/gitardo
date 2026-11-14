use super::{TODO, interactive};
use crate::rebase::testing::{line_of, with_line};
use crate::testing::{commit, context, lines, plan, state};
use model::contract::{Answer, Answers, Danger, Outcome, Step, Target};
use model::state::Operation;

/// `HEAD` is `c`, on `b`, on `a`, on `9`.
fn three() -> model::rebase::LineToHead {
    line_of(
        "9",
        &[('a', "First\n"), ('b', "Old words\n\nBody.\n"), ('c', "Third\n")],
    )
}

fn rows(text: &str) -> Answers {
    let text = text
        .replace("<A>", commit('a').as_str())
        .replace("<B>", commit('b').as_str())
        .replace("<C>", commit('c').as_str());
    with_line(Answers::new().with(TODO, Answer::Text(text)), three())
}

#[test]
fn rebase_interactive__a_reword_is_a_pick_and_an_amend_that_keeps_the_body() {
    let context = context(state(), Target::Head);
    let plan = plan(interactive(
        &context,
        &rows("pick <A>\nreword <B> Better words\npick <C>\n"),
    ));
    let folder = "repository/.git/gitardo-rebase";
    let on_disk = std::path::Path::new("repository/.git").join("gitardo-rebase");
    assert_eq!(
        lines(&plan),
        [
            format!("write {}", on_disk.join("message-1.txt").display()),
            format!("write {}", on_disk.join("todo").display()),
            format!(
                "git -c \"sequence.editor=cp \\\"{folder}/todo\\\"\" -c core.editor=true rebase -i {}",
                commit('9')
            ),
        ]
    );
    let Some(Step::Write { contents, .. }) = plan.steps.first() else {
        panic!("the message is written first")
    };
    assert_eq!(String::from_utf8_lossy(contents), "Better words\n\nBody.\n");
    let Some(Step::Write { contents, .. }) = plan.steps.get(1) else {
        panic!("then the todo list")
    };
    assert_eq!(
        String::from_utf8_lossy(contents),
        format!(
            "pick {}\npick {}\nexec git commit --amend -F \"{folder}/message-1.txt\"\npick {}\n",
            commit('a'),
            commit('b'),
            commit('c')
        )
    );
}

#[test]
fn rebase_interactive__the_confirmation_shows_the_todo_list() {
    let context = context(state(), Target::Head);
    let plan = plan(interactive(&context, &rows("pick <A>\ndrop <B>\nfixup <C>\n")));
    let Danger::Confirm(note) = &plan.danger else {
        panic!("it asks first")
    };
    assert!(
        note.as_str()
            .starts_with("This rewrites the commits after 9999999 with this todo list:\n\n")
    );
    assert!(
        note.as_str()
            .contains(&format!("drop {}\nfixup {}", commit('b'), commit('c')))
    );
    assert_eq!(plan.title.as_str(), "Interactive rebase of main");
}

#[test]
fn rebase_interactive__from_the_root_runs_with_root() {
    let context = context(state(), Target::Head);
    let line = line_of("", &[('a', "First\n"), ('b', "Second\n"), ('c', "Third\n")]);
    let text = format!("pick {}\npick {}\npick {}\n", commit('b'), commit('a'), commit('c'));
    let answers = with_line(Answers::new().with(TODO, Answer::Text(text)), line);
    let plan = plan(interactive(&context, &answers));
    assert!(lines(&plan).last().unwrap().ends_with("rebase -i --root"));
}

#[test]
fn rebase_interactive__refuses_a_list_without_every_commit_or_while_stopped() {
    let context_now = context(state(), Target::Head);
    assert!(matches!(
        interactive(&context_now, &rows("pick <A>\npick <C>\n")),
        Outcome::Refuse(_)
    ));
    assert!(matches!(
        interactive(&context_now, &rows("drop <A>\ndrop <B>\ndrop <C>\n")),
        Outcome::Refuse(_)
    ));
    let mut stopped = state();
    stopped.operation = Some(Operation::Rebase);
    let Outcome::Refuse(refusal) =
        interactive(&context(stopped, Target::Head), &rows("pick <A>\npick <B>\npick <C>\n"))
    else {
        panic!("refused")
    };
    assert_eq!(refusal.message.as_str(), "Finish or abort the rebase first.");
}
