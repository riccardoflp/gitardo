//! Keys kept out of the Command log (`log.record`): a request to the AI provider that fails, against a test server on
//! 127.0.0.1 that answers with the key in what it says. The same for GitHub's token is in `github.rs`.

use app::store::{Message, State};
use engine::SecretStore as _;
use http::test_server::{Answer, TestServer, answer};
use model::ai::AiProvider;
use testkit::Repository;
use views::Intent;
use views::changes::ChangesIntent;

use super::support::{AiDriver, run};

/// The key of the AI provider of the test.
const KEY: &str = "sk-test-0123456789abcdef";

/// What a provider says of a key it refuses: the key itself, as some do.
pub(super) fn refused(secret: &str) -> Answer {
    let body = format!("{{\"error\": {{\"message\": \"Incorrect API key provided: {secret}.\"}}}}");
    Answer::Bytes(answer(
        "401 Unauthorized",
        &[("Content-Type", "application/json")],
        body.as_bytes(),
    ))
}

/// A window showing a repository with a change, the Changes page open and the provider of the settings at the server.
fn drafting(repository: &Repository, server: &TestServer) -> State {
    let mut state = super::showing(repository.path().to_path_buf());
    let ai = &mut state.settings.features.ai;
    ai.provider = Some(AiProvider::OpenAiCompatible);
    ai.endpoint = format!("http://127.0.0.1:{}", server.port());
    ai.model = String::from("test-model");
    let _effects = run(&mut state, Message::Intent(Intent::Changes(ChangesIntent::Open)));
    state
}

/// The lines of the Command log.
fn logged(state: &State) -> Vec<String> {
    state.log.entries().map(|entry| entry.line.clone()).collect()
}

#[test]
fn log_record__a_key_the_ai_provider_refuses_appears_neither_in_the_log_nor_in_what_the_window_says() {
    let mut repository = Repository::new();
    let _first = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    std::fs::write(repository.path().join("README.md"), "Gitardo Desktop\n").unwrap();
    let server = TestServer::plain(vec![refused(KEY)]);
    let mut state = drafting(&repository, &server);
    let secrets = engine::MemorySecrets::default();
    secrets.write(&AiProvider::OpenAiCompatible.key_name(), KEY).unwrap();
    let driver = AiDriver::new(secrets);
    let _effects = driver.intent(&mut state, Intent::Changes(ChangesIntent::DraftMessage));
    let Message::AiAnswered { result, .. } = driver.until_answered(&mut state) else {
        unreachable!("until_answered ends on an answer");
    };
    assert!(result.is_err(), "the provider refused the key");
    let sent = server.received();
    assert_eq!(
        sent.first().and_then(|request| request.header("authorization")),
        Some(format!("Bearer {KEY}")),
        "the key was sent"
    );
    let said = state
        .operations
        .said
        .as_ref()
        .map(|said| said.text.clone())
        .unwrap_or_default();
    assert!(!said.is_empty() && !said.contains(KEY), "{said}");
    assert!(
        logged(&state).iter().all(|line| !line.contains(KEY)),
        "{:?}",
        logged(&state)
    );
}
