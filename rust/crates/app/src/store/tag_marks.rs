//! The marks of the tags a remote lacks, as the store keeps them for each repository while the window is open
//! (`tag.remote-marks`): asked of the remote on a worker when **Check tags on `<remote>`** is chosen, never with the
//! reads of the repository, and changed by Gitardo's own operations on tags without asking it again.

use std::path::Path;

use git::read::tag::RemoteTagsRequest;
use model::tag::{RemoteTags, TagMarks, TagsOnRemote, compare_remote};

use super::message::{Effect, JobId, JobKind};
use super::state::State;

/// **Check tags on `<remote>`**: asks the remote for its tags, on a worker, for the repository shown.
pub(super) fn check(state: &mut State, remote: String) -> Vec<Effect> {
    let Some(folder) = state.shown_folder().cloned() else {
        return Vec::new();
    };
    let kind = JobKind::ReadRemoteTags {
        folder,
        request: RemoteTagsRequest { remote },
    };
    vec![Effect::Start(super::workspace::start_job(state, kind))]
}

/// **Check tags on the remote** from Quick search: the remote of `remote.default`, or what says there is none.
pub(super) fn check_default(state: &mut State) -> Vec<Effect> {
    let remote = state
        .shown_read()
        .and_then(|read| model::remote::default(&read.state.remotes));
    if let Some(remote) = remote {
        return check(state, remote);
    }
    state.last_error = Some(String::from("This repository has no remote."));
    Vec::new()
}

/// Takes the tags a remote said it has: the marks of the repository become those of this check, the tags compared at
/// the commits the repository has them at. A remote that could not be asked leaves the marks as they were and says
/// what git said.
pub(super) fn remote_tags_read(state: &mut State, job: JobId, result: Result<RemoteTags, String>) -> Vec<Effect> {
    let Some(JobKind::ReadRemoteTags { folder, .. }) = state.jobs.remove(&job) else {
        return Vec::new(); // A check no one waits for any more.
    };
    let folder = folder.as_path();
    match result {
        Ok(read) => {
            let comparison = compare_remote(&read.here, &read);
            let tags = state
                .shown_read()
                .filter(|shown| shown.folder == folder)
                .map_or(&[][..], |shown| shown.state.tags.as_slice());
            let marks = TagMarks::checked(&comparison, tags);
            let _before = state.rail.tag_marks.insert(folder.to_path_buf(), marks);
        }
        Err(error) => state.last_error = Some(error),
    }
    Vec::new()
}

/// What an operation of Gitardo changed of the tags on a remote: the marks of the repository follow it, when it was
/// checked.
pub(super) fn follow(state: &mut State, folder: &Path, changed: &TagsOnRemote) {
    let tags = state
        .shown_read()
        .filter(|shown| shown.folder == folder)
        .map(|shown| shown.state.tags.clone())
        .unwrap_or_default();
    if let Some(marks) = state.rail.tag_marks.get_mut(folder) {
        marks.follow(changed, &tags);
    }
}
