//! The rasterizer: how much of each pixel a filled outline covers, for anti-aliased edges.
//!
//! Every line of the outline adds, to each pixel row it crosses, the signed height it spans in that row, split
//! between the cell it is in and the cell after it by how much of the cell lies to the right of the line. A running
//! sum along each row then gives the signed area covered in every pixel: lines going down add, lines going up take
//! away, so the sum is the winding of the outline around the pixel, weighed by the area. This is exact for straight
//! lines, since inside one cell a line is a trapezoid's side whose area is its height times the mean distance to the
//! right side of the cell.

use crate::flatten::Polyline;
use crate::frame::MaskView;
use crate::geometry::{PixelRect, Point};
use crate::number;

/// Which parts of an outline that crosses itself are inside.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FillRule {
    /// A point is inside when the outline winds around it at least once in either direction: contours in the same
    /// direction add up, contours in opposite directions cut holes.
    #[default]
    NonZero,
    /// A point is inside when the outline crosses any ray from it an odd number of times: every contour inside
    /// another cuts a hole, whatever its direction.
    EvenOdd,
}

/// How much of each pixel of a rectangle a shape covers: a byte from 0 to 255 for each pixel, row after row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Mask {
    /// The pixels the mask is over.
    pub(crate) rect: PixelRect,
    /// `width × height` bytes of coverage.
    pub(crate) coverage: Vec<u8>,
}

impl Mask {
    /// The mask as a view to lay over a frame.
    pub(crate) fn view(&self) -> MaskView<'_> {
        MaskView {
            rect: self.rect,
            coverage: &self.coverage,
        }
    }

    /// The same mask over pixels moved by a number of columns and rows.
    pub(crate) fn view_at(&self, columns: i32, rows: i32) -> MaskView<'_> {
        MaskView {
            rect: self.rect.translated(columns, rows),
            coverage: &self.coverage,
        }
    }
}

/// The coverage of closed contours over a rectangle of pixels; contours left open are closed with a straight line.
/// Parts of the contours outside the rectangle still count for the pixels inside it: a shape cut by the rectangle is
/// covered as it would be whole.
pub(crate) fn rasterize(contours: &[Polyline], rect: PixelRect, rule: FillRule) -> Mask {
    let width = number::index_of_size(rect.width());
    let height = number::index_of_size(rect.height());
    if width == 0 || height == 0 {
        return Mask {
            rect,
            coverage: Vec::new(),
        };
    }
    // Two more cells than the row has pixels: a line at the right edge gives to the cell after the last one.
    let stride = width + 2;
    let mut cells = vec![0.0_f32; stride * height];
    let origin = Point::new(number::float(rect.left), number::float(rect.top));
    let area = Area {
        cells: &mut cells,
        stride,
        width: number::float_of_count(width),
        height: number::float_of_count(height),
    };
    accumulate_contours(area, contours, origin);
    let mut coverage = vec![0_u8; width * height];
    for (row_cells, row_coverage) in cells.chunks_exact(stride).zip(coverage.chunks_exact_mut(width)) {
        let mut winding = 0.0_f32;
        for (cell, pixel) in row_cells.iter().zip(row_coverage.iter_mut()) {
            winding += cell;
            *pixel = number::unit_to_byte(fold(winding, rule));
        }
    }
    Mask { rect, coverage }
}

/// The share of a pixel covered, from the signed area summed along its row, by the fill rule.
fn fold(winding: f32, rule: FillRule) -> f32 {
    let magnitude = winding.abs();
    match rule {
        FillRule::NonZero => magnitude.min(1.0),
        FillRule::EvenOdd => {
            let within_two = magnitude % 2.0;
            if within_two > 1.0 { 2.0 - within_two } else { within_two }
        }
    }
}

/// The cells a mask is summed from, and the size of the rectangle they cover.
struct Area<'cells> {
    /// One cell for each pixel and two more at the end of each row, row after row.
    cells: &'cells mut [f32],
    /// The cells of a row.
    stride: usize,
    /// The width of the rectangle, in pixels.
    width: f32,
    /// The height of the rectangle, in pixels.
    height: f32,
}

/// Adds every line of every contour to the cells, each contour closed.
fn accumulate_contours(mut area: Area<'_>, contours: &[Polyline], origin: Point) {
    for contour in contours {
        let (Some(first), Some(last)) = (contour.points.first(), contour.points.last()) else {
            continue;
        };
        for pair in contour.points.windows(2) {
            if let [from, to] = pair {
                accumulate_line(&mut area, *from - origin, *to - origin);
            }
        }
        accumulate_line(&mut area, *last - origin, *first - origin);
    }
}

/// Adds one line, in the coordinates of the rectangle, to the cells of each row it crosses.
fn accumulate_line(area: &mut Area<'_>, from: Point, to: Point) {
    if (from.y - to.y).abs() <= f32::EPSILON {
        return;
    }
    let (direction, top, bottom) = if from.y < to.y {
        (1.0, from, to)
    } else {
        (-1.0, to, from)
    };
    let first_y = top.y.max(0.0);
    let last_y = bottom.y.min(area.height);
    if first_y >= last_y {
        return;
    }
    let slope = (bottom.x - top.x) / (bottom.y - top.y);
    let x_at = |y: f32| top.x + (y - top.y) * slope;
    let mut row = number::floor(first_y);
    while number::float(row) < last_y {
        let row_top = first_y.max(number::float(row));
        let row_bottom = last_y.min(number::float(row + 1));
        let start = number::index(row) * area.stride;
        if let Some(cells) = area.cells.get_mut(start..start + area.stride) {
            let span = Span {
                start_x: x_at(row_top),
                end_x: x_at(row_bottom),
                height: (row_bottom - row_top) * direction,
            };
            accumulate_span(cells, span, area.width);
        }
        row += 1;
    }
}

/// The part of a line inside one row of pixels.
#[derive(Clone, Copy)]
struct Span {
    /// Where the line is at the top of its part of the row.
    start_x: f32,
    /// Where the line is at the bottom of its part of the row.
    end_x: f32,
    /// The height of its part of the row, negative for a line going up.
    height: f32,
}

/// Adds the part of a line inside a row to the cells of the row. Left of the rectangle, a line is moved to its left
/// side, where it still covers every pixel to its right; right of it, to its right side, where it covers none.
fn accumulate_span(cells: &mut [f32], span: Span, width: f32) {
    let start = span.start_x.clamp(0.0, width);
    let end = span.end_x.clamp(0.0, width);
    let (left, right) = if start <= end { (start, end) } else { (end, start) };
    let first_column = number::floor(left);
    let last_column = number::floor(right).min(number::floor(width));
    if first_column >= last_column || right - left <= f32::EPSILON {
        add_in_cell(cells, first_column, span.height, f32::midpoint(left, right));
        return;
    }
    let height_per_column = span.height / (right - left);
    let first_right = number::float(first_column + 1);
    add_in_cell(
        cells,
        first_column,
        (first_right - left) * height_per_column,
        f32::midpoint(left, first_right),
    );
    for column in first_column + 1..last_column {
        add_in_cell(cells, column, height_per_column, number::float(column) + 0.5);
    }
    let last_left = number::float(last_column);
    add_in_cell(
        cells,
        last_column,
        (right - last_left) * height_per_column,
        f32::midpoint(last_left, right),
    );
}

/// Adds the height of a piece of line inside one cell: the part of the cell right of the line goes to this cell, and
/// the rest to the next one, so that the running sum covers every pixel after it fully.
fn add_in_cell(cells: &mut [f32], column: i32, height: f32, middle_x: f32) {
    let right_share = middle_x - number::float(column);
    let index = number::index(column);
    if let Some(cell) = cells.get_mut(index) {
        *cell += height * (1.0 - right_share);
    }
    if let Some(cell) = cells.get_mut(index + 1) {
        *cell += height * right_share;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flatten::flatten;
    use crate::geometry::Rect;
    use crate::path::Path;

    fn coverage_of(path: &Path, rect: PixelRect, rule: FillRule) -> Vec<u8> {
        rasterize(&flatten(path), rect, rule).coverage
    }

    #[test]
    fn a_rectangle_on_pixel_edges_covers_its_pixels_fully() {
        let coverage = coverage_of(
            &Path::rect(Rect::new(1.0, 1.0, 2.0, 1.0)),
            PixelRect::new(0, 0, 4, 3),
            FillRule::NonZero,
        );
        assert_eq!(coverage, vec![0, 0, 0, 0, 0, 255, 255, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn half_pixels_are_half_covered() {
        let coverage = coverage_of(
            &Path::rect(Rect::new(0.5, 0.0, 1.0, 1.0)),
            PixelRect::new(0, 0, 3, 1),
            FillRule::NonZero,
        );
        assert_eq!(coverage, vec![128, 128, 0]);
        let mut triangle = Path::new();
        let _ = triangle
            .move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(1.0, 0.0))
            .line_to(Point::new(1.0, 1.0))
            .close();
        assert_eq!(
            coverage_of(&triangle, PixelRect::new(0, 0, 1, 1), FillRule::NonZero),
            vec![128]
        );
    }

    #[test]
    fn a_shape_cut_by_the_rectangle_covers_as_if_whole() {
        let path = Path::rect(Rect::new(-10.0, -10.0, 30.0, 30.0));
        assert_eq!(
            coverage_of(&path, PixelRect::new(0, 0, 2, 2), FillRule::NonZero),
            vec![255; 4]
        );
        let outside = Path::rect(Rect::new(5.0, 0.0, 30.0, 30.0));
        assert_eq!(
            coverage_of(&outside, PixelRect::new(0, 0, 2, 2), FillRule::NonZero),
            vec![0; 4]
        );
    }

    #[test]
    fn rules_differ_on_contours_inside_others() {
        let mut path = Path::rect(Rect::new(0.0, 0.0, 3.0, 1.0));
        let _ = path.extend(&Path::rect(Rect::new(1.0, 0.0, 1.0, 1.0)));
        let rect = PixelRect::new(0, 0, 3, 1);
        assert_eq!(coverage_of(&path, rect, FillRule::NonZero), vec![255, 255, 255]);
        assert_eq!(coverage_of(&path, rect, FillRule::EvenOdd), vec![255, 0, 255]);
    }
}
