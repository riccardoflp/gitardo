//! Writing an image as a PNG file: each row with the filter that suits it, compressed with deflate.

use std::fs;
use std::path::Path;

#[cfg(test)]
use super::checksum::adler32;
use super::checksum::crc32;
use super::deflate::zlib_compressed;
use super::filter::best_filter;
use super::image::{Image, pixel_count};
use crate::error::Error;

/// The eight bytes every PNG file starts with.
pub(crate) const SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

/// The largest width or height a PNG file may have: 2³¹ − 1.
pub(crate) const LARGEST_SIDE: u32 = 0x7FFF_FFFF;

/// The bit depth this writer writes: 8 bits for each channel.
const BIT_DEPTH: u8 = 8;

/// The colour type of red, green, blue and alpha.
pub(crate) const COLOUR_TYPE_RGBA: u8 = 6;

/// The most bytes of compressed data in one IDAT chunk: larger images have several, one after the other.
const LARGEST_IDAT: usize = 1 << 20;

/// The most bytes a stored deflate block holds, as its 16-bit length allows.
#[cfg(test)]
const LARGEST_STORED_BLOCK: usize = 0xFFFF;

/// Writes an image as a PNG file: red, green, blue and alpha at 8 bits, not interlaced, every row with the filter
/// that makes it smallest, compressed with deflate.
///
/// # Errors
///
/// When the width or the height is zero or larger than a PNG file allows, or when the pixels are not four bytes for
/// each pixel of the image.
pub fn encode(image: &Image) -> Result<Vec<u8>, Error> {
    let row_length = row_length(image)?;
    let mut rows = Vec::with_capacity(image.pixels.len() + usize::try_from(image.height).unwrap_or(0));
    let mut previous: &[u8] = &[];
    for row in image.pixels.chunks_exact(row_length) {
        let (filter, filtered) = best_filter(row, previous, 4);
        rows.push(filter.byte());
        rows.extend_from_slice(&filtered);
        previous = row;
    }
    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&image.width.to_be_bytes());
    header.extend_from_slice(&image.height.to_be_bytes());
    // The bit depth, the colour type, then the compression method, the filter method and the interlace method, all 0.
    header.extend_from_slice(&[BIT_DEPTH, COLOUR_TYPE_RGBA, 0, 0, 0]);
    let mut file = SIGNATURE.to_vec();
    append_chunk(&mut file, *b"IHDR", &header);
    for piece in zlib_compressed(&rows).chunks(LARGEST_IDAT) {
        append_chunk(&mut file, *b"IDAT", piece);
    }
    append_chunk(&mut file, *b"IEND", &[]);
    Ok(file)
}

/// Writes an image to a PNG file, making the folders it is in.
///
/// # Errors
///
/// When the image cannot be written as a PNG file, as [`encode`] says, or the file cannot be written.
pub fn write_file(path: impl AsRef<Path>, image: &Image) -> Result<(), Error> {
    let path = path.as_ref();
    let bytes = encode(image)?;
    let failed = |error| Error::File {
        path: path.to_path_buf(),
        error,
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(failed)?;
    }
    fs::write(path, bytes).map_err(failed)
}

/// The number of bytes of each row of an image, once its size and pixels are checked.
fn row_length(image: &Image) -> Result<usize, Error> {
    let (width, height) = (image.width, image.height);
    if width == 0 || height == 0 || width > LARGEST_SIDE || height > LARGEST_SIDE {
        return Err(Error::InvalidSize { width, height });
    }
    let expected = pixel_count(width, height).and_then(|count| count.checked_mul(4));
    let row_length = usize::try_from(width).ok().and_then(|width| width.checked_mul(4));
    let (Some(expected), Some(row_length)) = (expected, row_length) else {
        return Err(Error::InvalidSize { width, height });
    };
    if image.pixels.len() != expected {
        return Err(Error::PixelCount {
            expected,
            actual: image.pixels.len(),
        });
    }
    Ok(row_length)
}

/// Appends a chunk to a PNG file: the length of its data, its type, its data, and the CRC-32 of its type and data.
///
/// # Panics
///
/// When the data is longer than a chunk may be, 2³¹ − 1 bytes: the chunks of this writer are at most 1 MiB.
pub(crate) fn append_chunk(file: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    let length = u32::try_from(data.len()).ok().filter(|length| *length <= LARGEST_SIDE);
    let length = length.expect("the data of a chunk is shorter than 2^31 bytes");
    file.extend_from_slice(&length.to_be_bytes());
    file.extend_from_slice(&kind);
    file.extend_from_slice(data);
    file.extend_from_slice(&crc32(&[&kind, data]).to_be_bytes());
}

/// Bytes in a zlib stream of stored deflate blocks, which hold the bytes as they are.
///
/// RFC 1950, section 2.2: the header 0x78 0x01 says deflate with a 32 KiB window and the fastest compression, and
/// makes the header a multiple of 31; the Adler-32 of the bytes follows the data. RFC 1951, section 3.2.4: each
/// stored block is its header bits, padding to the byte, its length and the complement of its length. The tests of
/// the reader use it.
#[cfg(test)]
pub(crate) fn zlib_stored(bytes: &[u8]) -> Vec<u8> {
    let blocks = bytes.len() / LARGEST_STORED_BLOCK + 1;
    let mut stream = Vec::with_capacity(bytes.len() + blocks * 5 + 6);
    stream.extend_from_slice(&[0x78, 0x01]);
    let mut rest = bytes;
    loop {
        let (block, after) = rest.split_at(rest.len().min(LARGEST_STORED_BLOCK));
        let is_last = after.is_empty();
        // The first bit says whether this is the last block; the two after it, 00, say that it is stored.
        stream.push(u8::from(is_last));
        let length = u16::try_from(block.len()).unwrap_or(u16::MAX);
        stream.extend_from_slice(&length.to_le_bytes());
        stream.extend_from_slice(&(!length).to_le_bytes());
        stream.extend_from_slice(block);
        if is_last {
            break;
        }
        rest = after;
    }
    stream.extend_from_slice(&adler32(bytes).to_be_bytes());
    stream
}
