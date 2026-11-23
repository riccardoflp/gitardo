#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::fmt::Write as _;
use std::sync::mpsc;

use http::test_server::{Answer, TestServer, answer};
use http::{Cancel, Http, Proxies};
use json::Value;
use model::ai::{AiModels, AiProvider, AiSettings, Effort, Key};

use super::{
    AiError, Asked, ConflictReply, ConflictRequest, LanguageModel, Provider, ProviderModel, Purpose, Sending,
    ask_conflict, draft, list_models, models_answer,
};

/// The events of the Messages API that write a text in pieces.
pub(super) fn anthropic_stream(pieces: &[&str]) -> String {
    let mut stream = String::from(
        "event: message_start\ndata: {\"type\":\"message_start\"}\n\n\
         event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0}\n\n\
         event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\
         \"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"\"}}\n\n",
    );
    for piece in pieces {
        let data = json::to_compact_text(&Value::String((*piece).to_owned()));
        let _ = write!(
            stream,
            "event: content_block_delta\ndata: {{\"type\":\"content_block_delta\",\"index\":1,\
             \"delta\":{{\"type\":\"text_delta\",\"text\":{data}}}}}\n\n"
        );
    }
    stream.push_str("event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n");
    stream
}

/// An answer of server-sent events.
pub(super) fn events_answer(stream: &str) -> Answer {
    Answer::Bytes(answer(
        "200 OK",
        &[("Content-Type", "text/event-stream")],
        stream.as_bytes(),
    ))
}

/// The provider of a kind at the server of a test, with a key and a model.
pub(super) fn provider_at(server: &TestServer, kind: AiProvider, model: &str, effort: Option<Effort>) -> Provider {
    Provider {
        settings: AiSettings {
            provider: Some(kind),
            endpoint: format!("http://127.0.0.1:{}", server.port()),
            model: model.to_owned(),
            effort,
        },
        key: Some(Key(String::from("test-key"))),
    }
}

pub(super) fn http() -> Http {
    Http::new(TestServer::trust(), Proxies::none())
}

/// A question about a conflict, with nothing around it.
pub(super) fn conflict_request(question: &str) -> ConflictRequest {
    ConflictRequest {
        question: question.to_owned(),
        before: Vec::new(),
        after: Vec::new(),
        has_side_lines: true,
    }
}

pub(super) const REPLY: [&str; 4] = [
    "<resolution>\nboth",
    "();\n</resolution>",
    "<reason>Kept both.</reason>",
    "<confidence>high</confidence>",
];

/// The JSON of the body a server received.
fn body_of(server: &TestServer, index: usize) -> Value {
    let received = server.received();
    json::parse(std::str::from_utf8(&received[index].body).unwrap()).unwrap()
}

/// The question about a conflict asked of the provider over HTTP.
pub(super) fn asked(
    sending: Sending<'_>,
    provider: &Provider,
    request: &ConflictRequest,
    pieces: &mut dyn FnMut(&str),
) -> Result<ConflictReply, AiError> {
    ask_conflict(&mut ProviderModel { sending, provider }, request, pieces)
}

#[test]
fn ai_ask_conflict__the_request_is_the_question_and_the_pieces_come_in_order_before_the_answer() {
    let server = TestServer::plain(vec![events_answer(&anthropic_stream(&REPLY))]);
    let provider = provider_at(&server, AiProvider::Anthropic, "claude-test", None);
    let http = http();
    let cancel = Cancel::new();
    let mut seen = Vec::new();
    let reply = asked(
        Sending {
            http: &http,
            cancel: &cancel,
            waiting: None,
        },
        &provider,
        &conflict_request("Resolve one merge conflict in the file a.rs."),
        &mut |piece| seen.push(piece.to_owned()),
    )
    .unwrap();
    assert_eq!(seen, REPLY);
    assert_eq!(reply.answer.lines, ["both();"]);
    assert_eq!(reply.model, "claude-test");
    let received = server.received();
    assert_eq!(received[0].request_line(), "POST /v1/messages HTTP/1.1");
    assert_eq!(received[0].header("x-api-key").as_deref(), Some("test-key"));
    let body = body_of(&server, 0);
    let messages = body.get("messages").and_then(Value::as_array).unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].get("content").and_then(Value::as_str),
        Some("Resolve one merge conflict in the file a.rs.")
    );
    assert_eq!(body.get("stream").and_then(Value::as_bool), Some(true));
}

#[test]
fn ai_ask_conflict__high_effort_is_sent_as_the_provider_says_and_no_effort_when_empty() {
    let server = TestServer::plain(vec![
        events_answer(&anthropic_stream(&REPLY)),
        events_answer(&anthropic_stream(&REPLY)),
        events_answer(
            "data: {\"choices\":[{\"delta\":{\"content\":\"<resolution>\\nx();\\n</resolution>\"}}]}\n\n\
                       data: {\"choices\":[{\"delta\":{\"content\":\"<reason>Kept.</reason>\"}}]}\n\ndata: [DONE]\n\n",
        ),
    ]);
    let http = http();
    let cancel = Cancel::new();
    let sending = Sending {
        http: &http,
        cancel: &cancel,
        waiting: None,
    };
    let ask = |provider: &Provider| asked(sending, provider, &conflict_request("Q"), &mut |_| {}).unwrap();
    let _high = ask(&provider_at(&server, AiProvider::Anthropic, "m", Some(Effort::High)));
    let _empty = ask(&provider_at(&server, AiProvider::Anthropic, "m", None));
    let chat = ask(&provider_at(
        &server,
        AiProvider::OpenAiCompatible,
        "m",
        Some(Effort::High),
    ));
    assert_eq!(chat.answer.lines, ["x();"]);
    let effort = |body: &Value| {
        body.get("output_config")
            .and_then(|config| config.get("effort"))
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    assert_eq!(effort(&body_of(&server, 0)).as_deref(), Some("high"));
    assert_eq!(effort(&body_of(&server, 1)), None);
    assert_eq!(body_of(&server, 1).get("output_config"), None);
    let received = server.received();
    assert_eq!(received[2].request_line(), "POST /chat/completions HTTP/1.1");
    assert_eq!(received[2].header("authorization").as_deref(), Some("Bearer test-key"));
    assert_eq!(
        body_of(&server, 2).get("reasoning_effort").and_then(Value::as_str),
        Some("high")
    );
}

#[test]
fn ai_ask_conflict__cancelling_closes_the_request_and_returns_no_answer() {
    let (go_on, wait) = mpsc::channel();
    let stream = anthropic_stream(&REPLY);
    let (first, rest) = stream.split_at(
        stream
            .find("event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":1")
            .unwrap()
            + 200,
    );
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n",
        stream.len()
    );
    let server = TestServer::plain(vec![Answer::Parts(
        [head.as_bytes(), first.as_bytes()].concat(),
        wait,
        rest.as_bytes().to_vec(),
    )]);
    let provider = provider_at(&server, AiProvider::Anthropic, "m", None);
    let http = http();
    let cancel = Cancel::new();
    let mut seen = 0;
    let asked = asked(
        Sending {
            http: &http,
            cancel: &cancel,
            waiting: None,
        },
        &provider,
        &conflict_request("Q"),
        &mut |_| {
            seen += 1;
            cancel.cancel();
        },
    );
    drop(go_on);
    assert!(matches!(asked, Err(AiError::Cancelled)), "{asked:?}");
    assert_eq!(seen, 1);
}

#[test]
fn ai_choose_model__each_answer_names_the_model_asked_or_the_first_the_provider_lists() {
    let models = "{\"data\": [{\"id\": \"model-a\", \"display_name\": \"Model A\"}, {\"id\": \"model-b\"}]}";
    let server = TestServer::plain(vec![
        events_answer(&anthropic_stream(&REPLY)),
        Answer::Bytes(answer(
            "200 OK",
            &[("Content-Type", "application/json")],
            models.as_bytes(),
        )),
        events_answer(&anthropic_stream(&REPLY)),
    ]);
    let http = http();
    let cancel = Cancel::new();
    let sending = Sending {
        http: &http,
        cancel: &cancel,
        waiting: None,
    };
    let chosen = provider_at(&server, AiProvider::Anthropic, "model-b", None);
    let reply = asked(sending, &chosen, &conflict_request("Q"), &mut |_| {}).unwrap();
    assert_eq!(reply.model, "model-b");
    let first = provider_at(&server, AiProvider::Anthropic, "", None);
    let reply = asked(sending, &first, &conflict_request("Q"), &mut |_| {}).unwrap();
    assert_eq!(reply.model, "model-a");
    assert_eq!(
        server.received()[1].request_line(),
        "GET /v1/models?limit=1000 HTTP/1.1"
    );
}

#[test]
fn the_models_are_listed_with_their_names_and_a_refused_key_asks_for_another() {
    let models = "{\"data\": [{\"id\": \"model-a\", \"display_name\": \"Model A\"}]}";
    let refused =
        "{\"type\": \"error\", \"error\": {\"type\": \"authentication_error\", \"message\": \"invalid x-api-key\"}}";
    let server = TestServer::plain(vec![
        Answer::Bytes(answer("200 OK", &[], models.as_bytes())),
        Answer::Bytes(answer("401 Unauthorized", &[], refused.as_bytes())),
    ]);
    let http = http();
    let cancel = Cancel::new();
    let sending = Sending {
        http: &http,
        cancel: &cancel,
        waiting: None,
    };
    let provider = provider_at(&server, AiProvider::Anthropic, "", None);
    let listed = list_models(sending, &provider).unwrap();
    assert_eq!((listed[0].id.as_str(), listed[0].name.as_str()), ("model-a", "Model A"));
    assert_eq!(models_answer(sending, &provider), AiModels::KeyNeeded);
    let without_key = Provider { key: None, ..provider };
    assert!(matches!(list_models(sending, &without_key), Err(AiError::NoKey)));
    assert_eq!(server.received().len(), 2);
}

#[test]
fn a_model_the_provider_does_not_have_is_said_by_its_name() {
    let missing = "{\"type\": \"error\", \"error\": {\"type\": \"not_found_error\", \"message\": \"model: gone\"}}";
    let server = TestServer::plain(vec![Answer::Bytes(answer("404 Not Found", &[], missing.as_bytes()))]);
    let http = http();
    let cancel = Cancel::new();
    let provider = provider_at(&server, AiProvider::Anthropic, "gone", None);
    let asked = asked(
        Sending {
            http: &http,
            cancel: &cancel,
            waiting: None,
        },
        &provider,
        &conflict_request("Q"),
        &mut |_| {},
    );
    let said = asked.map_err(|error| error.to_string());
    assert_eq!(
        said,
        Err(String::from(
            "The model chosen to resolve conflicts, gone, is not available: choose another one."
        ))
    );
}

/// A language model that answers with what it is given, as the host's models answer the server.
struct Answering {
    /// The answer, in pieces.
    pieces: Vec<&'static str>,
    /// What it was asked, with what for.
    asked: Vec<(Purpose, String)>,
}

impl LanguageModel for Answering {
    fn models(&mut self) -> Result<Vec<model::ai::AiModel>, AiError> {
        Ok(Vec::new())
    }

    fn ask(
        &mut self,
        purpose: Purpose,
        request: &model::ai::AiRequest,
        pieces: &mut dyn FnMut(&str),
    ) -> Result<Asked, AiError> {
        self.asked.push((purpose, request.prompt.clone()));
        for piece in &self.pieces {
            pieces(piece);
        }
        Ok(Asked {
            model: String::from("Host model"),
            text: self.pieces.concat(),
        })
    }
}

#[test]
fn commit_draft_message__any_language_model_is_asked_and_its_answer_cleaned() {
    let mut model = Answering {
        pieces: vec!["```text\n", "Add the parser\n", "```"],
        asked: Vec::new(),
    };
    let subjects = [String::from("Fix the reader")];
    let input = model::commit::DraftInput {
        diff: "+a\n",
        new_files: &[],
        subjects: &subjects,
    };
    let mut seen = 0;
    let message = draft(&mut model, &input, &mut |_| seen += 1).unwrap();
    assert_eq!(message, "Add the parser");
    assert_eq!(seen, 3);
    assert_eq!(model.asked.len(), 1);
    assert_eq!(model.asked[0].0, Purpose::CommitMessage);
    assert_eq!(model.asked[0].1, model::commit::draft_message(&input).prompt);
}

#[test]
fn ai_ask_conflict__any_language_model_is_asked_and_its_answer_read_and_trimmed() {
    let mut model = Answering {
        pieces: vec![
            "<resolution>\nstart();\nbefore();\nboth();\n</resolution>",
            "<reason>Kept both.</reason>",
        ],
        asked: Vec::new(),
    };
    let request = ConflictRequest {
        before: vec![String::from("start();"), String::from("before();")],
        ..conflict_request("Q")
    };
    let reply = ask_conflict(&mut model, &request, &mut |_| {}).unwrap();
    assert_eq!(model.asked, [(Purpose::Conflict, String::from("Q"))]);
    assert_eq!(reply.model, "Host model");
    assert_eq!(reply.answer.lines, ["both();"]);
    assert_eq!(reply.answer.reason, "Kept both.");
}
