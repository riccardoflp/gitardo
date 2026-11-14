//! What the review of the conflicts reads to name the two sides (`conflicts.side-names`): the operation that stopped,
//! the branch checked out, the commits of `HEAD`, of what is brought in and of where a rebase goes, the branches at
//! each and their subjects.

use std::ffi::OsString;
use std::fs;
use std::path::Path;

use model::CommitId;
use model::conflicts::{NamedCommit, SideNamesInput};
use model::state::Operation;

use crate::read::fields::{arguments, optional_commit_id, read_or_nothing};
use crate::read::repo::{read_location, read_operation};
use crate::{Git, GitError};

/// What is being read, for the errors.
const WHAT: &str = "the sides of the conflicts";

/// Which repository: the one git runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SideNamesRequest;

/// A commit as the names of the sides need it: its hash, the branches at it and its subject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideCommit {
    /// Its hash.
    pub id: CommitId,
    /// The branches and remote branches at it, by full name.
    pub references: Vec<String>,
    /// The subject of its message.
    pub subject: Option<String>,
}

impl SideCommit {
    /// As [`side_names`](model::conflicts::side_names) takes it.
    #[must_use]
    pub fn named(&self) -> NamedCommit<'_> {
        NamedCommit {
            id: &self.id,
            references: &self.references,
            subject: self.subject.as_deref(),
        }
    }
}

/// What was read to name the sides.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SideNamesRead {
    /// The operation in progress.
    pub operation: Option<Operation>,
    /// The branch checked out, by its short name.
    pub checked_out: Option<String>,
    /// What `HEAD` is.
    pub head: Option<SideCommit>,
    /// What the operation brings in.
    pub brought_in: Option<SideCommit>,
    /// In a rebase, what it goes onto.
    pub onto: Option<SideCommit>,
    /// In a rebase, the branch being rebased, as `head-name` says it.
    pub rebased_branch: Option<String>,
    /// In `git am`, the first line of the message of the patch.
    pub patch_subject: Option<String>,
}

impl SideNamesRead {
    /// As [`side_names`](model::conflicts::side_names) takes it.
    #[must_use]
    pub fn input(&self) -> SideNamesInput<'_> {
        SideNamesInput {
            operation: self.operation,
            checked_out: self.checked_out.as_deref(),
            head: self.head.as_ref().map(SideCommit::named),
            brought_in: self.brought_in.as_ref().map(SideCommit::named),
            onto: self.onto.as_ref().map(SideCommit::named),
            rebased_branch: self.rebased_branch.as_deref(),
            patch_subject: self.patch_subject.as_deref(),
        }
    }
}

/// `git symbolic-ref --short -q HEAD`: the branch checked out.
#[must_use]
pub fn checked_out_arguments() -> Vec<OsString> {
    arguments(&["symbolic-ref", "--short", "-q", "HEAD"])
}

/// `git rev-parse --verify -q <name>^{commit}`: the commit of a name.
#[must_use]
pub fn commit_arguments(name: &str) -> Vec<OsString> {
    arguments(&["rev-parse", "--verify", "-q", &format!("{name}^{{commit}}")])
}

/// `git for-each-ref --points-at=<hash> --format=%(refname) refs/heads refs/remotes`: the branches at a commit.
#[must_use]
pub fn references_arguments(commit: &CommitId) -> Vec<OsString> {
    arguments(&[
        "for-each-ref",
        &format!("--points-at={commit}"),
        "--format=%(refname)",
        "refs/heads",
        "refs/remotes",
    ])
}

/// `git log -1 --format=%s <hash> --`: the subject of a commit.
#[must_use]
pub fn subject_arguments(commit: &CommitId) -> Vec<OsString> {
    arguments(&["log", "-1", "--format=%s", commit.as_str(), "--"])
}

/// The lines git printed, without the empty ones.
#[must_use]
pub fn parse_lines(output: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(output)
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// What names the two sides of the conflicts: the operation in progress, the branch checked out, `HEAD`, the commit
/// the operation brings in, and in a rebase where it goes and which branch it rebases, in `git am` the subject of the
/// patch.
///
/// # Errors
///
/// When git cannot be started or fails.
pub fn read_side_names(git: &Git, _request: &SideNamesRequest) -> Result<SideNamesRead, GitError> {
    let location = read_location(git)?;
    let git = git.in_directory(&location.folder);
    let operation = read_operation(&location.git_folder);
    let first_line = |output: Option<Vec<u8>>| output.and_then(|output| parse_lines(&output).into_iter().next());
    let checked_out = first_line(read_or_nothing(&git, &checked_out_arguments())?);
    let brought_in_name = match operation {
        Some(Operation::Merge) => Some("MERGE_HEAD"),
        Some(Operation::CherryPick) => Some("CHERRY_PICK_HEAD"),
        Some(Operation::Revert) => Some("REVERT_HEAD"),
        Some(Operation::Rebase) => Some("REBASE_HEAD"),
        Some(Operation::Patch) | None => None,
    };
    let rebase = rebase_folder(&location.git_folder);
    let file = |name: &str| {
        rebase
            .as_ref()
            .and_then(|folder| fs::read(folder.join(name)).ok())
            .and_then(|text| parse_lines(&text).into_iter().next())
    };
    let onto = match (operation, file("onto")) {
        (Some(Operation::Rebase), Some(onto)) => side_commit(&git, &onto)?,
        _ => None,
    };
    Ok(SideNamesRead {
        operation,
        checked_out,
        head: side_commit(&git, "HEAD")?,
        brought_in: brought_in_name
            .map(|name| side_commit(&git, name))
            .transpose()?
            .flatten(),
        onto,
        rebased_branch: (operation == Some(Operation::Rebase))
            .then(|| file("head-name"))
            .flatten(),
        patch_subject: (operation == Some(Operation::Patch))
            .then(|| file("final-commit"))
            .flatten(),
    })
}

/// The folder of a rebase or of `git am` in progress: `rebase-merge`, else `rebase-apply`.
fn rebase_folder(git_folder: &Path) -> Option<std::path::PathBuf> {
    ["rebase-merge", "rebase-apply"]
        .into_iter()
        .map(|name| git_folder.join(name))
        .find(|folder| folder.is_dir())
}

/// A commit by a name, with the branches at it and its subject; none when the name stands for no commit.
fn side_commit(git: &Git, name: &str) -> Result<Option<SideCommit>, GitError> {
    let Some(output) = read_or_nothing(git, &commit_arguments(name))? else {
        return Ok(None);
    };
    let Some(id) = optional_commit_id(WHAT, String::from_utf8_lossy(&output).trim())? else {
        return Ok(None);
    };
    let references = parse_lines(&git.run_read(&references_arguments(&id))?);
    let subject = parse_lines(&git.run_read(&subject_arguments(&id))?).into_iter().next();
    Ok(Some(SideCommit {
        id,
        references,
        subject,
    }))
}

#[cfg(test)]
mod tests {
    use super::{checked_out_arguments, commit_arguments, parse_lines};

    #[test]
    fn asks_for_the_branch_and_the_commit_of_a_name() {
        assert_eq!(checked_out_arguments(), ["symbolic-ref", "--short", "-q", "HEAD"]);
        assert_eq!(
            commit_arguments("MERGE_HEAD"),
            ["rev-parse", "--verify", "-q", "MERGE_HEAD^{commit}"]
        );
    }

    #[test]
    fn reads_the_lines_printed_without_the_empty_ones() {
        assert_eq!(
            parse_lines(b"refs/heads/main\r\n\nrefs/remotes/origin/main\n"),
            ["refs/heads/main", "refs/remotes/origin/main"]
        );
    }
}
