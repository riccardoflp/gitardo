//! Conversions between counts of rows or characters and the floating-point lengths of layout, written once so that
//! the rest of the crate never casts.

// Every function here is a cast with its behaviour at the edges written down: a float out of range saturates (Rust's
// `as` from a float does), NaN becomes zero, and the counts laid out are far below 2^53, where `f64` stops being exact.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "these functions are the casts of the crate, each with its behaviour at the edges stated"
)]

/// A count as an `f32`, for small counts such as pieces of a layout or characters of a line.
pub(crate) fn float_of_count(count: usize) -> f32 {
    count as f32
}

/// A count as an `f64`, for the rows of long lists, whose offsets go past where `f32` is exact.
pub(crate) fn wide_of_count(count: usize) -> f64 {
    count as f64
}

/// The whole number of times a length goes into a position, zero when the position is negative or not a number.
pub(crate) fn index_at(position: f64, length: f64) -> usize {
    if length <= 0.0 {
        return 0;
    }
    (position / length).floor().max(0.0) as usize
}

/// A wide length, small enough to be laid out, as an `f32`.
pub(crate) fn narrow(value: f64) -> f32 {
    value as f32
}

/// A duration in milliseconds as a whole number of steps of a period, for animations.
pub(crate) fn steps(milliseconds: u128, period: u128) -> u128 {
    milliseconds.checked_div(period).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_become_indices_at_the_edges_as_said() {
        assert_eq!(index_at(-3.0, 22.0), 0);
        assert_eq!(index_at(44.0, 22.0), 2);
        assert_eq!(index_at(f64::NAN, 22.0), 0);
        assert_eq!(index_at(10.0, 0.0), 0);
        assert_eq!(index_at(21_999_978.0, 22.0), 999_999);
        assert_eq!(steps(1000, 0), 0);
    }
}
