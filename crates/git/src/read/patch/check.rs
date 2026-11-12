//! What applying a patch with `--3way` would do, without doing it.

use std::ffi::OsString;
use std::path::PathBuf;

use model::patch::PatchCheck;

use crate::{Git, GitError, Output};

/// What `git apply` says of a patch that applies, for each file it would leave in conflict.
const CONFLICT_START: &str = "Applied patch to '";
/// The end of the same line.
const CONFLICT_END: &str = "' with conflicts.";

/// Where the patch to check is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchSource {
    /// In a file.
    File(PathBuf),
    /// Not in a file, as a patch on the clipboard: it is given to git on its standard input.
    Text(Vec<u8>),
}

/// The patch to check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchCheckRequest {
    /// Where it is.
    pub patch: PatchSource,
}

/// `git apply --check --3way <file>`, or `git apply --check --3way` with the patch on standard input.
#[must_use]
pub fn check_arguments(request: &PatchCheckRequest) -> Vec<OsString> {
    let mut arguments: Vec<OsString> = ["apply", "--check", "--3way"].into_iter().map(OsString::from).collect();
    if let PatchSource::File(path) = &request.patch {
        arguments.push(OsString::from("--"));
        arguments.push(path.clone().into_os_string());
    }
    arguments
}

/// What `git apply --check --3way` said, by how it ended. With the exit code 0 the patch applies, and each line
/// `Applied patch to '<file>' with conflicts.` names a file it would leave in conflict. Otherwise it does not apply, and
/// the reasons are its lines that start with `error:` or `fatal:`, without that word, each once; when there is none, all
/// it printed on its error output, or **git apply exited with code N**.
#[must_use]
pub fn parse_check(output: &Output) -> PatchCheck {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let lines = stdout.lines().chain(stderr.lines());
    if output.is_success() {
        let conflicts = lines
            .filter_map(|line| line.strip_prefix(CONFLICT_START)?.strip_suffix(CONFLICT_END))
            .map(str::to_owned)
            .collect();
        return PatchCheck::Applies { conflicts };
    }
    let mut reasons: Vec<String> = Vec::new();
    for line in lines {
        let Some(reason) = line.strip_prefix("error:").or_else(|| line.strip_prefix("fatal:")) else {
            continue;
        };
        let reason = reason.trim();
        if !reasons.iter().any(|known| known == reason) {
            reasons.push(reason.to_owned());
        }
    }
    if reasons.is_empty() {
        let said = stderr.trim();
        reasons.push(if said.is_empty() {
            match output.code {
                Some(code) => format!("git apply exited with code {code}"),
                None => String::from("git apply was ended before it said anything"),
            }
        } else {
            said.to_owned()
        });
    }
    PatchCheck::Refused { reasons }
}

/// What applying a patch with `--3way` would do, without doing it: the files it would leave in conflict, or what keeps
/// it from applying at all. A patch of several commits is checked as a whole, each change on top of the ones before it.
/// It runs as a read: it takes no lock and never asks.
///
/// Feature: patch.check
///
/// # Errors
///
/// When git cannot be started or is cancelled. A patch that does not apply is an answer, not an error.
pub fn check(git: &Git, request: &PatchCheckRequest) -> Result<PatchCheck, GitError> {
    let arguments = check_arguments(request);
    let output = match &request.patch {
        PatchSource::File(_) => git.read_output(&arguments)?,
        PatchSource::Text(text) => git.read_output_with_input(&arguments, text)?,
    };
    Ok(parse_check(&output))
}

#[cfg(test)]
mod tests {
    use super::parse_check;
    use crate::Output;
    use model::patch::PatchCheck;

    fn ended(code: i32, stderr: &str) -> Output {
        Output {
            code: Some(code),
            stdout: Vec::new(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    #[test]
    fn reads_conflicts_reasons_and_a_failure_that_says_nothing() {
        let applies = parse_check(&ended(0, "Applied patch to 'a b.txt' with conflicts.\nU a b.txt\n"));
        assert_eq!(
            applies,
            PatchCheck::Applies {
                conflicts: vec![String::from("a b.txt")]
            }
        );
        let refused = parse_check(&ended(
            1,
            "error: x: does not exist in index\nerror: x: does not exist in index\n",
        ));
        assert_eq!(
            refused,
            PatchCheck::Refused {
                reasons: vec![String::from("x: does not exist in index")]
            }
        );
        let said = parse_check(&ended(1, "something else\n"));
        assert_eq!(
            said,
            PatchCheck::Refused {
                reasons: vec![String::from("something else")]
            }
        );
        let silent = parse_check(&ended(128, ""));
        assert_eq!(
            silent,
            PatchCheck::Refused {
                reasons: vec![String::from("git apply exited with code 128")]
            }
        );
    }
}
