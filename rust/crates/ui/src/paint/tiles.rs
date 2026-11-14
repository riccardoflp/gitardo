//! Which parts of the window changed since the last frame: the window is cut into square tiles, each tile keeps a
//! hash of every shape drawn over it in order, and the tiles whose hash differs from the last frame's are drawn again.
//!
//! A hash of what was drawn says that something changed even when the widget that drew it does not know, such as a
//! row that scrolled under the pointer, and says nothing changed when a view was declared again with the same state.

use render::Rect;

use crate::number;

/// The side of a tile, in logical pixels: small enough that a blinking caret costs little, large enough that a frame
/// has a few hundred tiles to compare.
pub(crate) const TILE: f32 = 64.0;

/// The hash of a tile over which nothing was drawn.
const EMPTY: u64 = 0x9E37_79B9_7F4A_7C15;

/// The hashes of the tiles of a window of a size.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Tiles {
    /// How many tiles across.
    columns: usize,
    /// How many tiles down.
    rows: usize,
    /// The hash of each tile, row after row.
    hashes: Vec<u64>,
}

impl Tiles {
    /// The tiles of a window of a size in logical pixels, nothing drawn over them yet.
    pub(crate) fn new(width: f32, height: f32) -> Tiles {
        let columns = tile_count(width);
        let rows = tile_count(height);
        Tiles {
            columns,
            rows,
            hashes: vec![EMPTY; columns * rows],
        }
    }

    /// Adds a shape drawn over a rectangle, by its hash, to every tile it touches.
    pub(crate) fn add(&mut self, bounds: Rect, hash: u64) {
        let (first_column, end_column) = span(bounds.x, bounds.right(), self.columns);
        let (first_row, end_row) = span(bounds.y, bounds.bottom(), self.rows);
        for row in first_row..end_row {
            let start = row * self.columns;
            let Some(tiles) = self.hashes.get_mut(start + first_column..start + end_column) else {
                continue;
            };
            for tile in tiles {
                *tile = (tile.rotate_left(7) ^ hash).wrapping_mul(0x0000_0100_0000_01B3);
            }
        }
    }

    /// The rectangles to draw again, in logical pixels: every tile whose hash differs from the one of the frame
    /// before, merged into runs across and then into blocks down. All of the window when the frame before had another
    /// size or there was none.
    pub(crate) fn changed_since(&self, before: Option<&Tiles>) -> Vec<Rect> {
        let whole = Rect::new(0.0, 0.0, tile_length(self.columns), tile_length(self.rows));
        let Some(before) = before.filter(|before| before.columns == self.columns && before.rows == self.rows) else {
            return vec![whole];
        };
        let mut runs: Vec<(usize, usize, usize, usize)> = Vec::new();
        for row in 0..self.rows {
            let mut column = 0;
            while column < self.columns {
                if self.differs(before, row, column) {
                    let start = column;
                    while column < self.columns && self.differs(before, row, column) {
                        column += 1;
                    }
                    merge_run(&mut runs, (start, column, row));
                } else {
                    column += 1;
                }
            }
        }
        runs.into_iter()
            .map(|(start, end, first_row, end_row)| {
                Rect::new(
                    tile_length(start),
                    tile_length(first_row),
                    tile_length(end - start),
                    tile_length(end_row - first_row),
                )
            })
            .collect()
    }

    /// Whether a tile has another hash than in the frame before.
    fn differs(&self, before: &Tiles, row: usize, column: usize) -> bool {
        let index = row * self.columns + column;
        self.hashes.get(index) != before.hashes.get(index)
    }
}

/// Adds a run of tiles of a row to the blocks found so far: it extends down a block of the row above with the same
/// columns, or starts a block of its own.
fn merge_run(blocks: &mut Vec<(usize, usize, usize, usize)>, (start, end, row): (usize, usize, usize)) {
    let above = blocks
        .iter_mut()
        .find(|(block_start, block_end, _, end_row)| *block_start == start && *block_end == end && *end_row == row);
    match above {
        Some(block) => block.3 = row + 1,
        None => blocks.push((start, end, row, row + 1)),
    }
}

/// How many tiles cover a length.
fn tile_count(length: f32) -> usize {
    let tile = f64::from(TILE);
    let whole = number::index_at(f64::from(length.max(0.0)), tile);
    if number::wide_of_count(whole) * tile < f64::from(length) {
        whole + 1
    } else {
        whole
    }
}

/// The first tile a span touches and the one after the last, within a count of tiles.
fn span(start: f32, end: f32, count: usize) -> (usize, usize) {
    let tile = f64::from(TILE);
    let first = number::index_at(f64::from(start), tile).min(count);
    let last = if end <= 0.0 {
        0
    } else {
        number::index_at(f64::from(end).next_down(), tile) + 1
    };
    (first, last.min(count).max(first))
}

/// The length of a number of tiles.
fn tile_length(count: usize) -> f32 {
    number::float_of_count(count) * TILE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_tiles_under_a_changed_shape_are_drawn_again() {
        let mut before = Tiles::new(256.0, 128.0);
        before.add(Rect::new(0.0, 0.0, 256.0, 128.0), 1);
        before.add(Rect::new(70.0, 10.0, 20.0, 10.0), 2);
        let mut after = Tiles::new(256.0, 128.0);
        after.add(Rect::new(0.0, 0.0, 256.0, 128.0), 1);
        after.add(Rect::new(70.0, 10.0, 20.0, 10.0), 3);
        assert_eq!(
            after.changed_since(Some(&before)),
            vec![Rect::new(64.0, 0.0, 64.0, 64.0)]
        );
        assert_eq!(after.changed_since(Some(&after.clone())), Vec::<Rect>::new());
        assert_eq!(after.changed_since(None), vec![Rect::new(0.0, 0.0, 256.0, 128.0)]);
    }

    #[test]
    fn runs_of_the_same_columns_merge_down() {
        let before = Tiles::new(256.0, 256.0);
        let mut after = before.clone();
        after.add(Rect::new(64.0, 0.0, 128.0, 192.0), 5);
        after.add(Rect::new(0.0, 200.0, 10.0, 10.0), 5);
        assert_eq!(
            after.changed_since(Some(&before)),
            vec![Rect::new(64.0, 0.0, 128.0, 192.0), Rect::new(0.0, 192.0, 64.0, 64.0)]
        );
    }

    #[test]
    fn a_shape_on_the_edge_of_a_tile_touches_only_that_tile() {
        assert_eq!(span(0.0, 64.0, 4), (0, 1));
        assert_eq!(span(-10.0, 0.0, 4), (0, 0));
        assert_eq!(span(100.0, 1000.0, 4), (1, 4));
        assert_eq!(tile_count(65.0), 2);
        assert_eq!(tile_count(64.0), 1);
    }
}
