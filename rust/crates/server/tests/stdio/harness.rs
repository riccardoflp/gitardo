//! `gitardo-engine --stdio` run as the extension runs it: messages framed on its standard input, answers and
//! notifications read from its standard output, against throwaway repositories. These are the acceptance lines of
//! stage 1 of `docs/transition/06-roadmap.md` that the engine alone can show.

#![cfg(test)]

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use testkit::Repository;

/// How long a test waits for an answer before it fails.
const PATIENCE: Duration = Duration::from_secs(20);

/// The engine, started for one test.
struct Engine {
    child: Child,
    input: Option<ChildStdin>,
    messages: Receiver<Value>,
    /// The messages read while waiting for another, in the order they came.
    kept: Vec<Value>,
    next_id: i64,
}

impl Engine {
    /// Starts `gitardo-engine --stdio`, and a thread that reads its messages.
    fn start() -> Engine {
        let mut child = Command::new(env!("CARGO_BIN_EXE_gitardo-engine"))
            .arg("--stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
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
    fn write(&mut self, message: &Value) {
        let body = serde_json::to_vec(message).unwrap();
        let input = self.input.as_mut().unwrap();
        write!(input, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
        input.write_all(&body).unwrap();
        input.flush().unwrap();
    }

    /// Sends a request, and gives its id.
    fn send(&mut self, method: &str, params: &Value) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        self.write(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        id
    }

    /// Sends a notification.
    fn notify(&mut self, method: &str, params: &Value) {
        self.write(&json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }

    /// The answer to a request, the messages read before it kept.
    fn answer(&mut self, id: i64) -> Value {
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

    /// Sends a request and gives its answer.
    fn ask(&mut self, method: &str, params: &Value) -> Value {
        let id = self.send(method, params);
        self.answer(id)
    }

    /// The notifications of a method that come within a time, those kept before included.
    fn notifications(&mut self, method: &str, within: Duration) -> Vec<Value> {
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

    /// Sends `initialize` with version 1.0 and gives its answer.
    fn initialize(&mut self) -> Value {
        self.ask(
            "initialize",
            &json!({
                "protocol": "1.0",
                "client": { "name": "gitardo-vscode", "version": "0.0.0" },
                "settings": {},
                "host": {},
                "logLevel": "info"
            }),
        )
    }

    /// Closes the standard input and waits for the process to end.
    fn close_input(mut self) -> ExitStatus {
        drop(self.input.take());
        self.child.wait().unwrap()
    }

    /// Sends `exit` and waits for the process to end.
    fn exit(mut self) -> ExitStatus {
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
fn repository() -> Repository {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a\n")]);
    repository.branch("feature");
    let _second = repository.commit("Change a", &[("a.txt", "a, again\n")]);
    repository.tag("v1.0");
    repository
}

/// The folder of a repository, as the extension names it.
fn folder(repository: &Repository) -> String {
    repository.path().to_string_lossy().into_owned()
}

#[test]
fn initialize_answers_version_1_0_and_a_major_version_that_differs_with_its_error_and_the_engines_version() {
    let mut engine = Engine::start();
    let answer = engine.initialize();
    assert_eq!(answer["result"]["protocol"], json!("1.0"), "{answer}");
    assert_eq!(answer["result"]["engine"]["version"], json!(env!("CARGO_PKG_VERSION")));
    assert!(
        answer["result"]["engine"]["git"]
            .as_str()
            .is_some_and(|said| said.starts_with("git version"))
    );

    let refused = engine.ask(
        "initialize",
        &json!({ "protocol": "2.0", "client": { "name": "gitardo-vscode", "version": "9.0.0" } }),
    );
    assert_eq!(refused["error"]["code"], json!(-32_602), "{refused}");
    assert_eq!(refused["error"]["data"]["protocol"], json!("1.0"));
    assert_eq!(
        refused["error"]["data"]["engine"]["version"],
        json!(env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn an_unknown_method_answers_32601_and_the_engine_keeps_running() {
    let mut engine = Engine::start();
    let _ = engine.initialize();
    let unknown = engine.ask("graph/teleport", &json!({}));
    assert_eq!(unknown["error"]["code"], json!(-32_601), "{unknown}");
    let not_json = {
        engine
            .input
            .as_mut()
            .unwrap()
            .write_all(b"Content-Length: 5\r\n\r\n{ no ")
            .unwrap();
        let _none = engine.notifications("never", Duration::from_millis(300));
        engine.kept.pop().unwrap()
    };
    assert_eq!(not_json["error"]["code"], json!(-32_700), "{not_json}");
    assert_eq!(engine.ask("shutdown", &Value::Null)["result"], Value::Null);
    assert!(engine.exit().success(), "exit after shutdown ends with 0");
}

#[test]
fn the_input_closed_without_shutdown_ends_the_engine_with_1() {
    let mut engine = Engine::start();
    let _ = engine.initialize();
    assert_eq!(engine.close_input().code(), Some(1));
}

#[test]
fn open_answers_the_state_of_the_repository_and_read_state_the_same_until_it_is_closed() {
    let repository = repository();
    let mut engine = Engine::start();
    let _ = engine.initialize();
    let opened = engine.ask("repository/open", &json!({ "repository": folder(&repository) }));
    let state = &opened["result"]["state"];
    assert_eq!(state["head"]["branch"], json!("refs/heads/main"), "{opened}");
    let branches: Vec<&str> = state["branches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|branch| branch["name"].as_str().unwrap())
        .collect();
    assert_eq!(branches, ["feature", "main"]);
    assert_eq!(state["tags"][0]["name"], json!("v1.0"));
    assert_eq!(state["workingTree"]["untracked"], json!([]));

    let read = engine.ask("read/repo.read-state", &json!({ "repository": folder(&repository) }));
    assert_eq!(&read["result"], state);

    assert_eq!(
        engine.ask("repository/close", &json!({ "repository": folder(&repository) }))["result"],
        Value::Null
    );
    let closed = engine.ask("read/repo.read-state", &json!({ "repository": folder(&repository) }));
    assert_eq!(closed["error"]["code"], json!(1), "{closed}");
}

#[test]
fn ten_changes_in_a_burst_send_one_state() {
    let repository = repository();
    let mut engine = Engine::start();
    let _ = engine.initialize();
    let _ = engine.ask("repository/open", &json!({ "repository": folder(&repository) }));
    for round in 0..10 {
        std::fs::write(repository.path().join("b.txt"), format!("{round}\n")).unwrap();
        engine.notify(
            "repository/changed",
            &json!({ "repository": folder(&repository), "paths": ["b.txt"] }),
        );
        thread::sleep(Duration::from_millis(20));
    }
    let states = engine.notifications("repository/state", Duration::from_secs(4));
    assert_eq!(states.len(), 1, "{states:?}");
    let state = &states[0]["params"];
    assert_eq!(state["reason"], json!("changed"));
    assert_eq!(state["state"]["workingTree"]["untracked"], json!(["b.txt"]));
}

#[test]
fn cancel_of_a_read_that_runs_answers_cancelled() {
    let repository = repository();
    // git asks this hook before `git status` lists the changes, and the hook takes ten seconds.
    let _ = repository.git(&["config", "core.fsmonitor", "sleep 10; true"]);
    let mut engine = Engine::start();
    let _ = engine.initialize();
    let id = engine.send("repository/open", &json!({ "repository": folder(&repository) }));
    thread::sleep(Duration::from_millis(500));
    let started = Instant::now();
    engine.notify("$/cancel", &json!({ "id": id }));
    let answer = engine.answer(id);
    assert_eq!(answer["error"]["code"], json!(7), "{answer}");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the read stops without waiting for the hook"
    );
}
