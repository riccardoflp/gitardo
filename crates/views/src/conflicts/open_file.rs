//! `conflicts.open-file`: a file in conflict opened where it can be resolved by hand, which depends on what the two
//! sides did to it.

use model::conflicts::{SideNamesInput, side_names};
use model::state::{ConflictKind, ConflictedFile, Operation};
use ui::{Label, Sense, Ui};

use crate::diff::{DiffFile, DiffTarget, DiffVersion};
use crate::intent::Intent;
use crate::text::conflicts as words;

/// What is asked of a file in conflict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConflictsIntent {
    /// A file in conflict was clicked: open it where it can be resolved, unless it was resolved meanwhile.
    Open(String),
}

/// A file in conflict, where its name is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenFileState<'a> {
    /// The file.
    pub file: &'a ConflictedFile,
}

/// The name of a file in conflict, over the rectangle its caller clipped it to; a click on it asks for the file to be
/// opened where it can be resolved.
///
/// Feature: conflicts.open-file
pub fn open_file_view(state: &OpenFileState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let path = state.file.path.as_str();
    let response = ui.interact(ui.id(("conflict name", path)), rect, Sense::CLICK | Sense::HOVER);
    if response.hovered {
        let theme = *ui.theme();
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    let _ = ui.label(rect, &Label::new(path));
    if response.clicked {
        vec![Intent::Conflicts(ConflictsIntent::Open(path.to_owned()))]
    } else {
        Vec::new()
    }
}

/// Where a file in conflict opens: a file one side deleted in a Diff page of its base beside the file in the working
/// tree, titled with what each side did; any other file in a Diff page of what is checked out beside the working tree,
/// with git's markers in it, until the Merge and File pages are there. A file both sides deleted opens nothing.
#[must_use]
pub fn open_target(file: &ConflictedFile, operation: Option<Operation>) -> Option<DiffTarget> {
    let names = side_names(&SideNamesInput {
        operation,
        ..SideNamesInput::default()
    });
    let (current, incoming) = (names.current.role.words(), names.incoming.role.words());
    let deleter_and_keeper = match file.kind {
        ConflictKind::BothDeleted => return None,
        ConflictKind::DeletedByThem => Some((incoming, current)),
        ConflictKind::DeletedByUs => Some((current, incoming)),
        ConflictKind::BothModified | ConflictKind::BothAdded | ConflictKind::AddedByUs | ConflictKind::AddedByThem => {
            None
        }
    };
    let paths = DiffFile::same(&file.path);
    let Some((deleter, keeper)) = deleter_and_keeper else {
        return Some(DiffTarget::file(DiffVersion::Head, DiffVersion::WorkingTree, paths));
    };
    let name = file.path.rsplit('/').next().unwrap_or(&file.path);
    let mut target = DiffTarget::file(DiffVersion::Base, DiffVersion::WorkingTree, paths);
    target.title = Some(words::deleted_and_changed(
        name,
        &lower_first(deleter),
        &lower_first(keeper),
    ));
    Some(target)
}

/// Words put inside a sentence: their first letter in lower case.
fn lower_first(words: &str) -> String {
    let mut characters = words.chars();
    characters
        .next()
        .map_or_else(String::new, |first| first.to_lowercase().chain(characters).collect())
}
