//! `graph.read-commits` and `graph.read-position` against throwaway repositories, and the labels of `graph.labels` on a
//! state read from one.

#![cfg(test)]
#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

#[path = "graph_read/golden.rs"]
mod golden;
#[path = "graph_read/speed.rs"]
mod speed;

use std::ffi::OsString;
use std::fmt::Write as _;
use std::io::Write as _;
use std::process::{Command, Stdio};

use git::Git;
use git::read::graph::{CommitPage, CommitsRequest, History, Parents, PositionRequest, read_commits, read_position};
use git::read::repo::{StateRequest, read_state};
use model::CommitId;
use model::graph::{GraphSettings, LabelKind, PageSize, Row, labels};
use model::state::RepositoryState;
use testkit::{FIRST_COMMIT_TIME, Remote, Repository};

/// What `filter.log-arguments` gives while the graph shows everything.
fn everything() -> Vec<OsString> {
    words(&["--date-order", "--branches", "--remotes", "--tags", "HEAD"])
}

/// What `filter.log-arguments` gives for **Current**.
fn current() -> Vec<OsString> {
    words(&["--date-order", "HEAD"])
}

fn words(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
}

/// The history of the selection, all parents followed, narrowed by nothing.
fn history(selection: Vec<OsString>) -> History {
    History {
        selection,
        narrowing: Vec::new(),
        parents: Parents::All,
    }
}

/// A request for so many pages of a history.
fn request(history: History, page_size: PageSize, pages: usize) -> CommitsRequest {
    CommitsRequest {
        history,
        page_size,
        pages,
        stashes: Vec::new(),
    }
}

/// Reads the commits of a throwaway repository.
fn read(repository: &Repository, request: &CommitsRequest) -> CommitPage {
    read_commits(&Git::new(repository.path()), request).unwrap()
}

/// The state of a throwaway repository.
fn state(repository: &Repository) -> RepositoryState {
    let request = StateRequest {
        folder: repository.path().to_path_buf(),
    };
    read_state(&Git::new(repository.path()), &request).unwrap()
}

/// The commits of the rows, without the stashes.
fn commits(page: &CommitPage) -> Vec<&model::commit::Commit> {
    page.rows
        .iter()
        .filter_map(|row| match row {
            Row::Commit(commit) => Some(commit),
            Row::Stash { .. } | Row::Uncommitted { .. } => None,
        })
        .collect()
}

fn commit_id(text: &str) -> CommitId {
    CommitId::parse(text.trim()).unwrap()
}

/// Makes a line of so many commits on `main` with `git fast-import`, one a minute, and checks it out: far faster than
/// a commit at a time.
pub(crate) fn import_line(repository: &Repository, count: u64) {
    let mut stream = String::new();
    for number in 1..=count {
        let message = format!("Change {number}\n");
        let date = FIRST_COMMIT_TIME + number * 60;
        write!(
            stream,
            "commit refs/heads/main\nmark :{number}\ncommitter Test Author <author@example.com> {date} +0000\n\
             data {}\n{message}M 644 inline file.txt\ndata {}\n{number}\n\n",
            message.len(),
            number.to_string().len() + 1,
        )
        .unwrap();
    }
    let mut child = Command::new("git")
        .args(["fast-import", "--quiet"])
        .current_dir(repository.path())
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stream.as_bytes()).unwrap();
    assert!(child.wait().unwrap().success(), "git fast-import failed");
    let _ = repository.git(&["reset", "--quiet", "--hard", "main"]);
}

#[test]
fn graph_read_commits__one_page_of_2500_commits_reads_1000_and_three_read_them_all() {
    let repository = Repository::new();
    import_line(&repository, 2_500);
    let one = read(&repository, &request(history(everything()), PageSize::DEFAULT, 1));
    assert_eq!(one.rows.len(), 1_000);
    assert!(one.has_more);
    let three = read(&repository, &request(history(everything()), PageSize::DEFAULT, 3));
    assert_eq!(three.rows.len(), 2_500);
    assert!(!three.has_more);
    assert_eq!(commits(&three)[0].subject, "Change 2500");
}

#[test]
fn graph_read_commits__a_page_size_of_50_reads_100_and_one_that_is_not_a_number_1000() {
    let repository = Repository::new();
    import_line(&repository, 1_200);
    let small = read(
        &repository,
        &request(history(current()), PageSize::from_setting(Some(50.0)), 1),
    );
    assert_eq!(small.rows.len(), 100);
    let not_a_number = read(
        &repository,
        &request(history(current()), PageSize::from_setting(None), 1),
    );
    assert_eq!(not_a_number.rows.len(), 1_000);
    assert!(not_a_number.has_more);
}

#[test]
fn graph_read_commits__a_merge_of_three_parents_has_three_or_with_the_first_parent_one() {
    let mut repository = Repository::new();
    let base = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    for name in ["left", "right"] {
        repository.branch(name);
        repository.checkout(name);
        let _ = repository.commit(&format!("Change on {name}"), &[(&format!("{name}.txt"), "1\n")]);
        repository.checkout("main");
    }
    let _ = repository.commit("Change on main", &[("main.txt", "1\n")]);
    let _ = repository.git(&["merge", "--quiet", "--no-ff", "--no-edit", "left", "right"]);
    let all = read(&repository, &request(history(current()), PageSize::DEFAULT, 1));
    assert_eq!(commits(&all)[0].parents.len(), 3);
    assert_eq!(commits(&all).len(), 5);
    let mut first_parent = history(current());
    first_parent.parents = Parents::First;
    let first = read(&repository, &request(first_parent, PageSize::DEFAULT, 1));
    assert_eq!(commits(&first)[0].parents.len(), 1);
    assert_eq!(
        commits(&first).len(),
        3,
        "the commits of left and right are not on the line of main"
    );
    assert_eq!(commits(&first)[2].id, commit_id(&base));
}

#[test]
fn graph_read_commits__messages_with_a_tab_a_body_and_accents_are_read_whole() {
    let mut repository = Repository::new();
    let _ = repository.commit(
        "Fix\tthe parser of café\n\nThe body: a line\nand another",
        &[("a.txt", "1\n")],
    );
    let _ = repository.commit("Ajouter l'analyseur — 解析", &[("b.txt", "1\n")]);
    let page = read(&repository, &request(history(everything()), PageSize::DEFAULT, 1));
    let subjects: Vec<&str> = commits(&page).iter().map(|commit| commit.subject.as_str()).collect();
    assert_eq!(subjects, ["Ajouter l'analyseur — 解析", "Fix\tthe parser of café"]);
}

#[test]
fn graph_read_commits__with_the_stashes_off_no_stash_commit_is_read() {
    let mut repository = Repository::new();
    let base = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    std::fs::write(repository.path().join("parser.rs"), "2\n").unwrap();
    std::fs::write(repository.path().join("new.txt"), "new\n").unwrap();
    let _ = repository.git(&["stash", "push", "--quiet", "--include-untracked"]);
    let stashes = state(&repository).stashes;
    let off = read(&repository, &request(history(everything()), PageSize::DEFAULT, 1));
    assert_eq!(off.rows.len(), 1);
    assert_eq!(off.rows[0].id().map(CommitId::as_str), Some(base.as_str()));
    let mut on = request(history(everything()), PageSize::DEFAULT, 1);
    on.stashes.clone_from(&stashes);
    let on = read(&repository, &on);
    assert_eq!(on.rows.len(), 2);
    let Row::Stash { commit, index } = &on.rows[0] else {
        panic!("not a stash: {:?}", on.rows[0]);
    };
    assert_eq!((commit.id.clone(), *index), (stashes[0].commit.clone(), 0));
    assert_eq!(commit.parents, [commit_id(&base)]);
}

#[test]
fn graph_read_commits__a_search_leaves_the_stashes_out_and_an_empty_repository_has_no_commit() {
    let mut repository = Repository::new();
    let empty = read(
        &repository,
        &request(
            history(words(&["--date-order", "--branches", "--remotes", "--tags"])),
            PageSize::DEFAULT,
            1,
        ),
    );
    assert_eq!(empty, CommitPage::default());
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    std::fs::write(repository.path().join("parser.rs"), "2\n").unwrap();
    let _ = repository.git(&["stash", "push", "--quiet"]);
    let mut searched = request(history(everything()), PageSize::DEFAULT, 1);
    searched.history.narrowing = words(&["--fixed-strings", "--regexp-ignore-case", "--grep=parser"]);
    searched.stashes = state(&repository).stashes;
    let page = read(&repository, &searched);
    assert_eq!(page.rows.len(), 1);
    assert!(matches!(page.rows[0], Row::Commit(_)));
}

/// The position of a commit in a history of a throwaway repository.
fn position(repository: &Repository, selection: Vec<OsString>, commit: &str) -> Option<usize> {
    let request = PositionRequest {
        history: history(selection),
        commit: commit_id(commit),
    };
    read_position(&Git::new(repository.path()), &request).unwrap()
}

#[test]
fn graph_read_position__the_2345th_commit_is_at_position_2344() {
    let repository = Repository::new();
    import_line(&repository, 2_500);
    let wanted = repository.git(&["rev-list", "--max-count=1", "--skip=2344", "HEAD"]);
    assert_eq!(position(&repository, everything(), &wanted), Some(2_344));
}

#[test]
fn graph_read_position__a_commit_on_a_branch_not_shown_has_none_until_all_are() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    repository.branch("feature");
    repository.checkout("feature");
    let feature = repository.commit("Change on feature", &[("feature.txt", "1\n")]);
    repository.checkout("main");
    let _ = repository.commit("Change on main", &[("main.txt", "1\n")]);
    assert_eq!(position(&repository, current(), &feature), None);
    assert_eq!(position(&repository, everything(), &feature), Some(1));
}

#[test]
fn graph_read_position__a_commit_no_branch_or_tag_reaches_has_none() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let lost = repository.commit("Change lost to a reset", &[("parser.rs", "2\n")]);
    let _ = repository.git(&["reset", "--quiet", "--hard", "HEAD~1"]);
    assert_eq!(position(&repository, everything(), &lost), None);
}

#[test]
fn graph_labels__an_annotated_tag_labels_the_commit_and_origin_head_has_no_label() {
    let mut repository = Repository::new();
    let commit = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let _remote = Remote::new(&repository);
    let _ = repository.git(&["remote", "set-head", "origin", "main"]);
    let _ = repository.git(&["tag", "--annotate", "v1.0", "--message", "The first release"]);
    let found = labels(&state(&repository), &GraphSettings::default());
    assert_eq!(found.len(), 1, "{found:?}");
    let at_commit = &found[&commit_id(&commit)];
    let names: Vec<&str> = at_commit.iter().map(|label| label.name.as_str()).collect();
    assert_eq!(names, ["main", "v1.0"]);
    assert!(matches!(at_commit[0].kind, LabelKind::Branch { upstream: Some(_), .. }));
}
