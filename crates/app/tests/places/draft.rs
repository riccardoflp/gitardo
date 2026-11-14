//! The message of the commit box drafted by the AI provider (`commit.draft-message`), against a test server on
//! 127.0.0.1 that answers as an OpenAI-compatible provider would: what is sent, what the box gets, and that nothing is
//! sent before the sparkle is pressed.

use app::store::{Effect, Message, State};
use http::test_server::{Answer, TestServer, answer};
use json::Value;
use model::ai::AiProvider;
use platform::accesskit::Role;
use testkit::Repository;
use views::Intent;
use views::changes::ChangesIntent;

use super::support::{AiDriver, Window, run};

/// A provider that streams a message in a code fence.
fn fenced_answer() -> Answer {
    let stream = "data: {\"choices\":[{\"delta\":{\"content\":\"```text\\nAdd the \"}}]}\n\n\
                  data: {\"choices\":[{\"delta\":{\"content\":\"parser\\n```\"}}]}\n\ndata: [DONE]\n\n";
    Answer::Bytes(answer(
        "200 OK",
        &[("Content-Type", "text/event-stream")],
        stream.as_bytes(),
    ))
}

/// A window showing the repository, with the Changes page open and the provider of the settings at the server.
fn drafting(repository: &Repository, server: &TestServer) -> State {
    let mut state = super::showing(repository.path().to_path_buf());
    let ai = &mut state.settings.features.ai;
    ai.provider = Some(AiProvider::OpenAiCompatible);
    ai.endpoint = format!("http://127.0.0.1:{}", server.port());
    ai.model = String::from("test-model");
    let _effects = run(&mut state, Message::Intent(Intent::Changes(ChangesIntent::Open)));
    state
}

/// The text of the one message the server was sent.
fn sent(server: &TestServer) -> String {
    let received = server.received();
    let body = json::parse(std::str::from_utf8(&received.first().unwrap().body).unwrap()).unwrap();
    let messages = body.get("messages").and_then(Value::as_array).unwrap();
    messages
        .first()
        .unwrap()
        .get("content")
        .and_then(Value::as_str)
        .unwrap()
        .to_owned()
}

/// The text of the message box of the repository.
fn message_box(state: &State) -> String {
    state
        .persisted
        .commit_messages
        .first()
        .map_or_default(|kept| kept.message.clone())
}

/// A repository with `a.txt` and `b.txt` committed.
fn two_files() -> Repository {
    let mut repository = Repository::new();
    let _first = repository.commit("Add the files", &[("a.txt", "a\n"), ("b.txt", "b\n")]);
    repository
}

#[test]
fn commit_draft_message__with_a_txt_staged_and_b_txt_not_the_request_has_the_diff_of_a_txt_only() {
    let repository = two_files();
    std::fs::write(repository.path().join("a.txt"), "a changed\n").unwrap();
    std::fs::write(repository.path().join("b.txt"), "b changed\n").unwrap();
    let _added = repository.git(&["add", "a.txt"]);
    let server = TestServer::plain(vec![fenced_answer()]);
    let mut state = drafting(&repository, &server);
    let driver = AiDriver::new(engine::MemorySecrets::default());
    let _effects = driver.intent(&mut state, Intent::Changes(ChangesIntent::DraftMessage));
    let _answered = driver.until_answered(&mut state);
    let asked = sent(&server);
    assert!(asked.contains("diff --git a/a.txt b/a.txt"), "{asked}");
    assert!(!asked.contains("b.txt"), "{asked}");
    assert!(!asked.contains("New files that are not in the diff:"), "{asked}");
    assert!(asked.contains("- Add the files\n"), "{asked}");
    assert_eq!(message_box(&state), "Add the parser");
}

#[test]
fn commit_draft_message__with_nothing_staged_and_new_txt_untracked_it_is_listed_as_a_new_file() {
    let repository = two_files();
    std::fs::write(repository.path().join("a.txt"), "a changed\n").unwrap();
    std::fs::write(repository.path().join("new.txt"), "new\n").unwrap();
    let server = TestServer::plain(vec![fenced_answer()]);
    let mut state = drafting(&repository, &server);
    let driver = AiDriver::new(engine::MemorySecrets::default());
    let _effects = driver.intent(&mut state, Intent::Changes(ChangesIntent::DraftMessage));
    let _answered = driver.until_answered(&mut state);
    let asked = sent(&server);
    assert!(asked.contains("diff --git a/a.txt b/a.txt"), "{asked}");
    assert!(
        asked.contains("\nNew files that are not in the diff:\n- new.txt\n"),
        "{asked}"
    );
}

#[test]
fn commit_draft_message__no_request_is_made_before_the_button_is_pressed() {
    let repository = two_files();
    std::fs::write(repository.path().join("a.txt"), "a changed\n").unwrap();
    let server = TestServer::plain(vec![fenced_answer()]);
    let mut state = drafting(&repository, &server);
    let mut window = Window::new(render::ThemeName::Dark, (1200, 800));
    let _tree = window.draw(&state, &[]);
    let typed = run(
        &mut state,
        Message::Intent(Intent::Changes(ChangesIntent::Type(String::from("Fix")))),
    );
    assert!(!typed.iter().any(|effect| matches!(effect, Effect::Ask(_))));
    assert_eq!(server.received().len(), 0);
    let intents = window.click(
        &state,
        Role::Button,
        "Draft a message from the changes with the AI provider",
    );
    assert!(
        intents.contains(&Intent::Changes(ChangesIntent::DraftMessage)),
        "{intents:?}"
    );
}

#[test]
fn commit_draft_message__cancelling_while_it_runs_leaves_the_message_box_as_it_was() {
    let repository = two_files();
    std::fs::write(repository.path().join("a.txt"), "a changed\n").unwrap();
    let server = TestServer::plain(vec![fenced_answer()]);
    let mut state = drafting(&repository, &server);
    let _typed = run(
        &mut state,
        Message::Intent(Intent::Changes(ChangesIntent::Type(String::from("Fix the reader")))),
    );
    let effects = run(
        &mut state,
        Message::Intent(Intent::Changes(ChangesIntent::DraftMessage)),
    );
    let Some(Effect::Ask(question)) = effects.iter().find(|effect| matches!(effect, Effect::Ask(_))) else {
        panic!("nothing asked: {effects:?}");
    };
    let stopped = run(&mut state, Message::Intent(Intent::Changes(ChangesIntent::StopDraft)));
    assert_eq!(stopped, [Effect::StopAsking(vec![question.id])]);
    let late = Message::AiAnswered {
        ask: question.id,
        result: Ok(app::work::ai::AiReply::Draft(String::from("Add the parser"))),
    };
    let _effects = run(&mut state, late);
    assert_eq!(message_box(&state), "Fix the reader");
}

#[test]
fn with_nothing_changed_or_no_provider_the_sparkle_refuses() {
    let repository = two_files();
    let server = TestServer::plain(Vec::new());
    let mut state = drafting(&repository, &server);
    let effects = run(
        &mut state,
        Message::Intent(Intent::Changes(ChangesIntent::DraftMessage)),
    );
    assert_eq!(effects.len(), 0);
    let said = state.operations.said.take().unwrap();
    assert_eq!(said.text, "There are no changes to describe.");
    std::fs::write(repository.path().join("a.txt"), "a changed\n").unwrap();
    let _read = run(&mut state, Message::Intent(Intent::Refresh));
    state.settings.features.ai.provider = None;
    let _effects = run(
        &mut state,
        Message::Intent(Intent::Changes(ChangesIntent::DraftMessage)),
    );
    let said = state.operations.said.take().unwrap();
    assert_eq!(said.text, "No AI provider is set up.");
    assert_eq!(said.offer.map(|(feature, _)| feature.0), Some("settings.edit"));
}
