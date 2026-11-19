//! The bytes of a file read as an image, whatever its format, for every place that draws one (`file-view.read-image`,
//! D74): the File page, the Diff page, the page of a commit and the preview of Markdown.

use super::svg::{SvgFailure, draw_svg};
use super::{Image, MOST_PIXELS};

/// A format of images Gitardo draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    /// Portable Network Graphics.
    Png,
    /// JPEG, which photos are.
    Jpeg,
    /// Graphics Interchange Format, of which the first frame is drawn.
    Gif,
    /// WebP, of which the first frame is drawn.
    WebP,
    /// The bitmaps of Windows.
    Bmp,
    /// The icons of Windows, of which the largest image is drawn.
    Ico,
    /// Scalable Vector Graphics, which is text.
    Svg,
}

/// An image read from the bytes of a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageRead {
    /// Its pixels; for an SVG, drawn at the scale asked.
    pub image: Image,
    /// Its width, in pixels of the image; for an SVG, in logical pixels.
    pub width: u32,
    /// Its height, in pixels of the image; for an SVG, in logical pixels.
    pub height: u32,
    /// Its format.
    pub format: ImageFormat,
    /// Its weight: the bytes of the file.
    pub weight: u64,
}

/// Why the bytes of a file are not drawn as an image.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NotRead {
    /// The file is not an image: none of the reasons below is given.
    NotImage,
    /// The image has more pixels than an image may have.
    TooLarge,
    /// The bytes are not what the format says.
    Unreadable,
    /// An SVG with an error: what the library says.
    SvgError(String),
}

/// The format of an image file: by its first bytes, or for an SVG, which is text, by its name ending in `.svg` in any
/// case. None for a file that is not an image.
#[must_use]
pub fn image_format(bytes: &[u8], path: &str) -> Option<ImageFormat> {
    let signatures: [(&[u8], ImageFormat); 6] = [
        (PNG_SIGNATURE, ImageFormat::Png),
        (b"\xff\xd8\xff", ImageFormat::Jpeg),
        (b"GIF87a", ImageFormat::Gif),
        (b"GIF89a", ImageFormat::Gif),
        (b"BM", ImageFormat::Bmp),
        (b"\x00\x00\x01\x00", ImageFormat::Ico),
    ];
    if let Some((_, format)) = signatures.iter().find(|(signature, _)| bytes.starts_with(signature)) {
        return Some(*format);
    }
    if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(&b"WEBP"[..]) {
        return Some(ImageFormat::WebP);
    }
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let is_svg = name
        .rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("svg"));
    is_svg.then_some(ImageFormat::Svg)
}

/// The pixels of the bytes of a file, its size, its format and its weight; or why it is not read. PNG is read by the
/// library `tiny-skia` reads it with, and JPEG, GIF, WebP, BMP and ICO by `image`: the first frame of an animation, the
/// largest image of an ICO. An SVG is drawn at the scale of the screen, so that it stays sharp, with the fonts of the
/// program, and nothing outside the file read.
///
/// Feature: file-view.read-image
///
/// # Errors
///
/// [`NotRead::NotImage`] for a file that is not an image, and why an image could not be read.
pub fn read_image(bytes: &[u8], path: &str, scale: f32) -> Result<ImageRead, NotRead> {
    let format = image_format(bytes, path).ok_or(NotRead::NotImage)?;
    let weight = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    match format {
        ImageFormat::Png => {
            let (width, height) = png_size(bytes).ok_or(NotRead::Unreadable)?;
            if u64::from(width) * u64::from(height) > MOST_PIXELS {
                return Err(NotRead::TooLarge);
            }
            let pixmap = tiny_skia::Pixmap::decode_png(bytes).map_err(|_| NotRead::Unreadable)?;
            Ok(ImageRead {
                image: Image::of_pixmap(&pixmap),
                width: pixmap.width(),
                height: pixmap.height(),
                format,
                weight,
            })
        }
        ImageFormat::Jpeg | ImageFormat::Gif | ImageFormat::WebP | ImageFormat::Bmp | ImageFormat::Ico => {
            let image = super::raster::read_raster(bytes, format)?;
            Ok(ImageRead {
                width: image.width(),
                height: image.height(),
                image,
                format,
                weight,
            })
        }
        ImageFormat::Svg => {
            let drawn = draw_svg(bytes, scale).map_err(|failure| match failure {
                SvgFailure::Error(said) => NotRead::SvgError(said),
                SvgFailure::NoSize => NotRead::Unreadable,
            })?;
            Ok(ImageRead {
                image: drawn.image,
                width: drawn.width,
                height: drawn.height,
                format,
                weight,
            })
        }
    }
}

/// The first bytes of every PNG file.
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// The size a PNG says in its header, before its pixels are read, so that one too large is not decoded.
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let number = |range: std::ops::Range<usize>| {
        bytes
            .get(range)
            .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
            .map(u32::from_be_bytes)
    };
    Some((number(16..20)?, number(20..24)?))
}

#[cfg(test)]
mod tests;
