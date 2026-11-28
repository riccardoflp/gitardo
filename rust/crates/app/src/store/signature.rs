//! The signature of the commit of the inspector and of the page of a commit, as the store keeps it
//! (`details.signature`): read on a worker once the details are shown, so that they never wait for it, and cancelled
//! when another commit is selected first.

use std::path::PathBuf;

use git::read::details::SignatureRequest;
use model::CommitId;
use model::details::CommitSignature;
use views::details::SignatureRead;

use super::message::{Effect, JobId, JobKind};
use super::state::State;

/// How far the signature of a commit shown is read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SignatureCheck {
    /// It is read by a job.
    Checking(JobId),
    /// What git said of it.
    Read(CommitSignature),
    /// Git could not read it: no line is drawn, and what git said is the last error.
    Failed,
}

impl SignatureCheck {
    /// The job that reads it, while one does.
    #[must_use]
    pub fn job(&self) -> Option<JobId> {
        match self {
            SignatureCheck::Checking(job) => Some(*job),
            SignatureCheck::Read(_) | SignatureCheck::Failed => None,
        }
    }
}

/// Starts the read of the signature of a commit of a repository.
pub(super) fn check(state: &mut State, folder: PathBuf, commit: CommitId) -> (SignatureCheck, Effect) {
    let kind = JobKind::ReadSignature {
        folder,
        request: SignatureRequest { commit },
    };
    let job = super::workspace::start_job(state, kind);
    (SignatureCheck::Checking(job.id), Effect::Start(job))
}

/// What the views draw of a signature: nothing until it is read, so that the lines under it do not move twice.
#[must_use]
pub fn shown(check: Option<&SignatureCheck>) -> Option<SignatureRead<'_>> {
    match check? {
        SignatureCheck::Read(signature) => Some(SignatureRead(signature)),
        SignatureCheck::Checking(_) | SignatureCheck::Failed => None,
    }
}

/// Takes the signature read for the inspector or for the page that asked for it; a read no one waits for any more is
/// dropped.
pub(super) fn signature_read(state: &mut State, job: JobId, result: Result<CommitSignature, String>) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    let waiting = |check: &Option<SignatureCheck>| check.as_ref().and_then(SignatureCheck::job) == Some(job);
    let (checked, error) = match result {
        Ok(signature) => (SignatureCheck::Read(signature), None),
        Err(error) => (SignatureCheck::Failed, Some(error)),
    };
    if waiting(&state.inspector.signature) {
        state.inspector.signature = Some(checked);
    } else if let Some(page) = state
        .commit_pages
        .pages
        .iter_mut()
        .find(|page| waiting(&page.signature))
    {
        page.signature = Some(checked);
        page.version += 1;
    } else {
        return Vec::new();
    }
    if let Some(error) = error {
        state.last_error = Some(error);
    }
    Vec::new()
}
