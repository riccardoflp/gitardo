//! What each kind of provider is sent and answers: the Messages API of `anthropic`, the chat completions of
//! `openai-compatible`, and the models each lists.

use std::time::Duration;

use http::{Event, Head, Request, Url};
use json::{Member, Value};
use model::ai::{AiModel, AiProvider, AiRequest, Effort};

use super::{AiError, Provider, needs_key};

/// The version of the Messages API the requests are written for.
pub const ANTHROPIC_VERSION: &str = "2026-10-21";

/// The most tokens the model may write, its thinking included.
pub const MOST_TOKENS: u64 = 32_000;

/// The address of the provider's own interface, when `ai.endpoint` is empty.
fn default_endpoint(kind: AiProvider) -> &'static str {
    match kind {
        AiProvider::Anthropic => "https://api.anthropic.com",
        // GitHub Copilot is asked through its CLI, never over HTTP: `ai::http_kind` refuses it before a request.
        AiProvider::OpenAiCompatible | AiProvider::Copilot => "https://api.openai.com/v1",
    }
}

/// The address of a path of the provider: after `ai.endpoint`, or the provider's own address.
fn address(kind: AiProvider, provider: &Provider, path: &str) -> Result<Url, AiError> {
    let endpoint = provider.settings.endpoint.trim();
    let base = if endpoint.is_empty() {
        default_endpoint(kind)
    } else {
        endpoint
    };
    Ok(Url::parse(&format!("{}{path}", base.trim_end_matches('/')))?)
}

/// A request with the headers of the provider: its version and its key, which is never shown.
fn with_headers(kind: AiProvider, provider: &Provider, request: Request) -> Result<Request, AiError> {
    let request = request.header("User-Agent", "gitardo-desktop");
    match (kind, &provider.key) {
        (AiProvider::Anthropic, Some(key)) => Ok(request
            .header("anthropic-version", ANTHROPIC_VERSION)
            .secret_header("x-api-key", &key.0)),
        (AiProvider::OpenAiCompatible | AiProvider::Copilot, Some(key)) => {
            Ok(request.secret_header("Authorization", &format!("Bearer {}", key.0)))
        }
        (kind, None) if needs_key(kind) => Err(AiError::NoKey),
        (_, None) => Ok(request),
    }
}

/// The request that lists the models.
pub(super) fn models_request(kind: AiProvider, provider: &Provider) -> Result<Request, AiError> {
    let path = match kind {
        AiProvider::Anthropic => "/v1/models?limit=1000",
        AiProvider::OpenAiCompatible | AiProvider::Copilot => "/models",
    };
    with_headers(kind, provider, Request::get(address(kind, provider, path)?))
}

/// The models of an answer: `{"data": [{"id", "display_name"?}, …]}`.
pub(super) fn parse_models(kind: AiProvider, body: &[u8]) -> Result<Vec<AiModel>, AiError> {
    let answer = parse(body)?;
    let listed = answer
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| AiError::Unreadable(String::from("it lists no models")))?;
    Ok(listed
        .iter()
        .filter_map(|entry| {
            let id = entry.get("id").and_then(Value::as_str)?.to_owned();
            let name = match kind {
                AiProvider::Anthropic => entry.get("display_name").and_then(Value::as_str),
                AiProvider::OpenAiCompatible | AiProvider::Copilot => None,
            };
            Some(AiModel {
                name: name.map_or_else(|| id.clone(), str::to_owned),
                id,
            })
        })
        .collect())
}

/// A member of an object.
fn member(name: &str, value: Value) -> Member {
    Member {
        name: name.to_owned(),
        value,
    }
}

/// The value `reasoning_effort` of the chat completions takes for an effort: those above `high` are `xhigh`.
pub(super) fn chat_effort(effort: Effort) -> &'static str {
    match effort {
        Effort::Low => "low",
        Effort::Medium => "medium",
        Effort::High => "high",
        Effort::ExtraHigh | Effort::Max => "xhigh",
    }
}

/// The request that asks the model, its answer streamed as server-sent events.
pub(super) fn ask_request(
    kind: AiProvider,
    provider: &Provider,
    model: &str,
    request: &AiRequest,
) -> Result<Request, AiError> {
    let message = Value::Object(vec![
        member("role", Value::String(String::from("user"))),
        member("content", Value::String(request.prompt.clone())),
    ]);
    let mut body = vec![
        member("model", Value::String(model.to_owned())),
        member("stream", Value::Bool(true)),
        member("messages", Value::Array(vec![message])),
    ];
    let path = match kind {
        AiProvider::Anthropic => {
            body.insert(1, member("max_tokens", Value::from(MOST_TOKENS)));
            if let Some(effort) = provider.settings.effort {
                let config = Value::Object(vec![member("effort", Value::String(effort.setting().to_owned()))]);
                body.push(member("output_config", config));
            }
            "/v1/messages"
        }
        AiProvider::OpenAiCompatible | AiProvider::Copilot => {
            if let Some(effort) = provider.settings.effort {
                body.push(member(
                    "reasoning_effort",
                    Value::String(chat_effort(effort).to_owned()),
                ));
            }
            "/chat/completions"
        }
    };
    let text = json::to_compact_text(&Value::Object(body));
    let sent = Request::post(address(kind, provider, path)?, text.into_bytes())
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        // A model may think for minutes before it writes.
        .idle_timeout(Duration::from_secs(300));
    with_headers(kind, provider, sent)
}

/// What an event of the stream says.
pub(super) enum Read {
    /// A piece of the answer.
    Piece(String),
    /// Nothing of the answer.
    Nothing,
    /// The answer is whole.
    End,
}

/// What an event of the stream says, for a kind of provider.
pub(super) fn read_event(kind: AiProvider, event: &Event) -> Result<Read, AiError> {
    match kind {
        AiProvider::Anthropic => read_anthropic_event(event),
        AiProvider::OpenAiCompatible | AiProvider::Copilot => read_chat_event(event),
    }
}

/// An event of the Messages API: the text of a `content_block_delta`, an `error`, the end, or a refusal.
fn read_anthropic_event(event: &Event) -> Result<Read, AiError> {
    match event.name.as_str() {
        "content_block_delta" => {
            let data = parse(event.data.as_bytes())?;
            let delta = data.get("delta");
            let is_text = delta.and_then(|delta| delta.get("type")).and_then(Value::as_str) == Some("text_delta");
            let text = delta.and_then(|delta| delta.get("text")).and_then(Value::as_str);
            Ok(match (is_text, text) {
                (true, Some(text)) => Read::Piece(text.to_owned()),
                // The deltas of thinking say nothing of the answer.
                _ => Read::Nothing,
            })
        }
        "message_delta" => {
            let data = parse(event.data.as_bytes())?;
            let reason = data
                .get("delta")
                .and_then(|delta| delta.get("stop_reason"))
                .and_then(Value::as_str);
            if reason == Some("refusal") {
                return Err(AiError::Refused(String::from("the model declined to answer.")));
            }
            Ok(Read::Nothing)
        }
        "error" => Err(AiError::Refused(error_message(&parse(event.data.as_bytes())?))),
        "message_stop" => Ok(Read::End),
        // `message_start`, `content_block_start`, `content_block_stop` and `ping` say nothing of the answer.
        _ => Ok(Read::Nothing),
    }
}

/// An event of the chat completions: the content of the delta of its first choice, an error, or `[DONE]`.
fn read_chat_event(event: &Event) -> Result<Read, AiError> {
    if event.data.trim() == "[DONE]" {
        return Ok(Read::End);
    }
    let data = parse(event.data.as_bytes())?;
    if data.get("error").is_some() {
        return Err(AiError::Refused(error_message(&data)));
    }
    let content = data
        .get("choices")
        .and_then(Value::as_array)
        .and_then(<[Value]>::first)
        .and_then(|choice| choice.get("delta"))
        .and_then(|delta| delta.get("content"))
        .and_then(Value::as_str);
    Ok(content.map_or(Read::Nothing, |content| Read::Piece(content.to_owned())))
}

/// The JSON of an answer.
fn parse(body: &[u8]) -> Result<Value, AiError> {
    let text = std::str::from_utf8(body).map_err(|error| AiError::Unreadable(error.to_string()))?;
    json::parse(text).map_err(|error| AiError::Unreadable(error.to_string()))
}

/// The message of `{"error": {"message": …}}`, or of `{"error": "…"}`.
fn error_message(answer: &Value) -> String {
    let error = answer.get("error");
    error
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .or_else(|| error.and_then(Value::as_str))
        .unwrap_or("it gave no reason.")
        .to_owned()
}

/// What an answer whose status is not 2xx means: a key refused, a model chosen that is not there, or the status.
pub(super) fn status_error(head: &Head, body: &[u8], provider: &Provider, chosen: Option<&str>) -> AiError {
    match (head.status, chosen) {
        (401 | 403, _) if provider.key.is_some() => AiError::KeyRefused,
        (401 | 403, _) => AiError::NoKey,
        (404, Some(model)) => AiError::ModelMissing(model.to_owned()),
        _ => AiError::Status {
            line: head.status_line(),
            message: parse(body).map_or_default(|answer| error_message(&answer)),
        },
    }
}
