//! Golden files: what a test produced, compared with what was recorded before and looked at by a person.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::error::Error;

/// The environment variable that makes [`assert_golden`] record the actual content instead of comparing it.
const RECORD_VARIABLE: &str = "GITARDO_GOLDEN_RECORD";

/// Whether a golden file is compared with the actual content, or replaced by it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// The actual content must be the content of the file.
    Compare,
    /// The actual content is written to the file.
    Record,
}

impl Mode {
    /// [`Mode::Record`] when `GITARDO_GOLDEN_RECORD=1` is set, [`Mode::Compare`] otherwise.
    fn from_environment() -> Mode {
        match std::env::var_os(RECORD_VARIABLE) {
            Some(value) if value == "1" => Mode::Record,
            _ => Mode::Compare,
        }
    }
}

/// Checks that the actual content, text or bytes, is exactly the content of a golden file. With
/// `GITARDO_GOLDEN_RECORD=1` set, writes the actual content to the file instead, making its folders.
///
/// Nothing is normalized: a line ending, a trailing space or a missing last line break is a difference.
///
/// # Panics
///
/// When the content differs, with the path of the file, the first line that differs and that line on both sides;
/// when the file does not exist yet; and when it cannot be read or written.
pub fn assert_golden(golden_file: impl AsRef<Path>, actual: impl AsRef<[u8]>) {
    if let Err(error) = check(golden_file.as_ref(), actual.as_ref(), Mode::from_environment()) {
        panic!("{error}");
    }
}

/// Compares the actual content with a golden file, or records it, as the mode says.
fn check(golden_file: &Path, actual: &[u8], mode: Mode) -> Result<(), Error> {
    match mode {
        Mode::Record => record(golden_file, actual),
        Mode::Compare => compare(golden_file, actual),
    }
}

/// Writes the actual content to a golden file, making the folders it is in.
fn record(golden_file: &Path, actual: &[u8]) -> Result<(), Error> {
    let failed = |error| Error::File {
        path: golden_file.to_path_buf(),
        error,
    };
    if let Some(parent) = golden_file.parent() {
        fs::create_dir_all(parent).map_err(failed)?;
    }
    fs::write(golden_file, actual).map_err(failed)
}

/// Compares the actual content with the content of a golden file.
fn compare(golden_file: &Path, actual: &[u8]) -> Result<(), Error> {
    let expected = match fs::read(golden_file) {
        Ok(expected) => expected,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Err(Error::GoldenMissing {
                path: golden_file.to_path_buf(),
            });
        }
        Err(error) => {
            return Err(Error::File {
                path: golden_file.to_path_buf(),
                error,
            });
        }
    };
    match first_difference(&expected, actual) {
        None => Ok(()),
        Some(difference) => Err(Error::GoldenMismatch {
            path: golden_file.to_path_buf(),
            line: difference.line,
            expected: difference.expected,
            actual: difference.actual,
        }),
    }
}

/// The first line where two contents differ.
#[derive(Debug, PartialEq, Eq)]
struct LineDifference {
    /// The line, from 1.
    line: usize,
    /// The line in the expected content, or nothing when it ends before.
    expected: Option<String>,
    /// The line in the actual content, or nothing when it ends before.
    actual: Option<String>,
}

/// The first line where two contents differ, or nothing when they are the same bytes. Lines are split at line feeds
/// only, so that a carriage return stays in its line and is shown; a content that ends with a line feed has an empty
/// last line, so that a missing last line break is a difference too.
fn first_difference(expected: &[u8], actual: &[u8]) -> Option<LineDifference> {
    if expected == actual {
        return None;
    }
    let mut expected_lines = expected.split(|byte| *byte == b'\n');
    let mut actual_lines = actual.split(|byte| *byte == b'\n');
    let mut line = 1;
    loop {
        let (expected_line, actual_line) = (expected_lines.next(), actual_lines.next());
        if expected_line.is_none() && actual_line.is_none() {
            return None;
        }
        if expected_line != actual_line {
            return Some(LineDifference {
                line,
                expected: expected_line.map(|bytes| String::from_utf8_lossy(bytes).into_owned()),
                actual: actual_line.map(|bytes| String::from_utf8_lossy(bytes).into_owned()),
            });
        }
        line += 1;
    }
}

#[cfg(test)]
mod tests;
