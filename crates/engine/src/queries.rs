//! Answering the queries of features by reading git.

use git::read::branch;
use git::read::diff::{DiffRequest, FileVersion, VersionRequest, read_sides};
use git::read::identity::{IdentityRequest, read_identity};
use git::read::merge::{PredictedConflictsRequest, read_predicted_conflicts};
use git::read::reflog::{ReflogRequest, read_entries};
use git::read::remote::{RemotesRequest, read_remotes};
use git::read::search::{RevisionRequest, read_revision};
use git::read::stash::{StashRef, read_files};
use git::read::sync::{OverwrittenRequest, read_overwritten};
use git::{Git, GitError};
use model::contract::{Answer, IdentityKeys, Query};
use model::remote::{Overwritten, OverwrittenCommit, Remote};
use model::stash::StashReading;

/// The answer of a query, read now.
pub(crate) fn answer(git: &Git, query: &Query) -> Result<Answer, GitError> {
    match query {
        Query::Reflog { reference, count } => {
            let request = ReflogRequest {
                reference: reference.clone(),
                count: *count,
            };
            read_entries(git, &request).map(Answer::Reflog)
        }
        Query::Revision { revision } => {
            let request = RevisionRequest {
                revision: revision.clone(),
            };
            read_revision(git, &request).map(Answer::Commit)
        }
        // The engine answers it from its journal before asking git anything.
        Query::Journal => Ok(Answer::Journal(None)),
        Query::Counts { left, right } => {
            branch::read_counts(git, left, right).map(|(left, right)| Answer::Counts { left, right })
        }
        Query::IsAncestor { ancestor, descendant } => {
            branch::read_is_ancestor(git, ancestor, descendant).map(Answer::Flag)
        }
        Query::MergedBranches { into } => branch::read_merged(git, into).map(Answer::Names),
        Query::HasFile { revision, path } => branch::read_has_file(git, revision, path).map(Answer::Flag),
        Query::FilesMatch { commit } => files_match(git, commit),
        Query::Overwritten { upstream } => overwritten(git, upstream),
        Query::Remotes => Ok(Answer::Remotes(
            read_remotes(git, &RemotesRequest)?
                .into_iter()
                .map(|remote| Remote {
                    name: remote.name,
                    url: remote.url,
                })
                .collect(),
        )),
        Query::Stash { index } => stash(git, *index),
        Query::Identity => read_identity(git, &IdentityRequest).map(|read| {
            Answer::Identity(IdentityKeys {
                identity: read.identity,
                local_keys: read.local_keys,
            })
        }),
        Query::Message { commit } => message(git, commit),
        Query::FileVersions { path, commit } => file_versions(git, path, commit.as_ref()),
    }
}

/// The whole message of a commit as it was written, without the newlines at its end.
fn message(git: &Git, commit: &model::CommitId) -> Result<Answer, GitError> {
    let arguments = message_arguments(commit);
    let output = git.read_output(&arguments)?;
    if output.code != Some(0) {
        return Err(GitError::Failed {
            command: git::command_line(&arguments),
            code: output.code,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(Answer::Text(text.trim_end_matches(['\n', '\r']).to_owned()))
}

/// `git log -1 --format=%B <commit> --`.
fn message_arguments(commit: &model::CommitId) -> [&str; 5] {
    ["log", "-1", "--format=%B", commit.as_str(), "--"]
}

/// A file in a commit, in the index and on disk, each none where it is not.
fn file_versions(git: &Git, path: &str, commit: Option<&model::CommitId>) -> Result<Answer, GitError> {
    let version = |version: FileVersion| VersionRequest {
        version,
        path: path.to_owned(),
    };
    let committed = DiffRequest {
        left: version(commit.cloned().map_or(FileVersion::Absent, FileVersion::Commit)),
        right: version(FileVersion::Index),
    };
    let on_disk = DiffRequest {
        left: version(FileVersion::Absent),
        right: version(FileVersion::WorkingTree { unsaved: None }),
    };
    let first = read_sides(git, &committed)?;
    let second = read_sides(git, &on_disk)?;
    Ok(Answer::FileVersions(model::diff::FileVersions {
        commit: first.left,
        index: first.right,
        working_tree: second.right,
    }))
}

/// Whether the files of the working tree are as a commit has them.
fn files_match(git: &Git, commit: &model::CommitId) -> Result<Answer, GitError> {
    let arguments = ["diff", "--quiet", commit.as_str(), "--"];
    let output = git.read_output(&arguments)?;
    match output.code {
        Some(0) => Ok(Answer::Flag(true)),
        Some(1) => Ok(Answer::Flag(false)),
        _ => Err(GitError::Failed {
            command: git::command_line(&arguments),
            code: output.code,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }),
    }
}

/// The commits of an upstream that `HEAD` does not have.
fn overwritten(git: &Git, upstream: &str) -> Result<Answer, GitError> {
    let request = OverwrittenRequest {
        upstream: upstream.to_owned(),
    };
    let read = read_overwritten(git, &request)?;
    Ok(Answer::Overwritten(Overwritten {
        commits: read
            .commits
            .into_iter()
            .map(|commit| OverwrittenCommit {
                short_id: commit.short_id,
                subject: commit.subject,
            })
            .collect(),
        has_more: read.has_more,
    }))
}

/// The files of a stash, those of its untracked files that exist in the working tree, and the files a merge of it onto
/// `HEAD` from the commit it was made on would leave in conflict.
fn stash(git: &Git, index: usize) -> Result<Answer, GitError> {
    let name = format!("stash@{{{index}}}");
    let files = read_files(git, &StashRef { name: name.clone() })?;
    let untracked_there = files
        .untracked
        .iter()
        .filter(|file| git.directory().join(file).exists())
        .cloned()
        .collect();
    let request = PredictedConflictsRequest {
        ours: None,
        theirs: name.clone(),
        base: Some(format!("{name}^1")),
    };
    let conflicts = read_predicted_conflicts(git, &request)?.map(|paths| {
        paths
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect()
    });
    Ok(Answer::Stash(StashReading {
        files,
        untracked_there,
        conflicts,
    }))
}

/// The command a query runs, as the log writes it at the debug level.
pub(crate) fn describe(query: &Query) -> String {
    match query {
        Query::Reflog { reference, count } => format!("git log --walk-reflogs --max-count={count} {reference}"),
        Query::FilesMatch { commit } => format!("git diff --quiet {commit} --"),
        Query::Revision { revision } => format!("git rev-parse --verify --quiet {revision}^{{commit}}"),
        Query::Journal => String::from("the journal of Undo"),
        Query::Counts { left, right } => git::command_line(&branch::counts_arguments(left, right)),
        Query::IsAncestor { ancestor, descendant } => {
            git::command_line(&branch::ancestor_arguments(ancestor, descendant))
        }
        Query::MergedBranches { into } => git::command_line(&branch::merged_arguments(into)),
        Query::HasFile { revision, path } => git::command_line(&branch::has_file_arguments(revision, path)),
        Query::Overwritten { upstream } => {
            format!("git log --max-count=11 --format=%h%x00%s%x00 HEAD..{upstream} --")
        }
        Query::Remotes => String::from("git remote, git remote get-url"),
        Query::Stash { index } => format!(
            "git diff --name-only -z stash@{{{index}}}^1 stash@{{{index}}}, git merge-tree --write-tree \
             --merge-base=stash@{{{index}}}^1 HEAD stash@{{{index}}}"
        ),
        Query::Identity => String::from(r"git config -z --get-regexp ^user\.(name|email)$"),
        Query::Message { commit } => git::command_line(&message_arguments(commit)),
        Query::FileVersions { path, commit } => match commit {
            Some(commit) => format!("git show {commit}:{path}, git show :0:{path}, read {path}"),
            None => format!("git show :0:{path}, read {path}"),
        },
    }
}
