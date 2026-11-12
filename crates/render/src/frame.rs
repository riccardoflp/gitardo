//! The frame buffer: the pixels of a window, drawn by the program and shown by the platform.

use crate::color::{Color, blend, coverage_alpha};
use crate::dirty::DirtyRegions;
use crate::geometry::PixelRect;
use crate::number;

/// The pixels of a window, row after row from the top, each `0x00RRGGBB`: what a 32-bit bitmap of Windows shows. It
/// remembers the regions drawn since they were last taken, so that the platform sends only those to the screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameBuffer {
    /// The width in pixels.
    width: u32,
    /// The height in pixels.
    height: u32,
    /// `width × height` pixels.
    pixels: Vec<u32>,
    /// What was drawn since the regions were last taken.
    dirty: DirtyRegions,
}

impl FrameBuffer {
    /// A frame of a size, black, all of it to be shown.
    #[must_use]
    pub fn new(width: u32, height: u32) -> FrameBuffer {
        let mut frame = FrameBuffer {
            width,
            height,
            pixels: vec![0; pixel_count(width, height)],
            dirty: DirtyRegions::default(),
        };
        frame.dirty.add(frame.bounds());
        frame
    }

    /// The width in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The height in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The whole frame, as a rectangle of pixels.
    #[must_use]
    pub fn bounds(&self) -> PixelRect {
        PixelRect::of_size(self.width, self.height)
    }

    /// The pixels, row after row from the top, each `0x00RRGGBB`.
    #[must_use]
    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    /// The color of the pixel at a column and a row, or nothing outside the frame.
    #[must_use]
    pub fn pixel(&self, column: i32, row: i32) -> Option<Color> {
        let index = self.index(column, row)?;
        self.pixels.get(index).copied().map(Color::from_pixel)
    }

    /// Changes the size of the frame. Its pixels are then black, and all of it is to be shown.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.pixels.clear();
        self.pixels.resize(pixel_count(width, height), 0);
        self.dirty = DirtyRegions::default();
        self.dirty.add(self.bounds());
    }

    /// The regions drawn since the last call, merged and sorted from the top, leaving none: what the platform sends to
    /// the screen.
    pub fn take_dirty_regions(&mut self) -> Vec<PixelRect> {
        self.dirty.take()
    }

    /// Whether anything was drawn since the regions were last taken.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        !self.dirty.is_empty()
    }

    /// Marks a region as drawn, so that it is shown again even when nothing was drawn in it.
    pub fn mark_dirty(&mut self, region: PixelRect) {
        self.dirty.add(region.intersection(self.bounds()));
    }

    /// The pixels as 8-bit red, green, blue and alpha, row after row, the alpha always 255: what a PNG file holds.
    #[must_use]
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.pixels.len() * 4);
        for pixel in &self.pixels {
            let [_, red, green, blue] = pixel.to_be_bytes();
            bytes.extend_from_slice(&[red, green, blue, 255]);
        }
        bytes
    }

    /// Fills a rectangle with a color, laid over what is there when the color is not opaque. The rectangle is cut to
    /// the frame.
    pub(crate) fn fill_rect(&mut self, rect: PixelRect, color: Color) {
        let rect = rect.intersection(self.bounds());
        if rect.is_empty() || color.is_transparent() {
            return;
        }
        self.dirty.add(rect);
        let source = color.pixel();
        let opacity = u32::from(color.alpha);
        for row in rect.top..rect.bottom {
            let Some(span) = self.span_mut(row, rect.left, rect.right) else {
                continue;
            };
            if opacity == 255 {
                span.fill(source);
            } else {
                for pixel in span {
                    *pixel = blend(*pixel, source, opacity);
                }
            }
        }
    }

    /// Lays a color over a rectangle of pixels in the proportions of a mask of coverage, one byte from 0 to 255 for
    /// each pixel of the rectangle, row after row. Only the pixels inside the clip and the frame change.
    pub(crate) fn fill_mask(&mut self, mask: MaskView<'_>, color: Color, clip: PixelRect) {
        let rect = mask.rect.intersection(clip).intersection(self.bounds());
        if rect.is_empty() || color.is_transparent() {
            return;
        }
        self.dirty.add(rect);
        let source = color.pixel();
        let mask_width = number::index_of_size(mask.rect.width());
        let skipped_columns = number::index(rect.left - mask.rect.left);
        for row in rect.top..rect.bottom {
            let start = number::index(row - mask.rect.top) * mask_width + skipped_columns;
            let coverage = mask.coverage.get(start..).unwrap_or_default();
            let Some(span) = self.span_mut(row, rect.left, rect.right) else {
                continue;
            };
            for (pixel, amount) in span.iter_mut().zip(coverage) {
                let opacity = coverage_alpha(color.alpha, *amount);
                if opacity == 255 {
                    *pixel = source;
                } else if opacity > 0 {
                    *pixel = blend(*pixel, source, opacity);
                }
            }
        }
    }

    /// Lays an image in color over a rectangle of pixels, four bytes for each pixel of the rectangle, row after row:
    /// red, green, blue and opacity. Only the pixels inside the clip and the frame change.
    pub(crate) fn fill_colors(&mut self, rect: PixelRect, colors: &[u8], clip: PixelRect) {
        let shown = rect.intersection(clip).intersection(self.bounds());
        if shown.is_empty() {
            return;
        }
        self.dirty.add(shown);
        let image_width = number::index_of_size(rect.width());
        let skipped_columns = number::index(shown.left - rect.left);
        for row in shown.top..shown.bottom {
            let start = (number::index(row - rect.top) * image_width + skipped_columns) * 4;
            let source = colors.get(start..).unwrap_or_default();
            let Some(span) = self.span_mut(row, shown.left, shown.right) else {
                continue;
            };
            let (colors, _) = source.as_chunks::<4>();
            for (pixel, &[red, green, blue, opacity]) in span.iter_mut().zip(colors) {
                if opacity > 0 {
                    *pixel = blend(*pixel, Color::rgb(red, green, blue).pixel(), u32::from(opacity));
                }
            }
        }
    }

    /// The pixels of a row from a column to the one before another, when they are in the frame.
    fn span_mut(&mut self, row: i32, left: i32, right: i32) -> Option<&mut [u32]> {
        let start = self.index(left, row)?;
        let length = number::index(right - left);
        self.pixels.get_mut(start..start + length)
    }

    /// Where a pixel is in the list, when it is in the frame.
    fn index(&self, column: i32, row: i32) -> Option<usize> {
        let (column, row) = (u32::try_from(column).ok()?, u32::try_from(row).ok()?);
        if column >= self.width || row >= self.height {
            return None;
        }
        let index = u64::from(row) * u64::from(self.width) + u64::from(column);
        usize::try_from(index).ok()
    }
}

/// A mask of coverage over a rectangle of pixels: a byte from 0 (not covered) to 255 (covered) for each pixel, row
/// after row.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MaskView<'mask> {
    /// The pixels the mask is over.
    pub(crate) rect: PixelRect,
    /// `width × height` bytes of coverage.
    pub(crate) coverage: &'mask [u8],
}

/// The number of pixels of a frame of a size; zero when it is more than memory can index.
fn pixel_count(width: u32, height: u32) -> usize {
    usize::try_from(u64::from(width) * u64::from(height)).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangles_are_cut_to_the_frame_and_marked_as_drawn() {
        let mut frame = FrameBuffer::new(4, 3);
        let _ = frame.take_dirty_regions();
        frame.fill_rect(PixelRect::new(-2, 1, 2, 9), Color::WHITE);
        assert_eq!(frame.pixel(0, 1), Some(Color::WHITE));
        assert_eq!(frame.pixel(1, 2), Some(Color::WHITE));
        assert_eq!(frame.pixel(2, 1), Some(Color::BLACK));
        assert_eq!(frame.pixel(0, 0), Some(Color::BLACK));
        assert_eq!(frame.take_dirty_regions(), vec![PixelRect::new(0, 1, 2, 3)]);
    }

    #[test]
    fn masks_lay_their_color_in_proportion() {
        let mut frame = FrameBuffer::new(3, 1);
        let coverage = [255, 128, 0];
        let mask = MaskView {
            rect: PixelRect::new(0, 0, 3, 1),
            coverage: &coverage,
        };
        frame.fill_mask(mask, Color::WHITE, PixelRect::new(0, 0, 9, 9));
        assert_eq!(frame.pixels(), &[0x00FF_FFFF, 0x0080_8080, 0]);
        assert_eq!(frame.to_rgba().get(4..8), Some(&[128, 128, 128, 255][..]));
    }

    #[test]
    fn a_new_or_resized_frame_is_all_to_be_shown() {
        let mut frame = FrameBuffer::new(5, 5);
        assert_eq!(frame.take_dirty_regions(), vec![PixelRect::new(0, 0, 5, 5)]);
        frame.resize(7, 2);
        assert_eq!(frame.pixels().len(), 14);
        assert_eq!(frame.take_dirty_regions(), vec![PixelRect::new(0, 0, 7, 2)]);
        assert!(!frame.is_dirty());
    }
}
