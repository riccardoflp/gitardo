//! The files of the working tree, for the Files page: those git tracks, and those it does not track nor ignore.

use std::collections::BTreeSet;
use std::ffi::OsString;

use crate::read::fields::arguments;
use crate::{Git, GitError};

/// The paths of the files of the working tree from its top, with `/`, each once, in order: the files git tracks and the
/// files it does not track nor ignore. A submodule is one path, the folder git tracks for it.
///
/// # Errors
///
/// When git cannot be started or fails.
pub fn read_files(git: &Git) -> Result<Vec<String>, GitError> {
    git.run_read(&files_arguments()).map(|output| parse_files(&output))
}

/// `git ls-files -z --cached --others --exclude-standard`.
#[must_use]
pub fn files_arguments() -> Vec<OsString> {
    arguments(&["ls-files", "-z", "--cached", "--others", "--exclude-standard"])
}

/// The paths [`files_arguments`] listed, each ended by a NUL: a file in conflict, listed once for each of its stages,
/// is kept once.
#[must_use]
pub fn parse_files(output: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(output);
    let paths: BTreeSet<&str> = text.split('\0').filter(|path| !path.is_empty()).collect();
    paths.into_iter().map(str::to_owned).collect()
}

#[cfg(test)]
mod tests {
    use super::parse_files;

    #[test]
    fn keeps_each_file_once_in_order() {
        let output = b"src/b.rs\0README.md\0src/b.rs\0new file.txt\0";
        assert_eq!(parse_files(output), ["README.md", "new file.txt", "src/b.rs"]);
        assert_eq!(parse_files(b""), Vec::<String>::new());
    }
}
