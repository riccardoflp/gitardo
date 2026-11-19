//! `commit.read-authors` against throwaway repositories whose commits are made by several people.

use git::Git;
use git::read::commit::{AuthorsRequest, read_authors};
use model::commit::Author;
use testkit::Repository;

/// Makes an empty commit by an author.
fn commit_by(repository: &Repository, author: &str, message: &str) {
    let _ = repository.git(&["commit", "--quiet", "--allow-empty", "-m", message, "--author", author]);
}

/// The authors of a repository, read with the configuration of the tests alone.
fn authors(repository: &Repository) -> Vec<Author> {
    let folder = repository.path().parent().unwrap();
    let git = Git::new(repository.path())
        .with_variable("GIT_CONFIG_GLOBAL", folder.join("global.gitconfig"))
        .with_variable("GIT_CONFIG_NOSYSTEM", "1");
    read_authors(&git, &AuthorsRequest).unwrap()
}

/// Each author as its name, its email and its count.
fn said(authors: &[Author]) -> Vec<(&str, &str, usize)> {
    authors
        .iter()
        .map(|author| (author.name.as_str(), author.email.as_str(), author.commits))
        .collect()
}

#[test]
fn commit_read_authors__three_commits_of_ben_and_one_of_cy_give_ben_with_3_and_cy_with_1() {
    let repository = Repository::new();
    commit_by(&repository, "Cy <cy@example.com>", "Add the readme");
    for message in ["Fix the parser", "Fix the lexer", "Fix the printer"] {
        commit_by(&repository, "Ben <ben@example.com>", message);
    }
    assert_eq!(
        said(&authors(&repository)),
        [("Ben", "ben@example.com", 3), ("Cy", "cy@example.com", 1)]
    );
}

#[test]
fn commit_read_authors__two_cases_of_one_email_are_one_author_named_as_the_latest_commit_says() {
    let repository = Repository::new();
    commit_by(&repository, "Ben Moss <Ben@Example.com>", "Add the readme");
    commit_by(&repository, "Ben <ben@example.com>", "Fix the parser");
    let read = authors(&repository);
    assert_eq!(read.len(), 1, "{read:?}");
    assert_eq!((read[0].name.as_str(), read[0].commits), ("Ben", 2));
}

#[test]
fn commit_read_authors__a_mailmap_that_maps_an_old_email_to_a_new_one_gives_the_new_one_alone() {
    let mut repository = Repository::new();
    commit_by(&repository, "Ben <ben@old.example.com>", "Add the readme");
    let _ = repository.commit(
        "Map the old email",
        &[(".mailmap", "Ben <ben@example.com> <ben@old.example.com>\n")],
    );
    commit_by(&repository, "Ben <ben@example.com>", "Fix the parser");
    let read = authors(&repository);
    let emails: Vec<&str> = read.iter().map(|author| author.email.as_str()).collect();
    assert!(emails.contains(&"ben@example.com"), "{read:?}");
    assert!(!emails.contains(&"ben@old.example.com"), "{read:?}");
    let ben = read.iter().find(|author| author.email == "ben@example.com").unwrap();
    assert_eq!(ben.commits, 2);
}
