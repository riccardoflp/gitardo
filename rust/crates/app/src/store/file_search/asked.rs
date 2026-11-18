//! How the Search page is asked for (`file-search.open`), and what it searches kept for each repository
//! (`file-search.remember`): the text selected in the page in front becomes what is looked for, **Search here** names
//! where, and the search of a repository comes back when its page is shown again, after the window was closed too.

use model::file_search::{Asked, Search, open_with};
use views::page::Page;

use super::FileSearchPage;
use crate::persisted::file_search::{keep_search, search_of};
use crate::persisted::same_folder;
use crate::store::message::Effect;
use crate::store::state::State;

/// Shows the page as it was left, with the keyboard in its box; what it looks for and where follow how it was asked
/// for: from the bar of views, the key or Quick search, the text selected in the page in front. The way to it starts
/// again from the timeline.
pub fn show(state: &mut State, asked: Option<Asked>) -> Vec<Effect> {
    let asked = asked.unwrap_or_else(|| Asked::Page {
        selected: selected_text(state),
    });
    adopt_folder(state);
    let search = open_with(&state.file_search.search, &asked);
    state.file_search.focus += 1;
    let mut effects = remember(state, search);
    effects.extend(super::super::pages::starting_again(state, |state| {
        super::super::pages::show(state, Page::Search)
    }));
    effects
}

/// The text selected in the page in front: in the File page, or in the side of the Diff page that is edited.
fn selected_text(state: &State) -> Option<String> {
    if let Some(Page::Diff(id)) = state.page_shown() {
        return super::super::diff_page::selected_text(state, id);
    }
    let id = state.file_in_front()?;
    let page = state.file_pages.pages.iter().find(|page| page.id == id)?;
    Some(page.document()?.buffer().slice(page.cursor.range()).to_owned())
}

/// Makes the page that of the worktree shown, when it is another: what was searched there comes back, or a first
/// search starts; what the page listed and found is forgotten.
pub fn adopt_folder(state: &mut State) {
    let Some(folder) = state.shown_read().map(|read| read.state.folder.clone()) else {
        return;
    };
    let page = &mut state.file_search;
    if page.folder.as_deref().is_some_and(|known| same_folder(known, &folder)) {
        return;
    }
    let kept = search_of(&state.persisted.file_searches, &folder).cloned();
    let search = match kept {
        Some(kept) => kept,
        None if page.folder.is_none() => std::mem::take(&mut page.search),
        None => Search::default(),
    };
    *page = FileSearchPage {
        search,
        folder: Some(folder),
        focus: page.focus,
        reveal: page.reveal,
        ..FileSearchPage::default()
    };
}

/// Takes what the page searches now, and keeps it for its repository when it changed.
pub fn remember(state: &mut State, search: Search) -> Vec<Effect> {
    let page = &mut state.file_search;
    if page.search == search {
        return Vec::new();
    }
    page.search = search;
    let Some(folder) = page.folder.clone() else {
        return Vec::new();
    };
    keep_search(&mut state.persisted.file_searches, &folder, &state.file_search.search);
    vec![Effect::WritePersisted(state.persisted.clone())]
}
