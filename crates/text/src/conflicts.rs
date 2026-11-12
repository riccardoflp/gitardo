//! The conflicts git marks in a file: where they are (`conflicts.parse-markers`) and the key that finds the same
//! conflict again wherever it moves (`review.fingerprint`).

mod fingerprint;
mod markers;
mod sha1;

pub use fingerprint::{Fingerprint, fingerprint};
pub use markers::{Line, MarkedConflict, parse_markers, split_lines};
