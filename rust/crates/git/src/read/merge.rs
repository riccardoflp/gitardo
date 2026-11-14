//! The conflicts a merge would stop on, worked out without touching the index or the working tree.

use std::ffi::OsString;
use std::path::PathBuf;

use crate::read::fields::arguments;
use crate::{Git, GitError, Output};

/// The git that `merge-tree --write-tree --name-only` needs, 2.40: an older git ends with another exit code, and the
/// merge runs without a prediction.
pub const MERGE_TREE_NAME_ONLY_VERSION: &str = "2.40";

/// How many paths the sentence of a prediction lists before it says how many more there are.
pub const PREDICTED_PATHS_SHOWN: usize = 15;

/// What to merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredictedConflictsRequest {
    /// What the merge goes into; `HEAD` when none is given.
    pub ours: Option<String>,
    /// What is merged.
    pub theirs: String,
    /// The base to merge from, for a cherry-pick: the parent of the commit picked. None lets git find it.
    pub base: Option<String>,
}

/// The files a merge would leave in conflict, each once, in the order git gives them: none when it would stop on
/// none, and "cannot tell" (`None`) when git cannot work it out, as an older git or histories with no common ancestor.
///
/// Feature: merge.read-predicted-conflicts
///
/// # Errors
///
/// When git cannot be started or is cancelled.
pub fn read_predicted_conflicts(
    git: &Git,
    request: &PredictedConflictsRequest,
) -> Result<Option<Vec<PathBuf>>, GitError> {
    Ok(parse_predicted_conflicts(
        &git.read_output(&predicted_conflicts_arguments(request))?,
    ))
}

/// `git merge-tree --write-tree --no-messages --name-only -z [--merge-base=<base>] <ours> <theirs>`.
#[must_use]
pub fn predicted_conflicts_arguments(request: &PredictedConflictsRequest) -> Vec<OsString> {
    let mut words = arguments(&["merge-tree", "--write-tree", "--no-messages", "--name-only", "-z"]);
    if let Some(base) = &request.base {
        words.push(OsString::from(format!("--merge-base={base}")));
    }
    words.push(OsString::from(request.ours.as_deref().unwrap_or("HEAD")));
    words.push(OsString::from(&request.theirs));
    words
}

/// What `git merge-tree` said: with the exit code 0 no conflict, with 1 the paths after the tree it wrote, each once;
/// with any other, "cannot tell".
#[must_use]
pub fn parse_predicted_conflicts(output: &Output) -> Option<Vec<PathBuf>> {
    match output.code {
        Some(0) => Some(Vec::new()),
        Some(1) => {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut paths: Vec<PathBuf> = Vec::new();
            for path in text.split('\0').skip(1).filter(|path| !path.is_empty()) {
                let path = PathBuf::from(path);
                if !paths.contains(&path) {
                    paths.push(path);
                }
            }
            Some(paths)
        }
        _ => None,
    }
}

/// How a prediction is said by the operation that asked: *No conflicts expected.*, or *This will stop with conflicts
/// in N files:* and the paths one per line, at most [`PREDICTED_PATHS_SHOWN`], then *…and N more*.
#[must_use]
pub fn describe_predicted_conflicts(paths: &[PathBuf]) -> String {
    if paths.is_empty() {
        return String::from("No conflicts expected.");
    }
    let files = if paths.len() == 1 {
        String::from("1 file")
    } else {
        format!("{} files", paths.len())
    };
    let mut lines = vec![format!("This will stop with conflicts in {files}:")];
    lines.extend(
        paths
            .iter()
            .take(PREDICTED_PATHS_SHOWN)
            .map(|path| path.to_string_lossy().into_owned()),
    );
    if paths.len() > PREDICTED_PATHS_SHOWN {
        lines.push(format!("…and {} more", paths.len() - PREDICTED_PATHS_SHOWN));
    }
    lines.join("\n")
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use super::{
        PredictedConflictsRequest, describe_predicted_conflicts, parse_predicted_conflicts,
        predicted_conflicts_arguments,
    };
    use crate::Output;

    fn output(code: i32, stdout: &str) -> Output {
        Output {
            code: Some(code),
            stdout: stdout.as_bytes().to_vec(),
            stderr: Vec::new(),
        }
    }

    const TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

    #[test]
    fn merge_read_predicted_conflicts__conflicts_give_each_path_once() {
        let read = parse_predicted_conflicts(&output(1, &format!("{TREE}\0a.txt\0a.txt\0b.txt\0")));
        assert_eq!(read, Some(vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")]));
    }

    #[test]
    fn merge_read_predicted_conflicts__no_conflict_gives_an_empty_list() {
        assert_eq!(
            parse_predicted_conflicts(&output(0, &format!("{TREE}\0"))),
            Some(Vec::new())
        );
        assert_eq!(describe_predicted_conflicts(&[]), "No conflicts expected.");
    }

    #[test]
    fn merge_read_predicted_conflicts__exit_code_128_cannot_tell() {
        assert_eq!(parse_predicted_conflicts(&output(128, "")), None);
    }

    #[test]
    fn merge_read_predicted_conflicts__twenty_paths_are_said_as_fifteen_lines_and_5_more() {
        let paths: Vec<PathBuf> = (1..=20)
            .map(|index| PathBuf::from(format!("file{index}.txt")))
            .collect();
        let said = describe_predicted_conflicts(&paths);
        let lines: Vec<&str> = said.lines().collect();
        assert_eq!(lines.len(), 17);
        assert_eq!(lines[0], "This will stop with conflicts in 20 files:");
        assert_eq!(lines[15], "file15.txt");
        assert_eq!(lines[16], "…and 5 more");
        let one = describe_predicted_conflicts(&[PathBuf::from("a.txt")]);
        assert_eq!(one, "This will stop with conflicts in 1 file:\na.txt");
    }

    #[test]
    fn merge_read_predicted_conflicts__a_cherry_pick_gives_the_base_and_ours_defaults_to_head() {
        let request = PredictedConflictsRequest {
            ours: None,
            theirs: String::from("feature"),
            base: Some(String::from("abc1234")),
        };
        let words = predicted_conflicts_arguments(&request);
        let tail: Vec<&str> = words
            .iter()
            .rev()
            .take(3)
            .rev()
            .map(|word| word.to_str().unwrap())
            .collect();
        assert_eq!(tail, ["--merge-base=abc1234", "HEAD", "feature"]);
    }
}
