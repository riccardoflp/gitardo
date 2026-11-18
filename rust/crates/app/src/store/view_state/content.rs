//! What is under the header: the Welcome page while there is no workspace, the page shown with its title row, or the
//! timeline.

use model::trail::way_to;
use views::file_view::FilePageState;
use views::page::{Page, PageHostState, PageState};
use views::settings::SettingsPageState;
use views::welcome::WelcomeState;
use views::{Content, TimelineState};

use super::ViewData;
use crate::store::state::State;
use crate::store::timeline::workspace_panes;

impl ViewData {
    /// What is under the header: the Welcome page while there is no workspace, the page shown, or the timeline.
    pub(super) fn content<'a>(&'a self, state: &'a State) -> Content<'a> {
        let page = if state.workspace.is_none() {
            Some(Page::Welcome)
        } else {
            state.page_shown()
        };
        if let Some(Page::Diff(id)) = page
            && crate::store::diff_page::beside::is_beside(state, id)
        {
            return self.beside_inspector(state, id);
        }
        match page.and_then(|page| self.page_state(state, page)) {
            Some(page) => Content::Page(self.host(state, page)),
            None => Content::Timeline(self.timeline(state)),
        }
    }

    /// What a page shows, when it still has something to show.
    fn page_state<'a>(&'a self, state: &'a State, page: Page) -> Option<PageState<'a>> {
        match page {
            Page::Welcome => Some(PageState::Welcome(WelcomeState {
                recent: &self.recent,
                done: state.persisted.welcome_steps,
                has_timeline: state.workspace.is_some(),
            })),
            Page::Commit(_) | Page::Diff(_) | Page::Merge(_) | Page::Compare(_) => opened_page(state, page),
            Page::Review => crate::store::review::page_state(state).map(PageState::Review),
            Page::File(id) => self.file_page(state, id).map(|page| PageState::File(Box::new(page))),
            Page::Settings => Some(PageState::Settings(SettingsPageState {
                rows: &self.setting_rows,
                target: crate::store::settings_page::shown_target(state),
                has_repository: state.shown_folder().is_some(),
                page: &state.settings_page,
            })),
            Page::Rebase => crate::store::rebase::page_state(state).map(PageState::Rebase),
            Page::Changes => Some(crate::store::changes_beside::page_state(
                state,
                &self.changes,
                (&self.blame, self.panes.sizes.changes_column),
            )),
            Page::Files => Some(self.files_page(state)),
            Page::Search => Some(PageState::Search(crate::store::file_search::page_state(state))),
            Page::NewWorktrees => Some(PageState::NewWorktrees(
                crate::store::new_worktrees::page_state(state),
                &state.new_worktrees.title,
            )),
        }
    }

    /// The pages the page shown was reached through, before it, each named as its own title row names it
    /// (`ui.page-trail`).
    fn way<'a>(&'a self, state: &'a State, shown: Page) -> Vec<(Page, &'a str)> {
        let mut way = way_to(&state.pages.way, &state.pages.open, &shown);
        let _shown = way.pop();
        way.into_iter()
            .filter_map(|page| Some((page, self.page_state(state, page)?.title())))
            .collect()
    }

    /// A page with its title row: the trail while there is a workspace, whose timeline it comes back to, and the button
    /// of the side column there (`repo.side-column`).
    fn host<'a>(&'a self, state: &'a State, page: PageState<'a>) -> PageHostState<'a> {
        let has_workspace = state.workspace.is_some();
        PageHostState {
            has_trail: has_workspace,
            side: has_workspace.then(|| self.side_place(state)),
            way: self.way(state, page.page()),
            page,
        }
    }

    /// The Files page, with the file opened beside its tree when there is one (`file-view.beside-files`).
    fn files_page<'a>(&'a self, state: &'a State) -> PageState<'a> {
        let file = state.files_page.shown_file.and_then(|id| self.file_page(state, id));
        crate::store::files_page::beside_state(state, file, self.panes.sizes.files_tree)
    }

    /// A File page with the blame of its lines, when it is still kept.
    fn file_page<'a>(&'a self, state: &'a State, id: views::file_view::FilePageId) -> Option<FilePageState<'a>> {
        crate::store::file_page::page_state(state, id).map(|page| FilePageState {
            blame: self.blame.page_blame(state, id),
            ..page
        })
    }

    /// The Diff page beside the inspector, in place of the commits; in place of everything when no repository is read.
    /// It puts the side column away while it is shown (`diff.beside-inspector`), so its title row has no button for it.
    fn beside_inspector<'a>(&'a self, state: &'a State, id: views::diff::DiffPageId) -> Content<'a> {
        let Some(page) = crate::store::diff_page::page_state(state, id) else {
            return Content::Timeline(self.timeline(state));
        };
        let host = PageHostState {
            side: None,
            ..self.host(state, PageState::Diff(page))
        };
        let mut timeline = self.timeline(state);
        if let TimelineState::Repository { beside, .. } = &mut timeline {
            *beside = Some(Box::new(views::diff::BesideInspectorState { host }));
            return Content::Timeline(timeline);
        }
        Content::Page(host)
    }
}

/// A page of a commit, a diff, a file in conflict or a comparison, when it is still kept.
fn opened_page(state: &State, page: Page) -> Option<PageState<'_>> {
    match page {
        Page::Commit(id) => crate::store::commit_page::page_state(state, id).map(PageState::Commit),
        Page::Diff(id) => crate::store::diff_page::page_state(state, id).map(PageState::Diff),
        Page::Merge(id) => {
            crate::store::merge_page::page_state(state, id).map(|(page, title)| PageState::Merge(page, title))
        }
        Page::Compare(id) => {
            crate::store::compare::beside::page_state(state, id, workspace_panes(state).sizes.inspector)
        }
        _ => None,
    }
}
