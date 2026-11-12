//! Dates as the lists say them: `12 Nov 2026`.

/// The short names of the months, January first.
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// The seconds in a day.
const DAY: i64 = 86_400;

/// The day of a moment, in seconds since 1970 and in UTC, as `12 Nov 2026`.
#[must_use]
pub fn short_date(seconds: i64) -> String {
    let (year, month, day) = civil_date(seconds.div_euclid(DAY));
    let month_name = usize::try_from(month - 1)
        .ok()
        .and_then(|index| MONTHS.get(index))
        .copied()
        .unwrap_or_default();
    format!("{day} {month_name} {year}")
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
mod tests {
    use super::short_date;

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
}
