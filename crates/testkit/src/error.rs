//! What can go wrong in the parts of testkit that report errors rather than fail the test: PNG images and golden files.

use std::fmt;
use std::path::PathBuf;

/// Something that went wrong while writing or reading a PNG image, or while comparing with a golden file.
#[derive(Debug)]
pub enum Error {
    /// A file could not be read or written.
    File {
        /// The file.
        path: PathBuf,
        /// What the system said.
        error: std::io::Error,
    },
    /// The bytes do not start with the eight bytes that every PNG file starts with.
    NotPng,
    /// The data ends before the part it names is complete.
    Truncated {
        /// The part that is cut short, such as "a chunk" or "a stored block".
        part: &'static str,
    },
    /// A chunk whose CRC-32 is not the one of its type and data: the file is damaged.
    ChunkChecksum {
        /// The type of the chunk, such as `IDAT`.
        chunk: String,
        /// The checksum written in the file.
        stored: u32,
        /// The checksum of what the chunk holds.
        computed: u32,
    },
    /// A chunk that every PNG file has is missing or out of its place.
    MissingChunk {
        /// The type of the chunk, such as `IHDR`.
        chunk: &'static str,
    },
    /// A chunk that a reader must understand to show the image, which this reader does not.
    UnknownCriticalChunk {
        /// The type of the chunk.
        chunk: String,
    },
    /// The image uses a part of the PNG format that this reader does not read.
    Unsupported {
        /// The field of the header, such as "colour type" or "bit depth".
        field: &'static str,
        /// Its value in the file.
        value: u8,
    },
    /// The width or the height is zero, or the image is too large to hold.
    InvalidSize {
        /// The width in pixels.
        width: u32,
        /// The height in pixels.
        height: u32,
    },
    /// The pixels of an image are not four bytes for each pixel of its width and height.
    PixelCount {
        /// The number of bytes that the width and height call for.
        expected: usize,
        /// The number of bytes there are.
        actual: usize,
    },
    /// The zlib stream around the compressed data is not valid.
    Zlib {
        /// What is wrong with it.
        reason: &'static str,
    },
    /// The Adler-32 checksum at the end of the zlib stream is not the one of the data: the data is damaged.
    AdlerChecksum {
        /// The checksum written in the stream.
        stored: u32,
        /// The checksum of the data.
        computed: u32,
    },
    /// The compressed data is not valid deflate data.
    Deflate {
        /// What is wrong with it.
        reason: &'static str,
    },
    /// A row of the image starts with a filter type that does not exist.
    UnknownFilter {
        /// The row, from 0 at the top.
        row: usize,
        /// The filter type written before the row.
        filter: u8,
    },
    /// The data of the image, once decompressed, is not as long as its width and height call for.
    ImageDataLength {
        /// The number of bytes that the width and height call for.
        expected: usize,
        /// The number of bytes there are.
        actual: usize,
    },
    /// A golden file that does not exist yet.
    GoldenMissing {
        /// The golden file.
        path: PathBuf,
    },
    /// The actual content differs from the golden file.
    GoldenMismatch {
        /// The golden file.
        path: PathBuf,
        /// The first line that differs, from 1.
        line: usize,
        /// That line in the golden file, or nothing when the file ends before it.
        expected: Option<String>,
        /// That line in the actual content, or nothing when the content ends before it.
        actual: Option<String>,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::File { path, error } => write!(formatter, "{}: {error}", path.display()),
            Error::NotPng => write!(formatter, "the data is not a PNG image"),
            Error::Truncated { part } => write!(formatter, "the data ends in the middle of {part}"),
            Error::ChunkChecksum {
                chunk,
                stored,
                computed,
            } => write!(
                formatter,
                "the {chunk} chunk is damaged: its CRC-32 is {stored:#010x}, its content gives {computed:#010x}"
            ),
            Error::MissingChunk { chunk } => write!(formatter, "the {chunk} chunk is missing or out of place"),
            Error::UnknownCriticalChunk { chunk } => {
                write!(
                    formatter,
                    "the {chunk} chunk is needed to show the image and is not read here"
                )
            }
            Error::Unsupported { field, value } => write!(formatter, "a {field} of {value} is not read here"),
            Error::InvalidSize { width, height } => write!(formatter, "an image of {width}×{height} is not valid"),
            Error::PixelCount { expected, actual } => {
                write!(
                    formatter,
                    "the image has {actual} bytes of pixels where its size calls for {expected}"
                )
            }
            Error::Zlib { reason } => write!(formatter, "the zlib stream is not valid: {reason}"),
            Error::AdlerChecksum { stored, computed } => write!(
                formatter,
                "the compressed data is damaged: its Adler-32 is {stored:#010x}, the data gives {computed:#010x}"
            ),
            Error::Deflate { reason } => write!(formatter, "the compressed data is not valid: {reason}"),
            Error::UnknownFilter { row, filter } => write!(formatter, "row {row} has the unknown filter type {filter}"),
            Error::ImageDataLength { expected, actual } => {
                write!(
                    formatter,
                    "the image data has {actual} bytes where its size calls for {expected}"
                )
            }
            Error::GoldenMissing { path } => write!(
                formatter,
                "the golden file {} does not exist: run the test with GITARDO_GOLDEN_RECORD=1 to record it",
                path.display()
            ),
            Error::GoldenMismatch {
                path,
                line,
                expected,
                actual,
            } => write!(
                formatter,
                "the actual content differs from the golden file {} at line {line}:\n  expected: {}\n  actual:   {}\n\
                 Run the test with GITARDO_GOLDEN_RECORD=1 to record the actual content, after looking at it.",
                path.display(),
                shown(expected.as_deref()),
                shown(actual.as_deref()),
            ),
        }
    }
}

/// A line as a mismatch shows it: quoted with its control characters escaped, so that a carriage return or a tab is
/// seen rather than hidden.
fn shown(line: Option<&str>) -> String {
    match line {
        Some(line) => format!("{line:?}"),
        None => String::from("<end of file>"),
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::File { error, .. } => Some(error),
            // Only a file that could not be read or written has an error of the system beneath it.
            _ => None,
        }
    }
}
