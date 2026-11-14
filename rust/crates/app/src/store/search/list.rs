//! A search shown as a list (`search.list-mode`): while the funnel is pressed, `Enter` narrows the timeline to the
//! commits the search finds, read a page at a time with the arguments of the search, with no lines, no draft, no
//! stashes and no folds. Whether the funnel is pressed is kept between runs. Beside it, the history of a commit on no
//! branch or tag alone (`search.go-to-hash`), and the history of a file, of some lines or of a folder
//! (`file-history.file`, `file-history.lines`, `file-history.folder`).

use std::ffi::OsString;

use git::read::file_history::history_arguments;
use git::read::search::search_arguments;
use model::CommitId;
use model::file_history::HistoryOf;

use super::super::message::Effect;
use super::super::pages;
use super::super::state::State;
use super::super::timeline::read_again;
use super::{Asked, ask, drop_found};

/// What narrows the history the timeline lists: the arguments of the search shown as a list, or of the history of a
/// file, of some lines or of a folder, if one is.
#[must_use]
pub fn narrowing(state: &State) -> Vec<OsString> {
    let search = &state.search;
    match (&search.list, &search.history) {
        (Some(asked), _) => search_arguments(asked.mode, &asked.text),
        (None, Some(history)) => history_arguments(history),
        (None, None) => Vec::new(),
    }
}

/// Shows the history of a file, of some lines or of a folder in the timeline, in place of what narrowed it; none shows
/// all commits. The timeline is read again and comes to the front.
pub fn show_history(state: &mut State, history: Option<HistoryOf>) -> Vec<Effect> {
    let mut effects = drop_found(state);
    state.search.list = None;
    state.search.alone = None;
    state.search.history = history;
    effects.extend(read_again(state));
    effects.extend(pages::show_timeline(state));
    effects
}

/// Shows a search as a list: the timeline is read again narrowed to what it finds, and the search itself is asked for
/// the count.
pub fn show(state: &mut State, asked: Asked) -> Vec<Effect> {
    let mut effects = ask(state, asked.clone());
    state.search.list = Some(asked);
    state.search.alone = None;
    state.search.history = None;
    effects.extend(read_again(state));
    effects
}

/// Shows the history of a commit alone, in place of the branches shown, until **Show all commits**.
pub fn alone(state: &mut State, commit: CommitId) -> Vec<Effect> {
    state.search.list = None;
    state.search.alone = Some(commit);
    state.search.history = None;
    read_again(state)
}

/// **Show all commits**: the search shown as a list, the history of a commit alone, or the history of a file, of some
/// lines or of a folder, is taken away; the timeline is read again with the branches shown, and nothing is dimmed.
pub fn show_all(state: &mut State) -> Vec<Effect> {
    let was_list = state.search.list.take().is_some();
    let was_alone = state.search.alone.take().is_some();
    let was_history = state.search.history.take().is_some();
    if !was_list && !was_alone && !was_history {
        return Vec::new();
    }
    let mut effects = drop_found(state);
    effects.extend(read_again(state));
    effects
}

/// The funnel pressed: with text in the box, what is highlighted becomes the list, and the list a search highlighted
/// in the timeline; with nothing in it, all commits are shown again.
pub fn toggle(state: &mut State) -> Vec<Effect> {
    state.persisted.search_as_list = !state.persisted.search_as_list;
    let mut effects = vec![Effect::WritePersisted(state.persisted.clone())];
    let asked = state.search.mode.history().map(|mode| Asked {
        mode,
        text: state.search.text.trim().to_owned(),
    });
    let asked = asked.filter(|asked| !asked.text.is_empty());
    match (state.persisted.search_as_list, asked) {
        (true, Some(asked)) => effects.extend(show(state, asked)),
        (true, None) => {}
        (false, asked) => {
            effects.extend(show_all(state));
            if let Some(asked) = asked {
                effects.extend(ask(state, asked));
            }
        }
    }
    effects
}
