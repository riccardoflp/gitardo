//! Which of some files git ignores, for the files with edits not saved that the Changes page lists
//! (`changes.unsaved`).

use std::ffi::OsString;

use crate::read::fields::arguments;
use crate::{Git, GitError, command_line};

/// The paths among some, from the top of the working tree with `/`, that git ignores; a file git tracks is never one
/// of them.
///
/// # Errors
///
/// When git cannot be started, is cancelled or fails.
pub fn read_ignored(git: &Git, paths: &[String]) -> Result<Vec<String>, GitError> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let words = ignored_arguments();
    let output = git.read_output_with_input(&words, &ignored_input(paths))?;
    // `check-ignore` ends with 1 when it ignores none of them.
    match output.code {
        Some(0 | 1) => Ok(parse_ignored(&output.stdout)),
        code => Err(GitError::Failed {
            command: command_line(&words),
            code,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }),
    }
}

/// `git check-ignore -z --stdin`.
#[must_use]
pub fn ignored_arguments() -> Vec<OsString> {
    arguments(&["check-ignore", "-z", "--stdin"])
}

/// The paths given on the standard input of [`ignored_arguments`], each ended by a NUL.
#[must_use]
pub fn ignored_input(paths: &[String]) -> Vec<u8> {
    paths
        .iter()
        .flat_map(|path| path.as_bytes().iter().copied().chain([0]))
        .collect()
}

/// The paths [`ignored_arguments`] printed, each ended by a NUL.
#[must_use]
pub fn parse_ignored(output: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(output)
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{ignored_input, parse_ignored, read_ignored};
    use crate::Git;

    #[test]
    fn asks_with_nul_after_each_path_and_reads_the_paths_printed() {
        assert_eq!(ignored_input(&["a b".to_owned(), "c".to_owned()]), b"a b\0c\0");
        assert_eq!(parse_ignored(b"node_modules/x.js\0"), ["node_modules/x.js"]);
    }

    #[test]
    fn a_file_in_an_ignored_folder_is_ignored_and_a_tracked_one_is_not() {
        let mut repository = testkit::Repository::new();
        let _ = repository.commit("Add the files", &[(".gitignore", "build/\n"), ("src/a.rs", "a\n")]);
        let git = Git::new(repository.path());
        let paths = ["build/out.txt".to_owned(), "src/a.rs".to_owned()];
        assert_eq!(read_ignored(&git, &paths).unwrap(), ["build/out.txt"]);
        assert_eq!(read_ignored(&git, &paths[1..]).unwrap(), Vec::<String>::new());
    }
}
