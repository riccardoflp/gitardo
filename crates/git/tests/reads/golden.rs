//! Golden files of the reads: `<read>/<case>.out` is what one command printed in an awkward repository,
//! `<read>/<case>.expected` what its parser makes of it.
//!
//! With `GITARDO_GOLDEN_RECORD=1`, each test makes its repository, runs the command with the arguments of the read and
//! records both files again; otherwise the parsers read the files, without git.

use std::ffi::OsString;
use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;

use testkit::{Repository, assert_golden};

/// A case of a read: the folder of the read's golden files and the name of the case.
#[derive(Clone, Copy)]
pub struct Case {
    /// The folder under `tests/golden`, named after the read.
    pub read: &'static str,
    /// The name of the case.
    pub name: &'static str,
}

/// The file of a case, with its extension.
fn golden_file(case: Case, extension: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(case.read)
        .join(format!("{}.{extension}", case.name))
}

/// Whether the golden files are being recorded again.
pub fn is_recording() -> bool {
    std::env::var_os("GITARDO_GOLDEN_RECORD").is_some_and(|value| value == "1")
}

/// When recording, makes the repository of a case, runs a read's arguments in the folder the function gives with the
/// configuration every read gets, and keeps what git printed in `<case>.out`, with the folder of the test written
/// `<folder>` so that the file is the same on every machine. A command whose exit code is part of its answer may fail.
pub fn record(case: Case, repository: impl FnOnce() -> Repository, arguments: &[OsString]) {
    if !is_recording() {
        return;
    }
    let repository = repository();
    let output = repository
        .command("git")
        .args(["-c", "core.quotepath=false"])
        .args(arguments)
        .output()
        .unwrap();
    let printed = String::from_utf8(output.stdout).unwrap();
    // Git writes the folder with forward slashes, and a URL given with backslashes keeps them.
    let folder = repository.path().parent().unwrap().to_string_lossy().into_owned();
    let printed = printed
        .replace(&folder, "<folder>")
        .replace(&folder.replace('\\', "/"), "<folder>")
        .replace("<folder>\\", "<folder>/");
    assert_golden(golden_file(case, "out"), printed);
}

/// Parses `<case>.out` and compares what the parser made of it with `<case>.expected`.
pub fn check<Parsed: Debug>(case: Case, parse: impl Fn(&[u8]) -> Parsed) {
    let output = fs::read(golden_file(case, "out")).unwrap();
    assert_golden(golden_file(case, "expected"), format!("{:#?}\n", parse(&output)));
}
