//! `worktree.read-list` and `worktree.read-folder-links` against throwaway repositories.

use std::fs;
use std::path::{Path, PathBuf};

use git::Git;
use git::read::worktree::{FolderLinksRequest, WorktreeListRequest, parse_worktrees, read_folder_links, read_list};
use model::state::Worktree;
use testkit::Repository;

use crate::{beside, name_of, write_in};

/// The top folder of a worktree, as git names it.
fn top(folder: &Path) -> PathBuf {
    let output = Git::new(folder).run_read(&["rev-parse", "--show-toplevel"]).unwrap();
    PathBuf::from(String::from_utf8(output).unwrap().trim_end())
}

/// The worktrees, read from the main one as the views show it.
fn read(repository: &Repository) -> Vec<Worktree> {
    let request = WorktreeListRequest {
        folder: top(repository.path()),
        uncommitted_here: Some(0),
    };
    read_list(&Git::new(repository.path()), &request).unwrap()
}

/// A repository with one commit and a worktree for each of these branches beside it.
fn with_worktrees(branches: &[&str]) -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    for branch in branches {
        repository.branch(branch);
        let _ = repository.git(&["worktree", "add", "--quiet", &format!("../{branch}"), branch]);
    }
    repository
}

/// The worktree of a folder name.
fn named<'a>(worktrees: &'a [Worktree], name: &str) -> &'a Worktree {
    worktrees
        .iter()
        .find(|worktree| name_of(&worktree.folder) == name)
        .unwrap()
}

#[test]
fn worktree_read_list__a_main_worktree_and_two_others_are_three_the_main_one_marked() {
    let repository = with_worktrees(&["one", "two"]);
    let worktrees = read(&repository);
    assert_eq!(worktrees.len(), 3);
    assert!(worktrees[0].is_main && worktrees[0].is_current);
    assert_eq!(worktrees[0].uncommitted, Some(0));
    assert!(
        worktrees[1..]
            .iter()
            .all(|worktree| !worktree.is_main && !worktree.is_current)
    );
}

#[test]
fn worktree_read_list__a_worktree_whose_folder_was_deleted_is_missing_and_not_counted() {
    let repository = with_worktrees(&["gone"]);
    fs::remove_dir_all(beside(&repository).join("gone")).unwrap();
    let worktrees = read(&repository);
    let gone = named(&worktrees, "gone");
    assert!(gone.is_missing);
    assert_eq!(gone.uncommitted, None);
}

#[test]
fn worktree_read_list__a_staged_a_modified_and_two_untracked_files_count_4() {
    let repository = with_worktrees(&["other"]);
    let other = beside(&repository).join("other");
    write_in(&other, "staged.txt", "1\n");
    let _ = repository.git(&["-C", other.to_str().unwrap(), "add", "--", "staged.txt"]);
    write_in(&other, "parser.rs", "2\n");
    write_in(&other, "new folder/first.txt", "1\n");
    write_in(&other, "new folder/second.txt", "2\n");
    assert_eq!(named(&read(&repository), "other").uncommitted, Some(4));
}

#[test]
fn worktree_read_list__an_index_locked_by_another_program_is_counted_without_the_lock() {
    let repository = with_worktrees(&["other"]);
    write_in(&beside(&repository).join("other"), "parser.rs", "2\n");
    let lock = repository.path().join(".git/worktrees/other/index.lock");
    fs::write(&lock, "held by a commit in a terminal").unwrap();
    assert_eq!(named(&read(&repository), "other").uncommitted, Some(1));
    assert_eq!(fs::read_to_string(&lock).unwrap(), "held by a commit in a terminal");
    fs::remove_file(&lock).unwrap();
}

#[test]
fn worktree_read_list__a_worktree_git_cannot_read_has_no_count_and_the_others_are_counted() {
    let repository = with_worktrees(&["broken", "fine"]);
    fs::write(beside(&repository).join("broken/.git"), "not a link to a repository\n").unwrap();
    write_in(&beside(&repository).join("fine"), "notes.txt", "1\n");
    let worktrees = read(&repository);
    assert_eq!(named(&worktrees, "broken").uncommitted, None);
    assert!(!named(&worktrees, "broken").is_missing);
    assert_eq!(named(&worktrees, "fine").uncommitted, Some(1));
}

#[test]
fn worktree_read_list__a_detached_worktree_has_no_branch_and_its_commit() {
    let mut repository = Repository::new();
    let first = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let _ = repository.git(&["worktree", "add", "--quiet", "--detach", "../detached", &first]);
    let worktrees = read(&repository);
    let detached = named(&worktrees, "detached");
    assert_eq!(detached.branch, None);
    assert_eq!(
        detached.commit.as_ref().map(model::CommitId::as_str),
        Some(first.as_str())
    );
    assert_eq!(worktrees[0].branch.as_deref(), Some("refs/heads/main"));
}

#[test]
fn worktree_read_list__parses_recorded_output_with_spaces_and_accents_in_folders() {
    // Recorded by the golden tests of `repo.read-state`, which read the worktrees with the same parser.
    let golden = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/repo-state/worktrees.out");
    let worktrees = parse_worktrees(&fs::read(golden).unwrap()).unwrap();
    let names: Vec<String> = worktrees.iter().map(|worktree| name_of(&worktree.folder)).collect();
    assert_eq!(names, ["repository", "détaché", "feature with space", "gone"]);
    assert_eq!(worktrees[2].branch.as_deref(), Some("refs/heads/feature"));
    assert!(worktrees[0].is_main && !worktrees[1].is_main);
    assert!(worktrees[1].branch.is_none() && worktrees[1].commit.is_some());
    assert!(worktrees[3].is_missing);
}

/// The links a worktree has to folders outside it, from its top, with `/`.
fn folder_links(worktree: &Path) -> Vec<String> {
    let request = FolderLinksRequest {
        worktree: worktree.to_path_buf(),
    };
    read_folder_links(&Git::new(worktree), &request)
        .unwrap()
        .iter()
        .map(|link| {
            let relative = link.strip_prefix(worktree).unwrap();
            relative.to_string_lossy().replace('\\', "/")
        })
        .collect()
}

#[test]
fn worktree_read_folder_links__junctions_to_folders_outside_are_returned_nested_ones_too() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the package", &[("packages/api/index.js", "1\n")]);
    let outside = beside(&repository).join("store");
    fs::create_dir_all(outside.join("modules")).unwrap();
    fs::create_dir_all(outside.join("api modules")).unwrap();
    crate::link_folder(&repository.path().join("node_modules"), &outside.join("modules"));
    crate::link_folder(
        &repository.path().join("packages/api/node_modules"),
        &outside.join("api modules"),
    );
    assert_eq!(
        folder_links(repository.path()),
        ["node_modules", "packages/api/node_modules"]
    );
}

#[test]
fn worktree_read_folder_links__links_inside_to_a_file_or_to_nothing_are_not_returned() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the sources", &[("src/main.rs", "1\n")]);
    crate::link_folder(&repository.path().join("inside"), &repository.path().join("src"));
    let outside = beside(&repository).join("gone");
    fs::create_dir(&outside).unwrap();
    crate::link_folder(&repository.path().join("dangling"), &outside);
    fs::remove_dir(&outside).unwrap();
    let file = beside(&repository).join("file.txt");
    fs::write(&file, "1\n").unwrap();
    // A link to a file needs a right Windows gives only to some accounts; without it, that case is not made.
    #[cfg(windows)]
    let _ = std::os::windows::fs::symlink_file(&file, repository.path().join("to a file"));
    #[cfg(not(windows))]
    std::os::unix::fs::symlink(&file, repository.path().join("to a file")).unwrap();
    assert_eq!(folder_links(repository.path()), Vec::<String>::new());
}

#[test]
fn worktree_read_folder_links__tracked_files_are_not_looked_into() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the sources", &[("src/main.rs", "1\n")]);
    let outside = beside(&repository).join("store");
    fs::create_dir(&outside).unwrap();
    crate::link_folder(&repository.path().join("src/modules"), &outside);
    let _ = repository.git(&["add", "--", "src"]);
    let tracked = repository.git(&["ls-files", "--", "src"]);
    if tracked.contains("src/modules") {
        // Git tracks the link as a link: it is not looked at, being tracked.
        assert_eq!(folder_links(repository.path()), Vec::<String>::new());
    } else {
        assert_eq!(folder_links(repository.path()), ["src/modules"]);
    }
}

#[test]
fn worktree_locked__locked_with_a_reason_gives_the_reason_and_locked_alone_an_empty_one() {
    let output = b"worktree /r\0HEAD 1111111111111111111111111111111111111111\0branch refs/heads/main\0\0\
worktree /usb\0HEAD 1111111111111111111111111111111111111111\0branch refs/heads/a\0locked On the USB drive\0\0\
worktree /bare-lock\0HEAD 1111111111111111111111111111111111111111\0detached\0locked\0\0";
    let worktrees = parse_worktrees(output).unwrap();
    let locks: Vec<Option<&str>> = worktrees.iter().map(|worktree| worktree.locked.as_deref()).collect();
    assert_eq!(locks, [None, Some("On the USB drive"), Some("")]);
}

#[test]
fn worktree_locked__a_locked_worktree_whose_folder_was_deleted_is_missing() {
    let repository = with_worktrees(&["usb"]);
    let _ = repository.git(&["worktree", "lock", "--reason", "On the USB drive", "../usb"]);
    fs::remove_dir_all(beside(&repository).join("usb")).unwrap();
    let worktrees = read(&repository);
    let usb = named(&worktrees, "usb");
    assert!(
        usb.is_missing,
        "git does not say it is prunable, so the folder is looked for"
    );
    assert_eq!(usb.locked.as_deref(), Some("On the USB drive"));
    assert_eq!(usb.uncommitted, None);
}
