//! JPEG, GIF, WebP, BMP and ICO read by the library `image` (`file-view.read-image`): the size first, so that an image
//! too large is not decoded; then the first frame of an animation, or the largest image of an ICO, as pixels.

use std::io::Cursor;

use image::{ImageDecoder, ImageFormat as Format, ImageReader, Limits};

use super::read::{ImageFormat, NotRead};
use super::{Image, MOST_PIXELS};

/// The format of the library for one of ours; none for the formats it does not read here.
fn format_of(format: ImageFormat) -> Option<Format> {
    match format {
        ImageFormat::Jpeg => Some(Format::Jpeg),
        ImageFormat::Gif => Some(Format::Gif),
        ImageFormat::WebP => Some(Format::WebP),
        ImageFormat::Bmp => Some(Format::Bmp),
        ImageFormat::Ico => Some(Format::Ico),
        ImageFormat::Png | ImageFormat::Svg => None,
    }
}

/// The pixels of the bytes of an image of one of the five formats, with its width and height.
pub(super) fn read_raster(bytes: &[u8], format: ImageFormat) -> Result<Image, NotRead> {
    let format = format_of(format).ok_or(NotRead::NotImage)?;
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_alloc = Some(MOST_PIXELS * 4 * 2);
    reader.limits(limits);
    let decoder = reader.into_decoder().map_err(|_| NotRead::Unreadable)?;
    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) > MOST_PIXELS {
        return Err(NotRead::TooLarge);
    }
    let picture = image::DynamicImage::from_decoder(decoder).map_err(|_| NotRead::Unreadable)?;
    let pixels = picture.into_rgba8();
    let (width, height) = pixels.dimensions();
    Ok(Image::of_rgba(width, height, pixels.into_raw()))
}
