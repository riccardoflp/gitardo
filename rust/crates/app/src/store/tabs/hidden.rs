//! The tabs not shown (`repo.watch-hidden`): each is watched, a change in one marks it out of date and reads nothing,
//! and a tab out of date is read again once shown. With `repositories.keepOpen` on, once the tab shown has its first
//! read, the tabs never read are read one after the other, each in what it keeps; never beside an operation or a read
//! of the tab shown, which stop it.

use std::path::{Path, PathBuf};
use std::time::Duration;

use model::repo::{Change, HiddenTab, ReadAction, ReadCause, ReadEvent, StartedRead, schedule_hidden, watch_changes};

use super::super::message::{Effect, JobId, Message};
use super::super::state::{Shown, State};
use super::super::{reads, workspace};
use super::view::{RepositoryView, put_view, take_view};
use crate::persisted::same_folder;

/// What the window does for the tabs not shown.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Background {
    /// The working trees of the tabs not shown that are watched.
    pub watched: Vec<PathBuf>,
    /// The read of a tab not shown under way.
    pub reading: Option<BackgroundRead>,
    /// The folder of the tab the operation running was started in.
    pub operation: Option<PathBuf>,
    /// What an operation that ended in a tab not shown has to do once that tab is read again, with the tab's folder.
    pub after_operation: Option<(PathBuf, super::operation::AfterOperation)>,
}

/// The read of a tab not shown under way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackgroundRead {
    /// The folder of the tab.
    pub folder: PathBuf,
    /// Its job.
    pub job: JobId,
    /// Its number, why it runs and how many reads in a row it is, for the Activity tab.
    pub started: StartedRead,
}

impl Background {
    /// Why a read of a tab not shown runs, when the job is one.
    #[must_use]
    pub fn reason(&self, job: JobId) -> Option<&StartedRead> {
        self.reading
            .as_ref()
            .filter(|read| read.job == job)
            .map(|read| &read.started)
    }
}

/// Runs a change of the store in a tab not shown, as if it were the tab shown, and puts what it shows back in it: a tab
/// that kept nothing starts from what a repository not read yet shows. What is the window's stays the window's.
pub(super) fn in_tab<R>(state: &mut State, index: usize, change: impl FnOnce(&mut State) -> R) -> R {
    let shown = take_view(state);
    let kept = state
        .tabs
        .open
        .get_mut(index)
        .and_then(|tab| tab.kept.take())
        .map_or_else(RepositoryView::default, |kept| *kept);
    let hidden = kept.hidden.clone();
    put_view(state, kept);
    let result = change(state);
    let mut view = take_view(state);
    view.hidden = hidden;
    if let Some(tab) = state.tabs.open.get_mut(index) {
        tab.kept = Some(Box::new(view));
    }
    put_view(state, shown);
    result
}

/// The working tree a tab not shown watches: the folder its view shows, a worktree shown in place of its repository
/// included, or its own.
fn watched_folder(tab: &super::Tab) -> PathBuf {
    let shown = tab.kept.as_ref().and_then(|kept| match kept.shown.as_ref()? {
        Shown::Read(read) => Some(read.folder.clone()),
        Shown::Reading { folder } => Some(folder.clone()),
        Shown::Looking | Shown::NoRepository => None,
    });
    shown.unwrap_or_else(|| tab.folder.clone())
}

/// Changes seen by the watcher of a tab not shown: they read nothing, and mark it out of date. Changes of the tab shown,
/// seen before its watcher took over, are the tab shown's.
pub fn changed(state: &mut State, folder: &Path, changes: &[Change], at: Duration) -> Vec<Effect> {
    super::summaries::changed(state, folder, changes);
    let Some(index) = state
        .tabs
        .open
        .iter()
        .position(|tab| same_folder(&watched_folder(tab), folder))
    else {
        return Vec::new();
    };
    if state.tabs.shown == Some(index) {
        return reads::changed(state, changes, at);
    }
    let Some(kept) = state.tabs.open.get_mut(index).and_then(|tab| tab.kept.as_mut()) else {
        return Vec::new();
    };
    for request in changes.iter().filter_map(watch_changes) {
        let (hidden, _nothing_read) = schedule_hidden(&kept.hidden, &ReadEvent::Changed { request, at });
        kept.hidden = hidden;
    }
    Vec::new()
}

/// An answer kept for a tab not shown: the end of its read, when it is the read of the tab under way.
pub(super) fn answered(state: &mut State, index: usize, message: &Message) {
    let Message::RepositoryRead { job, .. } = message else {
        return;
    };
    let Some(read) = state.tabs.background.reading.take_if(|read| read.job == *job) else {
        return;
    };
    if let Some(kept) = state.tabs.open.get_mut(index).and_then(|tab| tab.kept.as_mut()) {
        let (hidden, _shown_when_the_tab_is) = schedule_hidden(&kept.hidden, &ReadEvent::Finished(read.started.number));
        kept.hidden = hidden;
    }
}

/// What the tabs not shown need after each message: the read of one stopped while an operation runs or the tab shown
/// is read, or the next tab never read read once nothing else is; and the tabs not shown watched.
pub fn follow(state: &mut State) -> Vec<Effect> {
    if !state.is_operation_running {
        state.tabs.background.operation = None;
    }
    forget_gone(state);
    let is_busy = state.is_operation_running
        || state.opening.is_some()
        || state.reads.schedule.is_reading()
        || state.shown_read().is_none();
    let mut effects = if is_busy { stop(state) } else { read_next(state) };
    effects.extend(super::summaries::follow(state));
    effects.extend(watch(state));
    effects
}

/// Forgets the read of a tab not shown whose tab is gone, or forgot what it kept.
fn forget_gone(state: &mut State) {
    let Some(read) = state.tabs.background.reading.as_ref() else {
        return;
    };
    let is_kept = state.tabs.open.iter().enumerate().any(|(index, tab)| {
        state.tabs.shown != Some(index)
            && same_folder(&tab.folder, &read.folder)
            && tab.kept.as_ref().is_some_and(|kept| kept.jobs.contains_key(&read.job))
    });
    if !is_kept {
        state.tabs.background.reading = None;
    }
}

/// Stops the read of a tab not shown: the tab, which was never read, keeps nothing again and waits for its turn.
fn stop(state: &mut State) -> Vec<Effect> {
    let Some(read) = state.tabs.background.reading.take() else {
        return Vec::new();
    };
    let event = if state.is_operation_running {
        ReadEvent::OperationStarted
    } else {
        ReadEvent::Stopped
    };
    let Some(tab) = state
        .tabs
        .open
        .iter_mut()
        .find(|tab| same_folder(&tab.folder, &read.folder))
    else {
        return vec![Effect::Cancel(read.job)];
    };
    let Some(kept) = tab.kept.take() else {
        return vec![Effect::Cancel(read.job)];
    };
    let (hidden, actions) = schedule_hidden(&kept.hidden, &event);
    if !actions.contains(&ReadAction::Discard(read.started.number)) || !hidden.is_waiting() {
        tab.kept = Some(kept);
        return Vec::new();
    }
    tab.waiting.clear();
    kept.jobs.into_keys().map(Effect::Cancel).collect()
}

/// With `repositories.keepOpen` on, reads the first tab not shown that was never read, in what it will keep.
fn read_next(state: &mut State) -> Vec<Effect> {
    if !state.settings.keep_open || state.tabs.background.reading.is_some() {
        return Vec::new();
    }
    let Some(index) = state
        .tabs
        .open
        .iter()
        .enumerate()
        .position(|(index, tab)| state.tabs.shown != Some(index) && tab.kept.is_none())
    else {
        return Vec::new();
    };
    let folder = state
        .tabs
        .open
        .get(index)
        .map(|tab| tab.folder.clone())
        .unwrap_or_default();
    let (hidden, actions) = schedule_hidden(&HiddenTab::new(), &ReadEvent::Asked(ReadCause::RepositoryChosen));
    let Some(ReadAction::Start(started)) = actions.into_iter().next() else {
        return Vec::new();
    };
    let effect = in_tab(state, index, |state| {
        let effect = workspace::start_read(state, folder.clone());
        if let Some(workspace) = state.workspace.as_mut() {
            workspace.shown = Shown::Reading { folder: folder.clone() };
        }
        effect
    });
    if let Some(kept) = state.tabs.open.get_mut(index).and_then(|tab| tab.kept.as_mut()) {
        kept.hidden = hidden;
    }
    if let Effect::Start(job) = &effect {
        state.tabs.background.reading = Some(BackgroundRead {
            folder,
            job: job.id,
            started,
        });
    }
    vec![effect]
}

/// The working trees of the tabs not shown to watch, when they are not those watched: none with
/// `repositories.keepOpen` off, a tab not shown having no watcher then.
fn watch(state: &mut State) -> Option<Effect> {
    let wanted: Vec<PathBuf> = if state.settings.keep_open && state.workspace.is_some() {
        state
            .tabs
            .open
            .iter()
            .enumerate()
            .filter(|(index, _)| state.tabs.shown != Some(*index))
            .map(|(_, tab)| watched_folder(tab))
            .collect()
    } else {
        Vec::new()
    };
    let mut wanted = wanted;
    // The other folders whose rows say what they are on are watched too, and a change in one reads it again.
    let summaries = if state.settings.keep_open {
        state.tabs.summaries.folders()
    } else {
        Vec::new()
    };
    for folder in summaries {
        if !wanted.iter().any(|known| same_folder(known, &folder)) {
            wanted.push(folder);
        }
    }
    if wanted == state.tabs.background.watched {
        return None;
    }
    state.tabs.background.watched.clone_from(&wanted);
    Some(Effect::WatchTabs(wanted))
}

/// A tab shown again, after what it kept was put back: its read under way goes on as the tab shown's; a tab out of
/// date is read again at once, as **Refresh** does, for the last change it saw; one in which nothing changed reads
/// nothing.
pub(super) fn shown_again(state: &mut State, hidden: &HiddenTab, folder: PathBuf) -> Vec<Effect> {
    if state
        .tabs
        .background
        .reading
        .take_if(|read| same_folder(&read.folder, &folder))
        .is_some()
    {
        return Vec::new();
    }
    super::operation::shown_again(state, &folder);
    let Some(cause) = hidden.out_of_date() else {
        return Vec::new();
    };
    if state.shown_read().is_some() {
        return reads::asked(state, cause.clone());
    }
    // A tab left before its first read ended is read as when it was chosen.
    let read_folder = state.shown_folder().cloned().unwrap_or(folder);
    vec![workspace::start_read(state, read_folder)]
}
