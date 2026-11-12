//! The pages open in place of the timeline, and the steps of the Welcome page (`welcome.page`).

use views::page::Page;

use super::message::Effect;
use super::state::State;

/// The step of the Welcome page ticked when the timeline is shown.
const TIMELINE_STEP: usize = 0;

/// Shows a page, opening it once: asking for it again shows the one there is.
pub fn show_page(state: &mut State, page: Page) {
    let index = state
        .pages
        .open
        .iter()
        .position(|open| *open == page)
        .unwrap_or_else(|| {
            state.pages.open.push(page);
            state.pages.open.len() - 1
        });
    state.pages.shown = Some(index);
}

/// Shows the timeline in place of the page shown.
pub fn show_timeline(state: &mut State) -> Vec<Effect> {
    state.pages.shown = None;
    tick_timeline(state).into_iter().collect()
}

/// `Ctrl+Tab`: the next of the timeline and the pages open, back to the timeline after the last page.
pub fn next_page(state: &mut State) -> Vec<Effect> {
    if state.workspace.is_none() {
        return Vec::new();
    }
    let next = match state.pages.shown {
        None => 0,
        Some(index) => index + 1,
    };
    if next < state.pages.open.len() {
        state.pages.shown = Some(next);
        Vec::new()
    } else {
        show_timeline(state)
    }
}

/// Closes the page shown; the timeline shows in its place.
pub fn close_page(state: &mut State) -> Vec<Effect> {
    if let Some(index) = state.pages.shown.take()
        && index < state.pages.open.len()
    {
        let _closed = state.pages.open.remove(index);
    }
    tick_timeline(state).into_iter().collect()
}

/// Ticks **Open the timeline** on the Welcome page once the timeline shows a repository, and writes it down.
pub fn tick_timeline(state: &mut State) -> Option<Effect> {
    let is_timeline_shown = state.pages.shown.is_none() && state.shown_read().is_some();
    let step = state.persisted.welcome_steps.get_mut(TIMELINE_STEP)?;
    if !is_timeline_shown || *step {
        return None;
    }
    *step = true;
    Some(Effect::WritePersisted(state.persisted.clone()))
}
