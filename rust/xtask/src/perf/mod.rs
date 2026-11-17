//! `cargo xtask perf <folder>`: the reads of the graph timed in the engine the extension starts, on a large repository
//! such as the one `extension/test/perf/big-repo.mjs` makes. Each read is timed from its request to the last byte of
//! its answer, as the extension waits for it: a first page of 1,000 rows, the next pages asked with `skip`, a page far
//! down the history, the position of a commit, two searches and the state. Then the whole history, page after page
//! as the extension reads it; the first page after the branches shown change, streamed as the extension asks it; a
//! read after a change as the extension asks it, with the git processes each read starts; and the whole history in the
//! window, read and laid out as `app` does, then after the branches shown change, by a test of `app` built in release.
//! It prints tables of times; it is not pass or fail (`docs/desktop/06-testing.md`).

mod after_change;
mod engine;
mod filters;
mod whole;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use engine::{Answered, Engine};

use crate::error::Error;

/// What `cargo xtask perf` with no folder says.
const USAGE: &str = "Usage: cargo xtask perf <folder of a repository> [times each read is asked]

Build the engine first: cargo build --release -p server
With --engine, the window is not timed, and the tests of app are not built in release.";

/// How many times each read is asked when the command does not say.
const TIMES: usize = 5;

/// The rows of a page, as `gitardo.graph.pageSize` has them by default.
const PAGE_SIZE: usize = 1_000;

/// The test of `app` that times the whole history in the window.
const WINDOW_TEST: &str = "speed::the_whole_history_of_the_perf_repository_read_and_laid_out_page_after_page";

/// The page read far down the history.
const FAR_PAGE: usize = 50;

/// One read of the table: its name, its method and its parameters besides `repository`.
struct Read {
    name: String,
    method: &'static str,
    params: String,
}

/// Runs the benchmark with the arguments after `perf`, and returns whether it could run.
pub fn run(root: &Path, arguments: &[String]) -> Result<bool, Error> {
    let engine_only = arguments.iter().any(|argument| argument == "--engine");
    let arguments: Vec<&String> = arguments.iter().filter(|argument| *argument != "--engine").collect();
    let Some(folder) = arguments.first() else {
        writeln!(std::io::stderr().lock(), "{USAGE}")?;
        return Ok(false);
    };
    let times = arguments
        .get(1)
        .and_then(|times| times.parse().ok())
        .unwrap_or(TIMES)
        .max(1);
    let Some(binary) = engine_binary(root)? else {
        return Ok(false);
    };
    let folder = std::path::absolute(folder).map_err(|source| Error::Read {
        path: PathBuf::from(folder),
        source,
    })?;
    let mut engine = Engine::start(&binary)?;
    let repository = json_string(&folder.to_string_lossy());
    let initialize = initialize_params(root)?;
    let started = engine.ask("initialize", &initialize)?;
    let opened = engine.ask("repository/open", &format!("{{\"repository\":{repository}}}"))?;
    if started.is_error() || opened.is_error() {
        writeln!(
            std::io::stderr().lock(),
            "The engine did not open {}: {}",
            folder.display(),
            opened.body
        )?;
        return Ok(false);
    }
    let mut out = std::io::stdout().lock();
    writeln!(
        out,
        "{}: each read asked {times} times, from the request to the last byte of the answer",
        folder.display()
    )?;
    writeln!(out, "Opened in {}\n", milliseconds(opened.took))?;
    writeln!(out, "| Read | Rows | Bytes | Fastest | Median | Slowest |")?;
    writeln!(out, "| --- | ---: | ---: | ---: | ---: | ---: |")?;
    let far = read_pages(&mut engine, &repository, times, &mut out)?;
    for read in other_reads(far.as_deref()) {
        let answers = ask_times(&mut engine, &repository, &read, times)?;
        row(&mut out, &read.name, &answers)?;
    }
    whole::run(&mut engine, &repository, times, &mut out)?;
    filters::run(&mut engine, &repository, times, &mut out)?;
    drop(engine);
    let place = after_change::Place {
        binary: &binary,
        folder: &folder,
        repository: &repository,
        initialize: &initialize,
    };
    after_change::run(&place, times.max(TIMES * 2), &mut out)?;
    drop(out);
    if engine_only { Ok(true) } else { window(root, &folder) }
}

/// The engine built in release; none, said on the standard error, when it is not built.
fn engine_binary(root: &Path) -> Result<Option<PathBuf>, Error> {
    let binary = root
        .join("target")
        .join("release")
        .join(format!("gitardo-engine{}", std::env::consts::EXE_SUFFIX));
    if binary.is_file() {
        return Ok(Some(binary));
    }
    writeln!(
        std::io::stderr().lock(),
        "{} is not built.

{USAGE}",
        binary.display()
    )?;
    Ok(None)
}

/// The whole history in the window, read and laid out as `app` does, page after page: the test of `app` that times it
/// on the repository `GITARDO_PERF_REPOSITORY` names, built in release.
fn window(root: &Path, folder: &Path) -> Result<bool, Error> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let arguments = [
        "test",
        "-p",
        "app",
        "--release",
        "--quiet",
        "--test",
        "places",
        "--",
        "--ignored",
        "--nocapture",
        "--exact",
        WINDOW_TEST,
    ];
    writeln!(
        std::io::stdout().lock(),
        "
The window: cargo {}",
        arguments.join(" ")
    )?;
    let status = std::process::Command::new(&cargo)
        .args(arguments)
        .env("GITARDO_PERF_REPOSITORY", folder)
        .current_dir(root)
        .status()
        .map_err(|source| Error::Tool {
            command: format!("cargo {}", arguments.join(" ")),
            source,
        })?;
    Ok(status.success())
}

/// The pages of the timeline: the first, the next three with `skip` as the extension asks them, and one far down.
/// Gives the last commit of the page far down, to look for.
fn read_pages(
    engine: &mut Engine,
    repository: &str,
    times: usize,
    out: &mut impl Write,
) -> Result<Option<String>, Error> {
    let mut last = None;
    for (name, pages) in [
        (String::from("graph.read-commits, page 1"), 1),
        (String::from("graph.read-commits, page 2 with skip"), 2),
        (String::from("graph.read-commits, page 3 with skip"), 3),
        (String::from("graph.read-commits, page 4 with skip"), 4),
        (format!("graph.read-commits, page {FAR_PAGE} with skip"), FAR_PAGE),
    ] {
        // The client has the rows of the pages before, and asks from the last of them, as the extension does.
        let skip = (pages - 1) * PAGE_SIZE;
        let skip = skip.saturating_sub(1);
        let read = Read {
            name,
            method: "read/graph.read-commits",
            params: format!("\"pageSize\":{PAGE_SIZE},\"pages\":{pages},\"skip\":{skip},\"stashes\":true"),
        };
        let answers = ask_times(engine, repository, &read, times)?;
        row(out, &read.name, &answers)?;
        last = answers.last().and_then(|answer| last_commit(&answer.body)).or(last);
    }
    Ok(last)
}

/// The reads besides the pages: the position of a commit far down, a search of the messages and of the authors, and
/// the state.
fn other_reads(far: Option<&str>) -> Vec<Read> {
    let timeline = format!("\"timeline\":{{\"pageSize\":{PAGE_SIZE}}}");
    let mut reads = Vec::new();
    if let Some(commit) = far {
        reads.push(Read {
            name: format!("graph.read-position, a commit of page {FAR_PAGE}"),
            method: "read/graph.read-position",
            params: format!("\"commit\":\"{commit}\",{timeline}"),
        });
    }
    reads.push(Read {
        name: String::from("search.read-matches, a message"),
        method: "read/search.read-matches",
        params: format!("\"text\":\"the parser\",\"mode\":\"message\",{timeline}"),
    });
    reads.push(Read {
        name: String::from("search.read-matches, an author"),
        method: "read/search.read-matches",
        params: format!("\"text\":\"Grace\",\"mode\":\"author\",{timeline}"),
    });
    reads.push(Read {
        name: String::from("repo.read-state"),
        method: "read/repo.read-state",
        params: String::new(),
    });
    reads
}

/// Asks a read a number of times, one after the other.
fn ask_times(engine: &mut Engine, repository: &str, read: &Read, times: usize) -> Result<Vec<Answered>, Error> {
    let separator = if read.params.is_empty() { "" } else { "," };
    let params = format!("{{\"repository\":{repository}{separator}{}}}", read.params);
    (0..times).map(|_| engine.ask(read.method, &params)).collect()
}

/// A line of the table: the rows and the bytes of the last answer, and the fastest, median and slowest times.
fn row(out: &mut impl Write, name: &str, answers: &[Answered]) -> Result<(), Error> {
    let mut times: Vec<Duration> = answers.iter().map(|answer| answer.took).collect();
    times.sort();
    let (Some(fastest), Some(median), Some(slowest)) = (times.first(), times.get(times.len() / 2), times.last()) else {
        return Ok(());
    };
    let last = answers.last().map_or("", |answer| answer.body.as_str());
    let rows = if answers.iter().any(Answered::is_error) {
        String::from("error")
    } else {
        rows(last).map_or_else(|| String::from("–"), |count| count.to_string())
    };
    writeln!(
        out,
        "| {name} | {rows} | {} | {} | {} | {} |",
        last.len(),
        milliseconds(*fastest),
        milliseconds(*median),
        milliseconds(*slowest)
    )?;
    Ok(())
}

/// How many rows a page has, or how many commits a search found; none for an answer that is neither.
fn rows(body: &str) -> Option<usize> {
    let rows = body.matches("\"kind\":\"commit\"").count() + body.matches("\"kind\":\"stash\"").count();
    if rows > 0 {
        return Some(rows);
    }
    let found = body.split_once("\"result\":{\"commits\":[")?.1.split(']').next()?;
    Some(found.matches('"').count() / 2)
}

/// The id of the last commit of a page.
fn last_commit(body: &str) -> Option<String> {
    let start = body.rfind("\"id\":\"")? + "\"id\":\"".len();
    let id = body.get(start..start + 40)?;
    id.bytes().all(|byte| byte.is_ascii_hexdigit()).then(|| id.to_owned())
}

/// A duration in milliseconds, with one decimal.
fn milliseconds(duration: Duration) -> String {
    format!("{:.1} ms", duration.as_secs_f64() * 1_000.0)
}

/// The parameters of `initialize`, with the version of the protocol the engine was built with, read from its source.
fn initialize_params(root: &Path) -> Result<String, Error> {
    let path = root.join("crates").join("protocol").join("src").join("messages.rs");
    let source = crate::files::read_text(&path)?;
    let version = source
        .lines()
        .find_map(|line| line.strip_prefix("pub const VERSION: &str = \""))
        .and_then(|rest| rest.strip_suffix("\";"))
        .unwrap_or("1.0");
    Ok(format!(
        "{{\"protocol\":\"{version}\",\"client\":{{\"name\":\"xtask-perf\",\"version\":\"0.0.0\"}},\"settings\":{{}},\"logLevel\":\"info\"}}"
    ))
}

/// A string as JSON writes it.
fn json_string(text: &str) -> String {
    let mut written = String::with_capacity(text.len() + 2);
    written.push('"');
    for character in text.chars() {
        match character {
            '"' => written.push_str("\\\""),
            '\\' => written.push_str("\\\\"),
            control if u32::from(control) < 0x20 => {
                // Writing to a string cannot fail.
                let _ = std::fmt::Write::write_fmt(&mut written, format_args!("\\u{:04x}", u32::from(control)));
            }
            other => written.push(other),
        }
    }
    written.push('"');
    written
}
