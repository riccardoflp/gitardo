//! The GitHub Copilot CLI in its server mode, spoken to in JSON-RPC 2.0 over its standard input and output: each
//! message framed with a `Content-Length` header, as the Language Server Protocol frames them and as the Copilot SDK
//! speaks to the CLI. A thread reads what the CLI writes: the answers to the requests sent, the events of the sessions,
//! and the requests the CLI makes, every one of which is refused (`ai.copilot`, **Tools**).

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::Child;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use http::Cancel;
use json::{Member, Value};

use super::CopilotError;

/// How often a wait for the CLI looks whether the question was cancelled.
const CANCEL_CHECK: Duration = Duration::from_millis(50);

/// What the CLI prints, and where what it reads is written: the pipes of the process, or those of a fake in a test.
pub struct CopilotStreams {
    /// What the CLI writes.
    pub output: Box<dyn Read + Send>,
    /// What the CLI reads.
    pub input: Box<dyn Write + Send>,
}

/// The answer to a request: its `result`, or the `message` of its `error`.
type Reply = Result<Value, String>;

/// Where what the CLI writes goes: the requests waiting for their answer by id, and the events of each session.
#[derive(Default)]
struct Routes {
    /// The requests sent and not answered yet.
    waiting: HashMap<u64, Sender<Reply>>,
    /// The sessions open, by their id.
    sessions: HashMap<String, Sender<Value>>,
}

/// The process of the CLI and the processes it started, which end with it.
pub(super) struct Process {
    /// The CLI.
    pub(super) child: Child,
    /// The CLI and what it starts: on Windows `copilot.cmd` starts the program that answers.
    pub(super) tree: Option<system::ProcessTree>,
}

/// The CLI running: what is written to it, and the thread that reads what it writes.
pub(super) struct Connection {
    /// Where requests and answers are written; none once it is closing.
    input: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
    /// Who waits for what the CLI writes.
    routes: Arc<Mutex<Routes>>,
    /// The id of the next request.
    next: AtomicU64,
    /// The thread that reads what the CLI writes; it ends when the CLI's output ends.
    reader: Option<JoinHandle<()>>,
    /// The process, when it is one.
    process: Option<Process>,
}

impl Connection {
    /// Speaks to a CLI over its streams, with the process they are of when they are a process's.
    pub(super) fn over(streams: CopilotStreams, process: Option<Process>) -> Result<Connection, CopilotError> {
        let input = Arc::new(Mutex::new(Some(streams.input)));
        let routes = Arc::new(Mutex::new(Routes::default()));
        let (reading_input, reading_routes) = (Arc::clone(&input), Arc::clone(&routes));
        let output = streams.output;
        let reader = thread::Builder::new()
            .name(String::from("copilot"))
            .spawn(move || read_all(BufReader::new(output), &reading_input, &reading_routes))
            .map_err(|error| CopilotError::Start(error.to_string()))?;
        Ok(Connection {
            input,
            routes,
            next: AtomicU64::new(1),
            reader: Some(reader),
            process,
        })
    }

    /// Whether the CLI still reads and writes: its output has not ended.
    pub(super) fn is_open(&self) -> bool {
        self.reader.as_ref().is_some_and(|reader| !reader.is_finished())
    }

    /// Sends a request and waits for its answer, or until it is cancelled.
    pub(super) fn request(&self, method: &str, params: Value, cancel: &Cancel) -> Result<Value, CopilotError> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (sender, answer) = mpsc::channel();
        let _before = lock(&self.routes).waiting.insert(id, sender);
        let message = Value::Object(vec![
            member("jsonrpc", Value::String(String::from("2.0"))),
            member("id", Value::from(id)),
            member("method", Value::String(method.to_owned())),
            member("params", params),
        ]);
        if write(&self.input, &message).is_err() {
            let _gone = lock(&self.routes).waiting.remove(&id);
            return Err(CopilotError::Ended);
        }
        let reply = wait(&answer, cancel);
        let _gone = lock(&self.routes).waiting.remove(&id);
        match reply? {
            Ok(result) => Ok(result),
            Err(message) => Err(CopilotError::Refused(message)),
        }
    }

    /// The events of a session, from now until it is forgotten.
    pub(super) fn follow(&self, session: &str) -> Receiver<Value> {
        let (sender, events) = mpsc::channel();
        let _before = lock(&self.routes).sessions.insert(session.to_owned(), sender);
        events
    }

    /// Stops passing on the events of a session.
    pub(super) fn forget(&self, session: &str) {
        let _gone = lock(&self.routes).sessions.remove(session);
    }
}

impl Drop for Connection {
    /// Closes the CLI's input, ends it and what it started, and waits for the thread that read it.
    fn drop(&mut self) {
        drop(lock(&self.input).take());
        if let Some(mut process) = self.process.take() {
            // A CLI that ended already, or that the system will not end, has nothing more to end.
            if let Some(tree) = &process.tree {
                let _ended = tree.end();
            }
            let _killed = process.child.kill();
            let _waited = process.child.wait();
        }
        if let Some(reader) = self.reader.take() {
            let _joined = reader.join();
        }
    }
}

/// Waits for the answer to a request, looking every [`CANCEL_CHECK`] whether it was cancelled.
pub(super) fn wait<T>(answer: &Receiver<T>, cancel: &Cancel) -> Result<T, CopilotError> {
    loop {
        if cancel.is_cancelled() {
            return Err(CopilotError::Cancelled);
        }
        match answer.recv_timeout(CANCEL_CHECK) {
            Ok(value) => return Ok(value),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Err(CopilotError::Ended),
        }
    }
}

/// A member of an object.
pub(super) fn member(name: &str, value: Value) -> Member {
    Member {
        name: name.to_owned(),
        value,
    }
}

/// A lock, also when a thread holding it panicked.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Writes a message, framed.
fn write(input: &Mutex<Option<Box<dyn Write + Send>>>, message: &Value) -> io::Result<()> {
    let mut input = lock(input);
    let Some(input) = input.as_mut() else {
        return Err(io::Error::from(io::ErrorKind::BrokenPipe));
    };
    write_message(input, &json::to_compact_text(message))
}

/// A message framed: its `Content-Length` header, an empty line, then its body.
///
/// # Errors
///
/// When it cannot be written.
fn write_message(output: &mut dyn Write, body: &str) -> io::Result<()> {
    write!(output, "Content-Length: {}\r\n\r\n{body}", body.len())?;
    output.flush()
}

/// The body of the next message framed, or none once the stream ended. Headers other than `Content-Length` are
/// skipped.
///
/// # Errors
///
/// When it cannot be read, or its header is not a length.
pub(super) fn read_message(input: &mut dyn BufRead) -> io::Result<Option<String>> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            if length.is_some() {
                break;
            }
            continue;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.trim().eq_ignore_ascii_case("content-length")
        {
            length = Some(value.trim().parse::<usize>().map_err(io::Error::other)?);
        }
    }
    let mut body = vec![0; length.unwrap_or(0)];
    input.read_exact(&mut body)?;
    String::from_utf8(body).map(Some).map_err(io::Error::other)
}

/// Reads what the CLI writes until its output ends, passing each message where it goes; then the requests still
/// waiting are told it ended, by their senders dropped.
fn read_all(
    mut output: BufReader<Box<dyn Read + Send>>,
    input: &Mutex<Option<Box<dyn Write + Send>>>,
    routes: &Mutex<Routes>,
) {
    // A stream that cannot be read is a CLI that ended, as one whose output ended.
    while let Ok(Some(body)) = read_message(&mut output) {
        // A message that is not JSON is not one of the protocol, and nothing waits for it.
        let Ok(message) = json::parse(&body) else {
            continue;
        };
        take(&message, input, routes);
    }
    let mut routes = lock(routes);
    routes.waiting.clear();
    routes.sessions.clear();
}

/// Passes on one message: the answer to a request, an event of a session, or a request of the CLI, refused.
fn take(message: &Value, input: &Mutex<Option<Box<dyn Write + Send>>>, routes: &Mutex<Routes>) {
    let id = message.get("id");
    match (message.get("method").and_then(Value::as_str), id) {
        (Some(method), Some(id)) => {
            // A CLI that ended before it read the refusal needs none.
            let _written = write(input, &refusal(method, id.clone()));
        }
        (Some("session.event"), None) => {
            let params = message.get("params");
            let session = params
                .and_then(|params| params.get("sessionId"))
                .and_then(Value::as_str);
            let event = params.and_then(|params| params.get("event"));
            if let (Some(session), Some(event)) = (session, event)
                && let Some(sender) = lock(routes).sessions.get(session)
            {
                let _sent = sender.send(event.clone());
            }
        }
        (Some(_), None) => {}
        (None, id) => {
            let Some(id) = id.and_then(Value::as_number).and_then(json::Number::as_u64) else {
                return;
            };
            let reply = match message.get("error") {
                Some(error) => Err(error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("no reason given")
                    .to_owned()),
                None => Ok(message.get("result").cloned().unwrap_or(Value::Null)),
            };
            if let Some(sender) = lock(routes).waiting.remove(&id) {
                let _sent = sender.send(reply);
            }
        }
    }
}

/// The answer to a request of the CLI: a permission it asks is rejected, a tool it calls is said to be refused, and
/// any other request is not one Gitardo answers (`ai.copilot`, **Tools**).
fn refusal(method: &str, id: Value) -> Value {
    let outcome = match method {
        "permission.request" => Ok(Value::Object(vec![member(
            "result",
            Value::Object(vec![member("kind", Value::String(String::from("reject")))]),
        )])),
        "tool.call" => Ok(Value::Object(vec![member(
            "result",
            Value::Object(vec![
                member("resultType", Value::String(String::from("rejected"))),
                member(
                    "textResultForLlm",
                    Value::String(String::from("Gitardo runs no tools: answer with text alone.")),
                ),
            ]),
        )])),
        _ => Err(Value::Object(vec![
            member("code", Value::from(-32_601_i64)),
            member("message", Value::String(format!("Gitardo does not answer {method}"))),
        ])),
    };
    let mut reply = vec![member("jsonrpc", Value::String(String::from("2.0"))), member("id", id)];
    match outcome {
        Ok(result) => reply.push(member("result", result)),
        Err(error) => reply.push(member("error", error)),
    }
    Value::Object(reply)
}
