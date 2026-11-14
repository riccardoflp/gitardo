//! Lists made of blocks: the inspector and the page of a commit are lists of rows of one height, so that only what is in
//! sight is drawn, find in page reaches every row and a row can be brought into sight; a block taller than a row, such
//! as the message of a commit, takes the rows its height needs and is drawn once, from its first row, or from the
//! first row in sight when its first row is above it.

use ui::Rect;

/// Where a block is drawn from, for a row being drawn: the top of the block when the row is the first of its block, or
/// the first row drawn in this frame; none otherwise, since the block is drawn already.
pub(crate) fn block_top(row: &ui::Row, block_start: usize, is_first_drawn: bool, row_height: f32) -> Option<Rect> {
    if row.index != block_start && !is_first_drawn {
        return None;
    }
    let above = crate::number::float_of_count(row.index.saturating_sub(block_start)) * row_height;
    Some(Rect::new(
        row.rect.x,
        row.rect.y - above,
        row.rect.width,
        row.rect.height,
    ))
}

/// How many rows of a height a block of a height takes, one at least.
pub(crate) fn rows_for(height: f32, row_height: f32) -> usize {
    crate::number::count_of_float((height / row_height.max(1.0)).ceil()).max(1)
}

#[cfg(test)]
mod tests {
    use super::{block_top, rows_for};
    use ui::Rect;

    /// A row drawn at a height.
    fn row(index: usize, y: f32) -> ui::Row {
        ui::Row {
            index,
            rect: Rect::new(0.0, y, 100.0, 20.0),
            is_selected: false,
            is_hovered: false,
            foreground: ui::Color::TRANSPARENT,
        }
    }

    #[test]
    fn a_block_is_drawn_from_its_first_row_or_from_the_first_row_in_sight() {
        assert_eq!(
            block_top(&row(3, 60.0), 3, false, 20.0),
            Some(Rect::new(0.0, 60.0, 100.0, 20.0))
        );
        assert_eq!(block_top(&row(4, 80.0), 3, false, 20.0), None);
        assert_eq!(
            block_top(&row(5, 0.0), 3, true, 20.0),
            Some(Rect::new(0.0, -40.0, 100.0, 20.0))
        );
        assert_eq!(rows_for(41.0, 20.0), 3);
        assert_eq!(rows_for(0.0, 20.0), 1);
    }
}
