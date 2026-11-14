use super::*;
use crate::png::filter::{Filter, filter_row};
use crate::png::inflate::decompress;
use crate::png::write::{append_chunk, encode, zlib_stored};

/// A PNG file of chunks, each with its CRC-32.
fn file(chunks: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut file = SIGNATURE.to_vec();
    for (kind, data) in chunks {
        append_chunk(&mut file, *kind, data);
    }
    file
}

/// The data of an IHDR chunk.
fn header(size: (u32, u32), bit_depth: u8, colour_type: u8, interlace: u8) -> Vec<u8> {
    let mut data = size.0.to_be_bytes().to_vec();
    data.extend_from_slice(&size.1.to_be_bytes());
    data.extend_from_slice(&[bit_depth, colour_type, 0, 0, interlace]);
    data
}

/// The rows of an image, each filtered with the next of the five filters in turn, compressed.
fn filtered_rows(rows: &[Vec<u8>], bytes_per_pixel: usize) -> Vec<u8> {
    let filters = [Filter::None, Filter::Sub, Filter::Up, Filter::Average, Filter::Paeth];
    let mut data = Vec::new();
    let mut previous: &[u8] = &[];
    for (row, filter) in rows.iter().zip(filters.iter().cycle()) {
        data.push(*filter as u8);
        data.extend(filter_row(*filter, row, previous, bytes_per_pixel));
        previous = row;
    }
    zlib_stored(&data)
}

/// Rows of bytes that change along the row and from row to row, so that every filter has something to predict.
fn gradient_rows(width: usize, height: usize, channels: usize) -> Vec<Vec<u8>> {
    (0..height)
        .map(|row| {
            (0..width * channels)
                .map(|index| (index * 37 + row * 91 + index * row).to_le_bytes()[0])
                .collect()
        })
        .collect()
}

/// A small RGBA image, the one of the decoding tests.
fn small_image() -> Vec<u8> {
    encode(&Image::filled(2, 2, [1, 2, 3, 4])).unwrap()
}

#[test]
fn a_one_pixel_image_is_written_as_the_specification_lays_it_out() {
    let bytes = encode(&Image {
        width: 1,
        height: 1,
        pixels: vec![255, 0, 0, 255],
    })
    .unwrap();
    assert_eq!(bytes[..8], SIGNATURE);
    assert_eq!(bytes[8..16], [0, 0, 0, 13, b'I', b'H', b'D', b'R']);
    assert_eq!(bytes[16..29], [0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0]);
    assert_eq!(
        bytes[bytes.len() - 12..],
        [0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82]
    );
    // The IDAT holds the row compressed: the filter type None, then the pixel.
    let idat_length = usize::try_from(u32::from_be_bytes(bytes[33..37].try_into().unwrap())).unwrap();
    assert_eq!(bytes[37..41], *b"IDAT");
    assert_eq!(decompress(&bytes[41..41 + idat_length]).unwrap(), [0, 255, 0, 0, 255]);
}

#[test]
fn rgba_rows_with_every_filter_type_are_read() {
    let rows = gradient_rows(7, 10, 4);
    let png = file(&[
        (*b"IHDR", header((7, 10), 8, 6, 0)),
        (*b"IDAT", filtered_rows(&rows, 4)),
        (*b"IEND", vec![]),
    ]);
    let image = decode(&png).unwrap();
    assert_eq!((image.width, image.height), (7, 10));
    assert_eq!(image.pixels, rows.concat());
}

#[test]
fn rgb_rows_with_every_filter_type_are_read_as_opaque() {
    let rows = gradient_rows(5, 6, 3);
    let png = file(&[
        (*b"IHDR", header((5, 6), 8, 2, 0)),
        (*b"IDAT", filtered_rows(&rows, 3)),
        (*b"IEND", vec![]),
    ]);
    let image = decode(&png).unwrap();
    let expected: Vec<u8> = rows
        .concat()
        .chunks(3)
        .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
        .collect();
    assert_eq!(image.pixels, expected);
}

#[test]
fn idat_chunks_are_joined_and_other_chunks_skipped() {
    let rows = gradient_rows(3, 4, 4);
    let compressed = filtered_rows(&rows, 4);
    let (first, second) = compressed.split_at(5);
    let png = file(&[
        (*b"IHDR", header((3, 4), 8, 6, 0)),
        (*b"tEXt", b"Comment\0a test".to_vec()),
        (*b"PLTE", vec![0, 0, 0]),
        (*b"IDAT", first.to_vec()),
        (*b"IDAT", vec![]),
        (*b"IDAT", second.to_vec()),
        (*b"IEND", vec![]),
    ]);
    assert_eq!(decode(&png).unwrap().pixels, rows.concat());
}

#[test]
fn a_file_that_is_not_png_or_is_damaged_is_an_error() {
    let good = small_image();
    assert!(matches!(decode(b"GIF89a"), Err(Error::NotPng)));
    assert!(matches!(decode(&good[..good.len() - 1]), Err(Error::Truncated { .. })));
    assert!(matches!(decode(&good[..8]), Err(Error::Truncated { .. })));
    let mut damaged = good.clone();
    damaged[20] ^= 1;
    assert!(matches!(decode(&damaged), Err(Error::ChunkChecksum { .. })));
    let mut trailing = good;
    trailing.push(0);
    assert!(matches!(decode(&trailing), Err(Error::MissingChunk { chunk: "IEND" })));
}

#[test]
fn missing_and_unknown_chunks_are_errors() {
    let compressed = filtered_rows(&gradient_rows(1, 1, 4), 4);
    let no_header = file(&[(*b"IDAT", compressed.clone()), (*b"IEND", vec![])]);
    assert!(matches!(decode(&no_header), Err(Error::MissingChunk { chunk: "IHDR" })));
    let no_data = file(&[(*b"IHDR", header((1, 1), 8, 6, 0)), (*b"IEND", vec![])]);
    assert!(matches!(decode(&no_data), Err(Error::MissingChunk { chunk: "IDAT" })));
    let no_end = file(&[(*b"IHDR", header((1, 1), 8, 6, 0)), (*b"IDAT", compressed.clone())]);
    assert!(matches!(decode(&no_end), Err(Error::Truncated { .. })));
    let unknown = file(&[
        (*b"IHDR", header((1, 1), 8, 6, 0)),
        (*b"ABCD", vec![]),
        (*b"IDAT", compressed),
        (*b"IEND", vec![]),
    ]);
    assert!(matches!(decode(&unknown), Err(Error::UnknownCriticalChunk { .. })));
}

#[test]
fn what_this_reader_does_not_read_is_an_error() {
    let compressed = filtered_rows(&gradient_rows(1, 1, 4), 4);
    let cases = [
        (header((1, 1), 16, 6, 0), "bit depth"),
        (header((1, 1), 8, 3, 0), "colour type"),
        (header((1, 1), 8, 0, 0), "colour type"),
        (header((1, 1), 8, 6, 1), "interlace method"),
    ];
    for (header, expected_field) in cases {
        let png = file(&[(*b"IHDR", header), (*b"IDAT", compressed.clone()), (*b"IEND", vec![])]);
        assert!(matches!(decode(&png), Err(Error::Unsupported { field, .. }) if field == expected_field));
    }
    let empty = file(&[
        (*b"IHDR", header((0, 1), 8, 6, 0)),
        (*b"IDAT", compressed),
        (*b"IEND", vec![]),
    ]);
    assert!(matches!(decode(&empty), Err(Error::InvalidSize { .. })));
}

#[test]
fn image_data_of_the_wrong_length_or_filter_is_an_error() {
    let short = file(&[
        (*b"IHDR", header((2, 2), 8, 6, 0)),
        (*b"IDAT", zlib_stored(&[0; 9])),
        (*b"IEND", vec![]),
    ]);
    assert!(matches!(
        decode(&short),
        Err(Error::ImageDataLength {
            expected: 18,
            actual: 9
        })
    ));
    let mut rows = vec![0; 18];
    rows[9] = 7;
    let filter = file(&[
        (*b"IHDR", header((2, 2), 8, 6, 0)),
        (*b"IDAT", zlib_stored(&rows)),
        (*b"IEND", vec![]),
    ]);
    assert!(matches!(
        decode(&filter),
        Err(Error::UnknownFilter { row: 1, filter: 7 })
    ));
}
