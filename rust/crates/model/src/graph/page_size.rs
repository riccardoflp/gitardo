//! How many commits the graph reads at a time: the setting `graph.pageSize`.

/// How many commits the graph reads at a time, between 100 and 10,000.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSize(usize);

impl PageSize {
    /// The size when the setting is not a number.
    pub const DEFAULT: PageSize = PageSize(1_000);
    /// The least a page can be: a smaller number counts as this.
    pub const LEAST: usize = 100;
    /// The most a page can be: a larger number counts as this.
    pub const MOST: usize = 10_000;

    /// The page size the setting `graph.pageSize` asks for: its number within 100 and 10,000, and 1,000 when it is not
    /// a number. A fraction is rounded down.
    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the number is clamped between 100 and 10,000 first, which a usize holds exactly"
    )]
    pub fn from_setting(value: Option<f64>) -> PageSize {
        match value {
            Some(number) if number.is_finite() => PageSize(number.clamp(100.0, 10_000.0).floor() as usize),
            _ => Self::DEFAULT,
        }
    }

    /// How many commits a page has.
    #[must_use]
    pub fn get(self) -> usize {
        self.0
    }
}

impl Default for PageSize {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::PageSize;

    #[test]
    fn keeps_the_number_within_its_limits() {
        let cases = [
            (Some(50.0), 100),
            (Some(250.0), 250),
            (Some(250.9), 250),
            (Some(20_000.0), 10_000),
            (Some(-3.0), 100),
            (Some(f64::NAN), 1_000),
            (None, 1_000),
        ];
        for (setting, expected) in cases {
            assert_eq!(PageSize::from_setting(setting).get(), expected, "{setting:?}");
        }
    }
}
