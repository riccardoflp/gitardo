//! `tag.read-remote-tags` against a throwaway repository and a bare remote made beside it.

use git::Git;
use git::read::tag::{RemoteTagsRequest, read_remote_tags};
use model::tag::RemoteTags;
use testkit::{Remote, Repository};

/// A repository with one commit and a remote `origin`.
fn with_remote() -> (Repository, Remote) {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let remote = Remote::new(&repository);
    (repository, remote)
}

/// The tags of a remote of a repository, read with the configuration of the tests alone.
fn read(repository: &Repository, remote: &str) -> Result<RemoteTags, git::GitError> {
    let folder = repository.path().parent().unwrap();
    let git = Git::new(repository.path())
        .with_variable("GIT_CONFIG_GLOBAL", folder.join("global.gitconfig"))
        .with_variable("GIT_CONFIG_NOSYSTEM", "1");
    read_remote_tags(
        &git,
        &RemoteTagsRequest {
            remote: remote.to_owned(),
        },
    )
}

#[test]
fn tag_read_remote_tags__a_lightweight_and_an_annotated_tag_are_both_given_the_annotated_with_its_tag_object() {
    let (repository, _remote) = with_remote();
    let _ = repository.git(&["tag", "v1"]);
    let _ = repository.git(&["tag", "-a", "v2", "-m", "Release v2"]);
    let _ = repository.git(&["push", "--quiet", "origin", "v1", "v2"]);
    let read = read(&repository, "origin").unwrap();
    let names: Vec<&str> = read.there.iter().map(|tag| tag.name.as_str()).collect();
    assert_eq!(names, ["v1", "v2"]);
    let object = repository.git(&["for-each-ref", "--format=%(objectname)", "refs/tags/v2"]);
    assert_eq!(read.there[1].object, object.trim());
    assert_eq!(read.here, read.there, "the repository has the same two");
}

#[test]
fn tag_read_remote_tags__a_remote_with_no_tag_gives_an_empty_list_and_no_error() {
    let (repository, _remote) = with_remote();
    let read = read(&repository, "origin").unwrap();
    assert!(read.there.is_empty(), "{read:?}");
}

#[test]
fn tag_read_remote_tags__a_remote_that_cannot_be_reached_gives_what_git_said_and_no_prompt() {
    let (repository, _remote) = with_remote();
    let nowhere = repository.path().parent().unwrap().join("nowhere.git");
    let _ = repository.git(&[
        "remote",
        "add",
        "nowhere",
        &nowhere.to_string_lossy().replace('\\', "/"),
    ]);
    let error = read(&repository, "nowhere").unwrap_err();
    assert!(error.to_string().contains("nowhere"), "{error}");
}
