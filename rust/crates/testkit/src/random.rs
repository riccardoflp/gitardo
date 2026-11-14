//! A seeded random source for tests, which says its seed when a test fails so that the test can run again the same way.

use std::io::Write;

/// The environment variable that gives the seed, to run a failing test again with the numbers it had.
const SEED_VARIABLE: &str = "GITARDO_TEST_SEED";

/// The state used for a seed of zero, which xorshift cannot start from: it would give zero forever. Any number but
/// zero would do; this one has its bits evenly mixed.
const STATE_FOR_SEED_ZERO: u64 = 0x9E37_79B9_7F4A_7C15;

/// A random source that gives the same numbers for the same seed, on every machine.
///
/// Marsaglia, "Xorshift RNGs", Journal of Statistical Software 8(14), 2003: the 64-bit generator with the shifts
/// 13, 7 and 17.
///
/// When it is dropped while its test is failing, it writes its seed to the standard error, with the variable that runs
/// the test again with it.
#[derive(Debug)]
pub struct Random {
    seed: u64,
    state: u64,
}

impl Random {
    /// A random source with a seed.
    #[must_use]
    pub fn new(seed: u64) -> Random {
        let state = if seed == 0 { STATE_FOR_SEED_ZERO } else { seed };
        Random { seed, state }
    }

    /// A random source with the seed of `GITARDO_TEST_SEED` when it is set, the default seed otherwise.
    ///
    /// # Panics
    ///
    /// When `GITARDO_TEST_SEED` is set to something that is not a number from 0 to 2⁶⁴ − 1.
    #[must_use]
    pub fn from_environment(default_seed: u64) -> Random {
        let Some(value) = std::env::var_os(SEED_VARIABLE) else {
            return Random::new(default_seed);
        };
        match value.to_str().map(str::parse) {
            Some(Ok(seed)) => Random::new(seed),
            _ => panic!(
                "{SEED_VARIABLE} is set to {}, which is not a seed: give a number from 0 to 2^64 - 1",
                value.display()
            ),
        }
    }

    /// The seed this source started from.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// The next number, any of the 2⁶⁴ − 1 that are not zero.
    pub fn next_u64(&mut self) -> u64 {
        let mut state = self.state;
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        self.state = state;
        state
    }

    /// A number from 0 up to the bound, the bound excluded, each as likely as the others.
    ///
    /// # Panics
    ///
    /// When the bound is zero: there is no number below it.
    pub fn below(&mut self, bound: usize) -> usize {
        let bound = u64::try_from(bound).expect("a usize fits in 64 bits");
        assert!(bound > 0, "there is no number below zero to choose");
        // The numbers under this threshold are left out, so that what remains is a whole number of times the bound
        // and no remainder is more likely than another.
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let number = self.next_u64();
            if number >= threshold {
                return usize::try_from(number % bound).expect("a number below a usize fits in a usize");
            }
        }
    }

    /// One of the items, each as likely as the others.
    ///
    /// # Panics
    ///
    /// When there are no items.
    pub fn pick<'items, Item>(&mut self, items: &'items [Item]) -> &'items Item {
        assert!(!items.is_empty(), "there is nothing to pick from");
        let index = self.below(items.len());
        items.get(index).expect("the index is below the number of items")
    }

    /// Whether something with a chance of `numerator` in `denominator` happens this time.
    ///
    /// # Panics
    ///
    /// When the denominator is zero.
    pub fn chance(&mut self, numerator: usize, denominator: usize) -> bool {
        self.below(denominator) < numerator
    }
}

impl Drop for Random {
    fn drop(&mut self) {
        if std::thread::panicking() {
            let seed = self.seed;
            // Nothing more can be done when the standard error cannot be written to: the test fails anyway.
            let _ = writeln!(
                std::io::stderr(),
                "The random source of this test started from the seed {seed}. Run it again with {SEED_VARIABLE}={seed}."
            );
        }
    }
}

#[cfg(test)]
mod tests;
