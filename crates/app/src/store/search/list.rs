//! A search shown as a list (`search.list-mode`): while the funnel is pressed, `Enter` narrows the timeline to the
//! commits the search finds, read a page at a time with the arguments of the search, with no lines, no draft, no
//! stashes and no folds. Whether the funnel is pressed is kept between runs. Beside it, the history of a commit on no
//! branch or tag alone (`search.go-to-hash`).

use std::ffi::OsString;

use git::read::search::search_arguments;
use model::CommitId;

use super::super::message::Effect;
use super::super::state::State;
use super::super::timeline::read_again;
use super::{Asked, ask, drop_found};

/// What narrows the history the timeline lists: the arguments of the search shown as a list, if one is.
#[must_use]
pub fn narrowing(state: &State) -> Vec<OsString> {
    state
        .search
        .list
        .as_ref()
        .map(|asked| search_arguments(asked.mode, &asked.text))
        .unwrap_or_default()
}

/// Shows a search as a list: the timeline is read again narrowed to what it finds, and the search itself is asked for
/// the count.
pub fn show(state: &mut State, asked: Asked) -> Vec<Effect> {
    let mut effects = ask(state, asked.clone());
    state.search.list = Some(asked);
    state.search.alone = None;
    effects.extend(read_again(state));
    effects
}

/// Shows the history of a commit alone, in place of the branches shown, until **Show all commits**.
pub fn alone(state: &mut State, commit: CommitId) -> Vec<Effect> {
    state.search.list = None;
    state.search.alone = Some(commit);
    read_again(state)
}

/// **Show all commits**: the search shown as a list, or the history of a commit alone, is taken away; the timeline
/// is read again with the branches shown, and nothing is dimmed.
pub fn show_all(state: &mut State) -> Vec<Effect> {
    let was_list = state.search.list.take().is_some();
    let was_alone = state.search.alone.take().is_some();
    if !was_list && !was_alone {
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
