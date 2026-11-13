use std::path::PathBuf;

use testkit::Repository;

use crate::{cli, cli_done, revision, write};

/// `main` with the commits `Add a` to `Add d`, their hashes in that order.
fn four_commits() -> (Repository, Vec<String>) {
    let mut repository = Repository::new();
    let hashes = ["a", "b", "c", "d"]
        .iter()
        .map(|name| {
            repository.commit(
                &format!("Add {name}"),
                &[(&format!("{name}.txt"), &format!("{name}\n"))],
            )
        })
        .map(|hash| hash.trim().to_owned())
        .collect();
    (repository, hashes)
}

/// A hash of the list, by its place.
fn at(hashes: &[String], place: usize) -> &str {
    hashes.get(place).map_or("", String::as_str)
}

/// What `gitardo-cli` put on the clipboard: the text after `Copy ` up to the line it said after.
fn copied(out: &str) -> String {
    let start = out.find("\nCopy ").map_or(0, |start| start + "\nCopy ".len());
    let end = out.rfind("\nCopied ").unwrap_or(out.len());
    out.get(start..end).unwrap_or_default().to_owned()
}

/// The author, date, subject and tree of each commit of a range, oldest first.
fn commits(repository: &Repository, range: &str) -> String {
    repository.git(&["log", "--reverse", "--format=%an %ae %ad %s %T", range])
}

/// Whether the plan printed has a git command on a file, whose path the log line may quote.
fn ran(out: &str, command: &str, file: &std::path::Path) -> bool {
    let name = file
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    out.lines()
        .any(|line| line.contains(&format!("> {command} ")) && line.contains(&name))
}

/// The paths of the temporary files the plan of `gitardo-cli` wrote.
fn temporary_files(out: &str) -> Vec<PathBuf> {
    out.lines()
        .filter_map(|line| line.trim_start().strip_prefix("> write "))
        .map(PathBuf::from)
        .collect()
}

#[test]
fn patch_copy__copies_the_patch_of_git_format_patch_for_the_commit() {
    let (repository, hashes) = four_commits();
    let target = format!("commit:{}", at(&hashes, 2));
    let ended = cli_done(&repository, &["--target", &target, "patch.copy"]);
    let expected = repository.git(&["format-patch", "--stdout", "-1", at(&hashes, 2)]);
    assert_eq!(copied(&ended.out).trim_end(), expected.trim_end());
    let short = repository.git(&["rev-parse", "--short", at(&hashes, 2)]);
    assert!(
        ended.out.contains(&format!("Copied {} as a patch.", short.trim())),
        "{}",
        ended.out
    );
}

#[test]
fn patch_copy__three_commits_chosen_newest_first_give_a_patch_with_the_oldest_first() {
    let (repository, hashes) = four_commits();
    let target = format!("commits:{},{},{}", at(&hashes, 3), at(&hashes, 1), at(&hashes, 2));
    let ended = cli_done(&repository, &["--target", &target, "patch.copy"]);
    let patch = copied(&ended.out);
    let froms: Vec<&str> = patch
        .lines()
        .filter_map(|line| line.strip_prefix("From "))
        .filter_map(|rest| rest.split(' ').next())
        .collect();
    assert_eq!(froms, [at(&hashes, 1), at(&hashes, 2), at(&hashes, 3)]);
    assert!(ended.out.contains("Copied 3 commits as a patch."), "{}", ended.out);
}

#[test]
fn patch_copy__git_am_of_the_patch_at_the_parent_makes_the_same_commits() {
    let (repository, hashes) = four_commits();
    let target = format!("commits:{},{}", at(&hashes, 3), at(&hashes, 2));
    let ended = cli_done(&repository, &["--target", &target, "patch.copy"]);
    write(&repository, "copied.patch", &copied(&ended.out));
    let _other = repository.git(&["checkout", "--quiet", "-b", "other", at(&hashes, 1)]);
    let _applied = repository.git(&["am", "--quiet", "--committer-date-is-author-date", "copied.patch"]);
    let original = commits(&repository, &format!("{}..main", at(&hashes, 1)));
    assert_eq!(commits(&repository, &format!("{}..other", at(&hashes, 1))), original);
}

#[test]
fn patch_save__proposes_the_short_hash_and_writes_the_patch_to_the_path_chosen() {
    let (repository, hashes) = four_commits();
    let target = format!("commit:{}", at(&hashes, 2));
    let asked = cli(&repository, &["--target", &target, "patch.save"]);
    assert_eq!(asked.code, Some(2));
    let short = repository
        .git(&["rev-parse", "--short", at(&hashes, 2)])
        .trim()
        .to_owned();
    assert!(asked.errors.contains(&format!("{short}.patch")), "{}", asked.errors);
    let path = repository.path().join(format!("{short}.patch"));
    let answer = format!("path={}", path.display());
    let ended = cli_done(&repository, &["--target", &target, "patch.save", &answer]);
    assert!(
        ended.out.contains(&format!("> write {}", path.display())),
        "{}",
        ended.out
    );
    let expected = repository.git(&["format-patch", "--stdout", "-1", at(&hashes, 2)]);
    assert_eq!(std::fs::read_to_string(&path).unwrap_or_default(), expected);
}

#[test]
fn patch_save__two_commits_propose_the_oldest_then_the_newest() {
    let (repository, hashes) = four_commits();
    let target = format!("commits:{},{}", at(&hashes, 3), at(&hashes, 1));
    let asked = cli(&repository, &["--target", &target, "patch.save"]);
    let short = |hash: &str| repository.git(&["rev-parse", "--short", hash]).trim().to_owned();
    let name = format!("{}-{}.patch", short(at(&hashes, 1)), short(at(&hashes, 3)));
    assert!(asked.errors.contains(&name), "{}", asked.errors);
}

/// A repository at `Add b`, with a patch of `Add c` and `Add d` made from its history in `two.patch`, outside the
/// working tree.
fn with_patch_of_two_commits() -> (Repository, PathBuf, Vec<String>) {
    let (repository, hashes) = four_commits();
    let patch = repository.git(&[
        "format-patch",
        "--stdout",
        &format!("{}..{}", at(&hashes, 1), at(&hashes, 3)),
    ]);
    let file = repository.path().join(".git").join("two.patch");
    std::fs::write(&file, patch).unwrap_or_default();
    let _back = repository.git(&["reset", "--quiet", "--hard", at(&hashes, 1)]);
    (repository, file, hashes)
}

#[test]
fn patch_apply__a_patch_of_two_commits_as_commits_makes_them_with_their_authors_and_messages() {
    let (repository, file, hashes) = with_patch_of_two_commits();
    let chosen = format!("file={}", file.display());
    let ended = cli_done(
        &repository,
        &["--yes", "patch.apply", "from=file", &chosen, "as=commits"],
    );
    assert!(ran(&ended.out, "git am --3way", &file), "{}", ended.out);
    let range = format!("{}..HEAD", at(&hashes, 1));
    let made = repository.git(&["log", "--reverse", "--format=%an %s", &range]);
    let original = repository.git(&[
        "log",
        "--reverse",
        "--format=%an %s",
        &format!("{}..{}", at(&hashes, 1), at(&hashes, 3)),
    ]);
    assert_eq!(made, original);
}

#[test]
fn patch_apply__to_the_working_tree_only_stages_the_changes_and_makes_no_commit() {
    let (repository, file, hashes) = with_patch_of_two_commits();
    let chosen = format!("file={}", file.display());
    let ended = cli_done(
        &repository,
        &["--yes", "patch.apply", "from=file", &chosen, "as=working-tree"],
    );
    assert!(ran(&ended.out, "git apply --3way", &file), "{}", ended.out);
    assert_eq!(revision(&repository, "HEAD"), at(&hashes, 1));
    assert_eq!(repository.git(&["diff", "--cached", "--name-only"]), "c.txt\nd.txt\n");
}

#[test]
fn patch_apply__a_patch_of_git_diff_is_applied_without_asking_how() {
    let (repository, _hashes) = four_commits();
    write(&repository, "a.txt", "changed\n");
    let diff = repository.git(&["diff"]);
    let _back = repository.git(&["checkout", "--", "a.txt"]);
    let file = repository.path().join(".git").join("diff.patch");
    std::fs::write(&file, diff).unwrap_or_default();
    let chosen = format!("file={}", file.display());
    let ended = cli_done(&repository, &["--yes", "patch.apply", "from=file", &chosen]);
    assert!(ended.out.contains("> git apply --3way"), "{}", ended.out);
    assert_eq!(repository.git(&["diff", "--cached", "--name-only"]), "a.txt\n");
}

#[test]
fn patch_apply__from_the_clipboard_writes_a_temporary_file_and_deletes_it_also_when_git_failed() {
    let (repository, _hashes) = four_commits();
    write(&repository, "a.txt", "changed\n");
    let diff = repository.git(&["diff"]);
    let _back = repository.git(&["checkout", "--", "a.txt"]);
    let clipboard = format!("clipboard={diff}");
    let ended = cli_done(&repository, &["--yes", "patch.apply", "from=clipboard", &clipboard]);
    let written = temporary_files(&ended.out);
    let [temporary] = written.as_slice() else {
        panic!("one temporary file is written:\n{}", ended.out)
    };
    let name = temporary
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    assert!(
        ended
            .out
            .lines()
            .any(|line| line.contains("> git apply --3way") && line.contains(&name)),
        "{}",
        ended.out
    );
    assert!(
        ended.out.contains(&format!("> delete {}", temporary.display())),
        "{}",
        ended.out
    );
    assert!(!temporary.exists());
    write(&repository, "a.txt", "theirs\n");
    let _committed = repository.git(&["commit", "--quiet", "-am", "Change a"]);
    let failed = cli(&repository, &["--yes", "patch.apply", "from=clipboard", &clipboard]);
    assert_ne!(
        failed.code,
        Some(0),
        "{}
{}",
        failed.out,
        failed.errors
    );
    let quoted = failed
        .errors
        .split('"')
        .nth(1)
        .unwrap_or_default()
        .replace("\\\\", "\\");
    assert!(quoted.contains("gitardo-"), "{}", failed.errors);
    assert!(
        !PathBuf::from(quoted).exists(),
        "the temporary file is deleted after git failed"
    );
}

#[test]
fn patch_apply__a_patch_that_does_not_apply_says_why_and_changes_nothing() {
    let (repository, _hashes) = four_commits();
    let file = repository.path().join(".git").join("missing.patch");
    let patch = "diff --git a/none.txt b/none.txt\n--- a/none.txt\n+++ b/none.txt\n@@ -1 +1 @@\n-x\n+y\n";
    std::fs::write(&file, patch).unwrap_or_default();
    let chosen = format!("file={}", file.display());
    let ended = cli(&repository, &["--yes", "patch.apply", "from=file", &chosen]);
    assert_eq!(ended.code, Some(1), "{}", ended.out);
    assert!(
        ended
            .errors
            .contains("Missing.patch does not apply, so nothing was changed."),
        "{}",
        ended.errors
    );
    assert!(!ended.out.contains("> git apply"), "{}", ended.out);
    assert_eq!(repository.git(&["status", "--porcelain"]), "");
}

#[test]
fn patch_apply__a_patch_that_will_conflict_names_the_files_and_asks_first() {
    let (repository, file, _hashes) = with_patch_of_two_commits();
    write(&repository, "c.txt", "mine\n");
    let _committed = {
        let _added = repository.git(&["add", "c.txt"]);
        repository.git(&["commit", "--quiet", "-m", "Add c too"])
    };
    let chosen = format!("file={}", file.display());
    let ended = cli(&repository, &["patch.apply", "from=file", &chosen, "as=working-tree"]);
    assert_eq!(ended.code, Some(2), "{}\n{}", ended.out, ended.errors);
    assert!(
        ended.out.contains("This will stop with conflicts in 1 file:"),
        "{}",
        ended.out
    );
    assert!(ended.out.contains("c.txt"), "{}", ended.out);
}

#[test]
fn patch_apply__as_commits_with_staged_changes_is_refused_before_git_runs() {
    let (repository, file, hashes) = with_patch_of_two_commits();
    write(&repository, "a.txt", "staged\n");
    let _staged = repository.git(&["add", "a.txt"]);
    let chosen = format!("file={}", file.display());
    let ended = cli(
        &repository,
        &["--yes", "patch.apply", "from=file", &chosen, "as=commits"],
    );
    assert_eq!(ended.code, Some(1));
    assert!(
        ended
            .errors
            .contains("git am applies patches only while nothing is staged"),
        "{}",
        ended.errors
    );
    assert_eq!(revision(&repository, "HEAD"), at(&hashes, 1));
}
