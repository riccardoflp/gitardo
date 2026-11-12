//! How long opening a large repository takes, from the folder to the first frame, how long each next page of the
//! timeline takes, from the intent to the rows laid out, and how long narrowing the timeline to a branch and back
//! takes: 56,000 commits, a merge of a topic branch every ten, with 40 branches and 50 tags. Not run by default:
//!
//! ```text
//! cargo test -p app --release --test places -- --ignored --nocapture
//! ```
//!
//! With `GITARDO_COMMIT_GRAPH` set, the repository gets a commit-graph file, as `git gc` writes one: `--date-order`
//! then no longer walks the whole history before the first commit.

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use app::store::{Effect, JobKind, Message, State};
use app::work::jobs::{find_repositories, read_repository};
use git::CancelToken;
use model::filter::{BranchesShown, Preset, preset, show_only};
use model::graph::PageSize;
use render::ThemeName;
use testkit::Repository;
use views::graph::GraphIntent;
use views::{FilterIntent, Intent};

use super::histories::{Commit, import, write_commit};
use super::support::{Window, run};

/// How many rounds of ten commits: six on `main`, three on a topic branch, and the merge.
const ROUNDS: u64 = 5_600;
/// How many branches, at the tips of topic branches spread over the history.
const BRANCHES: u64 = 40;
/// How many tags, spread over the history.
const TAGS: u64 = 50;
/// The longest an opening may take, from the folder to the first frame.
const OPEN_AT_MOST: Duration = Duration::from_millis(1_500);
/// The longest a next page may take, from the intent to the rows laid out.
const PAGE_AT_MOST: Duration = Duration::from_millis(300);
/// The longest narrowing the timeline may take, from the intent to the rows laid out.
const NARROW_AT_MOST: Duration = Duration::from_millis(500);

/// The stream of the history, the oldest commit first.
fn history() -> String {
    let mut stream = String::new();
    let mut mark = 0;
    let mut commit = |stream: &mut String, branch: &str, (from, merge): (Option<u64>, Option<u64>), message: &str| {
        mark += 1;
        let file = format!("src/file{}.txt", mark % 200);
        write_commit(
            stream,
            &Commit {
                branch,
                mark,
                from,
                merge,
                message,
                file: &file,
            },
        );
        mark
    };
    let mut topic_tips = Vec::new();
    for round in 0..ROUNDS {
        let mut base = 0;
        for step in 0..6 {
            base = commit(&mut stream, "main", (None, None), &format!("Change {round}.{step}"));
        }
        let mut tip = commit(&mut stream, "topic", (Some(base), None), &format!("Topic {round}.0"));
        for step in 1..3 {
            tip = commit(&mut stream, "topic", (None, None), &format!("Topic {round}.{step}"));
        }
        topic_tips.push(tip);
        let _merge = commit(&mut stream, "main", (None, Some(tip)), &format!("Merge topic {round}"));
    }
    let spread = |count: u64, index: u64| {
        *topic_tips
            .get(usize::try_from(index * ROUNDS / count).unwrap())
            .unwrap()
    };
    for number in 0..BRANCHES {
        let tip = spread(BRANCHES, number);
        write!(stream, "reset refs/heads/branch-{number}\nfrom :{tip}\n\n").unwrap();
    }
    for number in 0..TAGS {
        let at = spread(TAGS, number) + 1;
        write!(stream, "reset refs/tags/v{number}\nfrom :{at}\n\n").unwrap();
    }
    write!(stream, "reset refs/heads/topic\nfrom :{}\n\n", spread(1, 0)).unwrap();
    stream
}

#[test]
#[ignore = "makes a repository of 56,000 commits; run by hand, in release, to measure"]
fn opening_a_repository_of_56000_commits_and_loading_its_next_pages() {
    let repository = Repository::new();
    import(&repository, &history());
    if std::env::var_os("GITARDO_COMMIT_GRAPH").is_some() {
        let _ = repository.git(&["commit-graph", "write", "--reachable"]);
    }
    let folder = repository.path().to_path_buf();
    let (_, commits) = read_repository(&folder, &first_page(), &CancelToken::new()).unwrap();
    assert_eq!(commits.rows.len(), 1_000);
    let mut slowest_open = Duration::ZERO;
    for attempt in 1..=3 {
        let started = Instant::now();
        let _found = find_repositories(std::slice::from_ref(&folder), &CancelToken::new());
        let found = started.elapsed();
        let mut state = State::default();
        let _effects = run(
            &mut state,
            Message::Started {
                folders: vec![folder.clone()],
            },
        );
        let read = started.elapsed();
        let mut window = Window::new(ThemeName::Dark, (1280, 800));
        let _ = window.draw(&state, &[]);
        let opened = started.elapsed();
        println!(
            "attempt {attempt}: repositories found in {found:?}, opened and read in {:?}, first frame drawn after \
             {opened:?} in all",
            read.checked_sub(found).unwrap(),
        );
        slowest_open = slowest_open.max(opened);
        assert_eq!(state.shown_read().unwrap().state.branches.len(), 42);
    }
    let passes: Vec<Vec<Duration>> = (0..2).map(|_| next_pages(&folder)).collect();
    let pages: Vec<Duration> = passes[0]
        .iter()
        .zip(&passes[1])
        .map(|(first, second)| *first.min(second))
        .collect();
    let in_ms: Vec<u128> = pages.iter().map(Duration::as_millis).collect();
    println!("each next page, the faster of two passes, in ms: {in_ms:?}");
    let slowest_page = pages.iter().max().copied().unwrap_or_default();
    let narrowings = narrowings(&folder);
    for (name, took, rows) in &narrowings {
        println!("narrowed to {name} in {took:?}, {rows} rows");
    }
    let slowest_narrowing = narrowings.iter().map(|(_, took, _)| *took).max().unwrap_or_default();
    assert!(slowest_open < OPEN_AT_MOST, "opening took {slowest_open:?}");
    assert!(slowest_page < PAGE_AT_MOST, "a page took {slowest_page:?}");
    assert!(
        slowest_narrowing < NARROW_AT_MOST,
        "narrowing took {slowest_narrowing:?}"
    );
}

/// Opens the repository of a folder, narrows its timeline to a branch, to two, to the local branches and back to
/// everything, and gives how long each took, from the intent to the rows laid out, with the rows read. Narrowing reads
/// the first page of the history alone: never the repository again.
fn narrowings(folder: &std::path::Path) -> Vec<(&'static str, Duration, usize)> {
    let mut state = State::default();
    let _effects = run(
        &mut state,
        Message::Started {
            folders: vec![folder.to_path_buf()],
        },
    );
    let two = BranchesShown::References(vec![
        String::from("refs/heads/branch-20"),
        String::from("refs/heads/branch-30"),
    ]);
    let steps: [(&'static str, Option<BranchesShown>); 4] = [
        ("one branch", Some(show_only("refs/heads/branch-20"))),
        ("two branches", Some(two)),
        ("the local branches", preset(Preset::Local)),
        ("everything", None),
    ];
    let mut timings = Vec::new();
    for (name, shown) in steps {
        let started = Instant::now();
        let effects = run(
            &mut state,
            Message::Intent(Intent::Filter(FilterIntent::Show(shown.clone()))),
        );
        let took = started.elapsed();
        let reads: Vec<&JobKind> = effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::Start(job) => Some(&job.kind),
                _ => None,
            })
            .collect();
        assert!(
            matches!(reads.as_slice(), [JobKind::ReadTimeline { .. }]),
            "narrowing to {name} read {reads:?}"
        );
        assert_eq!(state.timeline.shown, shown);
        timings.push((name, took, state.timeline.rows.len()));
    }
    timings
}

/// Opens the repository of a folder and loads every next page of its timeline, and gives how long each took, from the
/// intent to the rows laid out.
fn next_pages(folder: &std::path::Path) -> Vec<Duration> {
    let mut state = State::default();
    let _effects = run(
        &mut state,
        Message::Started {
            folders: vec![folder.to_path_buf()],
        },
    );
    let mut timings = Vec::new();
    while state.timeline.has_more {
        let started = Instant::now();
        let loaded = state.timeline.loaded();
        let _effects = run(
            &mut state,
            Message::Intent(Intent::Graph(GraphIntent::LoadMore { loaded })),
        );
        timings.push(started.elapsed());
        assert!(state.timeline.loaded() > loaded);
    }
    assert_eq!(state.timeline.loaded(), 56_000);
    timings
}

/// The first page of the timeline, as a window reads it with the default settings.
fn first_page() -> app::store::TimelineRequest {
    app::store::TimelineRequest {
        page_size: PageSize::DEFAULT,
        pages: 1,
        parents: git::read::graph::Parents::All,
        shows_stashes: true,
        shown: None,
    }
}
