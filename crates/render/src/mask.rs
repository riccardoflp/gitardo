//! Coverage masks: how much of each pixel a shape covers, rasterized with anti-aliasing by `tiny-skia`, which the
//! canvas then lays over the frame in the color of the shape.
//!
//! Only this module and the text module speak to the libraries of drawing; the rest of the crate sees Gitardo's own
//! [`Path`], [`FillRule`] and [`LineCap`].

use crate::frame::MaskView;
use crate::geometry::PixelRect;
use crate::number;
use crate::path::{Command, FillRule, LineCap, Path};

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

/// A shape ready to be rasterized: closed contours in physical pixels, and the rule that says which parts are inside.
#[derive(Clone, Debug)]
pub(crate) struct Outline {
    /// The contours.
    path: tiny_skia::Path,
    /// Which parts of the contours are inside.
    rule: tiny_skia::FillRule,
}

impl Outline {
    /// The shape a path fills by a rule; nothing for a path that has no area, such as a path with one point.
    pub(crate) fn fill(path: &Path, rule: FillRule) -> Option<Outline> {
        let rule = match rule {
            FillRule::NonZero => tiny_skia::FillRule::Winding,
            FillRule::EvenOdd => tiny_skia::FillRule::EvenOdd,
        };
        Some(Outline {
            path: to_library_path(path)?,
            rule,
        })
    }

    /// The shape a line along a path covers: everything within half a width of it, round where two of its lines meet
    /// and ending as the cap says.
    pub(crate) fn stroke(path: &Path, width: f32, cap: LineCap) -> Option<Outline> {
        let stroke = tiny_skia::Stroke {
            width,
            line_cap: match cap {
                LineCap::Butt => tiny_skia::LineCap::Butt,
                LineCap::Round => tiny_skia::LineCap::Round,
                LineCap::Square => tiny_skia::LineCap::Square,
            },
            line_join: tiny_skia::LineJoin::Round,
            ..tiny_skia::Stroke::default()
        };
        // The path is already in physical pixels, so the stroke needs no more precision than one pixel gives.
        let outline = to_library_path(path)?.stroke(&stroke, 1.0)?;
        Some(Outline {
            path: outline,
            rule: tiny_skia::FillRule::Winding,
        })
    }

    /// The smallest rectangle of whole pixels that holds the shape.
    pub(crate) fn bounds(&self) -> PixelRect {
        let bounds = self.path.bounds();
        PixelRect::new(
            number::floor(bounds.left()),
            number::floor(bounds.top()),
            number::ceil(bounds.right()),
            number::ceil(bounds.bottom()),
        )
    }
}

/// The coverage of shapes over a rectangle of pixels, anti-aliased; the parts of the shapes outside the rectangle are
/// left out, and where shapes overlap their coverage adds up as two layers of paint would. Nothing for an empty
/// rectangle.
pub(crate) fn rasterize(outlines: &[Outline], rect: PixelRect) -> Option<Mask> {
    let mut mask = tiny_skia::Mask::new(rect.width(), rect.height())?;
    let to_rect = tiny_skia::Transform::from_translate(-number::float(rect.left), -number::float(rect.top));
    for outline in outlines {
        mask.fill_path(&outline.path, outline.rule, true, to_rect);
    }
    Some(Mask {
        rect,
        coverage: mask.take(),
    })
}

/// A path of Gitardo as a path of the library; nothing for a path with no line or curve.
fn to_library_path(path: &Path) -> Option<tiny_skia::Path> {
    let mut builder = tiny_skia::PathBuilder::with_capacity(path.commands().len(), path.commands().len() * 3);
    for command in path.commands() {
        match *command {
            Command::MoveTo(point) => builder.move_to(point.x, point.y),
            Command::LineTo(point) => builder.line_to(point.x, point.y),
            Command::QuadraticTo(control, point) => builder.quad_to(control.x, control.y, point.x, point.y),
            Command::CubicTo(first, second, point) => {
                builder.cubic_to(first.x, first.y, second.x, second.y, point.x, point.y);
            }
            Command::Close => builder.close(),
        }
    }
    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Point, Rect};

    #[test]
    fn a_square_on_whole_pixels_covers_them_fully_and_nothing_else() {
        let outline = Outline::fill(&Path::rect(Rect::new(1.0, 1.0, 2.0, 2.0)), FillRule::NonZero).unwrap();
        assert_eq!(outline.bounds(), PixelRect::new(1, 1, 3, 3));
        let mask = rasterize(&[outline], PixelRect::new(0, 0, 4, 4)).unwrap();
        let expected = [0, 0, 0, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 0, 0, 0];
        assert_eq!(mask.coverage, expected);
    }

    #[test]
    fn even_odd_cuts_a_hole_where_non_zero_does_not() {
        let mut path = Path::rect(Rect::new(0.0, 0.0, 6.0, 6.0));
        let _ = path.extend(&Path::rect(Rect::new(2.0, 2.0, 2.0, 2.0)));
        let middle = |rule| {
            let outline = Outline::fill(&path, rule).unwrap();
            let mask = rasterize(&[outline], PixelRect::new(0, 0, 6, 6)).unwrap();
            mask.coverage[3 * 6 + 3]
        };
        assert_eq!(middle(FillRule::NonZero), 255);
        assert_eq!(middle(FillRule::EvenOdd), 0);
    }

    #[test]
    fn a_stroke_covers_half_its_width_on_each_side_and_a_lone_point_draws_nothing() {
        let mut line = Path::new();
        let _ = line.move_to(Point::new(0.0, 2.0)).line_to(Point::new(8.0, 2.0));
        let outline = Outline::stroke(&line, 2.0, LineCap::Butt).unwrap();
        assert_eq!(outline.bounds(), PixelRect::new(0, 1, 8, 3));
        let mut point = Path::new();
        let _ = point.move_to(Point::new(1.0, 1.0));
        assert!(Outline::fill(&point, FillRule::NonZero).is_none());
        assert!(rasterize(&[], PixelRect::new(0, 0, 0, 3)).is_none());
    }
}
