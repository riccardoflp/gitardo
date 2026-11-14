//! How long `graph.read-commits` takes on a line of 50,000 commits, one page and every page; and how long the
//! derivations of the graph take on 50,000 rows of a main line with a merge of a short branch every ten commits and
//! twenty long branches open all along, where the lanes should take well under 100 ms. Not run by default, since the
//! model crate, where the derivations are, reads no clock:
//!
//! ```text
//! cargo test -p git --release --test graph_read -- --ignored --nocapture
//! ```

use std::collections::HashSet;
use std::time::Instant;

use model::commit::Commit;
use model::graph::{GraphCell, GraphInput, PageSize, Row, Trace, fold_rows, lane_tips, lanes, trace_history};
use model::{CommitId, Timestamp};
use testkit::Repository;

use crate::{everything, history, import_line, read, request};

#[test]
#[ignore = "makes a repository of 50,000 commits; run by hand to measure"]
fn graph_read_commits__speed_on_50000_commits() {
    let repository = Repository::new();
    let made = Instant::now();
    import_line(&repository, 50_000);
    println!("made the repository in {:?}", made.elapsed());
    for pages in [1, 50] {
        for attempt in 1..=3 {
            let started = Instant::now();
            let page = read(&repository, &request(history(everything()), PageSize::DEFAULT, pages));
            let elapsed = started.elapsed();
            assert_eq!(page.rows.len(), pages * 1_000);
            println!("{pages} pages, read {attempt}: {elapsed:?}");
            let inputs: Vec<GraphInput<'_>> = page.rows.iter().map(Row::graph_input).collect();
            let started = Instant::now();
            let cells = lanes(&inputs);
            println!(
                "{pages} pages, lanes {attempt}: {:?} for {} rows",
                started.elapsed(),
                cells.len()
            );
        }
    }
}

/// How many rows the graph has.
const ROWS: usize = 50_000;
/// How many long branches are open all along.
const LONG_BRANCHES: usize = 20;

/// The history, built from the oldest commit, as rows newest first.
struct History {
    /// The commits from the oldest.
    commits: Vec<Commit>,
}

impl History {
    /// A commit on top of its parents, numbered in the order made.
    fn commit(&mut self, parents: &[CommitId]) -> CommitId {
        let number = self.commits.len();
        let id = CommitId::parse(&format!("{number:040x}")).unwrap();
        self.commits.push(Commit {
            id: id.clone(),
            parents: parents.to_vec(),
            author_name: String::from("Test Author"),
            author_email: String::from("author@example.com"),
            author_date: Timestamp(i64::try_from(number).unwrap() * 60),
            subject: format!("Change {number}"),
        });
        id
    }
}

/// The rows of the history, newest first.
fn rows() -> Vec<Row> {
    let mut history = History { commits: Vec::new() };
    let root = history.commit(&[]);
    let mut main = root.clone();
    let mut long: Vec<CommitId> = (0..LONG_BRANCHES).map(|_| root.clone()).collect();
    let mut step = 0;
    while history.commits.len() < ROWS {
        main = history.commit(&[main]);
        if step % 10 == 9 {
            let mut feature = main.clone();
            for _ in 0..3 {
                feature = history.commit(&[feature]);
            }
            main = history.commit(&[main, feature]);
        }
        let branch = step % LONG_BRANCHES;
        long[branch] = history.commit(&[long[branch].clone()]);
        step += 1;
    }
    history.commits.into_iter().rev().map(Row::Commit).collect()
}

/// Runs a derivation and prints how long it took.
fn timed<Value>(what: &str, derivation: impl FnOnce() -> Value) -> Value {
    let started = Instant::now();
    let value = derivation();
    println!("{what}: {:?}", started.elapsed());
    value
}

#[test]
#[ignore = "lays out 50,000 rows; run by hand, in release, to measure"]
fn graph_lanes__speed_on_50000_rows() {
    let rows = rows();
    let inputs: Vec<GraphInput<'_>> = rows.iter().map(Row::graph_input).collect();
    let mut cells: Vec<GraphCell> = Vec::new();
    for attempt in 1..=5 {
        cells = timed(&format!("lanes {attempt}"), || lanes(&inputs));
    }
    let widest = cells.iter().map(GraphCell::lane_count).max().unwrap();
    println!("{} rows, {widest} lanes at most", rows.len());
    let merges: HashSet<CommitId> = rows
        .iter()
        .filter_map(Row::commit)
        .filter(|commit| commit.is_merge())
        .map(|commit| commit.id.clone())
        .collect();
    let folded = timed("fold every merge", || fold_rows(&rows, &merges));
    let folded_inputs = folded.graph_inputs(&rows);
    let _ = timed("lanes of the folded rows", || lanes(&folded_inputs));
    let middle = rows[ROWS / 2].id().unwrap().clone();
    let _ = timed("trace around a commit", || {
        trace_history(&rows, &cells, &Trace::Around(middle))
    });
    let named: Vec<bool> = (0..rows.len()).map(|row| row % 100 == 0).collect();
    let _ = timed("lane tips", || lane_tips(&cells, &named));
}
