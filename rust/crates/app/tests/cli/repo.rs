//! Repositories cloned and created by `gitardo-cli`, as the window makes them (`repo.clone`, `repo.init`): from a
//! throwaway repository on disk, into a folder beside it, never over the network.

use std::path::{Path, PathBuf};

use testkit::Repository;

use crate::{cli, cli_done};

/// A repository with some commits, to clone.
pub(crate) fn origin(commits: usize) -> Repository {
    let mut origin = Repository::new();
    for number in 1..=commits {
        let _commit = origin.commit(&format!("Change {number}"), &[("file.txt", &number.to_string())]);
    }
    origin
}

/// The URL of a repository on disk, as a clone takes it: `file://`, so that git honors the depth of a shallow clone.
pub(crate) fn file_url(repository: &Repository) -> String {
    let path = repository.path().to_string_lossy().replace('\\', "/");
    if path.starts_with('/') {
        format!("file://{path}")
    } else {
        format!("file:///{path}")
    }
}

/// A folder beside the repository, made empty, to clone or create in.
pub(crate) fn beside(repository: &Repository, name: &str) -> PathBuf {
    let folder = repository.path().parent().unwrap_or(repository.path()).join(name);
    std::fs::create_dir_all(&folder).unwrap_or_else(|error| panic!("could not make {}: {error}", folder.display()));
    folder
}

/// Clones a URL into `<parent>/lib` with `gitardo-cli`, ticking the options given.
pub(crate) fn clone_into(repository: &Repository, url: &str, parent: &Path, options: &[&str]) -> crate::Ended {
    let parent = format!("parent={}", parent.display());
    let url = format!("url={url}");
    let options = format!("options={}", options.join(","));
    let mut arguments = vec![
        "repo.clone",
        url.as_str(),
        parent.as_str(),
        "name=lib",
        options.as_str(),
    ];
    arguments.push("clipboard=");
    cli(repository, &arguments)
}

#[test]
fn repo_clone__a_url_that_does_not_exist_says_gits_message_and_leaves_no_folder() {
    let repository = Repository::new();
    let parent = beside(&repository, "clones");
    let missing = repository.path().parent().unwrap().join("missing");
    let url = format!(
        "file:///{}",
        missing.to_string_lossy().replace('\\', "/").trim_start_matches('/')
    );
    let ended = clone_into(&repository, &url, &parent, &[]);
    assert_eq!(ended.code, Some(1), "{}\n{}", ended.out, ended.errors);
    let said = format!("{}{}", ended.out, ended.errors);
    assert!(
        said.contains("does not appear to be a git repository") || said.contains("fatal"),
        "{said}"
    );
    assert!(!parent.join("lib").exists(), "git took away the folder it made");
}

#[test]
fn repo_clone__a_clone_of_a_repository_on_disk_has_its_history_and_its_files() {
    let origin = origin(3);
    let parent = beside(&origin, "clones");
    let _ended = cli_done(
        &origin,
        &[
            "repo.clone",
            &format!("url={}", file_url(&origin)),
            &format!("parent={}", parent.display()),
            "name=lib",
            "options=submodules",
            "clipboard=",
        ],
    );
    let clone = parent.join("lib");
    assert_eq!(std::fs::read_to_string(clone.join("file.txt")).unwrap(), "3");
    let count = git_in(&origin, &clone, &["rev-list", "--count", "HEAD"]);
    assert_eq!(count.trim(), "3");
}

#[test]
fn repo_init__with_init_default_branch_trunk_it_runs_git_init_and_the_repository_has_trunk_and_no_commit() {
    let repository = Repository::new();
    let _set = repository.git(&["config", "--global", "init.defaultBranch", "trunk"]);
    let folder = beside(&repository, "new");
    let ended = cli_done(&repository, &["repo.init", &format!("folder={}", folder.display())]);
    assert!(ended.out.contains("git init"), "{}", ended.out);
    let head = git_in(&repository, &folder, &["symbolic-ref", "HEAD"]);
    assert_eq!(head.trim(), "refs/heads/trunk");
    let commits = repository
        .command("git")
        .current_dir(&folder)
        .args(["rev-parse", "--verify", "--quiet", "HEAD"])
        .output()
        .unwrap();
    assert!(!commits.status.success(), "no commit yet");
}

/// Runs git in another folder, in the environment of a repository's commands.
pub(crate) fn git_in(repository: &Repository, folder: &Path, arguments: &[&str]) -> String {
    let output = repository
        .command("git")
        .current_dir(folder)
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("could not run git: {error}"));
    assert!(
        output.status.success(),
        "git {} failed: {}",
        arguments.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}
