//! The stashes of a repository, newest first.

use model::state::Stash;

use crate::read::repo::read_stashes;
use crate::{Git, GitError};

/// Which repository: the one git runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StashListRequest;

/// The stashes of the repository, newest first, as every pane, the references and the graph list them: the hash of
/// each, its place in the list, its message whole and its date. A repository with no stash has none.
///
/// Feature: stash.read-list
///
/// # Errors
///
/// When git cannot be started, fails, or prints a stash this does not understand.
pub fn read_list(git: &Git, _request: &StashListRequest) -> Result<Vec<Stash>, GitError> {
    read_stashes(git)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use crate::read::repo::parse_stashes;

    #[test]
    fn stash_read_list__a_message_with_spaces_colons_and_quotes_is_whole() {
        let hash = "df6469edf31612a57f44ab30850ce15975ab9c1d";
        let output = format!("{hash}\x1fstash@{{0}}\x1fOn main: \"quoted\": it's\x1f1700000000\0");
        let stashes = parse_stashes(output.as_bytes()).unwrap();
        assert_eq!(stashes[0].message, "On main: \"quoted\": it's");
        assert_eq!(stashes[0].index, 0);
    }
}
