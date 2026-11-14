//! `remote.read-remotes`, `sync.read-overwritten` and `stash.read-list` against throwaway repositories.

use git::Git;
use git::read::remote::{RemotesRequest, read_remotes};
use git::read::stash::{StashListRequest, read_list};
use git::read::sync::{OverwrittenRequest, read_overwritten};
use model::Timestamp;
use testkit::{FIRST_COMMIT_TIME, Remote, Repository};

use crate::write;

/// The remotes of a repository, by name and URL.
fn remotes(repository: &Repository) -> Vec<(String, String)> {
    read_remotes(&Git::new(repository.path()), &RemotesRequest)
        .unwrap()
        .into_iter()
        .map(|remote| (remote.name, remote.url))
        .collect()
}

#[test]
fn remote_read_remotes__origin_and_upstream_both_have_their_urls() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let remote = Remote::new(&repository);
    let _ = repository.git(&["remote", "add", "upstream", "https://example.com/team/app.git"]);
    let origin = remote.path().to_string_lossy().into_owned();
    assert_eq!(
        remotes(&repository),
        [
            (String::from("origin"), origin),
            (
                String::from("upstream"),
                String::from("https://example.com/team/app.git")
            )
        ]
    );
}

#[test]
fn remote_read_remotes__no_remote_is_an_empty_list() {
    let repository = Repository::new();
    assert_eq!(remotes(&repository), []);
}

#[test]
fn remote_read_remotes__a_url_changed_with_instead_of_comes_back_as_git_uses_it() {
    let repository = Repository::new();
    let _ = repository.git(&["remote", "add", "origin", "work:team/app.git"]);
    let _ = repository.git(&["config", "url.https://example.com/.insteadOf", "work:"]);
    assert_eq!(
        remotes(&repository),
        [(String::from("origin"), String::from("https://example.com/team/app.git"))]
    );
}

/// A repository whose upstream has so many commits it has not, and the subjects of those, oldest first.
fn behind_upstream(count: usize) -> (Repository, Vec<String>) {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "0\n")]);
    let _remote = Remote::new(&repository);
    let subjects: Vec<String> = (1..=count)
        .map(|index| format!("Change {index} on the remote"))
        .collect();
    for (index, subject) in subjects.iter().enumerate() {
        let _ = repository.commit(subject, &[("parser.rs", &format!("{}\n", index + 1))]);
    }
    if count > 0 {
        let _ = repository.git(&["push", "--quiet", "origin", "main"]);
        let _ = repository.git(&["reset", "--quiet", "--hard", &format!("HEAD~{count}")]);
    }
    (repository, subjects)
}

/// The subjects a force push would overwrite, and whether there are more.
fn overwritten(repository: &Repository) -> (Vec<String>, bool) {
    let request = OverwrittenRequest {
        upstream: String::from("origin/main"),
    };
    let read = read_overwritten(&Git::new(repository.path()), &request).unwrap();
    let subjects = read.commits.into_iter().map(|commit| commit.subject).collect();
    (subjects, read.has_more)
}

#[test]
fn sync_read_overwritten__the_upstream_two_ahead_gives_those_two_newest_first() {
    let (repository, subjects) = behind_upstream(2);
    let (read, has_more) = overwritten(&repository);
    assert_eq!(read, [subjects[1].clone(), subjects[0].clone()]);
    assert!(!has_more);
}

#[test]
fn sync_read_overwritten__fifteen_commits_give_ten_and_say_there_are_more() {
    let (repository, subjects) = behind_upstream(15);
    let (read, has_more) = overwritten(&repository);
    assert_eq!(read.len(), 10);
    assert_eq!(read[0], subjects[14]);
    assert!(has_more);
}

#[test]
fn sync_read_overwritten__nothing_only_on_the_upstream_gives_none() {
    let (repository, _) = behind_upstream(0);
    assert_eq!(overwritten(&repository), (Vec::new(), false));
}

#[test]
fn stash_read_list__two_stashes_newest_first_with_their_messages_and_dates() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    write(&repository, "parser.rs", "2\n");
    let _ = repository.git(&["stash", "push", "--quiet", "--message", "first"]);
    write(&repository, "parser.rs", "3\n");
    let _ = repository.git(&["stash", "push", "--quiet", "--message", "second"]);
    let stashes = read_list(&Git::new(repository.path()), &StashListRequest).unwrap();
    let read: Vec<(usize, &str)> = stashes
        .iter()
        .map(|stash| (stash.index, stash.message.as_str()))
        .collect();
    assert_eq!(read, [(0, "On main: second"), (1, "On main: first")]);
    // A stash gets the date of the next commit of the helpers, one minute after the first.
    let date = Timestamp(i64::try_from(FIRST_COMMIT_TIME).unwrap() + 60);
    assert!(stashes.iter().all(|stash| stash.date == date));
}

#[test]
fn stash_read_list__a_repository_with_no_stash_has_none() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    assert_eq!(read_list(&Git::new(repository.path()), &StashListRequest).unwrap(), []);
}

#[test]
fn stash_read_list__a_message_with_spaces_colons_and_quotes_is_whole() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    write(&repository, "parser.rs", "2\n");
    let message = "before the merge: \"draft\" it's [ok]";
    let _ = repository.git(&["stash", "push", "--quiet", "--message", message]);
    let stashes = read_list(&Git::new(repository.path()), &StashListRequest).unwrap();
    assert_eq!(stashes[0].message, format!("On main: {message}"));
}
