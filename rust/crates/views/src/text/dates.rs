//! Dates as the lists say them: `12 Nov 2026`; as the timeline says them, `5m ago` under a week and `Oct 3, 2026`
//! after (`graph.columns`), with the full date and time in a tooltip.

/// The short names of the months, January first.
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// The seconds in a day.
const DAY: i64 = 86_400;
/// The seconds in a week.
const WEEK: i64 = 7 * DAY;

/// The day of a moment, in seconds since 1970 and in UTC, as `12 Nov 2026`.
#[must_use]
pub fn short_date(seconds: i64) -> String {
    let (year, month, day) = civil_date(seconds.div_euclid(DAY));
    format!("{day} {} {year}", month_name(month))
}

/// How long ago a moment was, as the timeline says it: `now` under a minute, `5m ago`, `3h ago`, `2d ago` under a
/// week, and the day after, `Oct 3, 2026`; a moment after `now`, as from a clock ahead, is `now`.
#[must_use]
pub fn relative_date(seconds: i64, now: i64) -> String {
    let ago = now - seconds;
    match ago {
        ..60 => String::from("now"),
        60..3_600 => format!("{}m ago", ago / 60),
        3_600..DAY => format!("{}h ago", ago / 3_600),
        DAY..WEEK => format!("{}d ago", ago / DAY),
        _ => month_day_year(seconds),
    }
}

/// A moment in full, in UTC, for the tooltip of a date: `Oct 3, 2026, 14:05 UTC`.
#[must_use]
pub fn full_date(seconds: i64) -> String {
    let in_day = seconds.rem_euclid(DAY);
    format!(
        "{}, {:02}:{:02} UTC",
        month_day_year(seconds),
        in_day / 3_600,
        in_day % 3_600 / 60
    )
}

/// The day of a moment as `Oct 3, 2026`.
fn month_day_year(seconds: i64) -> String {
    let (year, month, day) = civil_date(seconds.div_euclid(DAY));
    format!("{} {day}, {year}", month_name(month))
}

/// The short name of a month, 1 for January.
fn month_name(month: i64) -> &'static str {
    usize::try_from(month - 1)
        .ok()
        .and_then(|index| MONTHS.get(index))
        .copied()
        .unwrap_or_default()
}

/// The year, month and day of a number of days since 1970-01-01.
///
/// Hinnant, "chrono-Compatible Low-Level Date Algorithms", 2013: `civil_from_days`.
fn civil_date(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_shifted = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_shifted + 2) / 5 + 1;
    let month = if month_shifted < 10 {
        month_shifted + 3
    } else {
        month_shifted - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{full_date, relative_date, short_date};

    #[test]
    fn says_the_day_month_and_year_in_utc() {
        let cases = [
            (0, "1 Jan 1970"),
            (951_782_400, "29 Feb 2000"),
            (1_794_441_600, "12 Nov 2026"),
            (-86_400, "31 Dec 1969"),
        ];
        for (seconds, expected) in cases {
            assert_eq!(short_date(seconds), expected, "{seconds}");
        }
    }

    #[test]
    fn graph_columns__a_commit_made_90_minutes_ago_says_1h_ago_and_its_tooltip_the_full_date() {
        let now = 1_794_441_600 + 15 * 3_600;
        let made = now - 90 * 60;
        assert_eq!(relative_date(made, now), "1h ago");
        assert_eq!(full_date(made), "Nov 12, 2026, 13:30 UTC");
        let cases = [
            (30, "now"),
            (5 * 60, "5m ago"),
            (2 * DAY_SECONDS, "2d ago"),
            (8 * DAY_SECONDS, "Nov 4, 2026"),
        ];
        for (ago, expected) in cases {
            assert_eq!(relative_date(now - ago, now), expected, "{ago}");
        }
    }

    /// The seconds in a day.
    const DAY_SECONDS: i64 = 86_400;
}
