//! Git's own order of the branches, `branch.sort`, which `branches.sortOrder` follows while it is not set (`refs.sort`).

use std::ffi::OsString;

use crate::read::fields::{arguments, read_or_nothing};
use crate::{Git, GitError};

/// `git config --get branch.sort`: the value git reads last, on a line; the exit code 1 when it is not set.
#[must_use]
pub fn branch_sort_arguments() -> Vec<OsString> {
    arguments(&["config", "--get", "branch.sort"])
}

/// The value that `git config --get branch.sort` printed, such as `-committerdate`; none when it is empty.
///
/// # Errors
///
/// None: any value is kept. The `Result` keeps the shape of every parser of a read.
pub fn parse_branch_sort(output: &[u8]) -> Result<Option<String>, GitError> {
    let text = String::from_utf8_lossy(output);
    let value = text.trim();
    Ok((!value.is_empty()).then(|| value.to_owned()))
}

/// Git's `branch.sort` in the repository, when it is set.
pub(super) fn read_branch_sort(git: &Git) -> Result<Option<String>, GitError> {
    read_or_nothing(git, &branch_sort_arguments())?.map_or(Ok(None), |output| parse_branch_sort(&output))
}

#[cfg(test)]
mod tests {
    use super::parse_branch_sort;

    #[test]
    fn reads_the_value_on_its_line_and_nothing_for_an_empty_one() {
        assert_eq!(
            parse_branch_sort(b"-committerdate\n").unwrap().as_deref(),
            Some("-committerdate")
        );
        assert_eq!(parse_branch_sort(b"\n").unwrap(), None);
    }
}
