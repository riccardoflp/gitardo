//! The repositories of these tests, and the helpers that make them: files staged straight into the index, so that names
//! a disk of Windows refuses (a tab, a double quote, a leading `:`) can be committed too.

use std::fs;
use std::io::Write;
use std::process::Stdio;

use git::Git;
use testkit::Repository;

/// Git, run in a throwaway repository.
pub fn git_in(repository: &Repository) -> Git {
    Git::new(repository.path())
}

/// Writes a file of the working tree, making its folders.
pub fn write(repository: &Repository, name: &str, content: &[u8]) {
    let path = repository.path().join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Stages a file with a mode and a content straight into the index, without the disk: `100644`, `100755`, `120000` for
/// a link (the content is its target), `160000` for a submodule (the content is the hash of its commit).
pub fn stage(repository: &Repository, mode: &str, path: &str, content: &[u8]) {
    let object = if mode == "160000" {
        String::from_utf8(content.to_vec()).unwrap()
    } else {
        let mut child = repository
            .command("git")
            .args(["hash-object", "-w", "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(content).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    };
    let entry = format!("{mode},{object},{path}");
    // Git for Windows refuses names that NTFS cannot hold unless told this is on purpose.
    let _ = repository.git(&[
        "-c",
        "core.protectNTFS=false",
        "update-index",
        "--add",
        "--cacheinfo",
        &entry,
    ]);
}

/// Takes a file out of the index, without the disk.
pub fn unstage(repository: &Repository, path: &str) {
    let _ = repository.git(&[
        "-c",
        "core.protectNTFS=false",
        "update-index",
        "--force-remove",
        "--",
        path,
    ]);
}

/// A first commit of files of every kind, and a second commit that changes them in every awkward way: a rename, names
/// with spaces, accents, brackets, a leading dash, a tab and a double quote, a file with no newline at its end, an empty
/// file, a binary file, a mode change, a file that becomes a link, a file deleted, a submodule added and a copy (which
/// `-M` lists as added). Returns the repository, the first commit and the second.
pub fn awkward_repository() -> (Repository, String, String) {
    let mut repository = Repository::new();
    let first = repository.commit(
        "Add the files",
        &[
            ("renamed from.txt", "one\ntwo\nthree\nfour\n"),
            ("with space.txt", "1\n"),
            ("accentué.txt", "1\n"),
            ("pages/[id].tsx", "1\n"),
            ("no newline.txt", "a\nb"),
            ("image.bin", "\0\u{1}\u{2}"),
            ("script.sh", "echo\n"),
            ("link", "target\n"),
            ("deleted.txt", "1\n"),
        ],
    );
    unstage(&repository, "renamed from.txt");
    stage(&repository, "100644", "renamed to.txt", b"one\ntwo\nthree\nfour\n");
    stage(&repository, "100644", "with space.txt", b"2\n");
    stage(&repository, "100644", "accentué.txt", b"2\n");
    stage(&repository, "100644", "pages/[id].tsx", b"2\n");
    stage(&repository, "100644", "-leading-dash.txt", b"new\n");
    stage(&repository, "100644", "tab\tname.txt", b"tab\n");
    stage(&repository, "100644", "q\"uote.txt", b"quote\n");
    stage(&repository, "100644", "no newline.txt", b"a\nc");
    stage(&repository, "100644", "empty.txt", b"");
    stage(&repository, "100644", "image.bin", b"\0\x03\x04");
    stage(&repository, "100755", "script.sh", b"echo\n");
    stage(&repository, "120000", "link", b"target");
    unstage(&repository, "deleted.txt");
    stage(&repository, "160000", "sub", first.as_bytes());
    stage(&repository, "100644", "copy.txt", b"one\ntwo\nthree\nfour\n");
    let message =
        "Change every file: [draft] é\n\nA body with\ta tab, and \"quotes\".\n\nCo-Authored-By: Zoë <zoe@example.com>";
    let second = repository.commit(message, &[]);
    (repository, first, second)
}

/// A merge of `feature` into `main`, each having changed a file of its own: `main` is the first parent. Returns the
/// repository, the merge, and the commit of `main` before it.
pub fn merge_repository() -> (Repository, String, String) {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n"), ("notes.txt", "a\n")]);
    repository.branch("feature");
    repository.checkout("feature");
    let _ = repository.commit("Change the notes", &[("notes.txt", "b\n"), ("feature.txt", "f\n")]);
    repository.checkout("main");
    let before = repository.commit("Change the parser", &[("parser.rs", "2\n")]);
    repository.merge("feature");
    let merge = repository.git(&["rev-parse", "HEAD"]).trim().to_owned();
    (repository, merge, before)
}

/// `main` and `feature` from one commit: `feature` three commits ahead with a change `main` cherry-picked, `main` two
/// ahead, both changing the same line of `shared.txt`. Returns the repository and the commit they diverged at.
pub fn diverged_repository() -> (Repository, String) {
    let mut repository = Repository::new();
    let base = repository.commit(
        "Add the files",
        &[
            ("shared.txt", "base\n"),
            ("src/app.rs", "1\n"),
            ("pages/[id].tsx", "1\n"),
        ],
    );
    repository.branch("feature");
    repository.checkout("feature");
    let picked = repository.commit("Fix the app: [draft] é", &[("src/app.rs", "2\n")]);
    let _ = repository.commit("Change the shared file on feature", &[("shared.txt", "feature\n")]);
    let _ = repository.commit("Add a page", &[("pages/i.tsx", "i\n"), ("pages/[id].tsx", "2\n")]);
    repository.checkout("main");
    let _ = repository.git(&["cherry-pick", &picked]);
    let _ = repository.commit("Change the shared file on main", &[("shared.txt", "main\n")]);
    (repository, base)
}
