//! Throwaway repositories and remotes do what their helpers say, as git itself reports it afterwards.

use testkit::{CONFLICT_BRANCH, CONFLICT_FILE, FIRST_COMMIT_TIME, Remote, Repository};

#[test]
fn a_new_repository_is_empty_on_main_with_its_settings() {
    let repository = Repository::new();
    let status = repository.git(&["status", "--porcelain=v2", "--branch"]);
    assert_eq!(status, "# branch.oid (initial)\n# branch.head main\n");
    assert_eq!(repository.git(&["config", "core.autocrlf"]), "false\n");
    assert_eq!(repository.git(&["config", "commit.gpgsign"]), "false\n");
    assert_eq!(repository.git(&["config", "tag.gpgsign"]), "false\n");
    assert!(repository.path().join(".git").is_dir());
    assert!(repository.path().parent().unwrap().join("global.gitconfig").is_file());
}

#[test]
fn git_sees_only_the_configuration_of_the_test() {
    let repository = Repository::new();
    let scopes = repository.git(&["config", "--list", "--show-scope"]);
    assert!(scopes.lines().all(|line| line.starts_with("local\t")), "{scopes}");
    let identity = repository.git(&["var", "GIT_AUTHOR_IDENT"]);
    assert_eq!(
        identity,
        format!("Test Author <author@example.com> {FIRST_COMMIT_TIME} +0000\n")
    );
    let committer = repository.git(&["var", "GIT_COMMITTER_IDENT"]);
    assert_eq!(
        committer,
        format!("Test Author <author@example.com> {FIRST_COMMIT_TIME} +0000\n")
    );
}

#[test]
fn commit_writes_its_files_and_returns_the_full_id() {
    let mut repository = Repository::new();
    let files = [
        ("README.md", "Read me\n"),
        ("src/parser/lexer.rs", "fn main() {}\n"),
        ("pages/[id].tsx", "page\n"),
    ];
    let id = repository.commit("Add the parser", &files);
    assert_eq!(id.len(), 40);
    assert!(id.chars().all(|character| character.is_ascii_hexdigit()));
    assert_eq!(repository.git(&["rev-parse", "HEAD"]), format!("{id}\n"));
    assert_eq!(repository.git(&["log", "--format=%s"]), "Add the parser\n");
    let tracked = repository.git(&["ls-files"]);
    assert_eq!(tracked, "README.md\npages/[id].tsx\nsrc/parser/lexer.rs\n");
    assert_eq!(repository.git(&["status", "--porcelain=v2"]), "");
    let content = std::fs::read_to_string(repository.path().join("src/parser/lexer.rs")).unwrap();
    assert_eq!(content, "fn main() {}\n");
}

#[test]
fn a_commit_with_no_files_is_empty() {
    let mut repository = Repository::new();
    let _ = repository.commit("Start", &[]);
    assert_eq!(repository.git(&["ls-files"]), "");
    assert_eq!(repository.git(&["rev-list", "--count", "HEAD"]), "1\n");
}

#[test]
fn each_commit_is_one_minute_after_the_one_before() {
    let mut repository = Repository::new();
    for (index, name) in ["one", "two", "three"].into_iter().enumerate() {
        let _ = repository.commit(&format!("Add {name}"), &[(name, name)]);
        let dates = repository.git(&["log", "-1", "--format=%at %ct"]);
        let expected = FIRST_COMMIT_TIME + 60 * u64::try_from(index).unwrap();
        assert_eq!(dates, format!("{expected} {expected}\n"));
    }
}

#[test]
fn branch_checkout_and_tag_move_what_they_say() {
    let mut repository = Repository::new();
    let first = repository.commit("Add one", &[("one.txt", "1\n")]);
    repository.branch("feature");
    repository.tag("v1");
    assert_eq!(repository.git(&["symbolic-ref", "--short", "HEAD"]), "main\n");
    repository.checkout("feature");
    assert_eq!(repository.git(&["symbolic-ref", "--short", "HEAD"]), "feature\n");
    let second = repository.commit("Add two", &[("two.txt", "2\n")]);
    assert_eq!(
        repository.git(&["rev-parse", "feature", "main", "v1"]),
        format!("{second}\n{first}\n{first}\n")
    );
    repository.checkout("v1");
    assert_eq!(repository.git(&["rev-parse", "HEAD"]), format!("{first}\n"));
    assert!(
        repository
            .git(&["status", "--porcelain=v2", "--branch"])
            .contains("# branch.head (detached)")
    );
}

#[test]
fn merge_makes_a_merge_commit_even_when_it_could_fast_forward() {
    let mut repository = Repository::new();
    let base = repository.commit("Add one", &[("one.txt", "1\n")]);
    repository.branch("feature");
    repository.checkout("feature");
    let feature = repository.commit("Add two", &[("two.txt", "2\n")]);
    repository.checkout("main");
    repository.merge("feature");
    let parents = repository.git(&["rev-list", "--parents", "-n", "1", "HEAD"]);
    let parents: Vec<&str> = parents.split_whitespace().skip(1).collect();
    assert_eq!(parents, [base.as_str(), feature.as_str()]);
    assert_eq!(
        repository.git(&["log", "-1", "--format=%s"]),
        "Merge branch 'feature'\n"
    );
    let expected = FIRST_COMMIT_TIME + 120;
    assert_eq!(
        repository.git(&["log", "-1", "--format=%at %ct"]),
        format!("{expected} {expected}\n")
    );
}

#[test]
fn conflict_leaves_a_merge_stopped_on_one_file() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add one", &[("one.txt", "1\n")]);
    repository.conflict();
    assert_conflicted(&repository);
}

#[test]
fn conflict_works_in_an_empty_repository() {
    let mut repository = Repository::new();
    repository.conflict();
    assert_conflicted(&repository);
}

fn assert_conflicted(repository: &Repository) {
    let status = repository.git(&["status", "--porcelain=v2"]);
    let lines: Vec<&str> = status.lines().collect();
    assert_eq!(lines.len(), 1, "{status}");
    assert!(
        lines
            .first()
            .is_some_and(|line| line.starts_with("u UU ") && line.ends_with(CONFLICT_FILE)),
        "{status}"
    );
    let merging = repository.git(&["rev-parse", "MERGE_HEAD"]);
    assert_eq!(merging, repository.git(&["rev-parse", CONFLICT_BRANCH]));
    let content = std::fs::read_to_string(repository.path().join(CONFLICT_FILE)).unwrap_or_default();
    assert!(
        content.contains("<<<<<<<") && content.contains("ours") && content.contains("theirs"),
        "{content}"
    );
}

#[test]
#[should_panic(expected = "git rev-parse --verify missing failed")]
fn a_failing_git_command_fails_the_test_with_what_git_said() {
    let repository = Repository::new();
    let _ = repository.git(&["rev-parse", "--verify", "missing"]);
}

#[test]
fn the_folder_is_gone_after_the_repository_is_dropped() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add one", &[("one.txt", "1\n")]);
    let remote = Remote::new(&repository);
    let folder = repository.path().parent().unwrap().to_path_buf();
    assert!(folder.join("repository").is_dir() && remote.path().is_dir());
    drop(remote);
    drop(repository);
    assert!(!folder.exists(), "{} is still there", folder.display());
}

#[test]
fn repositories_made_at_once_have_folders_of_their_own() {
    let first = Repository::new();
    let second = Repository::new();
    assert_ne!(first.path(), second.path());
    let name = first
        .path()
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(
        name.starts_with(&format!("gitardo-test-{}-", std::process::id())),
        "{name}"
    );
}

#[test]
fn a_remote_is_a_bare_repository_with_main_pushed_and_followed() {
    let mut repository = Repository::new();
    let first = repository.commit("Add one", &[("one.txt", "1\n")]);
    let remote = Remote::new(&repository);
    assert_eq!(remote.path(), repository.path().parent().unwrap().join("remote.git"));
    assert_eq!(remote.git(&["rev-parse", "--is-bare-repository"]), "true\n");
    assert_eq!(remote.git(&["rev-parse", "main"]), format!("{first}\n"));
    assert_eq!(
        repository.git(&["rev-parse", "--abbrev-ref", "main@{upstream}"]),
        "origin/main\n"
    );
    let second = repository.commit("Add two", &[("two.txt", "2\n")]);
    let _ = repository.git(&["push", "--quiet"]);
    assert_eq!(remote.git(&["rev-parse", "main"]), format!("{second}\n"));
}
