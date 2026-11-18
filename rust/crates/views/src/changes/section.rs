//! **Changes**, the section of the side column between **Repositories** and **Branches** (`changes.section`, D71):
//! what the Changes page shows as a column, under the heading of the section, so that the changes are at hand beside
//! the timeline and beside every page; a file of it opens in the middle of the window (`changes.beside-file`).

use ui::Ui;
use ui::layout::Insets;

use super::{ChangesRepository, ChangesState, Part, body_height, page_body};
use crate::intent::Intent;

/// The **Changes** section of the side column, over the rectangle its caller clipped it to, under its heading: the
/// banner of an operation that stopped, the commit box, the identity, **Conflicts** and the list of the changed files
/// with their boxes, as the Changes page shows them in a column; nothing while no repository is shown. Its widgets are
/// named apart from those of the page, which may be drawn in the same frame, and the keyboard that **Show changes**
/// asks for goes to the page, not here.
///
/// Feature: changes.section
pub fn section_view(state: &ChangesState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let ChangesState::Repository(repository) = state else {
        return Vec::new();
    };
    let repository = ChangesRepository {
        wants_focus: false,
        ..repository.clone()
    };
    let insets = section_insets(ui);
    ui.scope("changes section", |ui| {
        page_body(&repository, ui, (insets, Part::Section))
    })
}

/// How many files the heading of the section counts: the changed files and the files in conflict together (Q769); none
/// while no repository is shown.
#[must_use]
pub fn section_count(state: &ChangesState<'_>) -> Option<usize> {
    match state {
        ChangesState::NoRepository(_) => None,
        ChangesState::Repository(repository) => Some(repository.list.rows.len() + repository.list.conflicts.len()),
    }
}

/// The height of what the section shows under its heading, which it is no taller than where it starts (Q768); none
/// while no repository is shown, when the section is its heading alone.
#[must_use]
pub fn section_height(state: &ChangesState<'_>, ui: &Ui<'_>) -> Option<f32> {
    match state {
        ChangesState::NoRepository(_) => None,
        ChangesState::Repository(repository) => Some(body_height(repository, ui, section_insets(ui))),
    }
}

/// The margins of the section: those of the column of the Changes page beside a file.
fn section_insets(ui: &Ui<'_>) -> Insets {
    let spacing = ui.theme().sizes.spacing;
    Insets::symmetric(spacing.medium, spacing.medium)
}
