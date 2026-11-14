//! Reading a PNG file: 8-bit RGB or RGBA, not interlaced, every filter type, compressed in any way deflate allows.

use std::fs;
use std::path::Path;

use super::checksum::crc32;
use super::filter::unfilter;
use super::image::Image;
use super::inflate::decompress;
use super::write::{COLOUR_TYPE_RGBA, LARGEST_SIDE, SIGNATURE};
use crate::error::Error;

/// The colour type of red, green and blue, with no alpha.
const COLOUR_TYPE_RGB: u8 = 2;

/// Reads a PNG file of 8-bit red, green and blue, with or without alpha, not interlaced. The image has alpha in any
/// case: 255, opaque, when the file has none.
///
/// Chunks that are not needed to show the image are skipped, after their CRC-32 is checked; so is the suggested
/// palette, `PLTE`.
///
/// # Errors
///
/// When the data is not a PNG file, is damaged or cut short, or uses what this reader does not read: another bit
/// depth or colour type, interlacing, or a chunk it must understand and does not.
pub fn decode(bytes: &[u8]) -> Result<Image, Error> {
    let mut rest = bytes.strip_prefix(&SIGNATURE[..]).ok_or(Error::NotPng)?;
    let first = next_chunk(&mut rest)?;
    if first.kind != *b"IHDR" {
        return Err(Error::MissingChunk { chunk: "IHDR" });
    }
    let header = Header::parse(first.data)?;
    let mut compressed = Vec::new();
    loop {
        let chunk = next_chunk(&mut rest)?;
        match &chunk.kind {
            b"IDAT" => compressed.extend_from_slice(chunk.data),
            b"IEND" => break,
            b"IHDR" => return Err(Error::MissingChunk { chunk: "IHDR" }),
            // The palette is only a suggestion for an image of red, green and blue.
            b"PLTE" => {}
            kind if is_critical(*kind) => {
                return Err(Error::UnknownCriticalChunk {
                    chunk: String::from_utf8_lossy(kind).into_owned(),
                });
            }
            // Any other chunk is ancillary: the image can be shown without it.
            _ => {}
        }
    }
    if compressed.is_empty() {
        return Err(Error::MissingChunk { chunk: "IDAT" });
    }
    if !rest.is_empty() {
        return Err(Error::MissingChunk { chunk: "IEND" });
    }
    header.image(&decompress(&compressed)?)
}

/// Reads an image from a PNG file.
///
/// # Errors
///
/// When the file cannot be read, or its content cannot be read as [`decode`] says.
pub fn read_file(path: impl AsRef<Path>) -> Result<Image, Error> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|error| Error::File {
        path: path.to_path_buf(),
        error,
    })?;
    decode(&bytes)
}

/// A chunk of a PNG file, whose CRC-32 is checked.
#[derive(Debug)]
struct Chunk<'a> {
    kind: [u8; 4],
    data: &'a [u8],
}

/// The next chunk of a PNG file, after which the rest of the file starts.
fn next_chunk<'a>(rest: &mut &'a [u8]) -> Result<Chunk<'a>, Error> {
    let truncated = Error::Truncated { part: "a chunk" };
    let Some((length, after_length)) = rest.split_first_chunk::<4>() else {
        return Err(truncated);
    };
    let Some((kind, after_kind)) = after_length.split_first_chunk::<4>() else {
        return Err(truncated);
    };
    let length = usize::try_from(u32::from_be_bytes(*length)).unwrap_or(usize::MAX);
    let Some((data, after_data)) = after_kind.split_at_checked(length) else {
        return Err(truncated);
    };
    let Some((stored, after)) = after_data.split_first_chunk::<4>() else {
        return Err(truncated);
    };
    let (stored, computed) = (u32::from_be_bytes(*stored), crc32(&[kind, data]));
    if stored != computed {
        let chunk = String::from_utf8_lossy(kind).into_owned();
        return Err(Error::ChunkChecksum {
            chunk,
            stored,
            computed,
        });
    }
    *rest = after;
    Ok(Chunk { kind: *kind, data })
}

/// Whether a chunk must be understood to show the image: the first letter of its type is a capital.
fn is_critical(kind: [u8; 4]) -> bool {
    kind[0].is_ascii_uppercase()
}

/// What the IHDR chunk says of the image, once checked that this reader reads it.
#[derive(Clone, Copy, Debug)]
struct Header {
    width: u32,
    height: u32,
    /// The bytes of each pixel: 3 for red, green and blue, 4 with alpha.
    channels: usize,
}

impl Header {
    /// The header of the data of an IHDR chunk.
    fn parse(data: &[u8]) -> Result<Header, Error> {
        if data.len() != 13 {
            return Err(Error::Truncated { part: "the IHDR chunk" });
        }
        // The length is checked: every field is there.
        let number = |start: usize| data.get(start..start + 4).and_then(|bytes| bytes.try_into().ok());
        let byte = |index: usize| data.get(index).copied().unwrap_or_default();
        let width = number(0).map_or_default(u32::from_be_bytes);
        let height = number(4).map_or_default(u32::from_be_bytes);
        let (bit_depth, colour_type) = (byte(8), byte(9));
        let (compression, filter, interlace) = (byte(10), byte(11), byte(12));
        if width == 0 || height == 0 || width > LARGEST_SIDE || height > LARGEST_SIDE {
            return Err(Error::InvalidSize { width, height });
        }
        let channels = match (bit_depth, colour_type) {
            (8, COLOUR_TYPE_RGB) => 3,
            (8, COLOUR_TYPE_RGBA) => 4,
            (8, _) => {
                return Err(Error::Unsupported {
                    field: "colour type",
                    value: colour_type,
                });
            }
            _ => {
                return Err(Error::Unsupported {
                    field: "bit depth",
                    value: bit_depth,
                });
            }
        };
        let methods = [
            ("compression method", compression),
            ("filter method", filter),
            ("interlace method", interlace),
        ];
        if let Some((field, value)) = methods.into_iter().find(|(_, value)| *value != 0) {
            return Err(Error::Unsupported { field, value });
        }
        Ok(Header {
            width,
            height,
            channels,
        })
    }

    /// The image of the decompressed data, once its rows are unfiltered.
    fn image(self, data: &[u8]) -> Result<Image, Error> {
        let too_large = Error::InvalidSize {
            width: self.width,
            height: self.height,
        };
        let row_length = usize::try_from(self.width)
            .ok()
            .and_then(|width| width.checked_mul(self.channels));
        let row_length = row_length.ok_or(too_large)?;
        let expected = usize::try_from(self.height)
            .ok()
            .and_then(|height| height.checked_mul(row_length + 1));
        let expected = expected.ok_or(Error::InvalidSize {
            width: self.width,
            height: self.height,
        })?;
        if data.len() != expected {
            return Err(Error::ImageDataLength {
                expected,
                actual: data.len(),
            });
        }
        let rows = unfilter(data, row_length, self.channels)?;
        let pixels = if self.channels == 4 {
            rows
        } else {
            rows.as_chunks::<3>()
                .0
                .iter()
                .flat_map(|[red, green, blue]| [*red, *green, *blue, u8::MAX])
                .collect()
        };
        Ok(Image {
            width: self.width,
            height: self.height,
            pixels,
        })
    }
}

#[cfg(test)]
mod tests;
