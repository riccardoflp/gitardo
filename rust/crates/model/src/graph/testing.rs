//! What the tests of the graph share: commits named by a word, and rows written as `X: P Q`.

use std::fmt::Write as _;

use crate::commit::Commit;
use crate::graph::row::Row;
use crate::state::{Branch, RemoteBranch, Upstream};
use crate::{CommitId, Timestamp};

/// The hash a test gives a commit of a name: the hexadecimal digits of the name, then zeros.
pub(crate) fn id(name: &str) -> CommitId {
    let mut hex = String::new();
    for byte in name.bytes() {
        write!(hex, "{byte:02x}").unwrap();
    }
    CommitId::parse(&format!("{hex:0<40}")).unwrap()
}

/// A commit of a name, with its parents, made at a date.
pub(crate) fn commit_at(name: &str, parents: &[&str], date: i64) -> Commit {
    Commit {
        id: id(name),
        parents: parents.iter().map(|parent| id(parent)).collect(),
        author_name: String::from("Test Author"),
        author_email: String::from("author@example.com"),
        author_date: Timestamp(date),
        subject: format!("Commit {name}"),
    }
}

/// Rows of commits, each written `X: P Q` for X with parents P and Q, dated one minute apart, the newest first.
pub(crate) fn rows(lines: &[&str]) -> Vec<Row> {
    let count = i64::try_from(lines.len()).unwrap();
    lines
        .iter()
        .zip(0..)
        .map(|(line, index)| {
            let (name, parents) = line.split_once(':').unwrap();
            let parents: Vec<&str> = parents.split_whitespace().collect();
            Row::Commit(commit_at(name.trim(), &parents, (count - index) * 60))
        })
        .collect()
}

/// A local branch at a commit, not checked out, with an upstream on a remote (`("origin", "main")`) at no distance.
pub(crate) fn branch(name: &str, commit: &str, upstream: Option<(&str, &str)>) -> Branch {
    Branch {
        name: name.to_owned(),
        commit: id(commit),
        upstream: upstream.map(|(remote, remote_name)| Upstream {
            reference: format!("refs/remotes/{remote}/{remote_name}"),
            remote: remote.to_owned(),
            remote_branch: format!("refs/heads/{remote_name}"),
            ahead: 0,
            behind: 0,
            is_gone: false,
        }),
        is_checked_out: false,
        date: Timestamp(0),
    }
}

/// A remote branch at a commit: `origin/main`.
pub(crate) fn remote_branch(name: &str, commit: &str) -> RemoteBranch {
    RemoteBranch {
        name: name.to_owned(),
        remote: name.split('/').next().unwrap().to_owned(),
        commit: id(commit),
        date: Timestamp(0),
    }
}
