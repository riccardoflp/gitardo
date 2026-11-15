//! The stashes, newest first.

use std::ffi::OsString;

use model::state::Stash;

use crate::read::fields::{arguments, commit_id, timestamp, unexpected};
use crate::{Git, GitError};

/// What is being read, for the errors.
const WHAT: &str = "the stashes";

/// `git stash list -z` with, separated by the unit separator, the commit, `stash@{n}`, the message and the date.
#[must_use]
pub fn stashes_arguments() -> Vec<OsString> {
    arguments(&["stash", "list", "-z", "--format=%H%x1f%gd%x1f%gs%x1f%at"])
}

/// The stashes that `git stash list` printed with the format of [`stashes_arguments`].
///
/// # Errors
///
/// When a stash does not have its four fields, or one of them cannot be read.
pub fn parse_stashes(output: &[u8]) -> Result<Vec<Stash>, GitError> {
    let text = String::from_utf8_lossy(output);
    text.split('\0')
        .map(|record| record.trim_start_matches('\n'))
        .filter(|record| !record.is_empty())
        .map(stash)
        .collect()
}

/// The stashes of the repository. When the last stash goes while git reads them, as when one is dropped or a pull sets
/// the changes aside and takes them back, `refs/stash` is gone between git's check and its read: there are none.
///
/// # Errors
///
/// When git cannot be started, fails for another reason, or prints a stash this does not understand.
pub fn read_stashes(git: &Git) -> Result<Vec<Stash>, GitError> {
    match git.run_read(&stashes_arguments()) {
        Ok(output) => parse_stashes(&output),
        Err(error) if stash_went_away(&error) => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}

/// Whether git failed because `refs/stash` was there when it looked and gone when it read it.
fn stash_went_away(error: &GitError) -> bool {
    error
        .stderr()
        .is_some_and(|said| said.contains("bad revision 'refs/stash'"))
}

/// One stash, from its four fields. The message is what is between the second field and the last, whatever it holds.
fn stash(record: &str) -> Result<Stash, GitError> {
    let mut fields = record.splitn(3, '\x1f');
    let (Some(hash), Some(selector), Some((message, date))) = (
        fields.next(),
        fields.next(),
        fields.next().and_then(|rest| rest.rsplit_once('\x1f')),
    ) else {
        return Err(unexpected(WHAT, record));
    };
    let index = selector
        .strip_prefix("stash@{")
        .and_then(|rest| rest.strip_suffix('}'))
        .and_then(|number| number.parse().ok())
        .ok_or_else(|| unexpected(WHAT, record))?;
    Ok(Stash {
        commit: commit_id(WHAT, hash)?,
        index,
        message: message.to_owned(),
        date: timestamp(WHAT, date)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_stashes, stash_went_away};
    use crate::GitError;

    #[test]
    fn a_stash_gone_while_git_reads_it_is_no_stash_and_another_failure_is_one() {
        let failed = |stderr: &str| GitError::Failed {
            command: "git stash list".to_owned(),
            code: Some(128),
            stderr: stderr.to_owned(),
        };
        assert!(stash_went_away(&failed("fatal: bad revision 'refs/stash'\n")));
        assert!(!stash_went_away(&failed("fatal: not a git repository\n")));
    }

    #[test]
    fn refuses_a_stash_without_its_place_in_the_list() {
        let hash = "df6469edf31612a57f44ab30850ce15975ab9c1d";
        assert!(parse_stashes(format!("{hash}\x1fstash@{{x}}\x1fOn main: x\x1f1\0").as_bytes()).is_err());
        assert!(parse_stashes(format!("{hash}\x1fstash@{{0}}\x1fOn main: x\0").as_bytes()).is_err());
        assert_eq!(parse_stashes(b"").unwrap(), Vec::new());
    }
}
