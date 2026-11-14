//! The local branches with their upstream, the remote branches and the tags.

use std::ffi::OsString;

use model::state::{Branch, RemoteBranch, Tag, Upstream};

use crate::read::fields::{arguments, commit_id, timestamp, unexpected};
use crate::{Git, GitError};

/// What is being read, for the errors.
const WHAT: &str = "the references";

/// The fields of each reference, NUL-separated, one reference per line: a reference name never holds a line break.
const FORMAT: &str = "--format=%(refname)%00%(objectname)%00%(*objectname)%00%(upstream)%00%(upstream:remotename)\
%00%(upstream:remoteref)%00%(upstream:track)%00%(HEAD)%00%(creatordate:unix)";

/// The references of a repository, each list by name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct References {
    /// The local branches.
    pub branches: Vec<Branch>,
    /// The remote branches, without `<remote>/HEAD`.
    pub remote_branches: Vec<RemoteBranch>,
    /// The tags, each at the commit it points at.
    pub tags: Vec<Tag>,
}

/// `git for-each-ref` with the fields of [`FORMAT`], over the local branches, the remote branches and the tags.
#[must_use]
pub fn references_arguments() -> Vec<OsString> {
    arguments(&["for-each-ref", FORMAT, "refs/heads", "refs/remotes", "refs/tags"])
}

/// The references that `git for-each-ref` printed with the fields of [`references_arguments`].
///
/// # Errors
///
/// When a line does not have the nine fields, or a hash, a date or the distance to an upstream cannot be read.
pub fn parse_references(output: &[u8]) -> Result<References, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut references = References::default();
    for line in text.split('\n').filter(|line| !line.is_empty()) {
        add_reference(&mut references, &record(line)?)?;
    }
    Ok(references)
}

/// The references of the repository.
pub(super) fn read_references(git: &Git) -> Result<References, GitError> {
    parse_references(&git.run_read(&references_arguments())?)
}

/// Gives each remote branch the remote it belongs to among the remotes of the repository, the longest name first, as a
/// remote name may hold a `/`; and leaves out `<remote>/HEAD` of such a remote.
pub(super) fn with_remotes(remote_branches: Vec<RemoteBranch>, remotes: &[String]) -> Vec<RemoteBranch> {
    remote_branches
        .into_iter()
        .filter_map(|branch| {
            let remote = remotes
                .iter()
                .filter(|remote| branch.name.starts_with(&format!("{remote}/")))
                .max_by_key(|remote| remote.len());
            match remote {
                Some(remote) if branch.name.get(remote.len() + 1..) == Some("HEAD") => None,
                Some(remote) => Some(RemoteBranch {
                    remote: remote.clone(),
                    ..branch
                }),
                None => Some(branch),
            }
        })
        .collect()
}

/// The fields of one line of `git for-each-ref`.
struct Record<'a> {
    name: &'a str,
    object: &'a str,
    peeled: &'a str,
    upstream: &'a str,
    remote: &'a str,
    remote_branch: &'a str,
    track: &'a str,
    head: &'a str,
    date: &'a str,
}

/// The nine fields of a line.
fn record(line: &str) -> Result<Record<'_>, GitError> {
    let fields: Vec<&str> = line.split('\0').collect();
    let &[name, object, peeled, upstream, remote, remote_branch, track, head, date] = fields.as_slice() else {
        return Err(unexpected(WHAT, line));
    };
    Ok(Record {
        name,
        object,
        peeled,
        upstream,
        remote,
        remote_branch,
        track,
        head,
        date,
    })
}

/// Adds a reference to the list of its kind.
fn add_reference(references: &mut References, record: &Record<'_>) -> Result<(), GitError> {
    if let Some(name) = record.name.strip_prefix("refs/heads/") {
        references.branches.push(Branch {
            name: name.to_owned(),
            commit: commit_id(WHAT, record.object)?,
            upstream: upstream(record)?,
            is_checked_out: record.head == "*",
            date: timestamp(WHAT, record.date)?,
        });
    } else if let Some(name) = record.name.strip_prefix("refs/remotes/") {
        let (remote, branch) = name.split_once('/').unwrap_or((name, ""));
        // `<remote>/HEAD` names the branch the remote checks out; it is not a branch of its own.
        if branch != "HEAD" {
            references.remote_branches.push(RemoteBranch {
                name: name.to_owned(),
                remote: remote.to_owned(),
                commit: commit_id(WHAT, record.object)?,
                date: timestamp(WHAT, record.date)?,
            });
        }
    } else if let Some(name) = record.name.strip_prefix("refs/tags/") {
        let commit = if record.peeled.is_empty() {
            record.object
        } else {
            record.peeled
        };
        references.tags.push(Tag {
            name: name.to_owned(),
            commit: commit_id(WHAT, commit)?,
            date: tag_date(record.date)?,
        });
    }
    Ok(())
}

/// The date of a tag. A tag object with no tagger, as some of the oldest are, or a tag of a tree has no date, and is
/// read as the oldest.
fn tag_date(text: &str) -> Result<model::Timestamp, GitError> {
    if text.is_empty() {
        Ok(model::Timestamp::default())
    } else {
        timestamp(WHAT, text)
    }
}

/// The upstream of a local branch, if it has one.
fn upstream(record: &Record<'_>) -> Result<Option<Upstream>, GitError> {
    if record.upstream.is_empty() {
        return Ok(None);
    }
    let distance = parse_track(record.track)?;
    Ok(Some(Upstream {
        reference: record.upstream.to_owned(),
        remote: record.remote.to_owned(),
        remote_branch: record.remote_branch.to_owned(),
        ahead: distance.ahead,
        behind: distance.behind,
        is_gone: distance.is_gone,
    }))
}

/// How far a branch is from its upstream.
#[derive(Debug, Default, PartialEq, Eq)]
struct Distance {
    ahead: u32,
    behind: u32,
    is_gone: bool,
}

/// The text of `%(upstream:track)`: empty when the two are at the same commit, `[gone]`, or `[ahead 2, behind 1]`
/// with either part alone.
fn parse_track(track: &str) -> Result<Distance, GitError> {
    let inside = track.strip_prefix('[').and_then(|rest| rest.strip_suffix(']'));
    let mut distance = Distance::default();
    match inside {
        None if track.is_empty() => {}
        None => return Err(unexpected("the distance to an upstream", track)),
        Some("gone") => distance.is_gone = true,
        Some(parts) => {
            for part in parts.split(", ") {
                let count = |text: &str| {
                    text.parse()
                        .map_err(|_| unexpected("the distance to an upstream", track))
                };
                match part.split_once(' ') {
                    Some(("ahead", number)) => distance.ahead = count(number)?,
                    Some(("behind", number)) => distance.behind = count(number)?,
                    _ => return Err(unexpected("the distance to an upstream", track)),
                }
            }
        }
    }
    Ok(distance)
}

#[cfg(test)]
mod tests {
    use super::{Distance, parse_track, with_remotes};
    use model::state::RemoteBranch;
    use model::{CommitId, Timestamp};

    #[test]
    fn reads_the_distance_to_an_upstream() {
        let cases = [
            ("", 0, 0, false),
            ("[ahead 2]", 2, 0, false),
            ("[behind 1]", 0, 1, false),
            ("[ahead 2, behind 1]", 2, 1, false),
            ("[gone]", 0, 0, true),
        ];
        for (track, ahead, behind, is_gone) in cases {
            let expected = Distance { ahead, behind, is_gone };
            assert_eq!(parse_track(track).unwrap(), expected, "{track}");
        }
        for track in ["ahead 2", "[ahead two]", "[sideways 1]"] {
            assert!(parse_track(track).is_err(), "{track}");
        }
    }

    #[test]
    fn gives_each_remote_branch_the_longest_remote_it_starts_with() {
        let branch = |name: &str| RemoteBranch {
            name: name.to_owned(),
            remote: name.split('/').next().unwrap().to_owned(),
            commit: CommitId::parse(&"a".repeat(40)).unwrap(),
            date: Timestamp(0),
        };
        let remotes = [String::from("my"), String::from("my/fork"), String::from("origin")];
        let read = with_remotes(
            vec![
                branch("my/fork/HEAD"),
                branch("my/fork/main"),
                branch("my/main"),
                branch("other/x"),
            ],
            &remotes,
        );
        let names: Vec<(&str, &str)> = read
            .iter()
            .map(|branch| (branch.name.as_str(), branch.remote.as_str()))
            .collect();
        assert_eq!(
            names,
            [("my/fork/main", "my/fork"), ("my/main", "my"), ("other/x", "other")]
        );
    }
}
