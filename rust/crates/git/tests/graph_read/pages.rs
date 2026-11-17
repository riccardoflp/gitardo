//! `graph.read-commits` as git prints: the pages given one after the other by one git, the read stopped at 100,000
//! commits and the read after them, a read cancelled after its first page, and the stashes of each page.

use std::path::Path;

use git::read::graph::{
    CommitPage, CommitsRequest, READ_LIMIT, ReadEnd, parse_stash_commits, read_commits, stashes_arguments,
};
use git::{CancelToken, Git, GitError};
use model::graph::{PageSize, Row, place_stashes};
use testkit::{FIRST_COMMIT_TIME, Repository};

use crate::{commits, everything, history, import_line, request, state};

/// Reads the commits of a throwaway repository with a git, and gives each page and how the read ended.
fn read_pages(git: &Git, request: &CommitsRequest) -> (Vec<CommitPage>, Result<ReadEnd, GitError>) {
    let mut pages = Vec::new();
    let end = read_commits(git, request, &mut |page| pages.push(page));
    (pages, end)
}

/// A request for the whole read, as the window makes it: up to 100,000 commits, in pages of a size.
fn whole(page_size: PageSize) -> CommitsRequest {
    CommitsRequest {
        count: READ_LIMIT,
        ..request(history(everything()), page_size, 1)
    }
}

/// The git processes that started, from the events of trace2 written to a file: a process another git started is not
/// counted, and nor is one that is not a `git log`.
fn logs_started(trace: &Path) -> (usize, usize) {
    let events = std::fs::read_to_string(trace).unwrap_or_default();
    let starts: Vec<&str> = events
        .lines()
        .filter(|line| line.contains("\"event\":\"start\""))
        .filter(|line| {
            let sid = line.split("\"sid\":\"").nth(1).unwrap().split('"').next().unwrap();
            !sid.contains('/')
        })
        .collect();
    let logs = starts.iter().filter(|line| line.contains("\"log\"")).count();
    (starts.len(), logs)
}

#[test]
fn graph_read_commits__2500_commits_come_in_three_pages_the_first_before_git_printed_the_last_and_one_git_runs() {
    let repository = Repository::new();
    import_line(&repository, 2_500);
    let trace = repository.path().join(".git").join("trace.json");
    let git = Git::new(repository.path()).with_variable("GIT_TRACE2_EVENT", &trace);
    let (pages, end) = read_pages(&git, &whole(PageSize::DEFAULT));
    let sizes: Vec<usize> = pages.iter().map(|page| page.rows.len()).collect();
    assert_eq!(sizes, [1_000, 1_000, 500]);
    assert_eq!(end.unwrap(), ReadEnd { has_more: false });
    assert!(pages[0].has_more && pages[1].has_more && !pages[2].has_more);
    assert_eq!(commits(&pages[0])[0].subject, "Change 2500");
    assert_eq!(commits(&pages[2])[499].subject, "Change 1");
    assert_eq!(logs_started(&trace), (1, 1), "one git, a git log, for the commits");
}

#[test]
fn graph_read_commits__the_first_page_comes_while_git_still_prints_the_rest() {
    let repository = Repository::new();
    import_line(&repository, 2_500);
    let cancel = CancelToken::new();
    let git = Git::new(repository.path()).with_cancel(cancel.clone());
    let mut pages = 0;
    // Were the first page given only once git had printed everything, cancelling the read there would not stop the
    // pages after it: they would all be in hand already.
    let end = read_commits(&git, &whole(PageSize::DEFAULT), &mut |_page| {
        pages += 1;
        cancel.cancel();
    });
    assert!(matches!(end, Err(GitError::Cancelled { .. })), "{end:?}");
    assert_eq!(pages, 1);
}

#[test]
fn graph_read_commits__a_read_cancelled_after_its_first_page_gives_no_other_and_its_git_has_ended() {
    let repository = Repository::new();
    import_line(&repository, 5_000);
    let cancel = CancelToken::new();
    let git = Git::new(repository.path()).with_cancel(cancel.clone());
    let mut sizes = Vec::new();
    let end = read_commits(&git, &whole(PageSize::from_setting(Some(100.0))), &mut |page| {
        sizes.push(page.rows.len());
        cancel.cancel();
    });
    assert!(matches!(end, Err(GitError::Cancelled { .. })), "{end:?}");
    assert_eq!(sizes, [100]);
    // The git of the read was killed and waited for: the repository can be deleted at once, which Windows refuses
    // while a process still runs in it.
    let folder = repository.path().to_path_buf();
    drop(repository);
    assert!(!folder.exists());
}

#[test]
fn graph_read_commits__100050_commits_stop_at_100000_and_the_read_from_the_last_gives_the_50_left() {
    let repository = Repository::new();
    import_line(&repository, 100_050);
    let git = Git::new(repository.path());
    let (pages, end) = read_pages(&git, &whole(PageSize::from_setting(Some(10_000.0))));
    assert_eq!(end.unwrap(), ReadEnd { has_more: true });
    let read: usize = pages.iter().map(|page| page.rows.len()).sum();
    assert_eq!(read, 100_000);
    let last = pages.last().unwrap().rows.last().unwrap().id().unwrap().clone();
    let after = CommitsRequest {
        skip: READ_LIMIT - 1,
        ..whole(PageSize::from_setting(Some(10_000.0)))
    };
    let (rest, end) = read_pages(&git, &after);
    assert_eq!(end.unwrap(), ReadEnd { has_more: false });
    let rows: Vec<&Row> = rest.iter().flat_map(|page| &page.rows).collect();
    assert_eq!(rows.len(), 51, "the last commit read before, and the 50 left");
    assert_eq!(rows[0].id(), Some(&last));
    assert_eq!(commits(rest.last().unwrap()).last().unwrap().subject, "Change 1");
}

#[test]
fn graph_read_commits__with_pages_of_50_a_page_is_100_commits_and_with_abc_1000() {
    let repository = Repository::new();
    import_line(&repository, 1_200);
    let git = Git::new(repository.path());
    let (small, _) = read_pages(&git, &whole(PageSize::from_setting(Some(50.0))));
    assert_eq!(small.len(), 12);
    assert!(small.iter().all(|page| page.rows.len() == 100));
    let (not_a_number, _) = read_pages(&git, &whole(PageSize::from_setting(None)));
    let sizes: Vec<usize> = not_a_number.iter().map(|page| page.rows.len()).collect();
    assert_eq!(sizes, [1_000, 200]);
}

#[test]
fn graph_read_commits__the_stashes_of_each_page_are_where_they_would_be_among_all_the_commits() {
    let repository = Repository::new();
    import_line(&repository, 450);
    // A stash on the commit checked out; one on a commit of the third page made after every commit, so that its date
    // puts it at the top, above the first page; and one on a commit of the second page dated among the commits of the
    // page before it, from minute 351 to 450.
    for (back, file, minute) in [(0, "a.txt", 460), (250, "b.txt", 10_000), (150, "c.txt", 420)] {
        let _ = repository.git(&["checkout", "--quiet", &format!("main~{back}")]);
        std::fs::write(repository.path().join(file), "changed\n").unwrap();
        let _ = repository.git(&["add", "--", file]);
        let date = format!("@{} +0000", FIRST_COMMIT_TIME + minute * 60);
        let stashed = repository
            .command("git")
            .args(["stash", "push", "--quiet"])
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date)
            .status()
            .unwrap();
        assert!(stashed.success());
    }
    let _ = repository.git(&["checkout", "--quiet", "main"]);
    let stashes = state(&repository).stashes;
    assert_eq!(stashes.len(), 3);
    let git = Git::new(repository.path());
    let with_stashes = CommitsRequest {
        stashes: stashes.clone(),
        ..whole(PageSize::from_setting(Some(100.0)))
    };
    let (stashed_pages, _) = read_pages(&git, &with_stashes);
    let paged: Vec<Row> = stashed_pages.into_iter().flat_map(|page| page.rows).collect();
    let (plain, _) = read_pages(&git, &whole(PageSize::from_setting(Some(100.0))));
    let all: Vec<Row> = plain.into_iter().flat_map(|page| page.rows).collect();
    let stash_commits = parse_stash_commits(&git.run_read(&stashes_arguments(&stashes)).unwrap(), &stashes).unwrap();
    assert_eq!(paged, place_stashes(&all, &stash_commits));
    assert_eq!(paged.len(), 453);
}
