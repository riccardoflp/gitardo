//! How long opening a large repository takes, from the folder to the first frame: 5,000 commits over 200 files, with 50
//! branches and 50 tags. Not run by default:
//!
//! ```text
//! cargo test -p app --release --test places -- --ignored --nocapture
//! ```

use std::fmt::Write as _;
use std::io::Write as _;
use std::process::{Command, Stdio};
use std::time::Instant;

use app::store::{Message, State};
use app::work::jobs::{find_repositories, read_repository};
use git::CancelToken;
use model::graph::PageSize;
use render::ThemeName;
use testkit::{FIRST_COMMIT_TIME, Repository};

use super::support::{Window, run};

/// How many commits the repository has.
const COMMITS: u64 = 5_000;
/// How many files the commits change, one each, in turn.
const FILES: u64 = 200;
/// How many branches and how many tags, spread over the history.
const REFERENCES: u64 = 50;

/// The stream `git fast-import` reads: one commit a minute on `main`, and the branches and tags spread over it.
fn history() -> String {
    let mut stream = String::new();
    for number in 1..=COMMITS {
        let content = format!("content {number}\n");
        let message = format!("Change {number}\n");
        let date = FIRST_COMMIT_TIME + number * 60;
        let file = number % FILES;
        write!(
            stream,
            "commit refs/heads/main\nmark :{number}\ncommitter Test Author <author@example.com> {date} +0000\n\
             data {}\n{message}M 644 inline src/file{file}.txt\ndata {}\n{content}\n",
            message.len(),
            content.len(),
        )
        .unwrap();
    }
    for number in 1..=REFERENCES {
        let mark = number * (COMMITS / REFERENCES);
        write!(stream, "reset refs/heads/branch-{number}\nfrom :{mark}\n\n").unwrap();
        write!(stream, "reset refs/tags/v{number}\nfrom :{mark}\n\n").unwrap();
    }
    stream
}

/// Makes the history in the repository and checks it out.
fn import(repository: &Repository) {
    let mut child = Command::new("git")
        .args(["fast-import", "--quiet"])
        .current_dir(repository.path())
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(history().as_bytes()).unwrap();
    assert!(child.wait().unwrap().success(), "git fast-import failed");
    let _ = repository.git(&["reset", "--quiet", "--hard", "main"]);
}

#[test]
#[ignore = "makes a repository of 5,000 commits; run by hand to measure"]
fn opening_a_repository_of_5000_commits() {
    let repository = Repository::new();
    import(&repository);
    let folder = repository.path().to_path_buf();
    for attempt in 1..=3 {
        let started = Instant::now();
        let _found = find_repositories(std::slice::from_ref(&folder), &CancelToken::new());
        let found = started.elapsed();
        let (state, commits) = read_repository(&folder, PageSize::DEFAULT, &CancelToken::new()).unwrap();
        let read = started.elapsed();
        assert_eq!(state.branches.len(), 51);
        assert_eq!(commits.rows.len(), 1_000);
        let mut window_state = State::default();
        let _effects = run(
            &mut window_state,
            Message::Started {
                folders: vec![folder.clone()],
            },
        );
        let mut window = Window::new(ThemeName::Dark, (1280, 800));
        let drawn = Instant::now();
        let _ = window.draw(&window_state, &[]);
        println!(
            "attempt {attempt}: repositories found in {found:?}, state and first page read in {:?}, \
             first frame drawn in {:?}",
            read.checked_sub(found).unwrap(),
            drawn.elapsed()
        );
    }
}
