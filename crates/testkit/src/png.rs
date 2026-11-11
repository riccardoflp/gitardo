//! PNG images for the view snapshots, written and read here with no library: stored deflate blocks to write, a full
//! inflate to read, and an image of the differences when a snapshot does not match its reference.
//!
//! The format is the one of the W3C, "Portable Network Graphics (PNG) Specification (Second Edition)", 2003, with the
//! zlib stream of RFC 1950 and the deflate data of RFC 1951.

mod checksum;
mod compare;
mod filter;
mod image;
mod inflate;
mod read;
mod write;

pub use compare::Difference;
pub use image::Image;
pub use read::{decode, read_file};
pub use write::{encode, write_file};
