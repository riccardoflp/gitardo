//! Conversions between the floating-point coordinates of drawing and the whole numbers of pixels and font files.
//!
//! Drawing works in `f32` and pixels are counted in integers, so the same few conversions happen everywhere. They are
//! written once here, each saying what it does at the edges, so that the rest of the crate never casts.

// Every function here is a cast with its behaviour at the edges written down: a float that is out of range saturates
// (Rust's `as` from a float does), NaN becomes zero, and the integers drawing uses are far below 2^24, where `f32`
// stops being exact.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "these functions are the casts of the crate, each with its behaviour at the edges stated"
)]

/// The largest integer below or equal to a coordinate, saturating at the limits of `i32`.
pub(crate) fn floor(value: f32) -> i32 {
    value.floor() as i32
}

/// The smallest integer above or equal to a coordinate, saturating at the limits of `i32`.
pub(crate) fn ceil(value: f32) -> i32 {
    value.ceil() as i32
}

/// The nearest integer to a coordinate, halves away from zero, saturating at the limits of `i32`.
pub(crate) fn round(value: f32) -> i32 {
    value.round() as i32
}

/// A pixel coordinate as a float.
pub(crate) fn float(value: i32) -> f32 {
    value as f32
}

/// An unsigned size as a float.
pub(crate) fn float_of_size(value: u32) -> f32 {
    value as f32
}

/// A coordinate that cannot be negative as an index, zero when it is.
pub(crate) fn index(value: i32) -> usize {
    usize::try_from(value).unwrap_or(0)
}

/// A size in pixels as an index.
pub(crate) fn index_of_size(value: u32) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

/// A size in pixels as a coordinate, saturating at the largest `i32`.
pub(crate) fn coordinate(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

/// A coordinate that cannot be negative as a size, zero when it is.
pub(crate) fn size(value: i32) -> u32 {
    u32::try_from(value).unwrap_or(0)
}

/// A coordinate of a library that works in `f64` as one of drawing, to the nearest `f32`.
pub(crate) fn narrowed(value: f64) -> f32 {
    value as f32
}

/// A float as an unsigned key for a cache, rounded, zero when negative.
pub(crate) fn key(value: f32) -> u32 {
    value.round().max(0.0) as u32
}

/// A position in an image as the whole pixel it is in, zero when it is negative, saturating at the largest `u32`.
pub(crate) fn whole(value: f64) -> u32 {
    value.floor().max(0.0) as u32
}

/// A channel of a color from 0 to 255 as a byte, rounded, saturating at 0 and 255.
pub(crate) fn channel(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

/// A large count, such as of pixels, as a float.
pub(crate) fn float_of_large(value: u64) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_become_pixels_at_the_edges_as_said() {
        assert_eq!((floor(-0.5), ceil(-0.5), round(-0.5)), (-1, 0, -1));
        assert_eq!((floor(f32::NAN), round(f32::INFINITY)), (0, i32::MAX));
        assert_eq!(
            (index(-3), index(7), size(-1), coordinate(u32::MAX)),
            (0, 7, 0, i32::MAX)
        );
        assert_eq!((key(-4.0), key(12.6)), (0, 13));
    }
}
