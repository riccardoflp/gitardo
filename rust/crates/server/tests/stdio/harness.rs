//! The engine started as the extension starts it, and the repositories the tests run it on.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::thread;
use std::time::{Duration, Instant};

use protocol::messages::VERSION;
use serde_json::{Value, json};
use testkit::Repository;

/// How long a test waits for an answer before it fails.
pub const PATIENCE: Duration = Duration::from_secs(20);

/// The engine, started for one test.
pub struct Engine {
    child: Child,
    pub input: Option<ChildStdin>,
    messages: Receiver<Value>,
    /// The messages read while waiting for another, in the order they came.
    pub kept: Vec<Value>,
    next_id: i64,
}

impl Engine {
    /// Starts `gitardo-engine --stdio`, and a thread that reads its messages.
    pub fn start() -> Engine {
        Engine::start_with(None)
    }

    /// Starts `gitardo-engine --stdio` as [`Engine::start`] does, every git it runs writing its trace2 events to a file.
    pub fn start_traced(trace: &Path) -> Engine {
        Engine::start_with(Some(trace))
    }

    /// Starts `gitardo-engine --stdio`, with the file of git's trace2 events when there is one.
    fn start_with(trace: Option<&Path>) -> Engine {
        let mut command = Command::new(env!("CARGO_BIN_EXE_gitardo-engine"));
        let _ = command
            .arg("--stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        if let Some(trace) = trace {
            let _ = command.env("GIT_TRACE2_EVENT", trace);
        }
        let mut child = command.spawn().unwrap();
        let input = child.stdin.take();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let (sender, messages) = channel();
        let _reading = thread::spawn(move || {
            while let Some(message) = read_frame(&mut output) {
                if sender.send(message).is_err() {
                    break;
                }
            }
        });
        Engine {
            child,
            input,
            messages,
            kept: Vec::new(),
            next_id: 1,
        }
    }

    /// Writes one message, framed.
    pub fn write(&mut self, message: &Value) {
        let body = serde_json::to_vec(message).unwrap();
        let input = self.input.as_mut().unwrap();
        write!(input, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
        input.write_all(&body).unwrap();
        input.flush().unwrap();
    }

    /// Sends a request, and gives its id.
    pub fn send(&mut self, method: &str, params: &Value) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        self.write(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        id
    }

    /// Sends a notification.
    pub fn notify(&mut self, method: &str, params: &Value) {
        self.write(&json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }

    /// The answer to a request, the messages read before it kept.
    pub fn answer(&mut self, id: i64) -> Value {
        if let Some(place) = self.kept.iter().position(|message| message["id"] == json!(id)) {
            return self.kept.remove(place);
        }
        let deadline = Instant::now() + PATIENCE;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let message = self.messages.recv_timeout(left).expect("the engine answers");
            if message["id"] == json!(id) && message.get("method").is_none() {
                return message;
            }
            self.kept.push(message);
        }
    }

    /// The answer to a request, each request of the engine read before it answered with what `respond` gives for it.
    pub fn answer_asked(&mut self, id: i64, mut respond: impl FnMut(&Value) -> Value) -> Value {
        let deadline = Instant::now() + PATIENCE;
        loop {
            if let Some(place) = self
                .kept
                .iter()
                .position(|message| message.get("method").is_some() && message.get("id").is_some())
            {
                let request = self.kept.remove(place);
                let result = respond(&request);
                self.write(&json!({ "jsonrpc": "2.0", "id": request["id"], "result": result }));
                continue;
            }
            if let Some(place) = self
                .kept
                .iter()
                .position(|message| message["id"] == json!(id) && message.get("method").is_none())
            {
                return self.kept.remove(place);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            let message = self.messages.recv_timeout(left).expect("the engine answers");
            self.kept.push(message);
        }
    }

    /// The first request of the engine of a method, waited for; the messages read before it kept.
    pub fn request_of(&mut self, method: &str) -> Value {
        let deadline = Instant::now() + PATIENCE;
        loop {
            if let Some(place) = self
                .kept
                .iter()
                .position(|message| message["method"] == json!(method) && message.get("id").is_some())
            {
                return self.kept.remove(place);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            let message = self.messages.recv_timeout(left).expect("the engine asks");
            self.kept.push(message);
        }
    }

    /// The first notification of the engine of a method, waited for; the messages read before it kept.
    pub fn notification_of(&mut self, method: &str) -> Value {
        let deadline = Instant::now() + PATIENCE;
        loop {
            if let Some(place) = self
                .kept
                .iter()
                .position(|message| message["method"] == json!(method) && message.get("id").is_none())
            {
                return self.kept.remove(place);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            let message = self.messages.recv_timeout(left).expect("the engine says it");
            self.kept.push(message);
        }
    }

    /// Sends a request and gives its answer.
    pub fn ask(&mut self, method: &str, params: &Value) -> Value {
        let id = self.send(method, params);
        self.answer(id)
    }

    /// The notifications of a method that come within a time, those kept before included.
    pub fn notifications(&mut self, method: &str, within: Duration) -> Vec<Value> {
        let deadline = Instant::now() + within;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.messages.recv_timeout(left) {
                Ok(message) => self.kept.push(message),
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
            }
        }
        let (found, others): (Vec<Value>, Vec<Value>) = self
            .kept
            .drain(..)
            .partition(|message| message["method"] == json!(method));
        self.kept = others;
        found
    }

    /// Sends `initialize` with the version of the protocol and gives its answer.
    pub fn initialize(&mut self) -> Value {
        self.ask(
            "initialize",
            &json!({
                "protocol": VERSION,
                "client": { "name": "gitardo-vscode", "version": "0.0.0" },
                "settings": {},
                "host": {},
                "logLevel": "info"
            }),
        )
    }

    /// Closes the standard input and waits for the process to end.
    pub fn close_input(mut self) -> ExitStatus {
        drop(self.input.take());
        self.child.wait().unwrap()
    }

    /// Sends `exit` and waits for the process to end.
    pub fn exit(mut self) -> ExitStatus {
        self.notify("exit", &Value::Null);
        self.child.wait().unwrap()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// One message from the engine's output; none when it ends.
fn read_frame(output: &mut impl BufRead) -> Option<Value> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if output.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            length = value.trim().parse::<usize>().ok();
        }
    }
    let mut body = vec![0; length?];
    output.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

/// A repository with two commits on `main`, a branch `feature` and a tag `v1.0`.
pub fn repository() -> Repository {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a\n")]);
    repository.branch("feature");
    let _second = repository.commit("Change a", &[("a.txt", "a, again\n")]);
    repository.tag("v1.0");
    repository
}

/// The git commands a traced engine started, from the trace2 events git wrote: the first word of each, such as
/// `status`, and not those git ran itself.
pub fn traced_commands(trace: &Path) -> Vec<String> {
    commands_started(trace, 0)
}

/// The git commands a traced engine started, from a byte of the file of trace2 events on: not those git ran itself.
pub fn commands_started(trace: &Path, from: usize) -> Vec<String> {
    let text = std::fs::read_to_string(trace).unwrap_or_default();
    text.get(from..)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let event: Value = serde_json::from_str(line).ok()?;
            let is_started = event["event"] == "start" && !event["sid"].as_str()?.contains('/');
            is_started.then(|| command_of(event["argv"].as_array()?))?
        })
        .collect()
}

/// The command of git's arguments: the first one that is not an option of git itself, as `-c name=value`.
fn command_of(argv: &[Value]) -> Option<String> {
    let mut arguments = argv.iter().skip(1).filter_map(Value::as_str);
    while let Some(argument) = arguments.next() {
        match argument {
            "-c" | "-C" => {
                let _value = arguments.next();
            }
            option if option.starts_with('-') => {}
            command => return Some(command.to_owned()),
        }
    }
    None
}

/// The folder of a repository, as the extension names it.
pub fn folder(repository: &Repository) -> String {
    repository.path().to_string_lossy().into_owned()
}
