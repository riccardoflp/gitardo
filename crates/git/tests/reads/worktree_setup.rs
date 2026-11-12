//! `worktree-setup.read-ignored` and `worktree-setup.read-what-patterns-take` against throwaway repositories.

use git::Git;
use git::read::worktree_setup::{IgnoredRequest, PatternsRequest, read_ignored, read_what_patterns_take};
use model::worktree_setup::{FOLDER_FILES_COUNTED, FolderSize, IgnoredEntry, IgnoredKind, PatternTake, SetupPreview};
use testkit::Repository;

use crate::{beside, write};

/// What these link and copy patterns take from a repository.
fn take(repository: &Repository, link: &[&str], copy: &[&str]) -> SetupPreview {
    let request = PatternsRequest {
        folder: repository.path().to_path_buf(),
        link_patterns: link.iter().map(|pattern| (*pattern).to_owned()).collect(),
        copy_patterns: copy.iter().map(|pattern| (*pattern).to_owned()).collect(),
    };
    read_what_patterns_take(&Git::new(repository.path()), &request).unwrap()
}

/// How many each pattern takes on its own.
fn counts(takes: &[PatternTake]) -> Vec<(&str, usize)> {
    takes.iter().map(|take| (take.pattern.as_str(), take.count)).collect()
}

#[test]
fn worktree_setup_read_what_patterns_take__node_modules_and_env_take_one_each() {
    let repository = ignoring("node_modules/\n.env\n");
    write(&repository, "node_modules/left-pad/index.js", "1\n");
    write(&repository, ".env", "KEY=1\n");
    let preview = take(&repository, &["node_modules/"], &["/.env"]);
    assert_eq!(preview.linked, ["node_modules"]);
    assert_eq!(preview.copied, [".env"]);
    assert_eq!(counts(&preview.link_patterns), [("node_modules/", 1)]);
    assert_eq!(counts(&preview.copy_patterns), [("/.env", 1)]);
}

#[test]
fn worktree_setup_read_what_patterns_take__a_copy_pattern_matching_only_inside_a_linked_folder_takes_0() {
    let repository = ignoring("node_modules/\n*.local.json\n");
    write(&repository, "node_modules/x.local.json", "{}\n");
    let preview = take(&repository, &["node_modules/"], &["*.local.json"]);
    assert_eq!(preview.linked, ["node_modules"]);
    assert_eq!(preview.copied, Vec::<String>::new());
    assert_eq!(counts(&preview.copy_patterns), [("*.local.json", 0)]);
}

#[test]
fn worktree_setup_folders_to_link__git_works_out_the_examples_of_the_table() {
    let mut repository = ignoring("node_modules/\n.vscode/*\n!.vscode/launch.json\n.env\n");
    let files = [("packages/api/index.js", "1\n"), (".vscode/launch.json", "{}\n")];
    let _ = repository.commit("Add the package", &files);
    write(&repository, "node_modules/a/node_modules/b/index.js", "1\n");
    write(&repository, "packages/api/node_modules/c/index.js", "1\n");
    write(&repository, ".vscode/settings.json", "{}\n");
    write(&repository, ".env", "KEY=1\n");
    let linked = |pattern: &str| take(&repository, &[pattern], &[]).linked;
    assert_eq!(linked("node_modules/"), ["node_modules", "packages/api/node_modules"]);
    assert_eq!(linked("/packages/api/node_modules/"), ["packages/api/node_modules"]);
    assert_eq!(linked(".vscode/"), Vec::<String>::new());
    assert_eq!(linked(".env"), Vec::<String>::new());
    assert_eq!(linked("/src/"), Vec::<String>::new());
}

#[test]
fn worktree_setup_paths_to_copy__git_works_out_the_examples_of_the_table() {
    let mut repository = ignoring(".env\n.vscode/\nconfig.json\n");
    write(&repository, "config.json", "{}\n");
    let _ = repository.git(&["add", "--force", "--", "config.json"]);
    let _ = repository.commit("Keep the configuration", &[("packages/api/index.js", "1\n")]);
    write(&repository, ".env", "KEY=1\n");
    write(&repository, "packages/api/.env", "KEY=2\n");
    write(&repository, ".vscode/settings.json", "{}\n");
    write(&repository, ".vscode/tasks.json", "{}\n");
    let copied = |pattern: &str| take(&repository, &[], &[pattern]).copied;
    assert_eq!(copied("/.env"), [".env"]);
    assert_eq!(copied(".env"), [".env", "packages/api/.env"]);
    assert_eq!(copied(".vscode/"), [".vscode"]);
    assert_eq!(copied("/config.json"), Vec::<String>::new());
}

/// What git ignores in a repository, with its size.
fn ignored(repository: &Repository) -> Vec<IgnoredEntry> {
    let request = IgnoredRequest {
        folder: repository.path().to_path_buf(),
    };
    read_ignored(&Git::new(repository.path()), &request).unwrap()
}

/// A repository whose `.gitignore` says these lines.
fn ignoring(lines: &str) -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Ignore what each machine makes", &[(".gitignore", lines)]);
    repository
}

#[test]
fn worktree_setup_read_ignored__a_folder_and_a_file_are_two_entries_with_their_sizes() {
    let repository = ignoring("node_modules/\n.env\n");
    write(&repository, "node_modules/left-pad/index.js", "12345\n");
    write(&repository, "node_modules/left-pad/package.json", "{}\n");
    write(&repository, ".env", "KEY=1\n");
    let entries = ignored(&repository);
    let expected = [
        IgnoredEntry {
            path: String::from(".env"),
            kind: IgnoredKind::File { bytes: 6 },
        },
        IgnoredEntry {
            path: String::from("node_modules"),
            kind: IgnoredKind::Folder(FolderSize {
                files: 2,
                bytes: 9,
                has_more: false,
            }),
        },
    ];
    assert_eq!(entries, expected);
}

#[test]
fn worktree_setup_read_ignored__a_folder_of_6000_files_is_said_to_have_more_than_5000() {
    let repository = ignoring("cache/\n");
    for index in 0..6_000 {
        write(&repository, &format!("cache/{}/{index}.txt", index % 10), "1");
    }
    let entries = ignored(&repository);
    let IgnoredKind::Folder(size) = entries[0].kind else {
        panic!("{entries:?}")
    };
    assert_eq!((size.files, size.has_more), (FOLDER_FILES_COUNTED, true));
}

#[test]
fn worktree_setup_read_ignored__a_node_modules_that_is_a_link_is_a_link_with_no_size() {
    let repository = ignoring("node_modules\n");
    let store = beside(&repository).join("store");
    std::fs::create_dir_all(store.join("left-pad")).unwrap();
    crate::link_folder(&repository.path().join("node_modules"), &store);
    let entries = ignored(&repository);
    assert_eq!(
        entries,
        [IgnoredEntry {
            path: String::from("node_modules"),
            kind: IgnoredKind::Link,
        }]
    );
}

#[test]
fn worktree_setup_read_ignored__a_folder_ignored_twice_is_listed_once() {
    let repository = ignoring(".vscode/*\n.vscode/\n");
    write(&repository, ".vscode/settings.json", "{}\n");
    let entries = ignored(&repository);
    let paths: Vec<&str> = entries.iter().map(|entry| entry.path.as_str()).collect();
    assert_eq!(paths, [".vscode"]);
}
