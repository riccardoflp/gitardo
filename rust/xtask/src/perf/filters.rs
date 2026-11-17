//! The first page after the branches shown change, as the extension asks it with version 1.8 of the protocol: a read
//! of the commits with `stream`, each page in a notification as git prints it, after a read of another history. Each
//! read is timed to its first page and to its answer, once its last row is there.

use std::io::Write;
use std::time::Duration;

use super::engine::{Engine, Streamed};
use super::{PAGE_SIZE, milliseconds};
use crate::error::Error;

/// One branch filter: its name in the table, and `shown` as the extension sends it.
struct Filter {
    name: String,
    shown: String,
}

/// Reads the history of each filter a number of times, one filter after the other so that each read follows a read of
/// another history, and writes a table of the median of each time.
pub fn run(engine: &mut Engine, repository: &str, times: usize, out: &mut impl Write) -> Result<(), Error> {
    // The extension has read the state before it changes the branches shown: the read of the commits takes it.
    let state = engine.ask("read/repo.read-state", &format!("{{\"repository\":{repository}}}"))?;
    let filters = filters(&state.body);
    let mut reads: Vec<Vec<Streamed>> = filters.iter().map(|_| Vec::with_capacity(times)).collect();
    for _ in 0..times {
        for (filter, timings) in filters.iter().zip(reads.iter_mut()) {
            let params = format!(
                "{{\"repository\":{repository},\"shown\":{},\"pageSize\":{PAGE_SIZE},\"pages\":1,\"skip\":0,\
                 \"stashes\":true,\"stream\":true}}",
                filter.shown
            );
            let streamed = engine.ask_streamed(&params)?;
            if streamed.answered.is_error() || streamed.first_page.is_none() {
                writeln!(
                    out,
                    "\nThe history of {} could not be timed: {}",
                    filter.name, streamed.answered.body
                )?;
                return Ok(());
            }
            timings.push(streamed);
        }
    }
    writeln!(
        out,
        "\nThe branches shown changed, each read streamed (protocol 1.8) after a read of another history, {times} \
         times (the median):\n"
    )?;
    writeln!(
        out,
        "| Branches shown | Pages | Rows | First page | Last row | Slowest first page |"
    )?;
    writeln!(out, "| --- | ---: | ---: | ---: | ---: | ---: |")?;
    for (filter, timings) in filters.iter().zip(&reads) {
        let firsts: Vec<Duration> = timings.iter().filter_map(|read| read.first_page).collect();
        let lasts: Vec<Duration> = timings.iter().map(|read| read.answered.took).collect();
        let (Some(first), Some(last)) = (median(firsts.clone()), median(lasts)) else {
            continue;
        };
        let slowest = firsts.iter().max().copied().unwrap_or_default();
        let (pages, rows) = timings.last().map_or((0, 0), |read| (read.pages, read.rows));
        writeln!(
            out,
            "| {} | {pages} | {rows} | {} | {} | {} |",
            filter.name,
            milliseconds(first),
            milliseconds(last),
            milliseconds(slowest)
        )?;
    }
    Ok(())
}

/// The filters timed: every branch, the branch checked out, the local branches, and one branch ticked alone, the
/// first local branch of the state besides the one checked out.
fn filters(state: &str) -> Vec<Filter> {
    let mut filters = vec![
        Filter {
            name: String::from("Every branch"),
            shown: String::from("null"),
        },
        Filter {
            name: String::from("The current branch"),
            shown: String::from("{\"kind\":\"current\"}"),
        },
        Filter {
            name: String::from("The local branches"),
            shown: String::from("{\"kind\":\"kinds\",\"local\":true,\"remote\":false,\"tags\":false}"),
        },
    ];
    if let Some(branch) = other_branch(state) {
        filters.push(Filter {
            name: format!("One branch, `{branch}`"),
            shown: format!("{{\"kind\":\"references\",\"names\":[\"refs/heads/{branch}\"]}}"),
        });
    }
    filters
}

/// The name of the first local branch of the answer of `repo.read-state` that is not checked out; the fields of each
/// branch are in the order of their names, as `serde_json` writes them.
fn other_branch(state: &str) -> Option<&str> {
    let branches = state.split_once("\"branches\":[")?.1;
    branches
        .split_once("\"isCheckedOut\":false,\"name\":\"")?
        .1
        .split('"')
        .next()
}

/// The median of some times.
fn median(mut times: Vec<Duration>) -> Option<Duration> {
    times.sort();
    times.get(times.len() / 2).copied()
}
