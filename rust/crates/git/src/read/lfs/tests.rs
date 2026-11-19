#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::Path;

use model::lfs::{LfsContent, LfsContentRequest, LfsFilesRequest, LfsStatusRequest, pointer};
use testkit::Repository;

use super::{content_in, parse_files_json, parse_files_long, parse_version, read_content, read_files, read_status};
use crate::Git;

/// Git in a repository, with no configuration but the repository's own: the system configuration of Git for Windows
/// sets up Git LFS for every repository.
fn git_of(repository: &Repository) -> Git {
    let empty = repository.path().join(".git").join("empty-config");
    std::fs::write(&empty, "").unwrap();
    Git::new(repository.path())
        .with_variable("GIT_CONFIG_NOSYSTEM", "1")
        .with_variable("GIT_CONFIG_GLOBAL", empty.as_os_str())
}

/// A repository set up for Git LFS that stores `*.bin` with it, with these files committed.
fn stored(files: &[(&str, &str)]) -> Repository {
    let mut repository = Repository::new();
    let _ = repository.git(&["lfs", "install", "--local"]);
    let mut all = vec![(".gitattributes", "*.bin filter=lfs diff=lfs merge=lfs -text\n")];
    all.extend_from_slice(files);
    let _ = repository.commit("Add the files", &all);
    repository
}

/// The pointer of a file as `HEAD` has it.
fn pointer_of(repository: &Repository, path: &str) -> model::lfs::LfsPointer {
    let committed = repository.git(&["cat-file", "blob", &format!("HEAD:{path}")]);
    pointer(committed.as_bytes()).unwrap()
}

#[test]
fn lfs_read_status__without_git_lfs_it_says_not_installed_and_still_lists_the_patterns() {
    let mut repository = Repository::new();
    let _ = repository.commit(
        "Add the attributes",
        &[(".gitattributes", "*.psd filter=lfs diff=lfs merge=lfs -text\n")],
    );
    // The version read once for the engine said Git LFS is not there.
    let request = LfsStatusRequest {
        version: Some(None),
        patterns: None,
    };
    let status = read_status(&git_of(&repository), &request).unwrap();
    assert_eq!(status.version, None);
    assert_eq!(status.patterns.len(), 1);
    assert_eq!(status.patterns[0].pattern, "*.psd");
    assert_eq!(status.patterns[0].file, ".gitattributes");
}

#[test]
fn lfs_read_status__with_git_lfs_3_7_1_it_reads_the_version() {
    assert_eq!(
        parse_version("git-lfs/3.7.1 (GitHub; windows amd64; go 1.25.1; git b84b3384)\n").as_deref(),
        Some("3.7.1")
    );
    let repository = Repository::new();
    let status = read_status(&git_of(&repository), &LfsStatusRequest::default()).unwrap();
    assert!(
        status
            .version
            .is_some_and(|version| version.starts_with(|first: char| first.is_ascii_digit()))
    );
}

#[test]
fn lfs_read_status__a_pattern_in_assets_gitattributes_is_listed_with_that_file_and_other_filters_are_not() {
    let mut repository = Repository::new();
    let attributes = "*.psd filter=lfs diff=lfs merge=lfs -text\n*.tmp -filter\n*.txt filter=other\n";
    let _ = repository.commit("Add the attributes", &[("assets/.gitattributes", attributes)]);
    let status = read_status(
        &git_of(&repository),
        &LfsStatusRequest {
            version: Some(None),
            patterns: None,
        },
    )
    .unwrap();
    let found: Vec<(&str, &str, usize)> = status
        .patterns
        .iter()
        .map(|pattern| (pattern.pattern.as_str(), pattern.file.as_str(), pattern.line))
        .collect();
    assert_eq!(found, [("*.psd", "assets/.gitattributes", 1)]);
}

#[test]
fn lfs_read_status__patterns_without_a_filter_lfs_key_in_the_configuration_are_not_set_up() {
    let mut repository = Repository::new();
    let _ = repository.commit(
        "Add the attributes",
        &[(".gitattributes", "*.psd filter=lfs diff=lfs merge=lfs -text\n")],
    );
    let request = LfsStatusRequest {
        version: Some(None),
        patterns: None,
    };
    assert!(!read_status(&git_of(&repository), &request).unwrap().is_set_up);
    let _ = repository.git(&["lfs", "install", "--local"]);
    assert!(read_status(&git_of(&repository), &request).unwrap().is_set_up);
}

#[test]
fn lfs_read_files__a_file_downloaded_and_one_left_as_a_pointer_are_both_listed_with_their_sizes() {
    let repository = stored(&[("a.bin", "first content\n"), ("b.bin", "the second content\n")]);
    let second = pointer_of(&repository, "b.bin");
    // `b.bin` as a clone made without downloading leaves it: its pointer on disk, nothing in the store.
    let object = repository
        .path()
        .join(".git/lfs/objects")
        .join(&second.oid[..2])
        .join(&second.oid[2..4]);
    std::fs::remove_file(object.join(&second.oid)).unwrap();
    let committed = repository.git(&["cat-file", "blob", "HEAD:b.bin"]);
    std::fs::write(repository.path().join("b.bin"), committed).unwrap();
    let request = LfsFilesRequest {
        version: String::from("3.7.1"),
    };
    let files = read_files(&git_of(&repository), &request).unwrap();
    let found: Vec<(&str, u64, bool, bool)> = files
        .iter()
        .map(|file| (file.path.as_str(), file.size, file.is_downloaded, file.is_checked_out))
        .collect();
    assert_eq!(found, [("a.bin", 14, true, true), ("b.bin", 19, false, false)]);
}

#[test]
fn lfs_read_files__a_path_with_spaces_and_non_ascii_letters_comes_back_as_git_has_it() {
    let printed = "{\n \"files\": [\n  {\n   \"name\": \"d\u{e9}j\u{e0} vu/caf\\u00e9 \\u003cnew\\u003e.bin\",\n   \"size\": 3,\n   \"checkout\": true,\n   \"downloaded\": true,\n   \"oid_type\": \"sha256\",\n   \"oid\": \"ab\",\n   \"version\": \"https://git-lfs.github.com/spec/v1\"\n  }\n ]\n}\n";
    let files = parse_files_json(printed);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "d\u{e9}j\u{e0} vu/caf\u{e9} <new>.bin");
    let repository = stored(&[("caf\u{e9} au lait.bin", "abc")]);
    let request = LfsFilesRequest {
        version: String::from("3.7.1"),
    };
    let listed = read_files(&git_of(&repository), &request).unwrap();
    assert_eq!(listed[0].path, "caf\u{e9} au lait.bin");
}

#[test]
fn lfs_read_files__without_git_lfs_it_is_not_run() {
    // A folder that does not exist: git run there would fail.
    let git = Git::new(Path::new("no-repository-here"));
    let request = LfsFilesRequest { version: String::new() };
    assert_eq!(read_files(&git, &request).unwrap(), Vec::new());
}

#[test]
fn lfs_read_status__a_read_after_a_change_to_the_working_tree_only_keeps_the_patterns_read_before() {
    let mut repository = Repository::new();
    let _ = repository.commit(
        "Add the attributes",
        &[(
            ".gitattributes",
            "*.psd filter=lfs
",
        )],
    );
    let before = read_status(
        &git_of(&repository),
        &LfsStatusRequest {
            version: Some(None),
            patterns: None,
        },
    )
    .unwrap();
    // Changed on disk without the store hearing of a `.gitattributes` changed: the patterns read before are kept.
    std::fs::write(
        repository.path().join(".gitattributes"),
        "*.bin filter=lfs
",
    )
    .unwrap();
    let request = LfsStatusRequest {
        version: Some(None),
        patterns: Some(before.patterns.clone()),
    };
    assert_eq!(
        read_status(&git_of(&repository), &request).unwrap().patterns,
        before.patterns
    );
}

#[test]
fn lfs_read_files__the_long_output_of_git_lfs_2_13_gives_the_same_files() {
    let printed = "ba7816bf * a b.bin (3 B)\n4d7a2146 - big/b.bin (1.2 KB)\n";
    let files = parse_files_long(printed);
    let found: Vec<(&str, u64, bool)> = files
        .iter()
        .map(|file| (file.path.as_str(), file.size, file.is_downloaded))
        .collect();
    assert_eq!(found, [("a b.bin", 3, true), ("big/b.bin", 1200, false)]);
}

#[test]
fn lfs_read_content__a_file_downloaded_reads_the_bytes_it_has_once_checked_out() {
    let repository = stored(&[("a.bin", "the content\n")]);
    let request = LfsContentRequest {
        pointer: pointer_of(&repository, "a.bin"),
    };
    let content = read_content(&git_of(&repository), &request).unwrap();
    assert_eq!(content, LfsContent::Downloaded(b"the content\n".to_vec()));
}

#[test]
fn lfs_read_content__a_pointer_whose_object_is_not_in_the_store_is_not_downloaded() {
    let repository = stored(&[("a.bin", "the content\n")]);
    let request = LfsContentRequest {
        pointer: pointer_of(&repository, "a.bin"),
    };
    // Nothing is asked of a server: the store is only looked into.
    assert_eq!(
        content_in(Path::new("no-store-here"), &request),
        LfsContent::NotDownloaded
    );
}

#[test]
fn lfs_read_content__an_object_shorter_than_the_size_of_its_pointer_is_not_downloaded() {
    let repository = stored(&[("a.bin", "the content\n")]);
    let mut request = LfsContentRequest {
        pointer: pointer_of(&repository, "a.bin"),
    };
    request.pointer.size += 1;
    let content = read_content(&git_of(&repository), &request).unwrap();
    assert_eq!(content, LfsContent::NotDownloaded);
}

#[test]
fn lfs_store_committed__the_files_to_store_are_those_committed_before_the_pattern_and_renormalize_stages_a_pointer() {
    let mut repository = Repository::new();
    let _ = repository.git(&["lfs", "install", "--local"]);
    let _ = repository.commit(
        "Add the logo",
        &[("logo.psd", "plain bytes\n"), ("art/old.psd", "old\n")],
    );
    let _ = repository.commit(
        "Store psd files with Git LFS",
        &[
            (".gitattributes", "*.psd filter=lfs diff=lfs merge=lfs -text\n"),
            ("art/new.psd", "new\n"),
        ],
    );
    let git = git_of(&repository);
    let every = [String::from(":(attr:filter=lfs)")];
    assert_eq!(super::read_to_store(&git, &every).unwrap(), ["art/old.psd", "logo.psd"]);
    let only_logo = [String::from(":(literal)logo.psd")];
    assert_eq!(super::read_to_store(&git, &only_logo).unwrap(), ["logo.psd"]);
    // Once added again with --renormalize, the index has a pointer and it is not listed any more.
    let _ = repository.git(&["add", "--renormalize", "--", "logo.psd"]);
    let staged = repository.git(&["cat-file", "blob", ":logo.psd"]);
    assert!(pointer(staged.as_bytes()).is_some(), "{staged}");
    assert_eq!(super::read_to_store(&git, &every).unwrap(), ["art/old.psd"]);
    // The commit before keeps it as it had it.
    assert_eq!(
        repository.git(&["cat-file", "blob", "HEAD~1:logo.psd"]),
        "plain bytes\n"
    );
}

#[test]
fn lfs_read_files__a_repository_with_no_file_prints_files_null_and_none_is_listed() {
    assert_eq!(parse_files_json("{\n \"files\": null\n}\n"), []);
}

#[test]
fn lfs_read_locks__parsing_one_lock_of_ours_gives_one_lock_of_this_user_on_a_psd() {
    let printed = r#"{"ours":[{"id":"1","path":"a.psd","owner":{"name":"reviewer"},"locked_at":"2026-11-19T10:00:00Z"}],"theirs":[]}"#;
    let model::lfs::LfsLocks::Read(locks) = super::parse_verified(printed) else {
        panic!("the locks are read");
    };
    assert_eq!(
        locks,
        [model::lfs::LfsLock {
            id: String::from("1"),
            path: String::from("a.psd"),
            owner: String::from("reviewer"),
            locked_at: String::from("2026-11-19T10:00:00Z"),
            is_ours: Some(true),
        }]
    );
    // Without --verify the locks are an array, with no word on whose they are.
    let plain = r#"[{"id":"7","path":"art/logo \"final\".psd","owner":{"name":"designer"},"locked_at":"x"}]"#;
    let model::lfs::LfsLocks::Read(locks) = super::parse_plain(plain) else {
        panic!("the locks are read");
    };
    assert_eq!(locks[0].path, "art/logo \"final\".psd");
    assert_eq!(locks[0].is_ours, None);
}

#[test]
fn lfs_read_locks__a_server_that_answers_that_it_does_not_support_locks_gives_no_lock_and_that_fact() {
    let said = "Remote \"origin\" does not support the Git LFS locking API. Consider disabling it with:";
    assert_eq!(
        super::unsupported(said),
        Some(model::lfs::LfsLocks::Unsupported(said.to_owned()))
    );
    assert_eq!(super::unsupported("fatal: unable to access the server"), None);
    // A server Git LFS cannot reach, or that answers 404, is a server with no lock to Git LFS 3.7.1.
    let repository = stored(&[]);
    let _ = repository.git(&["config", "lfs.url", "http://127.0.0.1:9/no-server"]);
    let git = git_of(&repository).with_variable("GIT_TERMINAL_PROMPT", "0");
    let read = super::read_locks(&git, &model::lfs::LfsLocksRequest::default());
    assert!(
        matches!(&read, Ok(model::lfs::LfsLocks::Read(locks)) if locks.is_empty()),
        "{read:?}"
    );
}
