//! What the repositories open and their worktrees are on, read in the background (`repo.worktree-summary`): the
//! worktree `login` of `api`, not shown, read once the window opens, again when a file of it is saved, and not when
//! nothing changed; and its row under the row of `api` (`repo.group-tabs`). Then three repositories open, each on
//! its own branch with its own files changed and commits to pull and to push, and a worktree in two of them.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use app::store::{Effect, Message, State, ViewData};
use app::work::summaries::{SummaryAsk, read_summaries};
use git::read::worktree::git_folder_changed;
use model::repo::{Change, SummaryHead};
use platform::accesskit::TreeUpdate;
use render::ThemeName;
use testkit::Repository;
use views::graph::{GraphIntent, SideSection};
use views::repo::{TabSummary, counts_words, head_words};
use views::text::repo::TAB_MEMBERS as MEMBERS;

use super::pointer::rect;
use super::side_column::{column, git_in};
use super::support::{self, NOW, Window};
use super::tabs::{WIDE, clone_three};
use super::where_one_is::api_with_login;

/// What the row of a folder under the first row says it is on.
fn member_summary(state: &State, folder: &Path) -> Option<TabSummary> {
    let data = ViewData::new(state, Vec::new(), NOW);
    let side = data.window_state(state).side.unwrap();
    side.tabs
        .tabs
        .first()?
        .members
        .iter()
        .find(|member| app::persisted::same_folder(&member.folder, folder))
        .and_then(|member| member.summary.clone())
}

/// A file of the working tree changed, as the watcher of a folder says it.
fn saved(folder: &Path, name: &str) -> Message {
    Message::TabChanged {
        folder: folder.to_path_buf(),
        changes: vec![Change::WorkingTree(PathBuf::from(name))],
        at: Duration::from_secs(60),
    }
}

#[test]
fn repo_worktree_summary__a_worktree_not_shown_is_read_once_the_window_opens_and_watched() {
    let (_kept, _clones, login, state) = api_with_login();
    let summary = member_summary(&state, &login).unwrap();
    assert_eq!(summary.summary.head, SummaryHead::Branch(String::from("feature/login")));
    assert_eq!(counts_words(&summary), "");
    assert!(
        state
            .tabs
            .background
            .watched
            .iter()
            .any(|folder| app::persisted::same_folder(folder, &login)),
        "its folder is watched"
    );
}

#[test]
fn repo_worktree_summary__saving_a_file_in_a_worktree_not_shown_updates_its_count_within_two_seconds() {
    let (_kept, _clones, login, mut state) = api_with_login();
    // The window has been open a while: the first read of the worktree is more than two seconds old.
    std::thread::sleep(Duration::from_secs(2));
    std::fs::write(login.join("notes.txt"), "new\n").unwrap();
    let started = Instant::now();
    let effects = support::run(&mut state, saved(&login, "notes.txt"));
    assert!(effects.iter().any(|effect| matches!(effect, Effect::ReadSummaries(_))));
    assert!(started.elapsed() < Duration::from_secs(2), "{:?}", started.elapsed());
    assert_eq!(member_summary(&state, &login).unwrap().summary.changed, 1);
}

#[test]
fn repo_worktree_summary__nothing_changing_in_a_worktree_runs_no_read_of_it() {
    let (_kept, _clones, login, mut state) = api_with_login();
    let effects = support::run(
        &mut state,
        Message::Wake {
            at: Duration::from_secs(60),
        },
    );
    assert!(
        !effects.iter().any(|effect| matches!(effect, Effect::ReadSummaries(_))),
        "nothing changed, nothing is read"
    );
    let ask = SummaryAsk {
        folder: login.clone(),
        last: Some(git_folder_changed(&login, None).stamp),
        read_at: None,
        is_forced: false,
        lists_worktrees: false,
    };
    let read = read_summaries(&[ask]);
    assert_eq!(
        read[0].result, None,
        "a change in the git folder that leaves HEAD, the index and the refs runs no git"
    );
}

/// Writes files in a folder, each saying it changed.
fn change(folder: &Path, names: &[&str]) {
    for name in names {
        std::fs::write(folder.join(name), "changed\n").unwrap();
    }
}

/// What the side column says, from the top and from the left: each name or value drawn in it.
fn said_in_column(tree: &TreeUpdate) -> Vec<String> {
    let column = column(tree);
    let mut said: Vec<(f32, f32, String)> = tree
        .nodes
        .iter()
        .filter_map(|(_, node)| {
            let words = node.label().or(node.value())?.to_owned();
            let at = rect(node.bounds()?);
            Some((at.y, at.x, words))
        })
        .filter(|(y, x, _)| *y >= column.y && *y < column.bottom() && *x >= column.x && *x < column.right())
        .collect();
    said.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    said.into_iter().map(|(_, _, words)| words).collect()
}

/// The three clones, each on its own branch with its own files changed and commits to pull and to push: `api` on
/// `main`, one file changed, two commits to pull and one to push, with its worktree `login` on `feature/login`, which
/// follows `origin/main`, two files changed and one commit to push; `web` on `feature/search`, three files changed and
/// two commits to pull; `docs` on `main` with nothing changed and two commits to pull, with its worktree `notes` on a
/// branch with no upstream and one file changed. A window opened on them shows `api` and has shown neither of the others.
fn three_on_their_own() -> ((Repository, Repository), State) {
    let ((mut origin, holder), clones) = clone_three();
    let [api, web, docs] = &clones;
    let login = holder.path().join("clones").join("login");
    let notes = holder.path().join("clones").join("notes");
    let track = ["checkout", "--quiet", "-b", "feature/search", "--track", "origin/main"];
    git_in(&origin, web, &track);
    let _ = origin.commit("Pull the first", &[("pull.txt", "1\n")]);
    let _ = origin.commit("Pull the second", &[("pull.txt", "2\n")]);
    for clone in &clones {
        git_in(&origin, clone, &["fetch", "--quiet"]);
    }
    commit_in(&origin, api, "pushed.txt");
    change(api, &["README.md"]);
    let login_folder = login.to_str().unwrap();
    let add_login = [
        "worktree",
        "add",
        "--quiet",
        "--track",
        "-b",
        "feature/login",
        login_folder,
        "origin/main",
    ];
    git_in(&origin, api, &add_login);
    commit_in(&origin, &login, "login.txt");
    change(&login, &["README.md", "form.txt"]);
    change(web, &["one.txt", "two.txt", "three.txt"]);
    git_in(
        &origin,
        docs,
        &["worktree", "add", "--quiet", "-b", "notes", notes.to_str().unwrap()],
    );
    change(&notes, &["notes.txt"]);
    let mut state = super::window();
    let _ = support::run(
        &mut state,
        Message::Started {
            folders: clones.to_vec(),
        },
    );
    ((origin, holder), state)
}

/// Commits a new file in a folder.
fn commit_in(origin: &Repository, folder: &Path, name: &str) {
    change(folder, &[name]);
    git_in(origin, folder, &["add", name]);
    git_in(origin, folder, &["commit", "--quiet", "-m", "Add a file"]);
}

/// Each row of **Repositories**, by its name, its branch and its counts; and the entries of the **▾** of each, by their
/// labels and their counts.
fn rows_and_members(state: &State) -> (Vec<String>, Vec<Vec<String>>) {
    let data = ViewData::new(state, Vec::new(), NOW);
    let side = data.window_state(state).side.unwrap();
    let rows = side
        .tabs
        .tabs
        .iter()
        .map(|tab| {
            let summary = tab.summary.clone().unwrap();
            format!(
                "{} {} {}",
                tab.name,
                head_words(&summary.summary.head),
                counts_words(&summary)
            )
        })
        .collect();
    let members = side
        .tabs
        .tabs
        .iter()
        .map(|tab| {
            tab.members
                .iter()
                .map(|member| {
                    let counts = member.summary.as_ref().map(counts_words).unwrap_or_default();
                    format!("{} {counts}", member.label)
                })
                .collect()
        })
        .collect();
    (rows, members)
}

#[test]
fn repo_worktree_summary__with_three_repositories_open_each_row_and_each_worktree_under_it_says_its_own_branch_files_and_commits()
 {
    let (_kept, mut state) = three_on_their_own();
    let (rows, members) = rows_and_members(&state);
    assert_eq!(
        rows,
        [
            "api main \u{25cf} 1 \u{2193}2 \u{2191}1",
            "web feature/search \u{25cf} 3 \u{2193}2",
            "docs main \u{2193}2",
        ]
    );
    let api = [
        "api \u{25cf} 1 \u{2193}2 \u{2191}1",
        "feature/login \u{25cf} 2 \u{2191}1",
    ];
    let docs = ["docs \u{2193}2", "notes \u{25cf} 1"];
    assert_eq!(
        members,
        [api.to_vec(), Vec::new(), docs.to_vec()],
        "web has no other worktree"
    );
    super::inspector::ask(
        &mut state,
        views::Intent::Graph(GraphIntent::FoldSection(SideSection::Branches)),
    );
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let said = said_in_column(&tree);
    let rows_said = [
        "api",
        MEMBERS,
        "api",
        "main",
        "\u{2193}2 \u{2191}1",
        "Close tab",
        "login",
        "login",
        "feature/login",
        "\u{2191}1",
        "web",
        "web",
        "feature/search",
        "\u{2193}2",
        "docs",
        MEMBERS,
        "docs",
        "main",
        "\u{2193}2",
        "notes",
        "notes",
        "notes",
    ];
    assert!(
        said.windows(rows_said.len()).any(|window| window == rows_said),
        "each row says its branch and its commits to pull and to push: {said:?}"
    );
}
