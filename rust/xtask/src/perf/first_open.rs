//! A repository opened for the first time, as an engine just started is asked for it: `repository/open`, then the first
//! page of `graph.read-commits`, then `repo.read-state`, one after the other. Each is timed with an engine of its own
//! for each round, since only the first of each is wanted, and the git processes each starts are counted from the
//! events of trace2, as for a read after a change. It is the engine alone: no window draws what it answers.

use std::collections::BTreeMap;
use std::io::Write;
use std::time::Duration;

use super::PAGE_SIZE;
use super::after_change::{Place, line, said, started_commands};
use super::engine::{Answered, Engine};
use crate::error::Error;

/// What is asked of the engine once it is initialized, in order: a name and a method.
const ASKED: [(&str, &str); 3] = [
    ("repository/open, the first", "repository/open"),
    ("graph.read-commits, page 1, the first", "read/graph.read-commits"),
    ("repo.read-state, after them", "read/repo.read-state"),
];

/// Times the first open, the first page and the state after them a number of times, each time with a new engine, then
/// counts the git processes of one more round, and writes a table.
pub fn run(place: &Place<'_>, times: usize, out: &mut impl Write) -> Result<(), Error> {
    let mut timings: Vec<Vec<Duration>> = Vec::with_capacity(times);
    for _ in 0..times {
        let mut engine = Engine::start(place.binary)?;
        let _started = engine.ask("initialize", place.initialize)?;
        let mut took = Vec::with_capacity(ASKED.len());
        for (_, method) in ASKED {
            let answer = ask(&mut engine, place.repository, method)?;
            if answer.is_error() {
                writeln!(
                    out,
                    "\nA repository opened for the first time could not be timed: the engine answered with an error."
                )?;
                return Ok(());
            }
            took.push(answer.took);
        }
        timings.push(took);
    }
    let counts = count_processes(place)?;
    writeln!(
        out,
        "\nA repository opened for the first time by an engine just started, then its state, {times} times:\n"
    )?;
    writeln!(out, "| Asked | Fastest | Median | Slowest | Git processes |")?;
    writeln!(out, "| --- | ---: | ---: | ---: | --- |")?;
    let column =
        |index: usize| -> Vec<Duration> { timings.iter().filter_map(|round| round.get(index).copied()).collect() };
    let processes = |index: usize| counts.get(index).map_or_else(String::new, said);
    for (index, (name, _)) in ASKED.iter().enumerate().take(2) {
        line(out, name, column(index), &processes(index))?;
    }
    let both: Vec<Duration> = timings.iter().map(|round| round.iter().take(2).sum()).collect();
    let both_counts = counts
        .iter()
        .take(2)
        .flatten()
        .fold(BTreeMap::new(), |mut all, (name, count)| {
            *all.entry(name.clone()).or_default() += count;
            all
        });
    line(out, "The open and the first page", both, &said(&both_counts))?;
    if let Some((name, _)) = ASKED.get(2) {
        line(out, name, column(2), &processes(2))?;
    }
    Ok(())
}

/// One of the three, by its method.
fn ask(engine: &mut Engine, repository: &str, method: &str) -> Result<Answered, Error> {
    let params = if method == "read/graph.read-commits" {
        format!("{{\"repository\":{repository},\"pageSize\":{PAGE_SIZE},\"pages\":1,\"skip\":0,\"stashes\":true}}")
    } else {
        format!("{{\"repository\":{repository}}}")
    };
    engine.ask(method, &params)
}

/// The git processes each of the three started, by command, from the events trace2 wrote for one more round.
fn count_processes(place: &Place<'_>) -> Result<Vec<BTreeMap<String, usize>>, Error> {
    let trace = std::env::temp_dir().join(format!("gitardo-perf-first-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&trace);
    let mut engine = Engine::start_traced(place.binary, &trace)?;
    let _started = engine.ask("initialize", place.initialize)?;
    let mut counts = Vec::with_capacity(ASKED.len());
    // What starting the engine ran is not part of what is asked.
    let mut seen = std::fs::read_to_string(&trace).unwrap_or_default().lines().count();
    for (_, method) in ASKED {
        let _answer = ask(&mut engine, place.repository, method)?;
        let events = std::fs::read_to_string(&trace).unwrap_or_default();
        let lines: Vec<&str> = events.lines().collect();
        counts.push(started_commands(lines.get(seen..).unwrap_or_default()));
        seen = lines.len();
    }
    drop(engine);
    let _ = std::fs::remove_file(&trace);
    Ok(counts)
}
