//! Conversions between the integers of the system and the floating point numbers of layout, in one place, so that
//! each rounding and each clamp is written once.

/// A count of pixels as a floating point number. Counts above 2^24 lose their last digits, which no screen has.
#[allow(clippy::cast_precision_loss, reason = "no screen has 2^24 pixels on a side")]
pub(crate) fn to_float(value: u32) -> f32 {
    value as f32
}

/// A length in pixels rounded to the nearest whole pixel, 0 for a negative length or one that is not a number.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is rounded and clamped to the range of u32 first"
)]
pub(crate) fn round_to_pixels(value: f32) -> u32 {
    if value.is_nan() || value <= 0.0 {
        0
    } else {
        value.round().min(to_float(u32::MAX)) as u32
    }
}

/// A coordinate rounded to the nearest whole pixel, clamped to the range of the system's coordinates.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the value is rounded and clamped to the range of i32 first; a cast of a float saturates"
)]
pub(crate) fn round_to_coordinate(value: f32) -> i32 {
    if value.is_nan() { 0 } else { value.round() as i32 }
}

/// A count of pixels as an index into a frame; every system Gitardo runs on has pointers of at least 32 bits.
pub(crate) fn to_index(value: u32) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::{round_to_coordinate, round_to_pixels};

    #[test]
    fn rounds_and_clamps() {
        assert_eq!(round_to_pixels(1.5), 2);
        assert_eq!(round_to_pixels(1.49), 1);
        assert_eq!(round_to_pixels(-3.0), 0);
        assert_eq!(round_to_pixels(f32::NAN), 0);
        assert_eq!(round_to_pixels(f32::INFINITY), u32::MAX);
        assert_eq!(round_to_coordinate(-2.5), -3);
        assert_eq!(round_to_coordinate(f32::NAN), 0);
    }
}
