//! Conversions between counts of rows and the floating-point lengths of layout, for the inspector and the page of a
//! commit, written once so that their code never casts.

// Each function is a cast with its behaviour at the edges written down: a float out of range saturates (Rust's `as`
// from a float does), NaN becomes zero, and the counts laid out are far below 2^24, where `f32` stops being exact.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "these functions are the casts of the module, each with its behaviour at the edges stated"
)]

/// A count as an `f32`, for the rows of a page.
pub(crate) fn float_of_count(count: usize) -> f32 {
    count as f32
}

/// A whole length as a count, zero when it is negative or not a number.
pub(crate) fn count_of_float(value: f32) -> usize {
    value.max(0.0) as usize
}

#[cfg(test)]
mod tests {
    use super::{count_of_float, float_of_count};

    #[test]
    fn counts_and_lengths_become_each_other_at_the_edges_as_said() {
        assert_eq!(count_of_float(-2.0), 0);
        assert_eq!(count_of_float(f32::NAN), 0);
        assert_eq!(count_of_float(3.0), 3);
        assert!((float_of_count(12) - 12.0).abs() < f32::EPSILON);
    }
}
