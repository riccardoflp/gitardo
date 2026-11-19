//! Images read from files, drawn scaled into a rectangle: the images of a repository wherever their files are shown
//! (`file-view.read-image`), and the diagrams of Mermaid, drawn from their SVG (`file-view.mermaid`).

mod raster;
mod read;
mod svg;

pub use read::{ImageFormat, ImageRead, NotRead, image_format, read_image};
pub use svg::SvgImage;

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use crate::color::Color;
use crate::frame::FrameBuffer;
use crate::geometry::PixelRect;
use crate::number;

/// The most pixels an image read may have: larger ones are not read, so that one file cannot take the memory.
const MOST_PIXELS: u64 = 64 * 1024 * 1024;

/// The pixels of an image, shared by the copies of it: four bytes for each, red, green, blue and opacity, row after
/// row, not multiplied by the opacity.
#[derive(Clone)]
pub struct Image {
    /// Its width, in pixels.
    width: u32,
    /// Its height, in pixels.
    height: u32,
    /// Its pixels.
    pixels: Arc<[u8]>,
    /// A number that tells it from another image, from its pixels, for the shapes drawn again only when they change.
    key: u64,
}

impl Image {
    /// The image of the bytes of a PNG file; none when they are not one, or the image is too large.
    #[must_use]
    pub fn decode_png(bytes: &[u8]) -> Option<Image> {
        let pixmap = tiny_skia::Pixmap::decode_png(bytes).ok()?;
        if u64::from(pixmap.width()) * u64::from(pixmap.height()) > MOST_PIXELS {
            return None;
        }
        Some(Image::of_pixmap(&pixmap))
    }

    /// The image of the pixels of a pixmap of `tiny-skia`, which are multiplied by their opacity.
    fn of_pixmap(pixmap: &tiny_skia::Pixmap) -> Image {
        let mut pixels = Vec::with_capacity(pixmap.pixels().len() * 4);
        for pixel in pixmap.pixels() {
            let color = pixel.demultiply();
            pixels.extend([color.red(), color.green(), color.blue(), color.alpha()]);
        }
        let mut hasher = DefaultHasher::new();
        pixels.hash(&mut hasher);
        (pixmap.width(), pixmap.height()).hash(&mut hasher);
        Image {
            width: pixmap.width(),
            height: pixmap.height(),
            pixels: pixels.into(),
            key: hasher.finish(),
        }
    }

    /// The image of pixels of four bytes each, red, green, blue and opacity, not multiplied by the opacity.
    fn of_rgba(width: u32, height: u32, pixels: Vec<u8>) -> Image {
        let mut hasher = DefaultHasher::new();
        pixels.hash(&mut hasher);
        (width, height).hash(&mut hasher);
        Image {
            width,
            height,
            pixels: pixels.into(),
            key: hasher.finish(),
        }
    }

    /// An image of one color, for tests.
    #[must_use]
    pub fn filled(width: u32, height: u32, color: Color) -> Image {
        let count = usize::try_from(u64::from(width) * u64::from(height)).unwrap_or(0);
        let Color {
            red,
            green,
            blue,
            alpha: opacity,
        } = color;
        let pixels: Vec<u8> = std::iter::repeat_n([red, green, blue, opacity], count)
            .flatten()
            .collect();
        let mut hasher = DefaultHasher::new();
        (width, height, red, green, blue, opacity).hash(&mut hasher);
        Image {
            width,
            height,
            pixels: pixels.into(),
            key: hasher.finish(),
        }
    }

    /// Its width, in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Its height, in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// A number that tells it from another image.
    #[must_use]
    pub fn key(&self) -> u64 {
        self.key
    }

    /// The color of the pixel of a column and a row, inside the image.
    fn pixel(&self, column: u32, row: u32) -> [u8; 4] {
        let index = (u64::from(row.min(self.height.saturating_sub(1))) * u64::from(self.width)
            + u64::from(column.min(self.width.saturating_sub(1))))
            * 4;
        usize::try_from(index)
            .ok()
            .and_then(|index| self.pixels.get(index..index + 4))
            .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
            .unwrap_or_default()
    }
}

impl PartialEq for Image {
    fn eq(&self, other: &Image) -> bool {
        self.key == other.key && self.width == other.width && self.height == other.height
    }
}

impl Eq for Image {}

impl std::fmt::Debug for Image {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Image")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

impl FrameBuffer {
    /// Lays an image over a rectangle of pixels, scaled to it: each pixel of the rectangle takes the mean of the pixels
    /// of the image under its four quarters. Only the pixels inside the clip and the frame change.
    pub(crate) fn fill_image(&mut self, rect: PixelRect, image: &Image, clip: PixelRect) {
        let shown = rect.intersection(clip).intersection(self.bounds());
        if shown.is_empty() || image.width == 0 || image.height == 0 {
            return;
        }
        let scale_x = f64::from(image.width) / f64::from(rect.width().max(1));
        let scale_y = f64::from(image.height) / f64::from(rect.height().max(1));
        let mut colors =
            Vec::with_capacity(number::index_of_size(shown.width()) * number::index_of_size(shown.height()) * 4);
        for row in shown.top..shown.bottom {
            let top = f64::from(row - rect.top) * scale_y;
            for column in shown.left..shown.right {
                let left = f64::from(column - rect.left) * scale_x;
                colors.extend(sampled(image, (left, top), (scale_x, scale_y)));
            }
        }
        self.fill_colors(shown, &colors, shown);
    }
}

/// The color of the part of an image under a pixel of the frame, from its top left corner in pixels of the image and
/// its size: the mean of the image at its four quarters, weighted by their opacity.
/// A pixel of the frame smaller than a pixel of the image takes the color of the one pixel under its middle, so that
/// an image drawn larger than it is keeps its pixels square and sharp (`file-view.image`).
fn sampled(image: &Image, (left, top): (f64, f64), (width, height): (f64, f64)) -> [u8; 4] {
    if width < 1.0 && height < 1.0 {
        return image.pixel(number::whole(left + width * 0.5), number::whole(top + height * 0.5));
    }
    let mut sums = [0u32; 4];
    for (across, down) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
        let column = number::whole(left + width * across);
        let row = number::whole(top + height * down);
        let [red, green, blue, opacity] = image.pixel(column, row);
        let weight = u32::from(opacity);
        sums[0] += u32::from(red) * weight;
        sums[1] += u32::from(green) * weight;
        sums[2] += u32::from(blue) * weight;
        sums[3] += weight;
    }
    let [red, green, blue, weight] = sums;
    if weight == 0 {
        return [0; 4];
    }
    let channel = |sum: u32| u8::try_from(sum / weight).unwrap_or(u8::MAX);
    [
        channel(red),
        channel(green),
        channel(blue),
        u8::try_from(weight / 4).unwrap_or(u8::MAX),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Assets, Canvas, Rect};
    use testkit::png;

    #[test]
    fn a_png_read_is_drawn_scaled_into_its_rectangle() {
        let mut source = png::Image::filled(4, 2, [255, 0, 0, 255]);
        for row in 0..2 {
            for column in 2..4 {
                source.set_pixel(column, row, [0, 0, 255, 255]);
            }
        }
        let bytes = png::encode(&source).unwrap();
        let image = Image::decode_png(&bytes).unwrap();
        assert_eq!((image.width(), image.height()), (4, 2));
        let mut frame = FrameBuffer::new(20, 10);
        let mut assets = Assets::new();
        let mut canvas = Canvas::new(&mut frame, &mut assets, 1.0);
        canvas.draw_image(&image, Rect::new(2.0, 2.0, 8.0, 4.0));
        assert_eq!(frame.pixel(3, 3), Some(Color::rgb(255, 0, 0)));
        assert_eq!(frame.pixel(8, 5), Some(Color::rgb(0, 0, 255)));
        assert_eq!(frame.pixel(11, 3), Some(Color::BLACK));
    }

    #[test]
    fn bytes_that_are_not_a_png_are_not_read() {
        assert_eq!(Image::decode_png(b"GIF89a"), None);
    }
}
