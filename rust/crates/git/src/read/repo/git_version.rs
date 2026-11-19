//! Which git runs the operations (`repo.read-git-version`).

use std::ffi::OsString;

use model::rebase::GitVersion;

use crate::read::fields::{arguments, unexpected};
use crate::{Git, GitError};

/// The git of the engine: it is the same for every repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GitVersionRequest;

/// The version of git, from `git --version` in the C locale.
///
/// Feature: repo.read-git-version
///
/// # Errors
///
/// When git cannot be started or fails, or its line is not `git version <major>.<minor>.<patch>…`.
pub fn read_git_version(git: &Git, _request: &GitVersionRequest) -> Result<GitVersion, GitError> {
    parse_git_version(&git.run_read(&git_version_arguments())?)
}

/// `git --version`.
#[must_use]
pub fn git_version_arguments() -> Vec<OsString> {
    arguments(&["--version"])
}

/// The version `git --version` printed: `git version 2.53.0.windows.1` gives 2.53.0, what follows the three numbers
/// kept as said.
///
/// # Errors
///
/// When the line is of another form, quoted in the error.
pub fn parse_git_version(output: &[u8]) -> Result<GitVersion, GitError> {
    const WHAT: &str = "the version of git";
    let text = String::from_utf8_lossy(output);
    let line = text.trim_end();
    let said = line
        .strip_prefix("git version ")
        .ok_or_else(|| unexpected(WHAT, line))?;
    let mut numbers = said.split(['.', ' ']).map(|part| part.parse::<u32>().ok());
    let mut next = || numbers.next().flatten().ok_or_else(|| unexpected(WHAT, line));
    Ok(GitVersion {
        major: next()?,
        minor: next()?,
        patch: next()?,
        said: said.to_owned(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::parse_git_version;

    #[test]
    fn repo_read_git_version__the_windows_build_gives_its_three_numbers() {
        let version = parse_git_version(b"git version 2.53.0.windows.1\n").unwrap();
        assert_eq!((version.major, version.minor, version.patch), (2, 53, 0));
        assert_eq!(version.said, "2.53.0.windows.1");
    }

    #[test]
    fn repo_read_git_version__the_apple_build_gives_its_three_numbers() {
        let version = parse_git_version(b"git version 2.39.5 (Apple Git-154)\n").unwrap();
        assert_eq!((version.major, version.minor, version.patch), (2, 39, 5));
        assert!(version.is_at_least((2, 38)));
        assert!(!version.is_at_least((2, 42)));
    }

    #[test]
    fn repo_read_git_version__a_line_of_another_form_is_an_error_that_quotes_it() {
        for line in ["hub version 2.14.2\n", "git version two\n", "git version 2.53\n"] {
            let error = parse_git_version(line.as_bytes()).unwrap_err();
            assert!(error.to_string().contains(line.trim_end()), "{error}");
        }
    }
}
