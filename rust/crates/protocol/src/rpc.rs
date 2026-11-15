//! JSON-RPC 2.0 as the protocol uses it: what one message is, how it is read from the bytes of a frame, and how an
//! answer, an error or a notification is written (`docs/transition/02-protocol.md`, Transport and Errors).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The codes of the errors: JSON-RPC's own, then the engine's.
pub mod codes {
    /// The bytes are not JSON.
    pub const PARSE_ERROR: i64 = -32_700;
    /// The JSON is not a request, a notification nor an answer.
    pub const INVALID_REQUEST: i64 = -32_600;
    /// No method has that name.
    pub const METHOD_NOT_FOUND: i64 = -32_601;
    /// The parameters are not those of the method, or the version of the protocol differs in its major number.
    pub const INVALID_PARAMS: i64 = -32_602;
    /// The engine could not write its own answer: a bug.
    pub const INTERNAL_ERROR: i64 = -32_603;
    /// The repository was not opened, or was closed.
    pub const NOT_OPEN: i64 = 1;
    /// A read of git failed.
    pub const GIT_FAILED: i64 = 2;
    /// A step of a plan failed.
    pub const STEP_FAILED: i64 = 3;
    /// An answer that its question does not take.
    pub const ANSWER_REFUSED: i64 = 4;
    /// A feature that kept asking without deciding.
    pub const ENDLESS_FEATURE: i64 = 5;
    /// An operation was asked while another runs in the same repository.
    pub const BUSY: i64 = 6;
    /// The request was cancelled with `$/cancel`.
    pub const CANCELLED: i64 = 7;
    /// No feature has that identifier.
    pub const UNKNOWN_FEATURE: i64 = 8;
}

/// The id of a request: a number or a string, as the side that sent it chose.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    /// A number.
    Number(i64),
    /// A string.
    Text(String),
}

/// An error, as a request that failed answers it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResponseError {
    /// One of [`codes`].
    pub code: i64,
    /// The sentence for the user.
    pub message: String,
    /// What git printed and the steps that ran, for the log; or, for a version that differs, the engine's version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl ResponseError {
    /// An error with its code and its sentence, and no data.
    #[must_use]
    pub fn new(code: i64, message: impl Into<String>) -> ResponseError {
        ResponseError {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// The same error with its data.
    #[must_use]
    pub fn with_data(mut self, data: Value) -> ResponseError {
        self.data = Some(data);
        self
    }
}

/// The data of an error of git or of a step: what git printed, and the lines of the steps that ran.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorData {
    /// What git printed; none when it printed nothing or did not run.
    pub output: Option<String>,
    /// The steps that ran before the error, as the log wrote them.
    pub steps: Vec<String>,
    /// What the extension offers for a step that failed, such as **Delete anyway**.
    #[serde(default)]
    pub follow_ups: Vec<crate::feature::FollowUpJson>,
}

/// One message read from the other side.
#[derive(Clone, Debug, PartialEq)]
pub enum Incoming {
    /// A request, which is answered.
    Request {
        /// Its id, which the answer repeats.
        id: RequestId,
        /// The method.
        method: String,
        /// Its parameters; `null` when it has none.
        params: Value,
    },
    /// A notification, which is not answered.
    Notification {
        /// The method.
        method: String,
        /// Its parameters; `null` when it has none.
        params: Value,
    },
    /// The answer to a request this side sent.
    Response {
        /// The id of the request.
        id: RequestId,
        /// Its result, or its error.
        outcome: Result<Value, ResponseError>,
    },
}

/// What a message whose bytes cannot be taken is answered: the error, and the id of the request when it could be
/// read.
#[derive(Clone, Debug, PartialEq)]
pub struct Unreadable {
    /// The id, when the message had one that could be read.
    pub id: Option<RequestId>,
    /// The error to answer with.
    pub error: ResponseError,
}

/// The message in the bytes of a frame.
///
/// # Errors
///
/// [`codes::PARSE_ERROR`] when they are not JSON, [`codes::INVALID_REQUEST`] when the JSON is not a message of
/// JSON-RPC 2.0.
pub fn read_message(bytes: &[u8]) -> Result<Incoming, Unreadable> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| Unreadable {
        id: None,
        error: ResponseError::new(codes::PARSE_ERROR, format!("The message is not JSON: {error}")),
    })?;
    let Value::Object(mut fields) = value else {
        return Err(invalid(None, "The message is not a JSON object."));
    };
    let id = match fields.remove("id") {
        None | Some(Value::Null) => None,
        Some(value) => Some(
            serde_json::from_value::<RequestId>(value)
                .map_err(|_| invalid(None, "The id of the message is neither a number nor a string."))?,
        ),
    };
    if fields.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(invalid(id, "The message does not say \"jsonrpc\": \"2.0\"."));
    }
    let params = fields.remove("params").unwrap_or(Value::Null);
    match (fields.remove("method"), id) {
        (Some(Value::String(method)), Some(id)) => Ok(Incoming::Request { id, method, params }),
        (Some(Value::String(method)), None) => Ok(Incoming::Notification { method, params }),
        (Some(_), id) => Err(invalid(id, "The method of the message is not a string.")),
        (None, Some(id)) => response(id, &mut fields),
        (None, None) => Err(invalid(None, "The message has neither a method nor an id.")),
    }
}

/// The answer in the fields of a message that has an id and no method.
fn response(id: RequestId, fields: &mut Map<String, Value>) -> Result<Incoming, Unreadable> {
    if let Some(error) = fields.remove("error") {
        let error = serde_json::from_value::<ResponseError>(error)
            .map_err(|_| invalid(Some(id.clone()), "The error of the answer has no code or no message."))?;
        return Ok(Incoming::Response {
            id,
            outcome: Err(error),
        });
    }
    match fields.remove("result") {
        Some(result) => Ok(Incoming::Response {
            id,
            outcome: Ok(result),
        }),
        None => Err(invalid(Some(id), "The answer has neither a result nor an error.")),
    }
}

/// The error of a message that is JSON and not one of JSON-RPC.
fn invalid(id: Option<RequestId>, message: &str) -> Unreadable {
    Unreadable {
        id,
        error: ResponseError::new(codes::INVALID_REQUEST, message),
    }
}

/// The answer of a request that did what it was asked.
#[must_use]
pub fn result_message(id: &RequestId, result: Value) -> Value {
    message([("id", id_value(id)), ("result", result)])
}

/// The answer of a request that failed; the id is `null` when the request's could not be read.
#[must_use]
pub fn error_message(id: Option<&RequestId>, error: &ResponseError) -> Value {
    let mut fields = Map::new();
    let _code = fields.insert(String::from("code"), Value::from(error.code));
    let _message = fields.insert(String::from("message"), Value::from(error.message.as_str()));
    if let Some(data) = &error.data {
        let _data = fields.insert(String::from("data"), data.clone());
    }
    message([
        ("id", id.map_or(Value::Null, id_value)),
        ("error", Value::Object(fields)),
    ])
}

/// A notification.
#[must_use]
pub fn notification_message(method: &str, params: Value) -> Value {
    message([("method", Value::from(method)), ("params", params)])
}

/// A request this side sends.
#[must_use]
pub fn request_message(id: &RequestId, method: &str, params: Value) -> Value {
    message([
        ("id", id_value(id)),
        ("method", Value::from(method)),
        ("params", params),
    ])
}

/// A message of JSON-RPC 2.0 with its fields.
fn message<const COUNT: usize>(fields: [(&str, Value); COUNT]) -> Value {
    let mut object = Map::new();
    let _version = object.insert(String::from("jsonrpc"), Value::from("2.0"));
    for (name, value) in fields {
        let _field = object.insert(name.to_owned(), value);
    }
    Value::Object(object)
}

/// The JSON of an id.
fn id_value(id: &RequestId) -> Value {
    match id {
        RequestId::Number(number) => Value::from(*number),
        RequestId::Text(text) => Value::from(text.as_str()),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{Incoming, RequestId, ResponseError, codes, error_message, read_message, result_message};

    #[test]
    fn reads_requests_notifications_and_answers() {
        let request = read_message(br#"{"jsonrpc":"2.0","id":3,"method":"shutdown"}"#).unwrap();
        assert_eq!(
            request,
            Incoming::Request {
                id: RequestId::Number(3),
                method: String::from("shutdown"),
                params: Value::Null
            }
        );
        let notification = read_message(br#"{"jsonrpc":"2.0","method":"exit"}"#).unwrap();
        assert!(matches!(notification, Incoming::Notification { method, .. } if method == "exit"));
        let answer = read_message(br#"{"jsonrpc":"2.0","id":"a","result":{"text":null}}"#).unwrap();
        assert_eq!(
            answer,
            Incoming::Response {
                id: RequestId::Text(String::from("a")),
                outcome: Ok(json!({ "text": null }))
            }
        );
        let failed = read_message(br#"{"jsonrpc":"2.0","id":1,"error":{"code":7,"message":"Cancelled."}}"#).unwrap();
        assert_eq!(
            failed,
            Incoming::Response {
                id: RequestId::Number(1),
                outcome: Err(ResponseError::new(codes::CANCELLED, "Cancelled."))
            }
        );
    }

    #[test]
    fn what_is_not_json_or_not_a_message_is_said_with_its_code_and_the_id_when_there_is_one() {
        let not_json = read_message(b"{ not json").unwrap_err();
        assert_eq!((not_json.id, not_json.error.code), (None, codes::PARSE_ERROR));
        let no_version = read_message(br#"{"id":4,"method":"initialize"}"#).unwrap_err();
        assert_eq!(
            (no_version.id, no_version.error.code),
            (Some(RequestId::Number(4)), codes::INVALID_REQUEST)
        );
        let neither = read_message(br#"{"jsonrpc":"2.0"}"#).unwrap_err();
        assert_eq!(neither.error.code, codes::INVALID_REQUEST);
    }

    #[test]
    fn writes_answers_with_their_id() {
        let id = RequestId::Number(9);
        assert_eq!(
            result_message(&id, Value::Null),
            json!({ "jsonrpc": "2.0", "id": 9, "result": null })
        );
        let error = ResponseError::new(codes::METHOD_NOT_FOUND, "No method is named nothing.");
        assert_eq!(
            error_message(None, &error),
            json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32601, "message": "No method is named nothing." } })
        );
    }
}
