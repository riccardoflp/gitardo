//! Answering the queries of features by reading git.

use git::read::branch;
use git::read::diff::{DiffRequest, FileVersion, VersionRequest, read_sides};
use git::read::identity::{IdentityRequest, read_identity};
use git::read::merge::{PredictedConflictsRequest, read_predicted_conflicts};
use git::read::rebase::{self, LineRequest, PushedRequest};
use git::read::reflog::{ReflogRequest, read_entries};
use git::read::remote::{RemotesRequest, read_remotes};
use git::read::rewrite;
use git::read::search::{RevisionRequest, read_revision};
use git::read::stash::{StashRef, read_files};
use git::read::sync::{OverwrittenRequest, read_overwritten};
use git::{Git, GitError};
use model::contract::{Answer, IdentityKeys, Query};
use model::remote::{Overwritten, OverwrittenCommit, Remote};
use model::stash::StashReading;

use crate::readings;

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
        Query::ReflogPositions { reference, count } => {
            let request = ReflogRequest {
                reference: reference.clone(),
                count: *count,
            };
            git::read::reflog::read(git, &request).map(Answer::Positions)
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
        Query::Remotes => remotes(git),
        Query::Stash { index } => stash(git, *index),
        Query::Identity => identity(git),
        Query::FileVersions { path, commit } => file_versions(git, path, commit.as_ref()),
        Query::PredictedConflicts { ours, theirs, base } => {
            predicted_conflicts(git, &conflicts_request(ours.as_ref(), theirs, base.as_ref()))
        }
        Query::Commits { revisions } => rewrite::read_commits(git, revisions).map(Answer::Commits),
        Query::LineAfter { commit } => rewrite::read_line_after(git, commit).map(Answer::Commits),
        Query::Message { commit } => rewrite::read_message(git, commit).map(Answer::Message),
        Query::EmptyStop => rewrite::read_empty_stop(git).map(Answer::EmptyStop),
        Query::Line { commit } => rebase::read_line(git, &LineRequest { commit: commit.clone() }).map(Answer::Line),
        Query::Messages { commits } => rebase::read_messages(git, commits).map(Answer::Messages),
        Query::Pushed { commit } => {
            rebase::read_pushed(git, &PushedRequest { commit: commit.clone() }).map(Answer::Names)
        }
        Query::CommitFiles { commit } => rebase::read_commit_files(git, commit).map(Answer::Names),
        Query::FormatPatch { commits } => readings::format_patch(git, commits),
        Query::PatchCheck { patch } => readings::patch_check(git, patch),
        Query::FileContents { path } => readings::file_contents(path),
        // The engine asks the window for it before asking git anything.
        Query::Clipboard => Ok(Answer::Clipboard(None)),
        Query::TemporaryFile { extension } => Ok(readings::temporary_file(extension)),
        Query::ComparePatch(target) => readings::compare_patch(git, target),
        Query::ConflictStages { path } => git::read::conflicts::read_stages(git, path).map(Answer::Stages),
        Query::ConflictTexts => git::read::conflicts::read_texts(git, &git::read::conflicts::ConflictFilesRequest)
            .map(Answer::ConflictTexts),
        Query::FileTexts { paths } => readings::file_texts(paths),
    }
}

/// The remotes of the repository, with their URLs.
fn remotes(git: &Git) -> Result<Answer, GitError> {
    Ok(Answer::Remotes(
        read_remotes(git, &RemotesRequest)?
            .into_iter()
            .map(|remote| Remote {
                name: remote.name,
                url: remote.url,
            })
            .collect(),
    ))
}

/// Who commits are made as, and which `user.*` keys the repository sets for itself.
fn identity(git: &Git) -> Result<Answer, GitError> {
    read_identity(git, &IdentityRequest).map(|read| {
        Answer::Identity(IdentityKeys {
            identity: read.identity,
            local_keys: read.local_keys,
        })
    })
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

/// The files a merge would leave in conflict, as the paths the features say.
fn predicted_conflicts(git: &Git, request: &PredictedConflictsRequest) -> Result<Answer, GitError> {
    let paths = read_predicted_conflicts(git, request)?;
    Ok(Answer::Conflicts(paths.map(|paths| {
        paths
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect()
    })))
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
        Query::ReflogPositions { reference, count } => describe_positions(reference, *count),
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
        Query::FileVersions { path, commit } => match commit {
            Some(commit) => format!("git show {commit}:{path}, git show :0:{path}, read {path}"),
            None => format!("git show :0:{path}, read {path}"),
        },
        Query::PredictedConflicts { ours, theirs, base } => git::command_line(
            &git::read::merge::predicted_conflicts_arguments(&conflicts_request(ours.as_ref(), theirs, base.as_ref())),
        ),
        Query::Commits { revisions } => git::command_line(&rewrite::commits_arguments(revisions)),
        Query::LineAfter { commit } => git::command_line(&rewrite::line_after_arguments(commit)),
        Query::Message { commit } => git::command_line(&rewrite::message_arguments(commit)),
        Query::EmptyStop => String::from(
            "git rev-parse -q --verify CHERRY_PICK_HEAD (or REVERT_HEAD), git ls-files -u -z, git diff --quiet HEAD --",
        ),
        Query::Line { commit } => describe_line(commit),
        Query::Messages { commits } => describe_messages(commits),
        Query::Pushed { commit } => {
            git::command_line(&rebase::pushed_arguments(&PushedRequest { commit: commit.clone() }))
        }
        Query::CommitFiles { commit } => git::command_line(&rebase::commit_files_arguments(commit)),
        Query::FormatPatch { commits } => describe_format_patch(commits),
        Query::PatchCheck { patch } => describe_patch_check(patch),
        Query::FileContents { path } => format!("read {}", path.display()),
        Query::Clipboard => String::from("the text of the clipboard"),
        Query::TemporaryFile { extension } => format!("a temporary .{extension} file"),
        Query::ComparePatch(target) => readings::describe_compare_patch(target),
        Query::ConflictStages { path } => {
            git::command_line(&git::read::conflicts::stages_arguments(std::slice::from_ref(path)))
        }
        Query::ConflictTexts => String::from(
            "git diff --name-only --diff-filter=U -z, git ls-files --unmerged -z, git cat-file blob <object>, read <file>",
        ),
        Query::FileTexts { paths } => paths
            .iter()
            .map(|path| format!("read {}", path.display()))
            .collect::<Vec<String>>()
            .join(", "),
    }
}

/// What a prediction of the conflicts of a merge reads.
fn conflicts_request(ours: Option<&String>, theirs: &str, base: Option<&String>) -> PredictedConflictsRequest {
    PredictedConflictsRequest {
        ours: ours.cloned(),
        theirs: theirs.to_owned(),
        base: base.cloned(),
    }
}

/// The command that reads the places of the entries of a reflog.
fn describe_positions(reference: &str, count: usize) -> String {
    let request = ReflogRequest {
        reference: reference.to_owned(),
        count,
    };
    git::command_line(&git::read::reflog::positions_arguments(&request))
}

/// The commands that read the line of a commit up to `HEAD`.
fn describe_line(commit: &model::CommitId) -> String {
    format!(
        "{}, {}, git log -z --topo-order --format=%H%x00%P%x00%B HEAD --not {commit}^@ --",
        git::command_line(&rebase::parents_arguments(commit)),
        git::command_line(&branch::ancestor_arguments(commit.as_str(), "HEAD"))
    )
}

/// The commands that read the messages of commits, one each.
fn describe_messages(commits: &[model::CommitId]) -> String {
    commits
        .iter()
        .map(|commit| git::command_line(&rebase::message_arguments(commit)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The command that makes commits one patch.
fn describe_format_patch(commits: &[model::CommitId]) -> String {
    let hashes: Vec<&str> = commits.iter().map(model::CommitId::as_str).collect();
    format!("git format-patch --stdout {}", hashes.join(" "))
}

/// The command that checks a patch would apply.
fn describe_patch_check(patch: &model::patch::PatchInput) -> String {
    match patch {
        model::patch::PatchInput::File(file) => format!("git apply --check --3way -- {}", file.display()),
        model::patch::PatchInput::Text(_) => String::from("git apply --check --3way"),
    }
}
