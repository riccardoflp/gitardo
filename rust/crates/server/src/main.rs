//! `gitardo-engine`: Gitardo's engine without a window. `gitardo-engine --stdio` reads requests and notifications on
//! its standard input and writes answers and notifications on its standard output, framed as the Language Server
//! Protocol frames them; what it says for people goes to its standard error (`docs/transition/01-engine.md`).
//!
//! One thread reads the standard input, one writes the standard output, and each request runs on a thread of its own,
//! so that a long read never holds the others back.

#![forbid(unsafe_code)]

mod ai;
mod asker;
mod discover;
mod feature;
mod framing;
mod host;
mod journal;
mod mermaid;
mod outgoing;
mod range_diff;
mod reads;
mod repository;
mod session;
mod settings;

use std::io::{self, Write};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use protocol::messages::methods;
use protocol::rpc::{Incoming, error_message, read_message};
use serde_json::Value;

use crate::session::Session;

/// What the program says when it is started without `--stdio`.
const USAGE: &str = "Usage: gitardo-engine --stdio\n       gitardo-engine --version";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["--stdio"] => run(),
        ["--version"] => {
            let _said = writeln!(io::stdout(), "gitardo-engine {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            let _said = writeln!(io::stderr(), "{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Speaks the protocol on the standard input and output until `exit`, or until the input ends: the exit code is 0
/// when `shutdown` came first, 1 otherwise, as the Language Server Protocol has it.
fn run() -> ExitCode {
    let (sender, receiver) = mpsc::channel::<Value>();
    let _writer = thread::spawn(move || write_messages(&receiver));
    let session = Arc::new(Session::new(sender));
    let mut input = io::stdin().lock();
    loop {
        let bytes = match framing::read_frame(&mut input) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => break,
            Err(error) => {
                let _said = writeln!(io::stderr(), "gitardo-engine stops reading its input: {error}");
                break;
            }
        };
        match read_message(&bytes) {
            Ok(Incoming::Request { id, method, params }) => session.request(id, method, params),
            Ok(Incoming::Notification { method, params }) => {
                if method == methods::EXIT {
                    break;
                }
                session.notification(&method, params);
            }
            // The answer of a question of a feature, which its thread waits for.
            Ok(Incoming::Response { id, outcome }) => session.response(&id, outcome),
            Err(unreadable) => session.send(error_message(unreadable.id.as_ref(), &unreadable.error)),
        }
    }
    session.cancel_all();
    if session.is_shut_down() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Writes each message given, in the order given, until no one can give any more or the output is closed.
fn write_messages(receiver: &mpsc::Receiver<Value>) {
    let mut output = io::stdout().lock();
    for message in receiver {
        let Ok(bytes) = serde_json::to_vec(&message) else {
            continue;
        };
        if framing::write_frame(&mut output, &bytes).is_err() {
            break;
        }
    }
}
