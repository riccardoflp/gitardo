//! The color of the bar beside a line, from how long ago it was changed.

use super::BlameLine;
use crate::Timestamp;

/// The color of a line just changed, unless set.
pub const RECENT_COLOR: &str = "#e5534b";

/// The color of a line changed long ago, unless set.
pub const OLD_COLOR: &str = "#316dca";

/// The days after which a line has the old color, unless set.
pub const OLD_AFTER_DAYS: f64 = 730.0;

/// What [`age_color`] looks at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AgeInput<'a> {
    /// The blame of the line; none when git said nothing of it.
    pub line: Option<&'a BlameLine>,
    /// The time now.
    pub now: Timestamp,
    /// Whether the bars have the colors of age (`fileBlame.age.enabled`).
    pub is_enabled: bool,
    /// The color of a line just changed, as the setting has it.
    pub recent: &'a str,
    /// The color of a line changed long ago, as the setting has it.
    pub old: &'a str,
    /// The days after which a line has the old color, as the setting has it.
    pub old_after_days: f64,
}

/// The color of the bar of a line, `#rrggbb`; none when the bars have no color of age, or git said nothing of the line.
///
/// The share of the way from the recent color to the old one is the logarithm of one plus the age in days over the
/// logarithm of one plus the days of the setting, at most 1, rounded to the nearest tenth.
///
/// Feature: blame.age-color
#[must_use]
pub fn age_color(input: &AgeInput<'_>) -> Option<String> {
    if !input.is_enabled {
        return None;
    }
    let time = match input.line? {
        BlameLine::NotCommitted => input.now,
        BlameLine::Committed(commit) => commit.author_time,
    };
    let recent = rgb(input.recent).or_else(|| rgb(RECENT_COLOR))?;
    let old = rgb(input.old).or_else(|| rgb(OLD_COLOR))?;
    let days_setting = if input.old_after_days < 1.0 {
        OLD_AFTER_DAYS
    } else {
        input.old_after_days
    };
    #[allow(
        clippy::cast_precision_loss,
        reason = "an age in seconds is far below what an f64 holds exactly"
    )]
    let days = ((input.now.0 - time.0) as f64 / 86_400.0).max(0.0);
    let share = ((1.0 + days).ln() / (1.0 + days_setting).ln()).min(1.0);
    let share = (share * 10.0).round() / 10.0;
    let channel = |from: u8, to: u8| -> u8 {
        let value = f64::from(from) + share * (f64::from(to) - f64::from(from));
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the value is between two channels, within 0 and 255"
        )]
        let channel = value.round().clamp(0.0, 255.0) as u8;
        channel
    };
    let [(red_from, red_to), (green_from, green_to), (blue_from, blue_to)] = [0, 1, 2].map(|index| {
        (
            recent.get(index).copied().unwrap_or(0),
            old.get(index).copied().unwrap_or(0),
        )
    });
    let (red, green, blue) = (
        channel(red_from, red_to),
        channel(green_from, green_to),
        channel(blue_from, blue_to),
    );
    Some(format!("#{red:02x}{green:02x}{blue:02x}"))
}

/// The red, green and blue of `#rgb` or `#rrggbb`.
fn rgb(color: &str) -> Option<[u8; 3]> {
    let digits = color.strip_prefix('#')?;
    if !digits.chars().all(|digit| digit.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |text: &str| u8::from_str_radix(text, 16).ok();
    match digits.len() {
        3 => {
            let mut channels = digits.chars().map(|digit| channel(&digit.to_string().repeat(2)));
            Some([channels.next()??, channels.next()??, channels.next()??])
        }
        6 => Some([
            channel(digits.get(0..2)?)?,
            channel(digits.get(2..4)?)?,
            channel(digits.get(4..6)?)?,
        ]),
        _ => None,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::sync::Arc;

    use super::{AgeInput, OLD_AFTER_DAYS, OLD_COLOR, RECENT_COLOR, age_color};
    use crate::blame::{BlameCommit, BlameLine};
    use crate::{CommitId, Timestamp};

    const NOW: i64 = 1_794_398_400;

    fn line(days_ago: f64) -> BlameLine {
        #[allow(clippy::cast_possible_truncation, reason = "whole seconds of a few days")]
        let seconds = (days_ago * 86_400.0) as i64;
        BlameLine::Committed(Arc::new(BlameCommit {
            id: CommitId::parse(&"a".repeat(40)).unwrap_or_else(|| panic!("a hash")),
            author: String::from("Ann Lee"),
            author_time: Timestamp(NOW - seconds),
            summary: String::from("Fix the parser"),
            previous: None,
        }))
    }

    fn color(line: &BlameLine, recent: &str, is_enabled: bool) -> Option<String> {
        age_color(&AgeInput {
            line: Some(line),
            now: Timestamp(NOW),
            is_enabled,
            recent,
            old: OLD_COLOR,
            old_after_days: OLD_AFTER_DAYS,
        })
    }

    /// The color a share of the way gives with the default colors.
    fn at(share: f64) -> String {
        let mix = |from: u8, to: u8| -> u8 {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a channel")]
            let value = (f64::from(from) + share * (f64::from(to) - f64::from(from))).round() as u8;
            value
        };
        format!("#{:02x}{:02x}{:02x}", mix(0xe5, 0x31), mix(0x53, 0x6d), mix(0x4b, 0xca))
    }

    #[test]
    fn blame_age_color__the_examples_hold() {
        assert_eq!(
            color(&BlameLine::NotCommitted, RECENT_COLOR, true).as_deref(),
            Some("#e5534b")
        );
        assert_eq!(color(&line(0.0), RECENT_COLOR, true).as_deref(), Some("#e5534b"));
        assert_eq!(color(&line(730.0), RECENT_COLOR, true).as_deref(), Some("#316dca"));
        assert_eq!(color(&line(2_000.0), RECENT_COLOR, true).as_deref(), Some("#316dca"));
        for (days, share) in [(1.0, 0.1), (7.0, 0.3), (30.0, 0.5), (365.0, 0.9)] {
            assert_eq!(color(&line(days), RECENT_COLOR, true), Some(at(share)), "{days} days");
        }
    }

    #[test]
    fn blame_age_color__a_color_that_is_not_one_is_the_default() {
        assert_eq!(color(&line(0.0), "red", true).as_deref(), Some("#e5534b"));
        assert_eq!(color(&line(0.0), "#f00", true).as_deref(), Some("#ff0000"));
    }

    #[test]
    fn blame_age_color__switched_off_there_is_no_bar() {
        assert_eq!(color(&line(3.0), RECENT_COLOR, false), None);
        let said_nothing = age_color(&AgeInput {
            line: None,
            now: Timestamp(NOW),
            is_enabled: true,
            recent: RECENT_COLOR,
            old: OLD_COLOR,
            old_after_days: 0.0,
        });
        assert_eq!(said_nothing, None);
    }
}
