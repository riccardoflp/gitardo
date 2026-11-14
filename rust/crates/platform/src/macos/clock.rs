//! The local time of the computer, as the C library works it out from `TZ` or the time zone the user chose.

use std::mem::MaybeUninit;
use std::time::{SystemTime, UNIX_EPOCH};

use super::libsystem;

/// How far the local time is from UTC now, in seconds; UTC itself when the C library cannot say.
pub(crate) fn utc_offset() -> i64 {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let now: libsystem::time_t = i64::try_from(seconds).unwrap_or(0);
    let mut local = MaybeUninit::<libsystem::tm>::zeroed();
    // SAFETY: `now` and `local` are alive for the call, and `local` has room for one `tm`, which the call fills.
    let filled = unsafe { libsystem::localtime_r(&raw const now, local.as_mut_ptr()) };
    if filled.is_null() {
        return 0;
    }
    // SAFETY: `localtime_r` gave back its result, so it filled `local`; it was zeroed before, so every field is set.
    let local = unsafe { local.assume_init() };
    local.tm_gmtoff
}

#[cfg(test)]
mod tests {
    use super::utc_offset;

    #[test]
    fn the_offset_is_within_a_day_and_whole_minutes() {
        let offset = utc_offset();
        assert!(offset.abs() <= 14 * 3_600, "{offset}");
        assert_eq!(offset % 60, 0);
    }
}
