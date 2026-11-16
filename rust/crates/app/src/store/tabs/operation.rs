//! The operation running while tabs are switched (`repo.switch-tab`): the window runs one at a time, in the tab it was
//! started in, which shows a spinner; its tab is shown again before it asks a question or shows its plan; another
//! operation started meanwhile, in any tab, is refused; and once it ends in a tab not shown, what it leaves to do waits
//! for that tab.

use std::path::Path;

use engine::LogEntry;
use model::CommitId;
use model::repo::{HiddenTab, ReadCause, name_repositories};
use model::state::Stash;

use super::super::feature::{Ended, Said};
use super::super::message::{Effect, JobId, Message};
use super::super::state::State;
use super::hidden::in_tab;
use super::{forget, switch_tab};
use crate::persisted::same_folder;

/// What an operation that ended leaves to do once its repository is read again, kept for its tab while another is
/// shown.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AfterOperation {
    /// The commit to select; `Some(None)` selects `HEAD`.
    pub select_after_read: Option<Option<CommitId>>,
    /// How many commits before `HEAD` to select instead.
    pub select_before_head: usize,
    /// The stashes before a pull with `--autostash`.
    pub autostash_before: Option<Vec<Stash>>,
    /// Whether to tell the first bad commit a bisect found.
    pub check_bisect: bool,
    /// The title of the plan that failed, to tell whether it stopped on conflicts.
    pub check_stopped: Option<String>,
}

impl AfterOperation {
    /// Takes out of the operations of the window what they leave to do after the next read.
    fn take(state: &mut State) -> AfterOperation {
        let operations = &mut state.operations;
        AfterOperation {
            select_after_read: operations.select_after_read.take(),
            select_before_head: std::mem::take(&mut operations.select_before_head),
            autostash_before: operations.autostash_before.take(),
            check_bisect: std::mem::take(&mut operations.check_bisect),
            check_stopped: operations.check_stopped.take(),
        }
    }

    /// Gives it back to the operations of the window.
    fn put(self, state: &mut State) {
        let operations = &mut state.operations;
        operations.select_after_read = self.select_after_read;
        operations.select_before_head = self.select_before_head;
        operations.autostash_before = self.autostash_before;
        operations.check_bisect = self.check_bisect;
        operations.check_stopped = self.check_stopped;
    }
}

/// Notes the tab an operation starts in: the tab shown.
pub fn operation_started(state: &mut State) {
    state.tabs.background.operation = state
        .tabs
        .shown
        .and_then(|shown| state.tabs.open.get(shown))
        .map(|tab| tab.folder.clone());
}

/// The place of the tab where an operation runs.
#[must_use]
pub fn operation_tab(state: &State) -> Option<usize> {
    let folder = state.tabs.background.operation.as_ref()?;
    if !state.is_operation_running {
        return None;
    }
    state.tabs.open.iter().position(|tab| same_folder(&tab.folder, folder))
}

/// The title of the operation running: the title of its plan, or its identifier until it shows one.
#[must_use]
pub fn operation_title(state: &State) -> String {
    state.operations.running.as_ref().map_or_else(String::new, |running| {
        running.title.clone().unwrap_or_else(|| running.feature.0.to_owned())
    })
}

/// An operation started while one runs, in any tab: refused, saying which runs and where.
pub fn refuse_operation(state: &mut State) -> Vec<Effect> {
    let repository = operation_tab(state)
        .and_then(|index| state.tabs.open.get(index))
        .map(|tab| {
            let roots = state
                .workspace
                .as_ref()
                .map_or_default(|workspace| workspace.folders.clone());
            name_repositories(&roots, std::slice::from_ref(&tab.folder))
                .into_iter()
                .next()
                .unwrap_or_default()
        })
        .unwrap_or_default();
    let title = operation_title(state);
    state.operations.said = Some(Said::error(views::text::repo::running_elsewhere(&title, &repository)));
    Vec::new()
}

/// What the operation running says, before the store hears of it: its tab is shown again before it asks a question or
/// shows its plan; once it ends in a tab not shown, the end is taken in that tab. Gives the message back when the store
/// is to take it as it is.
pub fn before(state: &mut State, message: Message) -> (Vec<Effect>, Option<Message>) {
    match message {
        message @ (Message::FeatureAsks(_) | Message::FeatureConfirms(_)) => {
            let effects = match operation_tab(state).filter(|index| state.tabs.shown != Some(*index)) {
                Some(index) => switch_tab(state, index),
                None => Vec::new(),
            };
            (effects, Some(message))
        }
        Message::FeatureEnded { ended, log } => match hidden_operation_tab(state) {
            Some(index) => (ended_in_tab(state, index, (ended, log)), None),
            None => (Vec::new(), Some(Message::FeatureEnded { ended, log })),
        },
        message => (Vec::new(), Some(message)),
    }
}

/// The tab where the operation runs, when it is not shown and keeps what it showed.
fn hidden_operation_tab(state: &State) -> Option<usize> {
    operation_tab(state).filter(|index| {
        state.tabs.shown != Some(*index) && state.tabs.open.get(*index).is_some_and(|tab| tab.kept.is_some())
    })
}

/// The operation ended in a tab not shown: what is the window's is said and logged at once; what is the tab's happens
/// in it. The read that follows the operation waits for the tab to be shown: the tab is out of date, and what the
/// operation leaves to do after that read is kept for it. With `repositories.keepOpen` off, the tab then forgets what
/// it kept, unless a page of it has edits not saved.
fn ended_in_tab(state: &mut State, index: usize, (ended, log): (Ended, Vec<LogEntry>)) -> Vec<Effect> {
    let effects = in_tab(state, index, |state| {
        let effects = super::super::feature_ended(state, ended, log);
        let cancelled = super::super::workspace::cancel_reads(state);
        without(effects, &cancelled)
    });
    let after = AfterOperation::take(state);
    let keep_open = state.settings.keep_open;
    let Some(tab) = state.tabs.open.get_mut(index) else {
        return effects;
    };
    state.tabs.background.after_operation = Some((tab.folder.clone(), after));
    let mut effects = effects;
    if let Some(kept) = tab.kept.as_mut() {
        kept.hidden = HiddenTab::left(Some(ReadCause::OperationEnded));
        if !keep_open && !kept.has_unsaved() {
            let forgotten = std::mem::replace(tab, super::Tab::new(tab.folder.clone()));
            effects.extend(forget(forgotten));
        }
    }
    effects
}

/// The effects of a change in a tab not shown, without the reads it started and cancelled, and without the watchers,
/// which are the tab shown's.
fn without(effects: Vec<Effect>, cancelled: &[Effect]) -> Vec<Effect> {
    let jobs: Vec<JobId> = cancelled
        .iter()
        .filter_map(|effect| match effect {
            Effect::Cancel(job) => Some(*job),
            _ => None,
        })
        .collect();
    effects
        .into_iter()
        .filter(|effect| match effect {
            Effect::Start(job) => !jobs.contains(&job.id),
            Effect::WatchSettings(_) | Effect::WatchRepository(_) => false,
            _ => true,
        })
        .collect()
}

/// A tab shown again: what an operation that ended in it left to do after the next read is given back.
pub(super) fn shown_again(state: &mut State, folder: &Path) {
    if let Some((_, after)) = state
        .tabs
        .background
        .after_operation
        .take_if(|(of, _)| same_folder(of, folder))
    {
        after.put(state);
    }
}
