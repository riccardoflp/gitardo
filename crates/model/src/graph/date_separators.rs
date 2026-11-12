//! The lines between days, weeks and months: a thin line before the first commit of yesterday, of this week, of last
//! week and of each month before, with its name.

use std::collections::BTreeMap;

use crate::Timestamp;

/// A moment, and how far local time is from UTC at it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DateTime {
    /// The moment.
    pub moment: Timestamp,
    /// How many seconds local time is ahead of UTC: 3,600 in Rome in winter.
    pub utc_offset: i64,
}

/// The seconds in a day.
const DAY: i64 = 86_400;

/// The names of the months, January first.
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// The rows that start a period further back than all the rows above them, with the period's name: **Yesterday**,
/// **This week** (from Monday, before yesterday), **Last week** (the seven days before this week), then each month by
/// its name and year, `September 2026`, the current month included for what is before last week. The dates are in
/// seconds since 1970 and are seen in the local time of `now`; rows with no date are passed over, and the first row
/// with a date starts nothing.
///
/// Feature: graph.date-separators
#[must_use]
pub fn date_separators(dates: &[Option<i64>], now: DateTime) -> BTreeMap<usize, String> {
    let calendar = Calendar::at(now);
    let mut furthest = None;
    let mut separators = BTreeMap::new();
    for (index, date) in dates.iter().enumerate() {
        let Some(date) = date else {
            continue;
        };
        let period = calendar.period(*date);
        let distance = calendar.distance(period);
        match furthest {
            Some(seen) if distance <= seen => continue,
            Some(_) => {
                let _ = separators.insert(index, period.name());
            }
            None => {}
        }
        furthest = Some(distance);
    }
    separators
}

/// The period a date falls in, from the nearest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Period {
    Today,
    Yesterday,
    ThisWeek,
    LastWeek,
    Month { year: i64, month: i64 },
}

impl Period {
    /// What its separator says.
    fn name(self) -> String {
        match self {
            Period::Today => String::from("Today"),
            Period::Yesterday => String::from("Yesterday"),
            Period::ThisWeek => String::from("This week"),
            Period::LastWeek => String::from("Last week"),
            Period::Month { year, month } => {
                let name = usize::try_from(month - 1)
                    .ok()
                    .and_then(|index| MONTHS.get(index))
                    .unwrap_or(&"");
                format!("{name} {year}")
            }
        }
    }
}

/// Today, its week and its month, in local time.
#[derive(Debug, Clone, Copy)]
struct Calendar {
    /// How many seconds local time is ahead of UTC.
    utc_offset: i64,
    /// Today, in days since 1970-01-01.
    today: i64,
    /// The Monday of this week, in days since 1970-01-01.
    monday: i64,
    /// The current month, counted in months since the year 0.
    month: i64,
}

impl Calendar {
    /// The calendar at a moment.
    fn at(now: DateTime) -> Self {
        let today = (now.moment.0 + now.utc_offset).div_euclid(DAY);
        // 1970-01-01 was a Thursday, three days after a Monday.
        let monday = today - (today + 3).rem_euclid(7);
        let (year, month) = year_and_month(today);
        Calendar {
            utc_offset: now.utc_offset,
            today,
            monday,
            month: year * 12 + month - 1,
        }
    }

    /// The period of a date.
    fn period(self, date: i64) -> Period {
        let day = (date + self.utc_offset).div_euclid(DAY);
        if day >= self.today {
            Period::Today
        } else if day == self.today - 1 {
            Period::Yesterday
        } else if day >= self.monday {
            Period::ThisWeek
        } else if day >= self.monday - 7 {
            Period::LastWeek
        } else {
            let (year, month) = year_and_month(day);
            Period::Month { year, month }
        }
    }

    /// How far back a period is: the larger, the further.
    fn distance(self, period: Period) -> i64 {
        match period {
            Period::Today => 0,
            Period::Yesterday => 1,
            Period::ThisWeek => 2,
            Period::LastWeek => 3,
            Period::Month { year, month } => 4 + self.month - (year * 12 + month - 1),
        }
    }
}

/// The year and the month (1 for January) of a day counted since 1970-01-01, in the proleptic Gregorian calendar.
///
/// Howard Hinnant, "chrono-Compatible Low-Level Date Algorithms", `civil_from_days`.
fn year_and_month(days: i64) -> (i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{DateTime, date_separators, year_and_month};
    use crate::Timestamp;

    /// Noon of Wednesday 11 November 2026, in UTC.
    const NOW: DateTime = DateTime {
        moment: Timestamp(1_794_398_400),
        utc_offset: 0,
    };

    fn separators(dates: &[Option<i64>], now: DateTime) -> Vec<(usize, String)> {
        date_separators(dates, now).into_iter().collect()
    }

    #[test]
    fn graph_date_separators__yesterday_this_week_last_week_and_a_month() {
        let dates = [
            1_794_384_000,
            1_794_312_000,
            1_794_225_600,
            1_793_793_600,
            1_792_497_600,
        ]
        .map(Some);
        let expected = [
            (1, "Yesterday"),
            (2, "This week"),
            (3, "Last week"),
            (4, "October 2026"),
        ];
        let expected: Vec<(usize, String)> = expected.iter().map(|(row, name)| (*row, (*name).to_owned())).collect();
        assert_eq!(separators(&dates, NOW), expected);
    }

    #[test]
    fn graph_date_separators__a_period_is_never_named_twice() {
        let dates = [1_778_846_400, 1_773_576_000, 1_778_414_400].map(Some);
        assert_eq!(separators(&dates, NOW), [(1, String::from("March 2026"))]);
    }

    #[test]
    fn graph_date_separators__rows_with_no_date_are_passed_over_and_the_first_starts_nothing() {
        let dates = [
            None,
            Some(1_792_497_600),
            None,
            Some(1_792_497_600),
            Some(1_773_576_000),
        ];
        assert_eq!(separators(&dates, NOW), [(4, String::from("March 2026"))]);
    }

    #[test]
    fn graph_date_separators__the_days_are_those_of_local_time() {
        // 23:30 UTC on Tuesday is already Wednesday in Rome, an hour ahead.
        let rome = DateTime {
            utc_offset: 3_600,
            ..NOW
        };
        let late_tuesday = 1_794_312_000 + 11 * 3_600 + 1_800;
        assert_eq!(separators(&[Some(1_794_398_400), Some(late_tuesday)], rome), []);
        assert_eq!(
            separators(&[Some(1_794_398_400), Some(late_tuesday)], NOW),
            [(1, String::from("Yesterday"))]
        );
    }

    #[test]
    fn graph_date_separators__the_current_month_names_what_is_before_last_week() {
        // On Wednesday 25 November, the 2nd is before last week and still in November.
        let now = DateTime {
            moment: Timestamp(1_794_398_400 + 14 * 86_400),
            utc_offset: 0,
        };
        let dates = [
            Some(1_794_398_400 + 14 * 86_400),
            Some(1_793_620_800),
            Some(1_792_497_600),
        ];
        let expected = [(1, String::from("November 2026")), (2, String::from("October 2026"))];
        assert_eq!(separators(&dates, now), expected);
        assert_eq!(year_and_month(0), (1970, 1));
        assert_eq!(year_and_month(-1), (1969, 12));
    }
}
