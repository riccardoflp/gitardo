//! The pages open in place of the timeline, and the steps of the Welcome page (`welcome.page`).

use views::page::Page;

use super::message::Effect;
use super::state::State;

/// The step of the Welcome page ticked when the timeline is shown.
const TIMELINE_STEP: usize = 0;
/// The step of the Welcome page ticked when the page of a commit is opened.
const COMMIT_STEP: usize = 2;

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

/// Shows a page, opening it when it is not open; the Changes page shows whole, nothing beside its list
/// (`changes.beside-file`).
pub fn show(state: &mut State, page: Page) -> Vec<Effect> {
    if page == Page::Settings {
        state.settings_page.shown();
    }
    if page == Page::Changes {
        super::changes_beside::leave(state);
    }
    show_page(state, page);
    Vec::new()
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
    if !is_timeline_shown {
        return None;
    }
    tick(state, TIMELINE_STEP)
}

/// Ticks **Open a commit, compare two branches** on the Welcome page once the page of a commit is opened, in any way,
/// and writes it down.
pub fn tick_commit_page(state: &mut State) -> Option<Effect> {
    tick(state, COMMIT_STEP)
}

/// Ticks a step of the Welcome page, which stays ticked, and writes it down the first time.
fn tick(state: &mut State, step: usize) -> Option<Effect> {
    let done = state.persisted.welcome_steps.get_mut(step)?;
    if *done {
        return None;
    }
    *done = true;
    Some(Effect::WritePersisted(state.persisted.clone()))
}

/// The recent workspaces as the Welcome page lists them: each by its folders, or by the name of the workspace file of
/// VS Code it was opened from (`repo.open-code-workspace`).
pub(super) fn recent_names(state: &State) -> Vec<String> {
    state
        .persisted
        .recent
        .iter()
        .map(|workspace| {
            workspace.file.as_ref().and_then(|file| file.file_stem()).map_or_else(
                || folder_list(&workspace.folders),
                |name| name.to_string_lossy().into_owned(),
            )
        })
        .collect()
}

/// The folders of a workspace, on one line.
fn folder_list(folders: &[std::path::PathBuf]) -> String {
    folders
        .iter()
        .map(|folder| folder.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
