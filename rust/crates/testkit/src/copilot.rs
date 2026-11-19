//! A fake GitHub Copilot CLI (`ai.copilot`), so that the tests need no account: it answers the parts of the CLI's
//! protocol Gitardo uses, in JSON-RPC framed with `Content-Length` headers, over pipes in place of a process's standard
//! input and output, on a thread of its own for each time it is started.
//!
//! It reads what it is sent by looking for the members it needs in the text, which is enough for the compact JSON
//! Gitardo writes, and records every request it was sent, by its method.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};

/// What the fake answers.
#[derive(Clone, Debug)]
pub struct CopilotScript {
    /// The models of the subscription: their ids and their names.
    pub models: Vec<(String, String)>,
    /// Whether it is signed in.
    pub is_signed_in: bool,
    /// The pieces of each answer, streamed one event each.
    pub pieces: Vec<String>,
    /// Whether it asks to read a file before it answers.
    pub asks_permission: bool,
    /// What it refuses each question with, in its own words.
    pub refusal: Option<String>,
    /// Whether it writes the first piece only, then waits until the session is aborted.
    pub holds_answer: bool,
}

impl Default for CopilotScript {
    /// Signed in, with two models, answering *Fixed* in two pieces.
    fn default() -> Self {
        CopilotScript {
            models: vec![
                (String::from("gpt-test"), String::from("GPT Test")),
                (String::from("claude-test"), String::from("Claude Test")),
            ],
            is_signed_in: true,
            pieces: vec![String::from("Fi"), String::from("xed")],
            asks_permission: false,
            refusal: None,
            holds_answer: false,
        }
    }
}

/// What the fake was sent and did, shared with the threads that answer.
#[derive(Debug, Default)]
struct Seen {
    /// The script, which a test may change between questions.
    script: CopilotScript,
    /// Every request, as `<method> <body>`, in the order received.
    requests: Vec<String>,
    /// The bodies of the answers to the permissions it asked.
    permission_answers: Vec<String>,
    /// How many times it was started.
    starts: usize,
}

/// A fake CLI, started as many times as Gitardo starts it.
#[derive(Clone, Debug)]
pub struct FakeCopilot {
    /// What it was sent and its script.
    seen: Arc<Mutex<Seen>>,
    /// The thread of the last start.
    running: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl FakeCopilot {
    /// A fake that answers as the script says.
    #[must_use]
    pub fn new(script: CopilotScript) -> FakeCopilot {
        FakeCopilot {
            seen: Arc::new(Mutex::new(Seen {
                script,
                ..Seen::default()
            })),
            running: Arc::new(Mutex::new(None)),
        }
    }

    /// Starts it: what it writes, to read, and where to write what it reads.
    ///
    /// # Panics
    ///
    /// When the system gives no pipe.
    #[must_use]
    pub fn start(&self) -> (Box<dyn Read + Send>, Box<dyn Write + Send>) {
        let (from_cli, cli_output) = io::pipe().expect("a pipe for what the fake writes");
        let (cli_input, to_cli) = io::pipe().expect("a pipe for what the fake reads");
        lock(&self.seen).starts += 1;
        let seen = Arc::clone(&self.seen);
        let thread = thread::spawn(move || serve(BufReader::new(cli_input), cli_output, &seen));
        *lock(&self.running) = Some(thread);
        (Box::new(from_cli), Box::new(to_cli))
    }

    /// Answers the questions after as another script says.
    pub fn answer_with(&self, script: CopilotScript) {
        lock(&self.seen).script = script;
    }

    /// Signs it in, or out, for the questions after.
    pub fn sign_in(&self, is_signed_in: bool) {
        lock(&self.seen).script.is_signed_in = is_signed_in;
    }

    /// Every request it was sent, as `<method> <body>`.
    #[must_use]
    pub fn requests(&self) -> Vec<String> {
        lock(&self.seen).requests.clone()
    }

    /// The methods of the requests it was sent, in order.
    #[must_use]
    pub fn methods(&self) -> Vec<String> {
        self.requests()
            .iter()
            .map(|request| request.split(' ').next().unwrap_or_default().to_owned())
            .collect()
    }

    /// The answers Gitardo gave to the permissions it asked.
    #[must_use]
    pub fn permission_answers(&self) -> Vec<String> {
        lock(&self.seen).permission_answers.clone()
    }

    /// How many times it was started.
    #[must_use]
    pub fn starts(&self) -> usize {
        lock(&self.seen).starts
    }

    /// Whether the last one started still runs: its input has not been closed.
    #[must_use]
    pub fn is_running(&self) -> bool {
        lock(&self.running).as_ref().is_some_and(|thread| !thread.is_finished())
    }
}

/// A lock, also when a thread holding it panicked.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The session a permission was asked for, whose answer waits for Gitardo's.
const PERMISSION_ID: u64 = 9_000;

/// Answers what it reads until its input ends.
fn serve(mut input: BufReader<io::PipeReader>, mut output: io::PipeWriter, seen: &Mutex<Seen>) {
    let mut sessions = 0;
    let mut waiting_permission: Option<String> = None;
    while let Some(body) = read(&mut input) {
        let Some(method) = string_member(&body, "method") else {
            if waiting_permission.is_some() && number_member(&body, "id") == Some(PERMISSION_ID) {
                lock(seen).permission_answers.push(body.clone());
                let session = waiting_permission.take().unwrap_or_default();
                let script = lock(seen).script.clone();
                stream_answer(&mut output, &session, &script);
            }
            continue;
        };
        lock(seen).requests.push(format!("{method} {body}"));
        let id = number_member(&body, "id").unwrap_or_default();
        let script = lock(seen).script.clone();
        let session = string_member(&body, "sessionId").unwrap_or_default();
        let result = match method.as_str() {
            "auth.getStatus" => Ok(format!("{{\"isAuthenticated\":{}}}", script.is_signed_in)),
            "models.list" => Ok(models(&script)),
            "session.create" => {
                sessions += 1;
                Ok(format!("{{\"sessionId\":\"session-{sessions}\"}}"))
            }
            "session.send" | "session.abort" | "session.destroy" => Ok(String::from("{}")),
            other => Err(format!("{other} is not a method of the fake")),
        };
        answer(&mut output, id, result);
        match method.as_str() {
            "session.send" if script.asks_permission => {
                let request = format!(
                    "{{\"jsonrpc\":\"2.0\",\"id\":{PERMISSION_ID},\"method\":\"permission.request\",\"params\":\
                     {{\"sessionId\":\"{session}\",\"permissionRequest\":{{\"kind\":\"read\",\"path\":\"secret.txt\"}}}}}}"
                );
                write(&mut output, &request);
                waiting_permission = Some(session);
            }
            "session.send" => stream_answer(&mut output, &session, &script),
            "session.abort" => event(&mut output, &session, "session.idle", "{}"),
            _ => {}
        }
    }
}

/// The answer of a session: its pieces then its whole message and idle; its refusal; or its first piece alone.
fn stream_answer(output: &mut io::PipeWriter, session: &str, script: &CopilotScript) {
    if let Some(refusal) = &script.refusal {
        event(
            output,
            session,
            "session.error",
            &format!("{{\"message\":{}}}", quoted(refusal)),
        );
        return;
    }
    for piece in &script.pieces {
        event(
            output,
            session,
            "assistant.message_delta",
            &format!("{{\"deltaContent\":{}}}", quoted(piece)),
        );
        if script.holds_answer {
            return;
        }
    }
    let whole = script.pieces.concat();
    event(
        output,
        session,
        "assistant.message",
        &format!("{{\"content\":{}}}", quoted(&whole)),
    );
    event(output, session, "session.idle", "{}");
}

/// The models of the script, as `models.list` answers them.
fn models(script: &CopilotScript) -> String {
    let listed: Vec<String> = script
        .models
        .iter()
        .map(|(id, name)| format!("{{\"id\":{},\"name\":{}}}", quoted(id), quoted(name)))
        .collect();
    format!("{{\"models\":[{}]}}", listed.join(","))
}

/// Writes the answer to a request: its result, or an error.
fn answer(output: &mut io::PipeWriter, id: u64, result: Result<String, String>) {
    let member = match result {
        Ok(result) => format!("\"result\":{result}"),
        Err(message) => format!("\"error\":{{\"code\":-32601,\"message\":{}}}", quoted(&message)),
    };
    write(output, &format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},{member}}}"));
}

/// Writes an event of a session.
fn event(output: &mut io::PipeWriter, session: &str, kind: &str, data: &str) {
    let params = format!(
        "{{\"sessionId\":{},\"event\":{{\"type\":\"{kind}\",\"data\":{data}}}}}",
        quoted(session)
    );
    write(
        output,
        &format!("{{\"jsonrpc\":\"2.0\",\"method\":\"session.event\",\"params\":{params}}}"),
    );
}

/// Writes a message framed. Gitardo having closed its side is the end of the test, not a failure of the fake.
fn write(output: &mut io::PipeWriter, body: &str) {
    let _written = write!(output, "Content-Length: {}\r\n\r\n{body}", body.len()).and_then(|()| output.flush());
}

/// The body of the next message, or none once the input ended.
fn read(input: &mut BufReader<io::PipeReader>) -> Option<String> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() && length.is_some() {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            length = value.trim().parse::<usize>().ok();
        }
    }
    let mut body = vec![0; length?];
    input.read_exact(&mut body).ok()?;
    String::from_utf8(body).ok()
}

/// The string of a member, the first one of that name in the text.
fn string_member(body: &str, name: &str) -> Option<String> {
    let after = body.split_once(&format!("\"{name}\":"))?.1.trim_start();
    let rest = after.strip_prefix('"')?;
    let mut value = String::new();
    let mut characters = rest.chars();
    while let Some(character) = characters.next() {
        match character {
            '"' => return Some(value),
            '\\' => match characters.next()? {
                'n' => value.push('\n'),
                other => value.push(other),
            },
            other => value.push(other),
        }
    }
    None
}

/// The number of a member, the first one of that name in the text.
fn number_member(body: &str, name: &str) -> Option<u64> {
    let after = body.split_once(&format!("\"{name}\":"))?.1.trim_start();
    let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// A string as JSON writes it.
fn quoted(text: &str) -> String {
    let mut quoted = String::from("\"");
    for character in text.chars() {
        match character {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}
