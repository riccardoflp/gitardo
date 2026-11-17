//! A read after a change, as the extension asks for it once a file of the working tree changed: `repo.read-state`, the
//! first page of `graph.read-commits` and `journal/last`, one after the other. Each is timed, and the git processes
//! each starts are counted from the events of trace2 that git writes when `GIT_TRACE2_EVENT` names a file.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use super::engine::{Answered, Engine};
use super::{PAGE_SIZE, milliseconds};
use crate::error::Error;

/// The file of the working tree written before each read, and removed at the end.
const CHANGED_FILE: &str = "gitardo-perf-change.txt";

/// The reads of a read after a change, in the order the extension asks them: a name, a method and the parameters
/// besides `repository`.
const READS: [(&str, &str, &str); 3] = [
    ("repo.read-state", "read/repo.read-state", ""),
    ("graph.read-commits, page 1", "read/graph.read-commits", "page"),
    ("journal/last", "journal/last", ""),
];

/// Where the engine and its git are, for one run.
pub struct Place<'a> {
    /// The engine built in release.
    pub binary: &'a Path,
    /// The folder of the repository.
    pub folder: &'a Path,
    /// The folder of the repository, as JSON.
    pub repository: &'a str,
    /// The parameters of `initialize`.
    pub initialize: &'a str,
}

/// Times a read after a change a number of times, then counts the git processes of one more with an engine of its
/// own whose git writes its events of trace2, and writes a table.
pub fn run(place: &Place<'_>, times: usize, out: &mut impl Write) -> Result<(), Error> {
    let changed = place.folder.join(CHANGED_FILE);
    let mut engine = open(place, None)?;
    let mut timings: Vec<[Duration; 3]> = Vec::with_capacity(times);
    for round in 0..times {
        write_change(&changed, round)?;
        let answers = ask_reads(&mut engine, place.repository)?;
        if answers.iter().any(Answered::is_error) {
            writeln!(
                out,
                "\nA read after a change could not be timed: the engine answered with an error."
            )?;
            remove(&changed);
            return Ok(());
        }
        let took: Vec<Duration> = answers.iter().map(|answer| answer.took).collect();
        if let [state, page, last] = took.as_slice() {
            timings.push([*state, *page, *last]);
        }
    }
    drop(engine);
    let counts = count_processes(place, &changed)?;
    remove(&changed);
    writeln!(
        out,
        "\nA read after a change, as the extension asks it, a file of the working tree written before each, {times} \
         times:\n"
    )?;
    writeln!(out, "| Read | Fastest | Median | Slowest | Git processes |")?;
    writeln!(out, "| --- | ---: | ---: | ---: | --- |")?;
    for (index, (name, _, _)) in READS.iter().enumerate() {
        let column: Vec<Duration> = timings.iter().filter_map(|round| round.get(index).copied()).collect();
        let processes = counts.get(index).map_or_else(String::new, said);
        line(out, name, column, &processes)?;
    }
    let totals: Vec<Duration> = timings.iter().map(|round| round.iter().sum()).collect();
    let all: BTreeMap<String, usize> = counts.iter().flatten().fold(BTreeMap::new(), |mut all, (name, count)| {
        *all.entry(name.clone()).or_default() += count;
        all
    });
    line(out, "All three", totals, &said(&all))?;
    Ok(())
}

/// The three reads, one after the other, with the answer of each.
fn ask_reads(engine: &mut Engine, repository: &str) -> Result<Vec<Answered>, Error> {
    READS
        .iter()
        .map(|(_, method, params)| ask_one(engine, repository, (method, params)))
        .collect()
}

/// One of the reads, by its method and what its parameters are.
fn ask_one(engine: &mut Engine, repository: &str, (method, params): (&str, &str)) -> Result<Answered, Error> {
    let params = if params.is_empty() {
        format!("{{\"repository\":{repository}}}")
    } else {
        format!("{{\"repository\":{repository},\"pageSize\":{PAGE_SIZE},\"pages\":1,\"skip\":0,\"stashes\":true}}")
    };
    engine.ask(method, &params)
}

/// An engine with the repository open, its git writing the events of trace2 to a file when one is given.
fn open(place: &Place<'_>, trace: Option<&Path>) -> Result<Engine, Error> {
    let mut engine = match trace {
        Some(trace) => Engine::start_traced(place.binary, trace)?,
        None => Engine::start(place.binary)?,
    };
    let _started = engine.ask("initialize", place.initialize)?;
    let _opened = engine.ask("repository/open", &format!("{{\"repository\":{}}}", place.repository))?;
    Ok(engine)
}

/// The git processes each of the three reads started, by command, from the events trace2 wrote for one more read.
fn count_processes(place: &Place<'_>, changed: &Path) -> Result<Vec<BTreeMap<String, usize>>, Error> {
    let trace = std::env::temp_dir().join(format!("gitardo-perf-trace-{}.json", std::process::id()));
    remove(&trace);
    let mut engine = open(place, Some(&trace))?;
    write_change(changed, usize::MAX)?;
    let mut counts = Vec::with_capacity(READS.len());
    // What starting the engine and opening the repository ran is not part of the reads.
    let mut seen = std::fs::read_to_string(&trace).unwrap_or_default().lines().count();
    for (_, method, params) in READS {
        let _answer = ask_one(&mut engine, place.repository, (method, params))?;
        let events = std::fs::read_to_string(&trace).unwrap_or_default();
        let lines: Vec<&str> = events.lines().collect();
        counts.push(started_commands(lines.get(seen..).unwrap_or_default()));
        seen = lines.len();
    }
    drop(engine);
    remove(&trace);
    Ok(counts)
}

/// The commands of the git processes the engine started, from their `start` events: a process that another git
/// started has a `sid` with a `/` and is not counted.
fn started_commands(lines: &[&str]) -> BTreeMap<String, usize> {
    let mut commands = BTreeMap::new();
    for line in lines.iter().filter(|line| line.contains("\"event\":\"start\"")) {
        let is_top = line
            .split("\"sid\":\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .is_some_and(|sid| !sid.contains('/'));
        if is_top {
            *commands.entry(command_of(line)).or_default() += 1;
        }
    }
    commands
}

/// The git command of a `start` event: the first of its arguments that is not an option or the value of `-c`.
fn command_of(line: &str) -> String {
    let arguments = line
        .split("\"argv\":[")
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .unwrap_or("");
    let mut words = arguments.split("\",\"").map(|word| word.trim_matches('"')).skip(1);
    while let Some(word) = words.next() {
        if word == "-c" || word == "-C" {
            let _value = words.next();
        } else if !word.starts_with('-') {
            return word.to_owned();
        }
    }
    String::from("git")
}

/// The count of the processes, and how many of each command, as `7 (log 1, status 6)`.
fn said(commands: &BTreeMap<String, usize>) -> String {
    let total: usize = commands.values().sum();
    let each: Vec<String> = commands
        .iter()
        .map(|(command, count)| format!("{command} {count}"))
        .collect();
    format!("{total} ({})", each.join(", "))
}

/// A line of the table: the fastest, median and slowest of the times, and the processes.
fn line(out: &mut impl Write, name: &str, mut times: Vec<Duration>, processes: &str) -> Result<(), Error> {
    times.sort();
    let (Some(fastest), Some(median), Some(slowest)) = (times.first(), times.get(times.len() / 2), times.last()) else {
        return Ok(());
    };
    writeln!(
        out,
        "| {name} | {} | {} | {} | {processes} |",
        milliseconds(*fastest),
        milliseconds(*median),
        milliseconds(*slowest)
    )?;
    Ok(())
}

/// Writes the file of the change, different each time.
fn write_change(path: &Path, round: usize) -> Result<(), Error> {
    std::fs::write(path, format!("A change made by cargo xtask perf, round {round}.\n")).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })
}

/// Removes a file the benchmark made; one already gone is what was wanted.
fn remove(path: &Path) {
    let _ = std::fs::remove_file(path);
}
