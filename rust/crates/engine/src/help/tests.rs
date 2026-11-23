#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::fmt::Write as _;

use http::test_server::{Answer, TestServer, answer};
use http::{Cancel, Http, Proxies};
use json::Value;
use model::ai::{AiProvider, AiRequest, AiSettings, Effort, Key};
use model::contract::FeatureId;
use model::help::{App, HelpInput, NO_PROVIDER, choose_cards, guide, question};
use testkit::Repository;

use super::{HelpRequest, ask, said};
use crate::ai::{AiError, Asked, LanguageModel, Provider, ProviderModel, Purpose, Sending};

/// The events of the Messages API that write a text in pieces.
fn anthropic_stream(pieces: &[&str]) -> Answer {
    let mut stream = String::from("event: message_start\ndata: {\"type\":\"message_start\"}\n\n");
    for piece in pieces {
        let data = json::to_compact_text(&Value::String((*piece).to_owned()));
        let _ = write!(
            stream,
            "event: content_block_delta\ndata: {{\"type\":\"content_block_delta\",\"index\":0,\
             \"delta\":{{\"type\":\"text_delta\",\"text\":{data}}}}}\n\n"
        );
    }
    stream.push_str("event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n");
    Answer::Bytes(answer(
        "200 OK",
        &[("Content-Type", "text/event-stream")],
        stream.as_bytes(),
    ))
}

/// The provider of `ai.provider` at the server of a test.
fn provider_at(server: &TestServer, provider: Option<AiProvider>, effort: Option<Effort>) -> Provider {
    Provider {
        settings: AiSettings {
            provider,
            endpoint: format!("http://127.0.0.1:{}", server.port()),
            model: String::from("claude-test"),
            effort,
        },
        key: Some(Key(String::from("test-key"))),
    }
}

/// The text of the message of the request a server received.
fn message_of(server: &TestServer, index: usize) -> String {
    let body = body_of(server, index);
    body.get("messages")
        .and_then(Value::as_array)
        .and_then(|messages| messages.first())
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// The JSON of the body of the request a server received.
fn body_of(server: &TestServer, index: usize) -> Value {
    let received = server.received();
    received
        .get(index)
        .and_then(|request| std::str::from_utf8(&request.body).ok())
        .and_then(|body| json::parse(body).ok())
        .unwrap_or(Value::Null)
}

/// A question asked in the window, with no earlier one.
fn desktop_question(question: &str) -> HelpRequest {
    HelpRequest {
        app: App::Desktop,
        input: HelpInput {
            question: question.to_owned(),
            earlier: Vec::new(),
        },
    }
}

/// Asks a question of the provider over HTTP.
fn asked_over_http(
    provider: &Provider,
    request: &HelpRequest,
    pieces: &mut dyn FnMut(&str),
) -> Result<model::help::HelpAnswer, AiError> {
    let http = Http::new(TestServer::trust(), Proxies::none());
    let cancel = Cancel::new();
    let mut model = ProviderModel {
        sending: Sending {
            http: &http,
            cancel: &cancel,
            waiting: None,
        },
        provider,
    };
    ask(&mut model, request, pieces)
}

const ANSWER: [&str; 3] = [
    "1. Select the three commits in the timeline.\n",
    "2. Press **Squash into one** in the selection bar: [[squash.",
    "commits]].",
];

#[test]
fn help_ask__two_requests_are_sent_each_as_its_card_makes_it_and_the_pieces_of_the_second_come_in_order() {
    let server = TestServer::plain(vec![
        anthropic_stream(&["- squash.commits\n", "selection.bar\n"]),
        anthropic_stream(&ANSWER),
    ]);
    let request = desktop_question("How do I squash my last three commits?");
    let mut seen = Vec::new();
    let answer = asked_over_http(
        &provider_at(&server, Some(AiProvider::Anthropic), None),
        &request,
        &mut |piece| seen.push(piece.to_owned()),
    )
    .unwrap();
    let guide = guide(App::Desktop);
    assert_eq!(server.received().len(), 2);
    assert_eq!(message_of(&server, 0), choose_cards(&request.input, &guide).prompt);
    let chosen = [FeatureId("squash.commits"), FeatureId("selection.bar")];
    assert_eq!(message_of(&server, 1), question(&request.input, &guide, &chosen).prompt);
    assert_eq!(seen, ANSWER);
    assert_eq!(answer.named, [FeatureId("squash.commits")]);
    assert!(
        answer
            .markdown
            .ends_with("[Squash commits into one](show-me:squash.commits)."),
        "{}",
        answer.markdown
    );
    assert_eq!(answer.model, "claude-test");
}

#[test]
fn help_ask__no_request_has_the_path_the_branch_or_a_subject_of_the_repository() {
    let mut repository = Repository::new();
    let _first = repository.commit("Add the zebra parser", &[("a.txt", "a\n")]);
    repository.branch("topic-okapi");
    repository.checkout("topic-okapi");
    let server = TestServer::plain(vec![anthropic_stream(&["none"]), anthropic_stream(&ANSWER)]);
    let _answer = asked_over_http(
        &provider_at(&server, Some(AiProvider::Anthropic), None),
        &desktop_question("How do I undo a push?"),
        &mut |_| {},
    )
    .unwrap();
    let path = repository.path().display().to_string();
    for request in server.received() {
        let body = String::from_utf8_lossy(&request.body).into_owned();
        for private in [path.as_str(), "topic-okapi", "Add the zebra parser"] {
            assert!(!body.contains(private), "{private}");
        }
    }
}

#[test]
fn help_ask__with_high_effort_both_requests_carry_the_providers_setting_for_it() {
    let server = TestServer::plain(vec![anthropic_stream(&["none"]), anthropic_stream(&ANSWER)]);
    let _answer = asked_over_http(
        &provider_at(&server, Some(AiProvider::Anthropic), Some(Effort::High)),
        &desktop_question("Q"),
        &mut |_| {},
    )
    .unwrap();
    for index in 0..2 {
        let effort = body_of(&server, index)
            .get("output_config")
            .and_then(|config| config.get("effort"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        assert_eq!(effort.as_deref(), Some("high"), "request {index}");
    }
}

/// A language model whose first question is cancelled while it runs.
struct CancelledFirst {
    asked: Vec<Purpose>,
}

impl LanguageModel for CancelledFirst {
    fn models(&mut self) -> Result<Vec<model::ai::AiModel>, AiError> {
        Ok(Vec::new())
    }

    fn ask(&mut self, purpose: Purpose, _request: &AiRequest, pieces: &mut dyn FnMut(&str)) -> Result<Asked, AiError> {
        self.asked.push(purpose);
        if self.asked.len() == 1 {
            return Err(AiError::Cancelled);
        }
        pieces("Answered");
        Ok(Asked {
            model: String::from("m"),
            text: String::from("Answered"),
        })
    }
}

#[test]
fn help_ask__cancelling_while_the_first_request_runs_sends_no_second_and_gives_no_answer() {
    let mut model = CancelledFirst { asked: Vec::new() };
    let mut seen = 0;
    let asked = ask(&mut model, &desktop_question("Q"), &mut |_| seen += 1);
    assert!(matches!(asked, Err(AiError::Cancelled)), "{asked:?}");
    assert_eq!(model.asked, [Purpose::Help]);
    assert_eq!(seen, 0);
}

#[test]
fn help_ask__with_no_provider_nothing_is_sent_and_it_refuses_with_the_message() {
    let server = TestServer::plain(Vec::new());
    let asked = asked_over_http(&provider_at(&server, None, None), &desktop_question("Q"), &mut |_| {});
    let error = asked.unwrap_err();
    assert_eq!(said(&error), NO_PROVIDER);
    assert_eq!(server.received().len(), 0);
}
