//! Ignoring files and stopping tracking them, run by the engine on throwaway repositories, and what git says after
//! (`changes.ignore`, `changes.stop-tracking`).

use testkit::Repository;

use crate::{cli, cli_done, write};

fn repository() -> Repository {
    let mut repository = Repository::new();
    let _first = repository.commit("Add the settings", &[("a.txt", "a\n"), ("config/local.json", "{}\n")]);
    repository
}

/// What `git status --porcelain` says, one line per file.
fn status(repository: &Repository) -> Vec<String> {
    repository
        .git(&["status", "--porcelain=v1", "--untracked-files=all"])
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The text of a file of the working tree.
fn text(repository: &Repository, name: &str) -> String {
    std::fs::read_to_string(repository.path().join(name)).unwrap_or_default()
}

/// Runs a feature on a path, with the answers given.
fn on_path(repository: &Repository, feature: &str, path: &str, answers: &[&str]) -> crate::Ended {
    let target = format!("file:{path}");
    let mut arguments = vec!["--yes", "--target", target.as_str(), feature];
    arguments.extend(answers);
    cli_done(repository, &arguments)
}

#[test]
fn changes_ignore__all_log_files_writes_star_log_and_debug_log_leaves_the_changes() {
    let repository = repository();
    write(&repository, "debug.log", "started\n");
    let ended = on_path(
        &repository,
        "changes.ignore",
        "debug.log",
        &["ignore-what=1", "ignore-where=0"],
    );
    assert!(
        ended.out.contains("> write ") && ended.out.contains(".gitignore"),
        "{}",
        ended.out
    );
    assert_eq!(text(&repository, ".gitignore"), "*.log\n");
    assert_eq!(status(&repository), ["?? .gitignore"]);
    assert!(
        ended.out.contains("debug.log is ignored by *.log in .gitignore."),
        "{}",
        ended.out
    );
}

#[test]
fn changes_ignore__only_on_this_computer_writes_the_exclude_file_and_no_file_of_the_working_tree() {
    let repository = repository();
    write(&repository, "debug.log", "started\n");
    let _ended = on_path(
        &repository,
        "changes.ignore",
        "debug.log",
        &["ignore-what=0", "ignore-where=1"],
    );
    assert!(text(&repository, ".git/info/exclude").ends_with("/debug.log\n"));
    assert!(status(&repository).is_empty(), "{:?}", status(&repository));
}

#[test]
fn changes_ignore__this_folder_on_build_writes_slash_build_slash() {
    let repository = repository();
    std::fs::create_dir(repository.path().join("build")).unwrap();
    write(&repository, "build/out.bin", "binary\n");
    let _ended = on_path(
        &repository,
        "changes.ignore",
        "build",
        &["ignore-what=0", "ignore-where=0"],
    );
    assert_eq!(text(&repository, ".gitignore"), "/build/\n");
    assert_eq!(status(&repository), ["?? .gitignore"]);
}

#[test]
fn changes_ignore__stop_tracking_it_writes_the_pattern_then_runs_git_rm_cached_and_keeps_the_file() {
    let repository = repository();
    let ended = on_path(
        &repository,
        "changes.ignore",
        "config/local.json",
        &["ignore-what=0", "ignore-where=0", "ignore-tracked=stop-tracking"],
    );
    assert!(
        ended.out.contains("> git rm --cached -- config/local.json"),
        "{}",
        ended.out
    );
    assert!(repository.path().join("config/local.json").exists());
    assert_eq!(status(&repository), ["D  config/local.json", "?? .gitignore"]);
}

#[test]
fn changes_ignore__only_write_the_pattern_leaves_a_tracked_file_listed_as_changed() {
    let repository = repository();
    write(&repository, "config/local.json", "{ \"port\": 1 }\n");
    let ended = on_path(
        &repository,
        "changes.ignore",
        "config/local.json",
        &["ignore-what=0", "ignore-where=0", "ignore-tracked=only-write"],
    );
    assert!(!ended.out.contains("git rm"), "{}", ended.out);
    assert_eq!(status(&repository), [" M config/local.json", "?? .gitignore"]);
}

#[test]
fn changes_ignore__a_path_ignored_already_is_refused_and_nothing_is_written() {
    let mut repository = repository();
    let _ignore = repository.commit("Ignore logs", &[(".gitignore", "*.log\n")]);
    write(&repository, "x.log", "x\n");
    let ended = cli(&repository, &["--target", "file:x.log", "changes.ignore"]);
    assert_eq!(ended.code, Some(1), "{}\n{}", ended.out, ended.errors);
    let said = format!("{}{}", ended.out, ended.errors);
    assert!(
        said.contains("x.log is ignored already, by *.log in .gitignore."),
        "{said}"
    );
    assert_eq!(text(&repository, ".gitignore"), "*.log\n");
}

#[test]
fn changes_ignore_patterns__a_name_with_brackets_is_ignored_by_its_line_and_not_the_names_it_would_match() {
    let repository = repository();
    write(&repository, "notes[1].txt", "one\n");
    write(&repository, "notes1.txt", "one\n");
    let _ended = on_path(
        &repository,
        "changes.ignore",
        "notes[1].txt",
        &["ignore-what=0", "ignore-where=0"],
    );
    assert_eq!(text(&repository, ".gitignore"), "/notes\\[1\\].txt\n");
    let ignored = repository.git(&["check-ignore", "--no-index", "-v", "--", "notes[1].txt"]);
    assert!(ignored.contains("/notes\\[1\\].txt"), "{ignored}");
    assert_eq!(status(&repository), ["?? .gitignore", "?? notes1.txt"]);
}

#[test]
fn changes_stop_tracking__the_file_stays_on_disk_and_its_deletion_is_staged() {
    let repository = repository();
    let ended = on_path(&repository, "changes.stop-tracking", "config/local.json", &[]);
    assert!(
        ended.out.contains("> git rm --cached -- config/local.json"),
        "{}",
        ended.out
    );
    assert_eq!(text(&repository, "config/local.json"), "{}\n");
    assert_eq!(status(&repository), ["D  config/local.json", "?? config/local.json"]);
}

#[test]
fn changes_stop_tracking__a_folder_is_taken_out_with_r() {
    let mut repository = repository();
    let _generated = repository.commit("Generate", &[("generated/a.rs", "a\n"), ("generated/b.rs", "b\n")]);
    let ended = on_path(&repository, "changes.stop-tracking", "generated", &[]);
    assert!(ended.out.contains("> git rm --cached -r -- generated"), "{}", ended.out);
    assert_eq!(
        status(&repository),
        [
            "D  generated/a.rs",
            "D  generated/b.rs",
            "?? generated/a.rs",
            "?? generated/b.rs"
        ]
    );
}

#[test]
fn changes_stop_tracking__gits_refusal_of_a_version_staged_nowhere_else_is_followed_by_stop_tracking_anyway() {
    let repository = repository();
    write(
        &repository,
        "a.txt",
        "staged
",
    );
    let _staged = repository.git(&["add", "a.txt"]);
    write(
        &repository,
        "a.txt",
        "changed again
",
    );
    let refused = cli(&repository, &["--target", "file:a.txt", "changes.stop-tracking"]);
    assert_eq!(
        refused.code,
        Some(2),
        "asked first: {}
{}",
        refused.out,
        refused.errors
    );
    assert_eq!(
        status(&repository),
        ["MM a.txt"],
        "nothing changes until it is accepted"
    );
    let ended = on_path(&repository, "changes.stop-tracking", "a.txt", &[]);
    assert!(
        ended.out.contains(
            "> git rm --cached -- a.txt
"
        ) && ended.out.contains("> git rm --cached --force -- a.txt"),
        "git refused, then Stop tracking anyway ran once accepted: {}",
        ended.out
    );
    assert_eq!(status(&repository), ["D  a.txt", "?? a.txt"]);
}

#[test]
fn changes_stop_tracking__unstaging_the_deletion_puts_the_file_back_as_it_was() {
    let repository = repository();
    let _ended = on_path(&repository, "changes.stop-tracking", "config/local.json", &[]);
    let _unstaged = on_path(&repository, "changes.unstage", "config/local.json", &[]);
    assert!(status(&repository).is_empty(), "{:?}", status(&repository));
}
