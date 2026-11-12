#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::{CopyInput, LinkInput, folders_holding_matches, folders_to_link, paths_to_copy};

fn strings(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|text| (*text).to_owned()).collect()
}

/// The folders to link, from what git says: the ignored files, the folders ignored whole, the files the patterns
/// match, the folders they name; git ignores every folder that holds an ignored file but those of `not_ignored`.
fn linked(ignored: &[&str], whole: &[&str], matched: &[&str], named: &[&str], not_ignored: &[&str]) -> Vec<String> {
    let ignored_files = strings(ignored);
    let candidates = folders_holding_matches(&ignored_files, &strings(matched));
    let input = LinkInput {
        ignored_folders: candidates
            .into_iter()
            .filter(|folder| !not_ignored.contains(&folder.as_str()))
            .collect(),
        ignored_files,
        ignored_whole: strings(whole),
        matched_files: strings(matched),
        named_folders: strings(named),
    };
    folders_to_link(&input)
}

#[test]
fn worktree_setup_folders_to_link__each_example_of_the_table_gives_what_it_says() {
    let both = ["node_modules/a/index.js", "packages/api/node_modules/b/index.js"];
    let folders = ["node_modules", "packages/api/node_modules"];
    let not_ignored = ["packages", "packages/api"];
    assert_eq!(linked(&both, &folders, &both, &folders, &not_ignored), folders);
    let nested = ["node_modules/a/index.js", "node_modules/a/node_modules/c/index.js"];
    let named = ["node_modules", "node_modules/a/node_modules"];
    assert_eq!(
        linked(&nested, &["node_modules"], &nested, &named, &[]),
        ["node_modules"]
    );
    let api = ["packages/api/node_modules/b/index.js"];
    assert_eq!(
        linked(&both, &folders, &api, &["packages/api/node_modules"], &not_ignored),
        ["packages/api/node_modules"]
    );
    // A tracked node_modules has no file git ignores.
    assert_eq!(linked(&[], &[], &[], &["node_modules"], &[]), Vec::<String>::new());
    // A `.vscode` with a tracked file in it is not ignored whole.
    let settings = [".vscode/settings.json"];
    assert_eq!(
        linked(&settings, &[], &settings, &[".vscode"], &[]),
        Vec::<String>::new()
    );
    assert_eq!(linked(&[".env"], &[], &[".env"], &[], &[]), Vec::<String>::new());
}

#[test]
fn worktree_setup_folders_to_link__no_pattern_gives_a_folder_git_does_not_ignore() {
    let files = ["node_modules/a/index.js"];
    assert_eq!(
        linked(
            &files,
            &["node_modules"],
            &files,
            &["node_modules"],
            &["node_modules", "node_modules/a"]
        ),
        Vec::<String>::new()
    );
}

#[test]
fn worktree_setup_folders_to_link__paths_are_relative_with_forward_slashes() {
    let files = ["packages/api/node_modules/b/index.js"];
    let folder = ["packages/api/node_modules"];
    let read = linked(&files, &folder, &files, &folder, &["packages", "packages/api"]);
    assert_eq!(read, ["packages/api/node_modules"]);
    assert!(read.iter().all(|path| !path.contains('\\') && !path.starts_with('/')));
}

/// The paths to copy, from what git says.
fn copied(ignored: &[&str], whole: &[&str], matched: &[&str], linked: &[&str], commit: &[&str]) -> Vec<String> {
    paths_to_copy(&CopyInput {
        ignored_files: strings(ignored),
        ignored_whole: strings(whole),
        matched_files: strings(matched),
        linked_folders: strings(linked),
        commit_files: strings(commit),
    })
}

#[test]
fn worktree_setup_paths_to_copy__each_example_of_the_table_gives_what_it_says() {
    let envs = [".env", "packages/api/.env"];
    assert_eq!(copied(&envs, &[], &[".env"], &[], &[]), [".env"]);
    assert_eq!(copied(&envs, &[], &envs, &[], &[]), envs);
    let vscode = [".vscode/launch.json", ".vscode/settings.json"];
    assert_eq!(copied(&vscode, &[".vscode"], &vscode, &[], &[]), [".vscode"]);
    let local = ["node_modules/x.local.json"];
    assert_eq!(
        copied(&local, &["node_modules"], &local, &["node_modules"], &[]),
        Vec::<String>::new()
    );
    let config = ["config.json"];
    assert_eq!(copied(&config, &[], &config, &[], &config), Vec::<String>::new());
}

#[test]
fn worktree_setup_paths_to_copy__a_folder_with_a_file_not_matched_is_copied_file_by_file() {
    let files = [".vscode/a.json", ".vscode/b.txt"];
    assert_eq!(
        copied(&files, &[".vscode"], &[".vscode/a.json"], &[], &[]),
        [".vscode/a.json"]
    );
    // A path the commit has a folder of, or a file above it, clashes too.
    assert_eq!(
        copied(&["out"], &[], &["out"], &[], &["out/x.txt"]),
        Vec::<String>::new()
    );
    assert_eq!(copied(&["lib/x"], &[], &["lib/x"], &[], &["lib"]), Vec::<String>::new());
}

#[test]
fn worktree_setup_paths_to_copy__no_pattern_gives_a_path_git_does_not_ignore() {
    assert_eq!(copied(&[".env"], &[], &[".env", "src/main.rs"], &[], &[]), [".env"]);
}
