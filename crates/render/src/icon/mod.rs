//! Icons: Gitardo's own, drawn from paths on a grid of 16 by 16 logical pixels, lines one pixel wide, so that they
//! are sharp at 100 % and 200 %. Each is rasterized once for each size and kept.

mod designs;

use std::collections::HashMap;

use crate::canvas::Canvas;
use crate::color::Color;
use crate::flatten::flatten;
use crate::geometry::{PixelRect, Point, Rect, Transform};
use crate::number;
use crate::path::Path;
use crate::raster::{FillRule, Mask, rasterize};
use crate::stroke::{LineCap, stroke_outline};

/// The side of the grid the icons are designed on, in logical pixels.
pub const GRID: f32 = 16.0;

/// The width of the lines of the icons, on their grid.
const LINE_WIDTH: f32 = 1.0;

/// An icon of Gitardo's own set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Icon {
    /// A chevron pointing right: a folded row, or a menu that opens to the side.
    ChevronRight,
    /// A chevron pointing down: an unfolded row, or a menu that opens below.
    ChevronDown,
    /// A cross: close, or remove.
    Close,
    /// A check mark: done, or chosen.
    Check,
    /// A plus: add, or create.
    Plus,
    /// A minus: remove, or fold.
    Minus,
    /// A magnifying glass: search.
    Search,
    /// A branch leaving a line of commits.
    Branch,
    /// A label with a hole: a tag.
    Tag,
    /// A cloud: a remote.
    Cloud,
    /// A commit: a ring on a line.
    Commit,
    /// A folder.
    Folder,
    /// A file with a folded corner.
    File,
    /// A circular arrow: read again.
    Refresh,
    /// A gear: settings.
    Gear,
    /// Three dots: more actions.
    More,
    /// An arrow pointing up: push, or move up.
    ArrowUp,
    /// An arrow pointing down: pull, or move down.
    ArrowDown,
    /// A triangle with an exclamation mark.
    Warning,
    /// A circle with a cross.
    Error,
    /// A circle with an `i`.
    Info,
    /// Lines growing from one trunk: a worktree.
    Worktree,
    /// A tray with layers above it: a stash.
    Stash,
}

impl Icon {
    /// Every icon, in the order of the enumeration.
    pub const ALL: [Icon; 23] = [
        Icon::ChevronRight,
        Icon::ChevronDown,
        Icon::Close,
        Icon::Check,
        Icon::Plus,
        Icon::Minus,
        Icon::Search,
        Icon::Branch,
        Icon::Tag,
        Icon::Cloud,
        Icon::Commit,
        Icon::Folder,
        Icon::File,
        Icon::Refresh,
        Icon::Gear,
        Icon::More,
        Icon::ArrowUp,
        Icon::ArrowDown,
        Icon::Warning,
        Icon::Error,
        Icon::Info,
        Icon::Worktree,
        Icon::Stash,
    ];
}

/// The paths of an icon on its grid: lines to stroke, and shapes to fill.
#[derive(Clone, Debug, Default)]
pub(crate) struct Design {
    /// The lines, stroked one pixel wide with round ends.
    pub(crate) strokes: Path,
    /// The shapes filled, such as dots.
    pub(crate) fills: Path,
}

/// An icon at a size in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct IconKey {
    /// The icon.
    icon: Icon,
    /// Its side, in 16ths of a physical pixel.
    size: u32,
}

/// The icons rasterized so far, each over pixels counted from its top left corner.
#[derive(Clone, Debug, Default)]
pub struct Icons {
    /// The coverage of each icon at each size.
    masks: HashMap<IconKey, Mask>,
}

impl Icons {
    /// The coverage of an icon at a size in physical pixels, rasterized the first time it is asked for.
    fn mask(&mut self, icon: Icon, size: f32) -> &Mask {
        let key = IconKey {
            icon,
            size: number::key(size * 16.0),
        };
        self.masks.entry(key).or_insert_with(|| rasterize_icon(icon, size))
    }
}

/// The coverage of an icon whose grid is scaled to a side in physical pixels.
fn rasterize_icon(icon: Icon, size: f32) -> Mask {
    let factor = size / GRID;
    let design = designs::design(icon);
    let transform = Transform::scale(factor, factor);
    let mut contours = stroke_outline(
        &flatten(&design.strokes.transformed(transform)),
        LINE_WIDTH * factor,
        LineCap::Round,
    );
    contours.extend(flatten(&design.fills.transformed(transform)));
    let pixels = number::ceil(size);
    rasterize(&contours, PixelRect::new(0, 0, pixels, pixels), FillRule::NonZero)
}

impl Canvas<'_> {
    /// Draws an icon in a color, its top left corner at a point snapped to a whole physical pixel, its side `size`
    /// logical pixels (16 is the size it is designed at).
    pub fn draw_icon(&mut self, icon: Icon, origin: Point, size: f32, color: Color) {
        let corner = self.to_physical_point(origin);
        let (column, row) = (number::round(corner.x), number::round(corner.y));
        let clip = self.physical_clip();
        let physical_size = size * self.scale();
        let bounds = PixelRect::enclosing(Rect::new(
            number::float(column),
            number::float(row),
            physical_size,
            physical_size,
        ));
        if bounds.intersection(clip).is_empty() {
            return;
        }
        let mask = self.assets.icons.mask(icon, physical_size);
        self.frame.fill_mask(mask.view_at(column, row), color, clip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_draws_something_inside_its_square() {
        for icon in Icon::ALL {
            for size in [16.0, 32.0] {
                let mask = rasterize_icon(icon, size);
                let covered = mask.coverage.iter().filter(|amount| **amount > 128).count();
                assert!(covered > 8, "{icon:?} at {size} covers {covered} pixels");
            }
        }
    }
}
