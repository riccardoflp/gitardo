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

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::time::{Duration, Instant};

use app::store::{Effect, Job, JobKind, Message, State};
use app::work::jobs::find_repositories;
use app::work::pages::read_repository_at_once;
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
    history_of(ROUNDS)
}

/// The stream of a history of some rounds of ten commits, the oldest commit first, with the branches and the tags.
pub(super) fn history_of(rounds: u64) -> String {
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
    for round in 0..rounds {
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
            .get(usize::try_from(index * rounds / count).unwrap())
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
    let (_, commits) = read_repository_at_once(&folder, &first_page(), &CancelToken::new()).unwrap();
    assert_eq!(commits.rows.len(), 56_000);
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

/// How long the first page of the timeline takes to read, the fastest of three, from the start of `git log` to its
/// rows: the state is read before.
fn first_page_read(folder: &std::path::Path) -> Duration {
    let git = git::Git::new(folder);
    let (repository, _) = read_repository_at_once(folder, &first_page(), &CancelToken::new()).unwrap();
    let request = app::store::commits_request(&repository, &first_page());
    (0..3)
        .map(|_| {
            let started = Instant::now();
            let _end = git::read::graph::read_commits(&git, &request, &mut |_page| {}).unwrap();
            started.elapsed()
        })
        .min()
        .unwrap_or_default()
}

#[test]
#[ignore = "makes a repository of 56,000 commits; run by hand, in release, to measure"]
fn upkeep_write_commit_graph__the_first_page_of_56000_commits_is_read_in_less_than_half_the_time_once_written() {
    let repository = Repository::new();
    import(&repository, &history());
    let folder = repository.path().to_path_buf();
    let before = first_page_read(&folder);
    let _ = repository.git(&["commit-graph", "write", "--reachable", "--changed-paths"]);
    let after = first_page_read(&folder);
    println!("the first page: {before:?} with no commit-graph file, {after:?} with one");
    assert!(after * 2 < before, "{after:?} is not less than half of {before:?}");
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
pub(super) fn first_page() -> app::store::TimelineRequest {
    app::store::TimelineRequest {
        page_size: PageSize::DEFAULT,
        pages: 1,
        parents: git::read::graph::Parents::All,
        shows_stashes: true,
        shown: None,
        narrowing: Vec::new(),
        alone: None,
    }
}

/// The whole history of the repository `GITARDO_PERF_REPOSITORY` names, in the window as `app` reads and lays it out:
/// opened, each page of the read taken as the worker sends it, and every next read asked for past those; then the
/// branches shown changed, to the branch checked out, to the local branches and to one branch, each timed to its first
/// page and to its last row. Run by `cargo xtask perf <folder>`; it prints the times, and asserts nothing about them.
#[test]
#[ignore = "times the repository GITARDO_PERF_REPOSITORY names; run by cargo xtask perf"]
fn the_whole_history_of_the_perf_repository_read_and_laid_out_page_after_page() {
    let Some(folder) = std::env::var_os("GITARDO_PERF_REPOSITORY").map(std::path::PathBuf::from) else {
        println!("GITARDO_PERF_REPOSITORY names no repository: nothing to time.");
        return;
    };
    let mut rounds = Vec::new();
    let mut filtered: Vec<(String, Vec<Narrowed>)> = Vec::new();
    for _ in 0..3 {
        let started = Instant::now();
        let mut state = State::default();
        let mut first_page = None;
        let mut message = Some(Message::Started {
            folders: vec![folder.clone()],
        });
        while let Some(next) = message.take() {
            timed(&mut state, next, &mut |state| {
                if !state.timeline.rows.is_empty() {
                    let _first = first_page.get_or_insert_with(|| started.elapsed());
                }
            });
            let loaded = state.timeline.loaded();
            if state.timeline.has_more {
                message = Some(Message::Intent(Intent::Graph(GraphIntent::LoadMore { loaded })));
            }
        }
        let last_row = started.elapsed();
        let layouts = layouts_page_after_page(&state);
        rounds.push((
            first_page.unwrap_or(last_row),
            last_row,
            layouts,
            state.timeline.loaded(),
        ));
        for (index, (name, shown)) in other_branches(&state).into_iter().enumerate() {
            let timing = narrowed(&mut state, shown);
            if filtered.len() <= index {
                filtered.push((name, Vec::new()));
            }
            if let Some((_, timings)) = filtered.get_mut(index) {
                timings.push(timing);
            }
        }
    }
    rounds.sort_by_key(|(_, last_row, _, _)| *last_row);
    let (first_page, last_row, layouts, rows) = rounds.get(rounds.len() / 2).copied().unwrap();
    println!(
        "The window, the median of three: {rows} commits; the first page in {first_page:?}, the last row in \
         {last_row:?}; laying out every page over every row before it would take {layouts:?}"
    );
    for (name, mut timings) in filtered {
        timings.sort_by_key(|(first_page, _, _)| *first_page);
        let (first_page, last_row, rows) = timings.get(timings.len() / 2).copied().unwrap();
        println!(
            "The window, the branches shown changed to {name}, the median of three: {rows} commits; the first \
             page in {first_page:?}, the last row in {last_row:?}"
        );
    }
}

/// How long the branches shown changed took in the window: to the first page and to the last row, with the commits
/// read.
type Narrowed = (Duration, Duration, usize);

/// The branches shown that the window is timed with after the whole history: the branch checked out, the local
/// branches, and the first local branch not checked out, ticked alone.
fn other_branches(state: &State) -> Vec<(String, Option<BranchesShown>)> {
    let mut shown = vec![
        (String::from("the current branch"), preset(Preset::Current)),
        (String::from("the local branches"), preset(Preset::Local)),
    ];
    let other = state
        .shown_read()
        .and_then(|read| read.state.branches.iter().find(|branch| !branch.is_checked_out));
    if let Some(branch) = other {
        shown.push((
            format!("one branch, {}", branch.name),
            Some(show_only(&format!("refs/heads/{}", branch.name))),
        ));
    }
    shown
}

/// Changes the branches shown and runs the read it starts to its end, as a worker would: how long the first page took
/// to reach the timeline, from the intent, how long the last row took, and the commits read.
fn narrowed(state: &mut State, shown: Option<BranchesShown>) -> Narrowed {
    let started = Instant::now();
    let mut first_page = None;
    timed(
        state,
        Message::Intent(Intent::Filter(FilterIntent::Show(shown))),
        &mut |state| {
            if state.timeline.narrow_job.is_none() {
                let _first = first_page.get_or_insert_with(|| started.elapsed());
            }
        },
    );
    let last_row = started.elapsed();
    (first_page.unwrap_or(last_row), last_row, state.timeline.loaded())
}

/// Reduces a message and runs every job it starts as a worker would, each message reduced as soon as the job sends
/// it; `reduced` is given the state after each.
fn timed(state: &mut State, message: Message, reduced: &mut dyn FnMut(&State)) {
    let mut jobs = VecDeque::new();
    jobs.extend(started_by(state, message));
    while let Some(job) = jobs.pop_front() {
        app::work::jobs::run(&job, &CancelToken::new(), &mut |message| {
            jobs.extend(started_by(state, message));
            reduced(state);
        });
    }
}

/// The jobs a message starts, in the order the store asks for them.
fn started_by(state: &mut State, message: Message) -> Vec<Job> {
    app::store::reduce(state, message)
        .into_iter()
        .filter_map(|effect| match effect {
            Effect::Start(job) => Some(job),
            _ => None,
        })
        .collect()
}

/// How long the timeline takes to be laid out again over every row read at each page, as `app` does when a page is
/// added: a layout of the first page, of the first two, and so on to all of them.
fn layouts_page_after_page(state: &State) -> Duration {
    let rows = &state.timeline.rows;
    let page = state.settings.page_size.get();
    let read = state.shown_read().unwrap();
    let mut took = Duration::ZERO;
    let mut end = 0;
    while end < rows.len() {
        end = (end + page).min(rows.len());
        let input = views::graph::LayoutInput {
            rows: rows.get(..end).unwrap(),
            repository: &read.state,
            folded: &state.timeline.folded,
            combine_upstream: false,
            separators_at: Some(model::graph::DateTime {
                moment: model::Timestamp(state.timeline.read_at),
                utc_offset: 0,
            }),
            one_lane: false,
        };
        let started = Instant::now();
        let _layout = views::graph::TimelineLayout::new(&input);
        took += started.elapsed();
    }
    took
}
