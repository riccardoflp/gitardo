//! The requests the engine sends the extension while a feature runs (`feature/ask`, `feature/confirm`) or a read or a
//! feature needs the host (`host/githubToken`, `host/languageModel`, `host/languageModels`), each waited for until its
//! answer comes, the request it belongs to is cancelled, or the extension is gone. A request cancelled is cancelled in
//! the extension too, with `$/cancel`, so that a language model stops answering what no one waits for.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::Duration;

use git::CancelToken;
use protocol::messages::{CancelParams, methods};
use protocol::rpc::{RequestId, ResponseError, notification_message, request_message};
use serde_json::Value;

use crate::session::locked;

/// How often a request waited for looks whether it was cancelled.
const LOOK_AGAIN: Duration = Duration::from_millis(100);

/// The requests sent and not answered yet.
#[derive(Debug, Default)]
pub struct Outgoing {
    /// The id of the next request; the engine's ids are strings, so that they never meet the extension's numbers.
    next: AtomicI64,
    /// Where the answer of each request is given.
    waiting: Mutex<HashMap<RequestId, Sender<Result<Value, ResponseError>>>>,
}

/// Why a request got no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoAnswer {
    /// The request of the extension it belongs to was cancelled.
    Cancelled,
    /// The extension answered with an error, or is gone.
    Failed(String),
}

impl Outgoing {
    /// Sends a request with `send` and waits for its answer; when `cancel` is cancelled first, sends `$/cancel` for it.
    ///
    /// # Errors
    ///
    /// When `cancel` is cancelled first, the extension answers with an error, or the input ends.
    pub fn ask(
        &self,
        send: impl Fn(Value),
        method: &str,
        params: Value,
        cancel: &CancelToken,
    ) -> Result<Value, NoAnswer> {
        let number = self.next.fetch_add(1, Ordering::SeqCst);
        let id = RequestId::Text(format!("engine-{number}"));
        let (sender, receiver) = mpsc::channel();
        let _none = locked(&self.waiting).insert(id.clone(), sender);
        send(request_message(&id, method, params));
        let answer = loop {
            match receiver.recv_timeout(LOOK_AGAIN) {
                Ok(answer) => break answer.map_err(|error| NoAnswer::Failed(error.message)),
                Err(RecvTimeoutError::Timeout) if cancel.is_cancelled() => break Err(NoAnswer::Cancelled),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    break Err(NoAnswer::Failed(String::from("The extension is gone.")));
                }
            }
        };
        let _done = locked(&self.waiting).remove(&id);
        if matches!(answer, Err(NoAnswer::Cancelled))
            && let Ok(params) = serde_json::to_value(CancelParams { id })
        {
            send(notification_message(methods::CANCEL, params));
        }
        answer
    }

    /// Gives an answer of the extension to the request waiting for it; an answer no one waits for is left.
    pub fn answer(&self, id: &RequestId, outcome: Result<Value, ResponseError>) {
        if let Some(sender) = locked(&self.waiting).remove(id) {
            let _given = sender.send(outcome);
        }
    }

    /// Ends every wait: the input ended.
    pub fn end(&self) {
        locked(&self.waiting).clear();
    }
}
