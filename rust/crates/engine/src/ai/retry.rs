//! A request to the AI provider sent again, as Anthropic's SDKs send theirs: when its connection failed, or when it was
//! answered 408, 409, 429 or a status of 500 or more, unless the answer says `x-should-retry: false`; at most twice,
//! after a wait that ends at once when the question is cancelled. Only the head of the answer is waited for: once the
//! answer streams, a failure is said, not hidden behind a second answer (D87).

use std::hash::{BuildHasher, Hasher};
use std::thread;
use std::time::{Duration, Instant};

use http::{Cancel, Head, Opened, Request};
use model::Timestamp;
use model::ai::AiProvider;
use model::contract::Text;

use super::{AiError, Sending};

/// How many times a request is sent again, after the first: two, as the SDKs do, and no setting (Q1512).
pub(super) const MOST_RETRIES: u32 = 2;

/// The longest wait that an answer's `retry-after-ms` or `retry-after` asks for: a minute.
pub(super) const LONGEST_ASKED_WAIT: Duration = Duration::from_secs(60);

/// The wait before the first retry, when the answer asks for none: half a second, doubled before each other one.
const FIRST_WAIT: Duration = Duration::from_millis(500);

/// The longest wait when the answer asks for none: 8 seconds.
const LONGEST_WAIT: Duration = Duration::from_secs(8);

/// The most taken off a wait the answer did not ask for, at random, so that many windows do not ask again together.
const RANDOM_PART: f64 = 0.25;

/// How long a wait sleeps at a time before it looks whether the question was cancelled.
const LOOK_EVERY: Duration = Duration::from_millis(50);

/// Why a request is sent again.
enum Failure {
    /// The connection failed before the head of the answer was read.
    Connection(http::Error),
    /// The answer has a status that passes: its head.
    Status(Head),
}

/// Sends a request, and sends it again after a wait when it failed for a reason that passes, at most
/// [`MOST_RETRIES`] times; gives the last answer whose head was read, whatever its status, for the caller to read.
///
/// # Errors
///
/// When the request could not be sent the last time, for a reason that does not pass, or when the question is
/// cancelled, while it is sent or while it waits.
pub(super) fn open(sending: Sending<'_>, request: &Request, kind: AiProvider) -> Result<Opened, AiError> {
    let mut retries = 0;
    loop {
        let failure = match sending.http.open(request, sending.cancel) {
            Ok(opened) if retries < MOST_RETRIES && is_retried(&opened.head) => Failure::Status(opened.head),
            Ok(opened) => return Ok(opened),
            Err(error) if retries < MOST_RETRIES && is_connection_failure(&error) => Failure::Connection(error),
            Err(error) => return Err(error.into()),
        };
        let (head, why) = match &failure {
            Failure::Connection(error) => (None, error.to_string()),
            Failure::Status(head) => (Some(head), status_words(head)),
        };
        let duration = wait_before(retries, head, random_fraction());
        let _shown = sending.waiting.map(|waiting| {
            let words = waiting_words(kind, request);
            waiting
                .activities
                .track(Text::from(words), Text::from(why), waiting.now())
        });
        wait(duration, sending.cancel)?;
        retries += 1;
    }
}

/// What the status bar says while a request waits to be sent again: *Waiting for Anthropic, then trying again…*, or
/// the host of the address of a server of the chat completions.
#[must_use]
pub(super) fn waiting_words(kind: AiProvider, request: &Request) -> String {
    let provider = match kind {
        AiProvider::Anthropic => "Anthropic",
        AiProvider::OpenAiCompatible | AiProvider::Copilot => request.url.host(),
    };
    format!("Waiting for {provider}, then trying again…")
}

/// Why a request answered with a status is sent again, in the words of its refusal.
fn status_words(head: &Head) -> String {
    AiError::Status {
        line: head.status_line(),
        message: String::new(),
    }
    .to_string()
}

/// Whether an answer is to be asked again: 408, 409, 429 or 500 and more, unless its `x-should-retry` is `false`.
#[must_use]
pub(super) fn is_retried(head: &Head) -> bool {
    let is_refused = head
        .header("x-should-retry")
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("false"));
    !is_refused && (matches!(head.status, 408 | 409 | 429) || head.status >= 500)
}

/// Whether a request failed because its connection did, which may pass: the host not found, not reached, silent, or
/// gone while the answer was read.
fn is_connection_failure(error: &http::Error) -> bool {
    match error {
        http::Error::Resolve { .. }
        | http::Error::Connect { .. }
        | http::Error::TimedOut { .. }
        | http::Error::Io { .. } => true,
        // An address, a certificate, a proxy or an answer that is wrong is as wrong the second time.
        http::Error::BadUrl { .. }
        | http::Error::Tls(_)
        | http::Error::Proxy { .. }
        | http::Error::BadResponse { .. }
        | http::Error::TooManyRedirects { .. }
        | http::Error::InsecureRedirect { .. }
        | http::Error::Cancelled => false,
    }
}

/// The wait before a request is sent again, after `retries` retries: what the answer's `retry-after-ms`, else its
/// `retry-after` in seconds, asks for, up to a minute; otherwise half a second doubled after each retry up to 8 seconds,
/// less up to a quarter of it as `random`, from 0 to 1, says.
#[must_use]
pub(super) fn wait_before(retries: u32, head: Option<&Head>, random: f64) -> Duration {
    if let Some(asked) = head.and_then(asked_wait) {
        return asked.min(LONGEST_ASKED_WAIT);
    }
    let doubled = FIRST_WAIT.saturating_mul(1_u32 << retries.min(5)).min(LONGEST_WAIT);
    doubled.mul_f64(1.0 - RANDOM_PART * random.clamp(0.0, 1.0))
}

/// The wait an answer asks for: `retry-after-ms` in milliseconds, else `retry-after` in seconds; none when neither
/// is a number that is not negative. A `retry-after` that is a date asks for none.
fn asked_wait(head: &Head) -> Option<Duration> {
    let seconds = |value: &str, per_second: f64| {
        let number = value.trim().parse::<f64>().ok()?;
        Duration::try_from_secs_f64(number / per_second).ok()
    };
    head.header("retry-after-ms")
        .and_then(|value| seconds(value, 1000.0))
        .or_else(|| head.header("retry-after").and_then(|value| seconds(value, 1.0)))
}

/// A number from 0 to 1 that differs from one call to the next, from the random keys the standard library gives each
/// hash map.
fn random_fraction() -> f64 {
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u32(std::process::id());
    let bits = u32::try_from(hasher.finish() >> 32).unwrap_or_default();
    f64::from(bits) / f64::from(u32::MAX)
}

/// Waits as long as `duration`, and ends at once when the question is cancelled.
///
/// # Errors
///
/// When the question is cancelled before the wait is over.
pub(super) fn wait(duration: Duration, cancel: &Cancel) -> Result<(), AiError> {
    let started = Instant::now();
    loop {
        if cancel.is_cancelled() {
            return Err(AiError::Cancelled);
        }
        let left = duration.saturating_sub(started.elapsed());
        if left.is_zero() {
            return Ok(());
        }
        thread::sleep(left.min(LOOK_EVERY));
    }
}

/// Where the wait before a request is sent again is shown while it lasts: among the activities of the window, whose
/// status bar says what runs, started at the time of the clock.
#[derive(Clone, Copy)]
pub struct Waiting<'a> {
    /// The activities of the window.
    pub activities: &'a crate::Activities,
    /// The time a wait starts at.
    pub clock: &'a dyn crate::Clock,
}

impl Waiting<'_> {
    /// The time now, by the clock.
    fn now(&self) -> Timestamp {
        self.clock.now()
    }
}

impl std::fmt::Debug for Waiting<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Waiting")
            .field("activities", self.activities)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
