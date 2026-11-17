//! Where a Diff page is shown: as a page of its own, beside the inspector (`diff.beside-inspector`), in a comparison
//! page (`compare.beside-diff`) or in the Changes page (`changes.beside-file`).

use views::diff::DiffPageId;
use views::page::Page;

use super::DiffPage;
use crate::store::state::State;

/// Where a Diff page is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    /// As a page of its own.
    Alone,
    /// Beside the inspector, with the number of the time it was given a file to show (`diff.beside-inspector`).
    Inspector(u64),
    /// Beside a comparison page, which it is shown in (`compare.beside-diff`).
    Comparison(views::compare::ComparePageId),
    /// Beside the list of the Changes page, which it is shown in (`changes.beside-file`).
    Changes,
}

impl Owner {
    /// The fields of a page that say where it is shown: beside the inspector, and beside a comparison; beside the list
    /// of the Changes page has a field of its own.
    pub(super) fn fields(self) -> (Option<u64>, Option<views::compare::ComparePageId>) {
        match self {
            Owner::Alone | Owner::Changes => (None, None),
            Owner::Inspector(serial) => (Some(serial), None),
            Owner::Comparison(page) => (None, Some(page)),
        }
    }
}

impl DiffPage {
    /// Where it is shown.
    #[must_use]
    pub fn owner(&self) -> Owner {
        if self.is_in_changes {
            return Owner::Changes;
        }
        match (self.beside, self.comparison) {
            (_, Some(page)) => Owner::Comparison(page),
            (Some(serial), None) => Owner::Inspector(serial),
            (None, None) => Owner::Alone,
        }
    }
}

/// Shows a page once it can be: in the comparison page or the Changes page it is beside, or as a page.
pub(super) fn show_opened(state: &mut State, id: DiffPageId) {
    let owner = state
        .diff_pages
        .pages
        .iter()
        .find(|page| page.id == id)
        .map(DiffPage::owner);
    match owner {
        Some(Owner::Comparison(page)) => crate::store::pages::show_page(state, Page::Compare(page)),
        Some(Owner::Changes) => crate::store::pages::show_page(state, Page::Changes),
        Some(Owner::Alone | Owner::Inspector(_)) | None => crate::store::pages::show_page(state, Page::Diff(id)),
    }
}
