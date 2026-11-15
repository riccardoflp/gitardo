//! The reads of files: the working tree, which paths git ignores, the sides of a Diff page, a file as a commit has it,
//! the blame, the names of a file and where some of its lines are, the message git prepared and the common git folder.

use std::path::PathBuf;

use git::read::blame::{BlameFileRequest, BlameLineRequest, Whitespace, read_file, read_line};
use git::read::changes::{WorkingTreeRequest, read_working_tree};
use git::read::commit::{PreparedMessageRequest, read_prepared_message};
use git::read::diff::{DiffRequest, VersionRequest, read_sides};
use git::read::file_history::{CommittedLinesRequest, NamesRequest, read_committed_lines, read_names};
use git::read::file_view::{RevisionFileRequest, read_revision};
use git::read::repo::{CommonFolderRequest, read_common_folder};
use git::read::search::{self, RevisionRequest};
use protocol::reads::{
    BlameFileParams, BlameLineJson, BlameLineParams, BytesJson, CommittedLinesJson, CommittedLinesParams,
    CommonFolderResult, DiffSidesJson, DiffSidesParams, FileRevisionParams, IgnoredParams, IgnoredResult, NamesParams,
    NamesResult, PreparedMessageResult,
};
use protocol::rpc::ResponseError;
use protocol::state::WorkingTreeJson;
use serde_json::Value;

use super::{Reading, answered, invalid, path_text};
use crate::session::git_failure;

/// What is not committed (`changes.read-working-tree`).
pub fn working_tree(reading: &Reading<'_>) -> Result<Value, ResponseError> {
    answered(read_working_tree(&reading.git, &WorkingTreeRequest), |read| {
        WorkingTreeJson::from(read)
    })
}

/// Those of some paths that git ignores (`changes.unsaved` asks it for the files with edits not saved).
pub fn ignored(reading: &Reading<'_>, params: &IgnoredParams) -> Result<Value, ResponseError> {
    answered(
        git::read::ignored::read_ignored(&reading.git, &params.paths),
        |ignored| IgnoredResult {
            ignored: ignored.clone(),
        },
    )
}

/// The two sides of a file of a Diff page (`diff.read-sides`).
pub fn diff_sides(reading: &Reading<'_>, params: &DiffSidesParams) -> Result<Value, ResponseError> {
    let request = DiffRequest {
        left: VersionRequest::try_from(&params.left).map_err(invalid)?,
        right: VersionRequest::try_from(&params.right).map_err(invalid)?,
    };
    answered(read_sides(&reading.git, &request), |read| DiffSidesJson::from(read))
}

/// A file as a commit has it (`file-view.read-revision`); none when the name is no commit, or the commit has no such
/// file.
pub fn file_revision(reading: &Reading<'_>, params: FileRevisionParams) -> Result<Value, ResponseError> {
    let commit = search::read_revision(
        &reading.git,
        &RevisionRequest {
            revision: params.revision,
        },
    )
    .map_err(|error| git_failure(&error))?;
    let Some(commit) = commit else {
        return Ok(Value::Null);
    };
    let request = RevisionFileRequest {
        commit,
        path: params.path,
    };
    answered(read_revision(&reading.git, &request), |bytes| {
        bytes.as_deref().map(BytesJson::of)
    })
}

/// Whether the blame leaves whitespace and moved lines to who wrote them, as `blame.ignoreWhitespaceAndMoves` says.
fn whitespace(reading: &Reading<'_>) -> Whitespace {
    if reading.session.feature_settings().blame.ignores_whitespace {
        Whitespace::IgnoredWithMoves
    } else {
        Whitespace::Counted
    }
}

/// The bytes not saved that a read was given, when it was.
fn contents(given: Option<&BytesJson>) -> Result<Option<Vec<u8>>, ResponseError> {
    given.map(BytesJson::bytes).transpose().map_err(invalid)
}

/// Who last changed one line of a file (`blame.read-line`).
pub fn blame_line(reading: &Reading<'_>, params: &BlameLineParams) -> Result<Value, ResponseError> {
    let request = BlameLineRequest {
        path: PathBuf::from(&params.path),
        line: params.line,
        whitespace: whitespace(reading),
        contents: contents(params.contents.as_ref())?,
    };
    answered(read_line(&reading.git, &request), |line| {
        line.as_ref().map(BlameLineJson::from)
    })
}

/// Who last changed each line of a file (`blame.read-file`).
pub fn blame_file(reading: &Reading<'_>, params: BlameFileParams) -> Result<Value, ResponseError> {
    let request = BlameFileRequest {
        path: PathBuf::from(&params.path),
        whitespace: whitespace(reading),
        contents: contents(params.contents.as_ref())?,
        revision: params.revision,
    };
    answered(read_file(&reading.git, &request), |lines| {
        lines.as_ref().map(|lines| {
            lines
                .iter()
                .map(|line| line.as_ref().map(BlameLineJson::from))
                .collect::<Vec<_>>()
        })
    })
}

/// Every name a file had (`file-history.read-names`).
pub fn names(reading: &Reading<'_>, params: NamesParams) -> Result<Value, ResponseError> {
    answered(read_names(&reading.git, &NamesRequest { path: params.path }), |names| {
        NamesResult { names: names.clone() }
    })
}

/// Where some lines of a file are in the commit checked out (`file-history.read-committed-lines`).
pub fn committed_lines(reading: &Reading<'_>, params: CommittedLinesParams) -> Result<Value, ResponseError> {
    let request = CommittedLinesRequest {
        path: params.path,
        renamed_from: params.renamed_from,
        text: params.text,
        lines: params.lines.into(),
    };
    answered(read_committed_lines(&reading.git, &request), |read| {
        CommittedLinesJson::from(read)
    })
}

/// The message git prepared for the commit of a squash merge (`commit.prepared-message`).
pub fn prepared_message(reading: &Reading<'_>) -> Result<Value, ResponseError> {
    answered(
        read_prepared_message(&reading.git, &PreparedMessageRequest),
        |message| PreparedMessageResult {
            message: message.clone(),
        },
    )
}

/// The git folder the worktrees share (`repo.common-folder`).
pub fn common_folder(reading: &Reading<'_>) -> Result<Value, ResponseError> {
    answered(read_common_folder(&reading.git, &CommonFolderRequest), |folder| {
        CommonFolderResult {
            folder: path_text(folder),
        }
    })
}
