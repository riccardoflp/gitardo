//! The operations of the submodules run by `gitardo-cli` on throwaway repositories, with submodules cloned from other
//! throwaway repositories on disk: git refuses those unless allowed, so the operations that clone are confirmed with
//! `--yes`, which takes the offer of `submodule.allow-file-protocol`.

use std::path::Path;

use git::Git;
use git::read::submodule::{SubmoduleListRequest, read_list};
use model::submodule::{Submodule, SubmoduleStatus, status};
use testkit::Repository;

use crate::{cli, cli_done, revision};

/// A repository of its own with commits, to be added as a submodule.
fn library(commits: usize) -> Repository {
    let mut library = Repository::new();
    for number in 1..=commits {
        let _commit = library.commit(
            &format!("Change the library {number}"),
            &[("lib.txt", &number.to_string())],
        );
    }
    library
}

/// The path of a repository as `git submodule add` takes it.
fn url_of(repository: &Repository) -> String {
    repository.path().to_string_lossy().replace('\\', "/")
}

/// Adds a repository as a submodule in a folder, allowed to clone from a folder on disk for this command only.
fn add(repository: &Repository, library: &Repository, folder: &str) {
    let url = url_of(library);
    let _added = repository.git(&[
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        "--quiet",
        &url,
        folder,
    ]);
}

/// A repository with one commit and the submodule `lib`, from a library with so many commits, committed.
fn with_lib(commits: usize) -> (Repository, Repository) {
    let mut repository = Repository::new();
    let _first = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let library = library(commits);
    add(&repository, &library, "lib");
    let _committed = repository.commit("Add the library", &[]);
    (repository, library)
}

/// The submodules of a repository's folder, as Gitardo reads them.
fn submodules(folder: &Path) -> Vec<Submodule> {
    let request = SubmoduleListRequest {
        folder: folder.to_path_buf(),
    };
    match read_list(&Git::new(folder), &request) {
        Ok(found) => found,
        Err(error) => panic!("could not read the submodules of {}: {error}", folder.display()),
    }
}

/// Where the submodule in a folder stands.
fn status_of(repository: &Repository, folder: &str) -> SubmoduleStatus {
    let found = submodules(repository.path());
    match found.iter().find(|submodule| submodule.folder == folder) {
        Some(submodule) => status(submodule),
        None => panic!("{folder} is not a submodule: {found:?}"),
    }
}

/// Whether a folder of the repository has anything in it.
fn has_files(repository: &Repository, folder: &str) -> bool {
    std::fs::read_dir(repository.path().join(folder)).is_ok_and(|mut entries| entries.next().is_some())
}

#[test]
fn submodule_update_to_recorded__on_lib_one_commit_behind_it_runs_the_update_and_lib_is_on_the_recorded_commit() {
    let (repository, _library) = with_lib(2);
    let _back = repository.git(&["-C", "lib", "checkout", "--quiet", "HEAD~1"]);
    assert!(matches!(
        status_of(&repository, "lib"),
        SubmoduleStatus::Behind { commits: 1, .. }
    ));
    let ended = cli_done(
        &repository,
        &["--target", "submodule:lib", "submodule.update-to-recorded"],
    );
    assert!(
        ended.out.contains("> git submodule update --init --recursive -- lib"),
        "{}",
        ended.out
    );
    assert!(matches!(status_of(&repository, "lib"), SubmoduleStatus::OnIt { .. }));
}

#[test]
fn submodule_stage_recorded__committing_after_it_records_the_commit_and_lib_is_on_it() {
    let (repository, _library) = with_lib(1);
    for message in ["First change", "Second change"] {
        let _ = repository.git(&["-C", "lib", "commit", "--quiet", "--allow-empty", "-m", message]);
    }
    let in_lib = repository.git(&["-C", "lib", "rev-parse", "HEAD"]).trim().to_owned();
    let ended = cli_done(&repository, &["--target", "submodule:lib", "submodule.stage-recorded"]);
    assert!(ended.out.contains("> git add -- lib"), "{}", ended.out);
    let _committed = repository.git(&["commit", "--quiet", "-m", "Record the library"]);
    assert_eq!(revision(&repository, "HEAD:lib"), in_lib);
    assert!(matches!(status_of(&repository, "lib"), SubmoduleStatus::OnIt { .. }));
}

#[test]
fn submodule_update_to_latest__fetches_the_three_commits_of_its_branch_and_lib_is_3_ahead() {
    let mut repository = Repository::new();
    let _first = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let mut library = library(1);
    let url = url_of(&library);
    let _added = repository.git(&[
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        "--quiet",
        "-b",
        "main",
        &url,
        "lib",
    ]);
    let _committed = repository.commit("Add the library", &[]);
    for number in 2..=4 {
        let _more = library.commit(&format!("More {number}"), &[("lib.txt", &number.to_string())]);
    }
    let ended = cli_done(
        &repository,
        &["--yes", "--target", "submodule:lib", "submodule.update-to-latest"],
    );
    assert!(ended.out.contains("Update lib to the latest of main"), "{}", ended.out);
    assert!(
        matches!(status_of(&repository, "lib"), SubmoduleStatus::Ahead { commits: 3, .. }),
        "{:?}",
        status_of(&repository, "lib")
    );
}

/// A repository whose submodule `lib` has a submodule `inner` of its own, and a clone of it made without
/// `--recursive`, where `lib` is not initialized.
fn cloned_without_recursive() -> (Repository, Repository, Repository, std::path::PathBuf) {
    let inner = library(1);
    let mut library = library(1);
    add(&library, &inner, "inner");
    let _with_inner = library.commit("Add inner", &[]);
    let mut repository = Repository::new();
    let _first = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    add(&repository, &library, "lib");
    let _committed = repository.commit("Add the library", &[]);
    let clone = repository.path().with_file_name("clone");
    let _cloned = repository.git(&["clone", "--quiet", ".", &clone.to_string_lossy()]);
    (repository, library, inner, clone)
}

#[test]
fn submodule_initialize__clones_lib_not_initialized_after_a_clone_and_lib_is_on_its_recorded_commit() {
    let (repository, _library, _inner, clone) = cloned_without_recursive();
    assert_eq!(submodules(&clone)[0].checkout, None);
    let folder = clone.to_string_lossy().into_owned();
    let ended = cli_done(
        &repository,
        &[
            "--repository",
            &folder,
            "--yes",
            "--target",
            "submodule:lib",
            "submodule.initialize",
        ],
    );
    assert!(ended.out.contains("Initialize lib"), "{}", ended.out);
    let found = submodules(&clone);
    assert!(matches!(status(&found[0]), SubmoduleStatus::OnIt { .. }), "{found:?}");
}

#[test]
fn submodule_initialize__its_own_submodules_are_initialized_too() {
    let (repository, _library, _inner, clone) = cloned_without_recursive();
    let folder = clone.to_string_lossy().into_owned();
    let _ended = cli_done(
        &repository,
        &[
            "--repository",
            &folder,
            "--yes",
            "--target",
            "submodule:lib",
            "submodule.initialize",
        ],
    );
    let found = submodules(&clone);
    let inner = found.iter().find(|submodule| submodule.folder == "lib/inner").unwrap();
    assert!(inner.checkout.is_some(), "{found:?}");
    assert!(clone.join("lib").join("inner").join("lib.txt").is_file());
}

#[test]
fn submodule_deinitialize__on_a_clean_lib_it_empties_its_folder_and_leaves_gitmodules_unchanged() {
    let (repository, _library) = with_lib(1);
    let gitmodules = std::fs::read_to_string(repository.path().join(".gitmodules")).unwrap();
    let asked = cli(&repository, &["--target", "submodule:lib", "submodule.deinitialize"]);
    assert_eq!(asked.code, Some(2), "it asks first: {}", asked.out);
    let ended = cli_done(
        &repository,
        &["--yes", "--target", "submodule:lib", "submodule.deinitialize"],
    );
    assert!(ended.out.contains("> git submodule deinit -- lib"), "{}", ended.out);
    assert!(!has_files(&repository, "lib"));
    assert_eq!(
        std::fs::read_to_string(repository.path().join(".gitmodules")).unwrap(),
        gitmodules
    );
}

#[test]
fn submodule_deinitialize__with_a_modified_file_git_s_refusal_is_followed_by_the_offer_with_force() {
    let (repository, _library) = with_lib(1);
    std::fs::write(repository.path().join("lib").join("lib.txt"), "changed\n").unwrap();
    let ended = cli_done(
        &repository,
        &["--yes", "--target", "submodule:lib", "submodule.deinitialize"],
    );
    assert!(
        ended.out.contains("> git submodule deinit --force -- lib"),
        "{}",
        ended.out
    );
    assert!(!has_files(&repository, "lib"));
}

#[test]
fn submodule_deinitialize__after_it_initialize_brings_lib_back() {
    let (repository, _library) = with_lib(1);
    let _emptied = cli_done(
        &repository,
        &["--yes", "--target", "submodule:lib", "submodule.deinitialize"],
    );
    assert_eq!(submodules(repository.path())[0].checkout, None, "Initialize is offered");
    let _back = cli_done(
        &repository,
        &["--yes", "--target", "submodule:lib", "submodule.initialize"],
    );
    assert!(repository.path().join("lib").join("lib.txt").is_file());
}

#[test]
fn submodule_remove__on_lib_it_deinitializes_removes_and_deletes_its_clone() {
    let (repository, _library) = with_lib(1);
    let clone = repository.path().join(".git").join("modules").join("lib");
    assert!(clone.is_dir());
    let ended = cli_done(&repository, &["--yes", "--target", "submodule:lib", "submodule.remove"]);
    for line in [
        "> git submodule deinit --force -- lib",
        "> git rm --force -- lib",
        "> delete ",
    ] {
        assert!(ended.out.contains(line), "{line}: {}", ended.out);
    }
    assert!(ended.out.contains("modules"), "{}", ended.out);
    assert!(!clone.exists());
}

#[test]
fn submodule_remove__gitmodules_no_longer_has_lib_and_both_changes_are_staged() {
    let (repository, _library) = with_lib(1);
    let _ended = cli_done(&repository, &["--yes", "--target", "submodule:lib", "submodule.remove"]);
    let gitmodules = std::fs::read_to_string(repository.path().join(".gitmodules")).unwrap_or_default();
    assert!(!gitmodules.contains("lib"), "{gitmodules}");
    let staged = repository.git(&["diff", "--cached", "--name-status"]);
    assert!(staged.contains("D\tlib"), "{staged}");
    assert!(staged.contains(".gitmodules"), "{staged}");
}

#[test]
fn submodule_remove__adding_a_submodule_at_lib_again_clones_it_anew() {
    let (repository, library) = with_lib(1);
    let _ended = cli_done(&repository, &["--yes", "--target", "submodule:lib", "submodule.remove"]);
    let _committed = repository.git(&["commit", "--quiet", "-m", "Remove the library"]);
    let url = url_of(&library);
    let added = repository
        .command("git")
        .args(["-c", "protocol.file.allow=always", "submodule", "add", &url, "lib"])
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&added.stderr);
    assert!(added.status.success(), "{said}");
    assert!(!said.contains("Reactivating"), "{said}");
}

#[test]
fn submodule_sync_urls__after_the_url_of_lib_changes_in_gitmodules_lib_s_origin_has_the_new_url() {
    let (repository, _library) = with_lib(1);
    let moved = library(1);
    let url = url_of(&moved);
    let _changed = repository.git(&["config", "--file", ".gitmodules", "submodule.lib.url", &url]);
    let ended = cli_done(&repository, &["--target", "submodule:lib", "submodule.sync-urls"]);
    assert!(ended.out.contains("> git submodule sync --recursive"), "{}", ended.out);
    assert_eq!(
        repository.git(&["-C", "lib", "remote", "get-url", "origin"]).trim(),
        url
    );
}

/// Adds the library as `lib` from `gitardo-cli`, with the answers given.
fn add_from_cli(repository: &Repository, library: &Repository, extra: &[&str]) -> crate::Ended {
    let url = format!("url={}", url_of(library));
    let mut arguments = vec!["--yes", "submodule.add", url.as_str(), "folder=lib", "branch="];
    arguments.extend(extra);
    cli(repository, &arguments)
}

#[test]
fn submodule_add__a_repository_whose_own_submodules_add_leaves_empty_has_them_initialized() {
    let inner = library(1);
    let mut library = library(1);
    add(&library, &inner, "inner");
    let _with_inner = library.commit("Add inner", &[]);
    let mut repository = Repository::new();
    let _first = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let ended = add_from_cli(&repository, &library, &[]);
    assert_eq!(ended.code, Some(0), "{}\n{}", ended.out, ended.errors);
    assert!(repository.path().join("lib").join("inner").join("lib.txt").is_file());
    let staged = repository.git(&["diff", "--cached", "--name-only"]);
    assert!(staged.contains(".gitmodules") && staged.contains("lib"), "{staged}");
}

#[test]
fn submodule_add__a_folder_with_files_in_it_is_refused() {
    let mut repository = Repository::new();
    let _first = repository.commit("Add lib", &[("lib/a.txt", "a\n")]);
    let library = library(1);
    let ended = add_from_cli(&repository, &library, &[]);
    assert_eq!(ended.code, Some(1), "{}", ended.out);
    assert!(ended.errors.contains("is not empty."), "{}", ended.errors);
    assert!(!repository.path().join(".gitmodules").exists());
}

#[test]
fn submodule_allow_file_protocol__adding_a_folder_on_disk_is_offered_again_and_succeeds_once_confirmed() {
    let mut repository = Repository::new();
    let _first = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let library = library(1);
    let refused = cli(
        &repository,
        &[
            "submodule.add",
            &format!("url={}", url_of(&library)),
            "folder=lib",
            "branch=",
        ],
    );
    assert_eq!(
        refused.code,
        Some(1),
        "without a confirmation the offer is not taken: {}",
        refused.out
    );
    let ended = add_from_cli(&repository, &library, &[]);
    assert_eq!(ended.code, Some(0), "{}\n{}", ended.out, ended.errors);
    assert!(
        ended
            .out
            .contains("> git -c protocol.file.allow=always submodule update --init --recursive -- lib"),
        "{}",
        ended.out
    );
    assert!(repository.path().join("lib").join("lib.txt").is_file());
}

#[test]
fn submodule_allow_file_protocol__afterwards_git_config_protocol_file_allow_gives_nothing() {
    let mut repository = Repository::new();
    let _first = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let library = library(1);
    let ended = add_from_cli(&repository, &library, &[]);
    assert_eq!(ended.code, Some(0), "{}\n{}", ended.out, ended.errors);
    for scope in ["--local", "--global"] {
        let read = repository
            .command("git")
            .args(["config", scope, "protocol.file.allow"])
            .output()
            .unwrap();
        assert!(
            !read.status.success(),
            "{scope}: {}",
            String::from_utf8_lossy(&read.stdout)
        );
    }
}

/// A repository whose `main` records the second commit of `lib`, and whose `feature` records its first.
fn recording_two_commits() -> Repository {
    let (repository, _library) = with_lib(2);
    let _branch = repository.git(&["checkout", "--quiet", "-b", "feature"]);
    let _back = repository.git(&["-C", "lib", "checkout", "--quiet", "HEAD~1"]);
    let _staged = repository.git(&["add", "lib"]);
    let _committed = repository.git(&["commit", "--quiet", "-m", "Go back in the library"]);
    let _main = repository.git(&["checkout", "--quiet", "main"]);
    let _updated = repository.git(&["submodule", "update", "--quiet"]);
    repository
}

#[test]
fn submodule_update_on_checkout__checking_out_a_branch_that_records_another_commit_takes_lib_to_it() {
    let repository = recording_two_commits();
    let ended = cli_done(&repository, &["--target", "branch:feature", "branch.checkout"]);
    let checkout = ended.out.find("> git checkout feature").unwrap();
    let update = ended.out.find("> git submodule update --init --recursive").unwrap();
    assert!(checkout < update, "{}", ended.out);
    assert_eq!(
        repository.git(&["-C", "lib", "rev-parse", "HEAD"]).trim(),
        revision(&repository, "feature:lib")
    );
}

#[test]
fn submodule_update_on_checkout__checking_out_a_commit_that_adds_a_submodule_clones_it() {
    let mut repository = Repository::new();
    let _first = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let _branch = repository.git(&["branch", "plain"]);
    let library = library(1);
    add(&repository, &library, "lib");
    let _committed = repository.commit("Add the library", &[]);
    let _plain = repository.git(&["checkout", "--quiet", "plain"]);
    let _gone = std::fs::remove_dir_all(repository.path().join("lib"));
    let _deinit = repository.git(&["submodule", "deinit", "--quiet", "--all", "--force"]);
    let _clone = std::fs::remove_dir_all(repository.path().join(".git").join("modules").join("lib"));
    let ended = cli_done(&repository, &["--yes", "--target", "branch:main", "branch.checkout"]);
    assert!(repository.path().join("lib").join("lib.txt").is_file(), "{}", ended.out);
}

#[test]
fn submodule_update_on_checkout__with_lib_two_commits_ahead_on_a_detached_head_the_checkout_asks_and_names_lib() {
    let repository = recording_two_commits();
    for message in ["First change", "Second change"] {
        let _ = repository.git(&["-C", "lib", "commit", "--quiet", "--allow-empty", "-m", message]);
    }
    let _detached = repository.git(&["-C", "lib", "checkout", "--quiet", "--detach"]);
    let asked = cli(&repository, &["--target", "branch:feature", "branch.checkout"]);
    assert_eq!(asked.code, Some(2), "it asks first: {}", asked.out);
    assert!(
        asked
            .out
            .contains("lib has 2 commits after the recorded one on a detached HEAD"),
        "{}",
        asked.out
    );
}

#[test]
fn submodule_update_on_checkout__in_a_repository_without_submodules_no_update_runs() {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a")]);
    let _branch = repository.git(&["branch", "feature"]);
    let ended = cli_done(&repository, &["--target", "branch:feature", "branch.checkout"]);
    assert!(!ended.out.contains("submodule"), "{}", ended.out);
}
