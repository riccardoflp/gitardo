//! The whole history read by the engine as the extension reads it: page after page, each asked with `skip` from the
//! last row it has, until the engine says there are no more; timed to the first page and to the last row.

use std::io::Write;
use std::time::{Duration, Instant};

use super::engine::Engine;
use super::{PAGE_SIZE, milliseconds, rows};
use crate::error::Error;

/// The most pages read, so that a history longer than any this is meant for still ends.
const MOST_PAGES: usize = 1_000;

/// How long the whole history took: to its first page, and to its last row.
struct Whole {
    /// How many pages were asked for.
    pages: usize,
    /// How many rows came.
    rows: usize,
    /// From the first request to the last byte of the first page.
    first_page: Duration,
    /// From the first request to the last byte of the last page.
    last_row: Duration,
}

/// Reads the whole history a number of times and writes a table of the median and the slowest of each time.
pub fn run(engine: &mut Engine, repository: &str, times: usize, out: &mut impl Write) -> Result<(), Error> {
    let mut reads = Vec::with_capacity(times);
    for _ in 0..times {
        let Some(whole) = read_whole(engine, repository)? else {
            writeln!(
                out,
                "\nThe whole history could not be read: the engine answered with an error."
            )?;
            return Ok(());
        };
        reads.push(whole);
    }
    let (Some(first), Some(last)) = (
        median(&reads, |whole| whole.first_page),
        median(&reads, |whole| whole.last_row),
    ) else {
        return Ok(());
    };
    let slowest = reads.iter().map(|whole| whole.last_row).max().unwrap_or_default();
    let (pages, count) = reads.last().map_or((0, 0), |whole| (whole.pages, whole.rows));
    writeln!(
        out,
        "\nThe whole history, page after page as the extension asks it, read {times} times (the median):\n"
    )?;
    writeln!(
        out,
        "| Read | Pages | Rows | First page | Last row | Slowest last row |"
    )?;
    writeln!(out, "| --- | ---: | ---: | ---: | ---: | ---: |")?;
    writeln!(
        out,
        "| graph.read-commits, every page with skip | {pages} | {count} | {} | {} | {} |",
        milliseconds(first),
        milliseconds(last),
        milliseconds(slowest)
    )?;
    Ok(())
}

/// Reads every page of the history, one after the other; none when the engine answered with an error.
fn read_whole(engine: &mut Engine, repository: &str) -> Result<Option<Whole>, Error> {
    let started = Instant::now();
    let mut whole = Whole {
        pages: 0,
        rows: 0,
        first_page: Duration::ZERO,
        last_row: Duration::ZERO,
    };
    while whole.pages < MOST_PAGES {
        whole.pages += 1;
        // The client has the rows of the pages before, and asks from the last of them, as the extension does.
        let skip = ((whole.pages - 1) * PAGE_SIZE).saturating_sub(1);
        let params = format!(
            "{{\"repository\":{repository},\"pageSize\":{PAGE_SIZE},\"pages\":{},\"skip\":{skip},\"stashes\":true}}",
            whole.pages
        );
        let answer = engine.ask("read/graph.read-commits", &params)?;
        if answer.is_error() {
            return Ok(None);
        }
        let new_rows = rows(&answer.body).unwrap_or(0);
        // Each page after the first starts again with the last row the client had.
        whole.rows += if whole.pages == 1 {
            new_rows
        } else {
            new_rows.saturating_sub(1)
        };
        if whole.pages == 1 {
            whole.first_page = started.elapsed();
        }
        if !answer.body.contains("\"hasMore\":true") {
            break;
        }
    }
    whole.last_row = started.elapsed();
    Ok(Some(whole))
}

/// The median of one time of the reads.
fn median(reads: &[Whole], time: impl Fn(&Whole) -> Duration) -> Option<Duration> {
    let mut times: Vec<Duration> = reads.iter().map(time).collect();
    times.sort();
    times.get(times.len() / 2).copied()
}
