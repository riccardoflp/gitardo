//! What a File page is drawn from (`file-view.page`), and the question asked before closing one with edits not saved.

use views::file_view::{CloseQuestion, FilePageId, FilePageState, FileShown, FileText, FindState, GoToLineState};

use super::{FileContent, TAB_WIDTH};
use crate::store::state::State;

/// What a page is drawn from.
#[must_use]
pub fn page_state(state: &State, id: FilePageId) -> Option<FilePageState<'_>> {
    let page = state.file_pages.pages.iter().find(|page| page.id == id)?;
    let shown = match &page.content {
        FileContent::Reading => FileShown::Reading,
        FileContent::NotText => FileShown::NotText,
        FileContent::Failed(error) => FileShown::Failed(error),
        FileContent::Text(document) => FileShown::Text(FileText {
            buffer: document.buffer(),
            cursor: page.cursor,
            colors: &page.colors.spans,
            reveal: page.reveal,
            focus: page.focus,
        }),
    };
    let line_count = page.document().map_or(1, |document| document.buffer().line_count());
    Some(FilePageState {
        id,
        title: &page.title,
        path: &page.path,
        is_read_only: page.revision.is_some(),
        is_unsaved: page.is_unsaved(),
        shown,
        find: page.find.as_ref().map(|find| FindState {
            typed: &find.typed,
            options: find.options,
            marks: &find.marks,
            current: find.current,
            is_invalid: find.is_invalid,
            focus: find.focus,
        }),
        go_to_line: page
            .go_to_line
            .as_deref()
            .map(|typed| GoToLineState { typed, line_count }),
        tab_width: TAB_WIDTH,
        wrap: state.settings.word_wrap,
        blame: views::blame::PageBlame::default(),
        markdown: super::markdown::shown(page),
    })
}

/// The question asked before closing a page with edits not saved, while it waits.
#[must_use]
pub fn close_question(state: &State) -> Option<CloseQuestion<'_>> {
    let id = state.file_pages.closing?;
    let page = state.file_pages.pages.iter().find(|page| page.id == id)?;
    let name = page.path.rsplit('/').next().unwrap_or(&page.path);
    Some(CloseQuestion { name })
}
