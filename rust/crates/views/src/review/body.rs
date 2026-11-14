//! The body of the review page: the files and the filters beside the cards, or over them in a narrow window; each
//! file's header with its arrow and **Accept the file**, its cards under it, and the files deleted on one side.

use ui::layout::{Align, Area, place};
use ui::{Icon, Label, Rect, TextKind, Ui};

use super::card::{CardState, card_height, card_view, fold_arrow};
use super::deleted::{DeletedState, deleted_card_view, deleted_height};
use super::files::{file_list_view, list_height};
use super::head::{FileState, accept_file_view};
use super::{FileCards, ReviewIntent, ReviewState};
use crate::intent::Intent;
use crate::text::review as words;

/// The width of the file list beside the cards.
const LIST_WIDTH: f32 = 260.0;
/// The narrowest the page shows the list beside the cards; narrower, the list is over them.
const NARROWEST: f32 = 720.0;

/// What the cards' column shows, one under the other.
enum Block<'a> {
    /// The header of a file.
    File(&'a FileCards),
    /// A card.
    Card(CardState<'a>),
    /// A file deleted on one side.
    Deleted(DeletedState<'a>),
}

/// The blocks of the cards' column.
fn blocks<'a>(state: &ReviewState<'a>) -> Vec<Block<'a>> {
    let mut blocks = Vec::new();
    for file in state.files {
        let shown: Vec<_> = file.conflicts.iter().filter(|card| state.is_shown(card)).collect();
        if shown.is_empty() {
            continue;
        }
        blocks.push(Block::File(file));
        if file.is_folded {
            continue;
        }
        blocks.extend(shown.into_iter().map(|card| {
            Block::Card(CardState {
                card,
                sides: state.sides,
                layout: state.layout,
                widths: state.widths,
                is_running: state.is_running,
                can_ask: state.proposers.ai,
                writing: state.writing.filter(|writing| *writing.key == card.key),
            })
        }));
    }
    blocks.extend(
        state
            .deleted
            .iter()
            .filter(|file| state.is_deleted_shown(file))
            .map(|file| {
                Block::Deleted(DeletedState {
                    file,
                    sides: state.sides,
                    is_running: state.is_running,
                })
            }),
    );
    blocks
}

/// The height of a block in a width.
fn block_height(block: &Block<'_>, ui: &Ui<'_>, width: f32) -> f32 {
    match block {
        Block::File(_) => ui.theme().sizes.control_height,
        Block::Card(card) => card_height(card, ui, width),
        Block::Deleted(deleted) => deleted_height(deleted.file, ui),
    }
}

/// The files beside the cards, or over them.
pub(super) fn body(state: &ReviewState<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let spacing = ui.theme().sizes.spacing;
    let mut area = Area::with_gap(rect, spacing.large);
    let list = if rect.width < NARROWEST {
        area.top(list_height(state, ui).min(rect.height * 0.4))
    } else {
        area.left(LIST_WIDTH)
    };
    let mut intents = Vec::new();
    let list_content = list_height(state, ui);
    let _ = ui.scroll_area("review list", list, (list.width, list_content), |ui, content| {
        intents.extend(ui.clip(content, |ui| file_list_view(state, ui)));
    });
    let cards = area.rest();
    let blocks = blocks(state);
    let gap = spacing.medium;
    let heights: Vec<f32> = blocks
        .iter()
        .map(|block| block_height(block, ui, cards.width))
        .collect();
    let total = heights.iter().map(|height| height + gap).sum::<f32>();
    let _ = ui.scroll_area("review cards", cards, (cards.width, total), |ui, content| {
        let mut column = Area::with_gap(content, gap);
        for (block, height) in blocks.iter().zip(heights) {
            let rect = column.top(height);
            if rect.bottom() < content.y || rect.y > content.bottom() {
                continue;
            }
            intents.extend(ui.clip(rect, |ui| match block {
                Block::File(file) => file_header(state, ui, file),
                Block::Card(card) => card_view(card, ui),
                Block::Deleted(deleted) => deleted_card_view(deleted, ui),
            }));
        }
    });
    intents
}

/// The header of a file's conflicts: the arrow that folds them, its path, and **Accept the file**.
fn file_header(state: &ReviewState<'_>, ui: &mut Ui<'_>, file: &FileCards) -> Vec<Intent> {
    let rect = ui.current_clip();
    let mut intents = Vec::new();
    let mut area = Area::with_gap(rect, ui.theme().sizes.spacing.small);
    ui.scope(("file header", &file.path), |ui| {
        let (icon, said) = if file.is_folded {
            (Icon::ChevronRight, words::UNFOLD)
        } else {
            (Icon::ChevronDown, words::FOLD)
        };
        if fold_arrow(ui, &mut area, ("fold file", icon, said)) {
            intents.push(Intent::Review(ReviewIntent::FoldFile(
                file.path.clone(),
                !file.is_folded,
            )));
        }
        let button_width = ui.button_size(&ui::Button::secondary(words::ACCEPT_FILE)).0;
        let accept_cell = area.right(button_width);
        let file_state = FileState {
            file,
            is_running: state.is_running,
        };
        intents.extend(ui.clip(accept_cell, |ui| accept_file_view(&file_state, ui)));
        let path_cell = area.rest();
        let height = ui.theme().sizes.row_height;
        let _ = ui.label(
            place(path_cell, (path_cell.width, height), (Align::Start, Align::Center)),
            &Label::new(&file.path).kind(TextKind::Strong),
        );
    });
    intents
}
