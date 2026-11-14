//! How long the inspector and the page of a commit take with large changes: a commit that touches 2,000 files, and a
//! commit whose one file changes 50,000 lines. Each read is timed from the intent to the details taken by the store,
//! and each frame from the state to the pixels. Not run by default:
//!
//! ```text
//! cargo test -p app --release --test places large_diffs -- --ignored --nocapture
//! ```

use std::time::{Duration, Instant};

use app::store::{Message, State};
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::commit_page::CommitPageIntent;
use views::details::{DetailsIntent, FileLayout, FilesIntent};
use views::graph::{GraphIntent, Selected};

use super::commit_page::lines;
use super::inspector::row_id;
use super::showing;
use super::support::{Window, run};

/// The longest a read of the details or of a page may take, from the intent to the details taken.
const READ_AT_MOST: Duration = Duration::from_millis(3_000);
/// The longest a frame may take.
const FRAME_AT_MOST: Duration = Duration::from_millis(100);

/// Runs something and says how long it took.
#[allow(clippy::print_stdout, reason = "the measure is printed")]
fn timed<R>(what: &str, run: impl FnOnce() -> R) -> (R, Duration) {
    let start = Instant::now();
    let returned = run();
    let took = start.elapsed();
    println!("{what}: {took:?}");
    (returned, took)
}

/// The slowest of a few frames of a window drawn from a store.
fn frames(what: &str, state: &State) -> Duration {
    let mut window = Window::new(ThemeName::Dark, (1200, 800));
    let mut slowest = Duration::ZERO;
    for frame in 0..3 {
        let (_, took) = timed(&format!("{what}, frame {frame}"), || window.draw(state, &[]));
        slowest = slowest.max(took);
    }
    slowest
}

/// Selects a row, opens its page and shows its changes, timing each read and the frames after it.
fn measure(state: &mut State, subject: &str) {
    let commit = row_id(state, subject);
    let select = Intent::Graph(GraphIntent::Select(Selected::Commit(commit.clone())));
    let (_, read) = timed("inspector read", || run(state, Message::Intent(select)));
    assert!(read < READ_AT_MOST);
    assert!(frames("inspector as a list", state) < FRAME_AT_MOST);
    let tree = Intent::Details(DetailsIntent::Files(FilesIntent::Layout(FileLayout::Tree)));
    let _ = run(state, Message::Intent(tree));
    assert!(frames("inspector as a tree", state) < FRAME_AT_MOST);
    let (_, read) = timed("page read", || {
        run(state, Message::Intent(Intent::Details(DetailsIntent::OpenPage(commit))))
    });
    assert!(read < READ_AT_MOST);
    assert!(frames("page", state) < FRAME_AT_MOST);
    let Some(views::page::Page::Commit(id)) = state.page_shown() else {
        panic!("the page is shown");
    };
    let show = Intent::CommitPage(id, CommitPageIntent::ShowChanges(true));
    let _ = run(state, Message::Intent(show));
    assert!(frames("page with its changes", state) < FRAME_AT_MOST);
    let all = Intent::CommitPage(id, CommitPageIntent::Files(FilesIntent::ShowAll));
    let _ = run(state, Message::Intent(all));
    let unfold = Intent::CommitPage(id, CommitPageIntent::FoldAll(model::commit_page::Fold::Unfolded));
    let _ = run(state, Message::Intent(unfold));
    assert!(frames("page with every file and every diff unfolded", state) < FRAME_AT_MOST);
}

#[test]
#[ignore = "a measure of speed, run in release"]
fn a_commit_that_touches_2000_files_is_shown_fast() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    for index in 0..2_000 {
        let folder = repository
            .path()
            .join(format!("src/part{}/module{}", index % 20, index % 7));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join(format!("file{index}.rs")), "fn main() {}\nfn more() {}\n").unwrap();
    }
    let _ = repository.git(&["add", "--all"]);
    let _ = repository.git(&["commit", "--quiet", "-m", "Touch two thousand files"]);
    let mut state = showing(repository.path().to_path_buf());
    measure(&mut state, "Touch two thousand files");
}

#[test]
#[ignore = "a measure of speed, run in release"]
fn a_file_of_50000_changed_lines_is_shown_fast() {
    let mut repository = Repository::new();
    let before = lines(50_000, "as it was, line");
    let _ = repository.commit("Add the long file", &[("data/long.txt", &before)]);
    let after = lines(50_000, "as it is now, line");
    let _ = repository.commit("Change every line of the long file", &[("data/long.txt", &after)]);
    let mut state = showing(repository.path().to_path_buf());
    measure(&mut state, "Change every line");
    let under = lines(19_000, "line");
    let _ = repository.commit("Add a file just under the cap", &[("data/under.txt", &under)]);
    let mut state = showing(repository.path().to_path_buf());
    measure(&mut state, "Add a file just under the cap");
}
