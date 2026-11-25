//! A worktree shown for the first time in the tab of its repository takes the rows the view left has read
//! (`repo.share-history`): they are shown at once, and its own read, behind them, reads the commits only when its
//! history is another.

use std::path::Path;
use std::sync::Arc;

use engine::timeline::TimelineRequest;
use git::read::graph::CommitPage;
use git::read::worktree::CountedWorktrees;
use model::filter::checked;
use model::refs::References;
use model::repo::{HistoryRead, ReadCause, SharedHistory, shared_history};
use model::state::RepositoryState;
use views::graph::TimelineSwitch;

use super::super::message::{Effect, JobKind};
use super::super::state::{RepositoryRead, Shown, State};
use super::super::{pages, rail, reads, timeline, workspace};
use crate::persisted::same_folder;

/// What the view shown gives a worktree of its repository chosen in its place.
pub(super) struct Offer {
    /// The rows and the references it takes.
    history: SharedHistory,
    /// The state of the worktree left, as read: the read of the worktree chosen tells from it whether its history is
    /// the same.
    left: Arc<RepositoryState>,
    /// How the timeline of the worktree chosen is read.
    request: TimelineRequest,
    /// When the rows were read, in seconds since 1970.
    read_at: i64,
}

/// What the view shown gives the folder chosen in its place, asked before the view is left: its rows and its
/// references, when the folder is a worktree of its repository, its commits are read and not being read again for
/// another history, and the worktree chosen asks the same history, by the branches kept for it and with no search.
pub(super) fn offered(state: &State, folder: &Path) -> Option<Offer> {
    let read = state.shown_read()?;
    let shown = &state.timeline;
    let is_read = shown
        .folder
        .as_deref()
        .is_some_and(|rows_of| same_folder(rows_of, &read.folder))
        && !shown.is_reading_again
        && shown.narrow_job.is_none();
    if !is_read {
        return None;
    }
    let left = timeline::read_request(state);
    let request = TimelineRequest {
        shows_stashes: state.settings.switches.is_on(TimelineSwitch::ShowStashes),
        shown: timeline::kept_shown(state, folder),
        narrowing: Vec::new(),
        alone: None,
        ..left.clone()
    };
    let history = HistoryRead {
        asked: &left,
        shown: left.shown.as_ref(),
        rows: shown.read_rows(),
        has_more: shown.has_more || shown.stream.is_some(),
        state: &read.state,
        chosen: folder,
    };
    Some(Offer {
        history: shared_history(&history, &request)?,
        left: Arc::clone(&read.state),
        request,
        read_at: shown.read_at,
    })
}

/// Shows a worktree with the rows another of its repository gave it, at once, and starts its own read behind them:
/// the first step of its state, then its commits only when what it has checked out is on none of the references, or
/// the references changed meanwhile, then the rest of its state (`repo.read-first`).
pub(super) fn show_shared(state: &mut State, folder: &Path, offer: Offer) -> Vec<Effect> {
    state.dialog = None;
    state.pages.shown = None;
    let repository = Arc::new(offer.history.state);
    let generation = state.next_generation;
    state.next_generation += 1;
    if let Some(workspace) = state.workspace.as_mut() {
        workspace.shown = Shown::Read(RepositoryRead {
            folder: folder.to_path_buf(),
            state: Arc::clone(&repository),
            generation,
        });
    }
    let kind = JobKind::ReadRepository {
        folder: folder.to_path_buf(),
        request: offer.request.clone(),
        kept: Some(offer.left),
        counted: CountedWorktrees::default(),
        in_steps: true,
    };
    let job = workspace::start_job(state, kind);
    reads::adopt(state, job.id, ReadCause::RepositoryChosen);
    let page = CommitPage {
        rows: offer.history.rows,
        has_more: offer.history.has_more,
        follows: false,
    };
    let shown = checked(offer.request.shown.as_ref(), &References::of(&repository));
    let _rows_are_new = timeline::take_read(state, (folder, &repository), (page, shown), (job.id, offer.read_at));
    rail::arrange(state);
    let mut effects = vec![workspace::watch_settings(state), Effect::Start(job)];
    effects.extend(timeline::keep_shown(state, folder));
    effects.extend(pages::tick_timeline(state));
    effects
}
