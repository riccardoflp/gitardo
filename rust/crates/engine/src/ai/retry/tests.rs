#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::thread;
use std::time::{Duration, Instant};

use http::test_server::{Answer, TestServer, answer};
use http::{Cancel, Head};
use model::Timestamp;
use model::ai::AiProvider;

use super::super::tests::{REPLY, anthropic_stream, asked, conflict_request, events_answer, http, provider_at};
use super::{LONGEST_ASKED_WAIT, Waiting, is_retried, wait_before};
use crate::ai::{AiError, ProviderModel, Sending, draft, list_models};
use crate::{Activities, FixedClock};

/// The sentence of an answer cut at its length.
const CUT: &str = "The answer of the model was cut at its length. Ask again with a lower effort.";

/// What the status bar says while Anthropic is waited for.
const WAITING: &str = "Waiting for Anthropic, then trying again…";

/// An answer of a status with headers, and the error of the Messages API as its body.
fn status_answer(status: &str, headers: &[(&str, &str)]) -> Answer {
    let body = "{\"type\": \"error\", \"error\": {\"type\": \"overloaded_error\", \"message\": \"Overloaded\"}}";
    Answer::Bytes(answer(status, headers, body.as_bytes()))
}

/// The head of an answer with a status and headers.
fn head(status: u16, headers: &[(&str, &str)]) -> Head {
    Head {
        status,
        reason: String::new(),
        headers: headers
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect(),
    }
}

/// The clock of the tests.
const CLOCK: FixedClock = FixedClock(Timestamp(1_000));

/// Asks the question about a conflict of Anthropic at the server, the waits shown among `activities`.
fn ask_anthropic(server: &TestServer, activities: &Activities, cancel: &Cancel) -> Result<Vec<String>, AiError> {
    let provider = provider_at(server, AiProvider::Anthropic, "claude-test", None);
    let http = http();
    let sending = Sending {
        http: &http,
        cancel,
        waiting: Some(Waiting {
            activities,
            clock: &CLOCK,
        }),
    };
    asked(sending, &provider, &conflict_request("Q"), &mut |_| {}).map(|reply| reply.answer.lines)
}

/// The descriptions and the causes of the activities that ended.
fn ended(activities: &Activities) -> Vec<(String, String)> {
    activities
        .report()
        .ended
        .iter()
        .map(|ended| (ended.entry.description.to_string(), ended.entry.cause.to_string()))
        .collect()
}

#[test]
fn ai_ask_conflict__an_answer_529_then_200_gives_the_answer_after_one_wait_which_the_status_bar_says() {
    let server = TestServer::plain(vec![
        status_answer("529 Overloaded", &[("retry-after-ms", "20")]),
        events_answer(&anthropic_stream(&REPLY)),
    ]);
    let activities = Activities::new();
    let lines = ask_anthropic(&server, &activities, &Cancel::new()).unwrap();
    assert_eq!(lines, ["both();"]);
    assert_eq!(server.received().len(), 2);
    assert_eq!(
        ended(&activities),
        [(
            String::from(WAITING),
            String::from("The AI provider answered 529 Overloaded.")
        )]
    );
}

#[test]
fn ai_ask_conflict__the_wait_is_as_asked_up_to_a_minute_else_half_a_second_doubled_less_a_random_quarter() {
    let millis = Duration::from_millis;
    let cases: [(u32, Head, f64, Duration); 10] = [
        (0, head(429, &[("retry-after", "2")]), 0.5, millis(2000)),
        (0, head(429, &[("retry-after-ms", "1500")]), 0.5, millis(1500)),
        (
            1,
            head(429, &[("retry-after", "9"), ("retry-after-ms", "250")]),
            0.5,
            millis(250),
        ),
        (0, head(529, &[("retry-after", "600")]), 0.5, LONGEST_ASKED_WAIT),
        (
            0,
            head(503, &[("retry-after", "Wed, 21 Oct 2026 07:28:00 GMT")]),
            0.0,
            millis(500),
        ),
        (0, head(500, &[]), 0.0, millis(500)),
        (0, head(500, &[]), 1.0, millis(375)),
        (1, head(500, &[]), 0.0, millis(1000)),
        (1, head(500, &[]), 0.5, millis(875)),
        (6, head(500, &[]), 0.0, millis(8000)),
    ];
    for (retries, head, random, expected) in cases {
        assert_eq!(
            wait_before(retries, Some(&head), random),
            expected,
            "{head:?} after {retries}"
        );
    }
    assert_eq!(
        wait_before(1, None, 0.0),
        millis(1000),
        "a failed connection has no answer"
    );
}

#[test]
fn ai_ask_conflict__an_answer_429_with_retry_after_2_is_sent_again_after_two_seconds() {
    let server = TestServer::plain(vec![
        status_answer("429 Too Many Requests", &[("retry-after", "2")]),
        events_answer(&anthropic_stream(&REPLY)),
    ]);
    let started = Instant::now();
    let lines = ask_anthropic(&server, &Activities::new(), &Cancel::new()).unwrap();
    let took = started.elapsed();
    assert_eq!(lines, ["both();"]);
    assert!(took >= Duration::from_secs(2), "{took:?}");
    assert!(took < Duration::from_secs(10), "{took:?}");
}

#[test]
fn ai_ask_conflict__408_409_429_and_500_and_more_are_sent_again_but_not_400_or_x_should_retry_false() {
    for status in [408, 409, 429, 500, 503, 529] {
        assert!(is_retried(&head(status, &[])), "{status}");
    }
    for status in [400, 401, 403, 404, 413] {
        assert!(!is_retried(&head(status, &[])), "{status}");
    }
    assert!(!is_retried(&head(529, &[("x-should-retry", "false")])));
    assert!(is_retried(&head(529, &[("x-should-retry", "maybe")])));
    let server = TestServer::plain(vec![
        status_answer(
            "529 Overloaded",
            &[("x-should-retry", "false"), ("retry-after-ms", "1")],
        ),
        status_answer("400 Bad Request", &[("retry-after-ms", "1")]),
        events_answer(&anthropic_stream(&REPLY)),
    ]);
    let activities = Activities::new();
    let refused = ask_anthropic(&server, &activities, &Cancel::new()).map_err(|error| error.to_string());
    assert_eq!(
        refused,
        Err(String::from("The AI provider answered 529 Overloaded: Overloaded"))
    );
    assert_eq!(server.received().len(), 1);
    let refused = ask_anthropic(&server, &activities, &Cancel::new()).map_err(|error| error.to_string());
    assert_eq!(
        refused,
        Err(String::from("The AI provider answered 400 Bad Request: Overloaded"))
    );
    assert_eq!(server.received().len(), 2);
    assert_eq!(ended(&activities), []);
}

#[test]
fn ai_ask_conflict__a_failed_connection_is_sent_again() {
    // The server closes the first connection without a word.
    let server = TestServer::plain(vec![
        Answer::Bytes(Vec::new()),
        events_answer(&anthropic_stream(&REPLY)),
    ]);
    let activities = Activities::new();
    let lines = ask_anthropic(&server, &activities, &Cancel::new()).unwrap();
    assert_eq!(lines, ["both();"]);
    assert_eq!(server.received().len(), 2);
    assert_eq!(ended(&activities).len(), 1);
}

#[test]
fn ai_ask_conflict__a_request_is_sent_again_at_most_twice() {
    let overloaded = || status_answer("529 Overloaded", &[("retry-after-ms", "1")]);
    let server = TestServer::plain(vec![
        overloaded(),
        overloaded(),
        overloaded(),
        events_answer(&anthropic_stream(&REPLY)),
    ]);
    let activities = Activities::new();
    let refused = ask_anthropic(&server, &activities, &Cancel::new());
    assert!(matches!(refused, Err(AiError::Status { .. })), "{refused:?}");
    assert_eq!(server.received().len(), 3);
    assert_eq!(ended(&activities).len(), 2);
}

#[test]
fn ai_ask_conflict__a_failure_after_the_first_piece_of_the_answer_is_said_not_sent_again() {
    let failed = anthropic_stream(&["<resolution>\n"]).replace(
        "event: message_stop\ndata: {\"type\":\"message_stop\"}",
        "event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}",
    );
    let server = TestServer::plain(vec![events_answer(&failed), events_answer(&anthropic_stream(&REPLY))]);
    let provider = provider_at(&server, AiProvider::Anthropic, "claude-test", None);
    let http = http();
    let cancel = Cancel::new();
    let activities = Activities::new();
    let sending = Sending {
        http: &http,
        cancel: &cancel,
        waiting: Some(Waiting {
            activities: &activities,
            clock: &CLOCK,
        }),
    };
    let mut seen = Vec::new();
    let refused = asked(sending, &provider, &conflict_request("Q"), &mut |piece| {
        seen.push(piece.to_owned());
    })
    .map_err(|error| error.to_string());
    assert_eq!(seen, ["<resolution>\n"]);
    assert_eq!(
        refused,
        Err(String::from("The AI provider refused the request: Overloaded"))
    );
    assert_eq!(server.received().len(), 1);
    assert_eq!(ended(&activities), []);
}

#[test]
fn ai_ask_conflict__cancel_ends_the_wait_at_once_and_nothing_more_is_sent() {
    let server = TestServer::plain(vec![
        status_answer("529 Overloaded", &[("retry-after", "60")]),
        events_answer(&anthropic_stream(&REPLY)),
    ]);
    let activities = Activities::new();
    let cancel = Cancel::new();
    let watcher = {
        let (activities, cancel) = (activities.clone(), cancel.clone());
        thread::spawn(move || {
            let started = Instant::now();
            while started.elapsed() < Duration::from_secs(30) {
                let running = activities.running();
                if running.iter().any(|entry| entry.description.as_str() == WAITING) {
                    cancel.cancel();
                    return Instant::now();
                }
                thread::sleep(Duration::from_millis(5));
            }
            panic!("the wait was never shown");
        })
    };
    let refused = ask_anthropic(&server, &activities, &cancel);
    let ended_at = Instant::now();
    let cancelled_at = watcher.join().unwrap();
    assert!(matches!(refused, Err(AiError::Cancelled)), "{refused:?}");
    assert!(
        ended_at.saturating_duration_since(cancelled_at) < Duration::from_secs(1),
        "the wait went on"
    );
    assert_eq!(server.received().len(), 1);
    assert_eq!(activities.running(), []);
}

/// A stream of the Messages API that stops at the most tokens the model may write.
fn cut_stream() -> String {
    anthropic_stream(&["<resolution>\nboth"]).replace(
        "event: message_stop",
        "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"max_tokens\"}}\n\n\
         event: message_stop",
    )
}

#[test]
fn ai_ask_conflict__an_answer_cut_at_its_length_is_refused_with_the_sentence() {
    let chat_cut = "data: {\"choices\":[{\"delta\":{\"content\":\"<resolution>\\nx\"}}]}\n\n\
                    data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"length\"}]}\n\ndata: [DONE]\n\n";
    let server = TestServer::plain(vec![events_answer(&cut_stream()), events_answer(chat_cut)]);
    let refused = ask_anthropic(&server, &Activities::new(), &Cancel::new()).map_err(|error| error.to_string());
    assert_eq!(refused, Err(String::from(CUT)));
    let provider = provider_at(&server, AiProvider::OpenAiCompatible, "m", None);
    let http = http();
    let cancel = Cancel::new();
    let sending = Sending {
        http: &http,
        cancel: &cancel,
        waiting: None,
    };
    let refused = asked(sending, &provider, &conflict_request("Q"), &mut |_| {}).map_err(|error| error.to_string());
    assert_eq!(refused, Err(String::from(CUT)));
}

#[test]
fn commit_draft_message__an_answer_cut_at_its_length_is_refused_with_the_sentence_and_no_draft() {
    let server = TestServer::plain(vec![events_answer(&cut_stream())]);
    let provider = provider_at(&server, AiProvider::Anthropic, "claude-test", None);
    let http = http();
    let cancel = Cancel::new();
    let mut model = ProviderModel {
        sending: Sending {
            http: &http,
            cancel: &cancel,
            waiting: None,
        },
        provider: &provider,
    };
    let input = model::commit::DraftInput {
        diff: "+a\n",
        new_files: &[],
        subjects: &[],
    };
    let drafted = draft(&mut model, &input, &mut |_| {}).map_err(|error| error.to_string());
    assert_eq!(drafted, Err(String::from(CUT)));
}

#[test]
fn ai_choose_model__a_list_answered_429_is_asked_again_after_the_wait() {
    let models = "{\"data\": [{\"id\": \"model-a\", \"display_name\": \"Model A\"}]}";
    let server = TestServer::plain(vec![
        status_answer("429 Too Many Requests", &[("retry-after-ms", "10")]),
        Answer::Bytes(answer("200 OK", &[], models.as_bytes())),
    ]);
    let provider = provider_at(&server, AiProvider::Anthropic, "", None);
    let http = http();
    let cancel = Cancel::new();
    let activities = Activities::new();
    let sending = Sending {
        http: &http,
        cancel: &cancel,
        waiting: Some(Waiting {
            activities: &activities,
            clock: &CLOCK,
        }),
    };
    let listed = list_models(sending, &provider).unwrap();
    assert_eq!(listed[0].id, "model-a");
    assert_eq!(server.received().len(), 2);
    assert_eq!(ended(&activities).len(), 1);
}
