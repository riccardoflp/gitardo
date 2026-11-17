//! `gitardo-engine --stdio` started for the benchmark, spoken to as the extension speaks to it: each request framed
//! with `Content-Length`, as `crates/server/src/framing.rs` reads it, and its answer read to the last byte.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use crate::error::Error;

/// The engine, started for one run of the benchmark.
pub struct Engine {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    next_id: u64,
}

/// An answer, and how long it took from the request to its last byte.
pub struct Answered {
    /// The JSON of the whole answer, `result` or `error`.
    pub body: String,
    /// From the first byte of the request written to the last byte of the answer read.
    pub took: Duration,
}

/// A read of the commits asked with `stream`: when its first page came, how many pages and rows came, and its answer.
pub struct Streamed {
    /// From the first byte of the request written to the last byte of its first page; none when no page came.
    pub first_page: Option<Duration>,
    /// How many pages came before the answer.
    pub pages: usize,
    /// How many rows those pages had.
    pub rows: usize,
    /// The answer, once the read ended, and how long it took.
    pub answered: Answered,
}

impl Answered {
    /// Whether the engine answered with an error.
    pub fn is_error(&self) -> bool {
        self.body.starts_with("{\"error\":")
    }
}

impl Engine {
    /// Starts the engine at `binary` with `--stdio`; what it writes on its standard error is left out.
    pub fn start(binary: &Path) -> Result<Engine, Error> {
        Engine::start_with(binary, None)
    }

    /// Starts the engine as [`Engine::start`] does, with every git it starts writing its events of trace2 to a file,
    /// so that the git processes of each read can be counted.
    pub fn start_traced(binary: &Path, trace: &Path) -> Result<Engine, Error> {
        Engine::start_with(binary, Some(trace))
    }

    fn start_with(binary: &Path, trace: Option<&Path>) -> Result<Engine, Error> {
        let mut command = Command::new(binary);
        if let Some(trace) = trace {
            let _ = command.env("GIT_TRACE2_EVENT", trace);
        }
        let mut child = command
            .arg("--stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|source| Error::Tool {
                command: format!("{} --stdio", binary.display()),
                source,
            })?;
        let (Some(input), Some(output)) = (child.stdin.take(), child.stdout.take()) else {
            return Err(Error::Output(io::Error::other(
                "the engine has no standard input or output",
            )));
        };
        Ok(Engine {
            child,
            input,
            output: BufReader::new(output),
            next_id: 1,
        })
    }

    /// Sends a request with its parameters, already JSON, and waits for its answer; the notifications that come
    /// before it are passed over.
    pub fn ask(&mut self, method: &str, params: &str) -> Result<Answered, Error> {
        self.ask_watching(method, params, &mut |_| {})
    }

    /// Sends a read of the commits asked with `stream` (version 1.8 of the protocol), as the extension asks it, and
    /// waits for its answer: the pages that come before it in notifications `read/graph.rows` of this read are timed
    /// and counted; other notifications are passed over.
    pub fn ask_streamed(&mut self, params: &str) -> Result<Streamed, Error> {
        let id = self.next_id;
        let rows = format!("{{\"jsonrpc\":\"2.0\",\"method\":\"read/graph.rows\",\"params\":{{\"read\":{id},");
        let started = Instant::now();
        let mut first_page = None;
        let mut pages = 0;
        let mut count = 0;
        let answered = self.ask_watching("read/graph.read-commits", params, &mut |message| {
            if message.starts_with(&rows) {
                let _first = first_page.get_or_insert_with(|| started.elapsed());
                pages += 1;
                count += message.matches("\"kind\":\"commit\"").count() + message.matches("\"kind\":\"stash\"").count();
            }
        })?;
        Ok(Streamed {
            first_page,
            pages,
            rows: count,
            answered,
        })
    }

    /// Sends a request and waits for its answer, as [`Engine::ask`] does, with each message that comes before it
    /// given to `before`.
    fn ask_watching(&mut self, method: &str, params: &str, before: &mut dyn FnMut(&str)) -> Result<Answered, Error> {
        let id = self.next_id;
        self.next_id += 1;
        let body = format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"{method}\",\"params\":{params}}}");
        let started = Instant::now();
        write!(self.input, "Content-Length: {}\r\n\r\n{body}", body.len())?;
        self.input.flush()?;
        // serde_json writes the fields of a message in the order of their names: an answer starts with its id, or
        // with its error and ends with its id.
        let result = format!("{{\"id\":{id},\"jsonrpc\":\"2.0\",\"result\"");
        let failure = format!(",\"id\":{id},\"jsonrpc\":\"2.0\"}}");
        loop {
            let message = self.read_message()?;
            if message.starts_with(&result) || (message.starts_with("{\"error\":") && message.ends_with(&failure)) {
                return Ok(Answered {
                    body: message,
                    took: started.elapsed(),
                });
            }
            before(&message);
        }
    }

    /// Sends a notification with its parameters, already JSON.
    pub fn notify(&mut self, method: &str, params: &str) -> Result<(), Error> {
        let body = format!("{{\"jsonrpc\":\"2.0\",\"method\":\"{method}\",\"params\":{params}}}");
        write!(self.input, "Content-Length: {}\r\n\r\n{body}", body.len())?;
        self.input.flush()?;
        Ok(())
    }

    /// One framed message from the engine, read whole.
    fn read_message(&mut self) -> Result<String, Error> {
        let mut length = None;
        loop {
            let mut line = String::new();
            if self.output.read_line(&mut line)? == 0 {
                return Err(Error::Output(io::Error::other("the engine ended before it answered")));
            }
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some(value) = line.strip_prefix("Content-Length:") {
                length = value.trim().parse::<usize>().ok();
            }
        }
        let length = length.ok_or_else(|| Error::Output(io::Error::other("a message of the engine has no length")))?;
        let mut body = vec![0; length];
        self.output.read_exact(&mut body)?;
        String::from_utf8(body).map_err(|_| Error::Output(io::Error::other("a message of the engine is not UTF-8")))
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // The benchmark is over: what the engine still had to say is not wanted.
        let _ = self.notify("exit", "null");
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
