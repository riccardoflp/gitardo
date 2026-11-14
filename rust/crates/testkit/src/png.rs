//! PNG images for the view snapshots, written and read here with no library: deflate with fixed codes to write, a full
//! inflate to read, an image of the differences when a snapshot does not match its reference, and the check that
//! compares them.
//!
//! The format is the one of the W3C, "Portable Network Graphics (PNG) Specification (Second Edition)", 2003, with the
//! zlib stream of RFC 1950 and the deflate data of RFC 1951.

mod checksum;
mod compare;
mod deflate;
mod filter;
mod image;
mod inflate;
mod read;
mod snapshot;
mod write;

pub use compare::Difference;
pub use image::Image;
pub use read::{decode, read_file};
pub use snapshot::assert_snapshot;
pub use write::{encode, write_file};
