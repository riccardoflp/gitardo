//! Rectangles with rounded corners and borders, drawn fast: the rows between the corners are filled as plain
//! rectangles, and only the rows of the corners go through the rasterizer.

use super::{Canvas, Fill};
use crate::color::Color;
use crate::flatten::flatten;
use crate::geometry::{PixelRect, Rect};
use crate::number;
use crate::path::Path;
use crate::raster::FillRule;

/// A border inside a rectangle: its width and color, and the radius of its corners, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Border {
    /// The width of the line, inside the rectangle.
    pub width: f32,
    /// The radius of the outer corners; zero for square corners.
    pub radius: f32,
    /// The color of the line.
    pub color: Color,
}

impl Canvas<'_> {
    /// Fills a rectangle with its corners rounded to a radius, its sides snapped to whole physical pixels and its
    /// corners anti-aliased.
    pub fn fill_rounded_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        let snapped = self.snap(rect);
        let radius = self.physical_radius(snapped, radius);
        if radius < 0.5 {
            self.frame.fill_rect(snapped.intersection(self.physical_clip()), color);
            return;
        }
        let path = Path::rounded_rect(pixel_rect_as_rect(snapped), radius);
        let band = number::ceil(radius);
        self.fill_banded(
            &path,
            snapped,
            band,
            Fill {
                color,
                rule: FillRule::NonZero,
            },
        );
        let middle = PixelRect::new(snapped.left, snapped.top + band, snapped.right, snapped.bottom - band);
        self.frame.fill_rect(middle.intersection(self.physical_clip()), color);
    }

    /// Draws a border inside a rectangle: a line of a width along its sides, inside them, with the outer corners
    /// rounded to the radius of the border and the inner ones following them.
    pub fn stroke_border(&mut self, rect: Rect, border: &Border) {
        let snapped = self.snap(rect);
        let width = number::round(border.width * self.scale()).max(1);
        if 2 * width >= number::coordinate(snapped.width().min(snapped.height())) {
            self.fill_rounded_rect(rect, border.radius, border.color);
            return;
        }
        let radius = self.physical_radius(snapped, border.radius);
        let band = if radius < 0.5 {
            width
        } else {
            number::ceil(radius).max(width)
        };
        let inner = PixelRect::new(
            snapped.left + width,
            snapped.top + width,
            snapped.right - width,
            snapped.bottom - width,
        );
        if radius < 0.5 {
            self.fill_frame_rows(snapped, inner, border.color);
        } else {
            let mut path = Path::rounded_rect(pixel_rect_as_rect(snapped), radius);
            let inner_radius = (radius - number::float(width)).max(0.0);
            let _ = path.extend(&Path::rounded_rect(pixel_rect_as_rect(inner), inner_radius));
            self.fill_banded(
                &path,
                snapped,
                band,
                Fill {
                    color: border.color,
                    rule: FillRule::EvenOdd,
                },
            );
        }
        for column in [
            PixelRect::new(snapped.left, snapped.top + band, inner.left, snapped.bottom - band),
            PixelRect::new(inner.right, snapped.top + band, snapped.right, snapped.bottom - band),
        ] {
            self.frame
                .fill_rect(column.intersection(self.physical_clip()), border.color);
        }
    }

    /// The radius of the corners of a snapped rectangle in physical pixels, no more than half its smaller side.
    fn physical_radius(&self, snapped: PixelRect, radius: f32) -> f32 {
        let smaller = number::float_of_size(snapped.width().min(snapped.height()));
        (radius * self.scale()).min(smaller * 0.5).max(0.0)
    }

    /// Fills a path over a rectangle only in the rows of its top and bottom bands, where its corners are.
    fn fill_banded(&mut self, path: &Path, rect: PixelRect, band: i32, fill: Fill) {
        let contours = flatten(path);
        let top = PixelRect::new(rect.left, rect.top, rect.right, (rect.top + band).min(rect.bottom));
        let bottom = PixelRect::new(rect.left, (rect.bottom - band).max(top.bottom), rect.right, rect.bottom);
        self.fill_polylines_within(&contours, top, fill);
        self.fill_polylines_within(&contours, bottom, fill);
    }

    /// Fills the rows above and below an inner rectangle, inside an outer one: the top and bottom of a square border.
    fn fill_frame_rows(&mut self, outer: PixelRect, inner: PixelRect, color: Color) {
        let clip = self.physical_clip();
        let top = PixelRect::new(outer.left, outer.top, outer.right, inner.top);
        let bottom = PixelRect::new(outer.left, inner.bottom, outer.right, outer.bottom);
        self.frame.fill_rect(top.intersection(clip), color);
        self.frame.fill_rect(bottom.intersection(clip), color);
    }
}

/// A rectangle of whole pixels as a floating-point one.
fn pixel_rect_as_rect(rect: PixelRect) -> Rect {
    Rect::new(
        number::float(rect.left),
        number::float(rect.top),
        number::float_of_size(rect.width()),
        number::float_of_size(rect.height()),
    )
}
