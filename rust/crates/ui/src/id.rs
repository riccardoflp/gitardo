//! The names of widgets: the same widget has the same name from one frame to the next, because the name is made of
//! the keys the caller gives, each within the scope of its parent.

use std::hash::{Hash, Hasher};

/// The name of a widget, the same in every frame for the same path of keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WidgetId(u64);

impl WidgetId {
    /// The scope every other name is made in.
    pub const ROOT: WidgetId = WidgetId(0xCBF2_9CE4_8422_2325);

    /// The name of a child of this one, by its key: a word, a number, or anything that can be hashed, unique among
    /// the children of the same parent.
    #[must_use]
    pub fn with(self, key: impl Hash) -> WidgetId {
        let mut hasher = StableHasher::starting_at(self.0);
        key.hash(&mut hasher);
        WidgetId(hasher.finish())
    }

    /// The number the name is made of, which names the widget to the screen reader too.
    pub(crate) fn number(self) -> u64 {
        self.0
    }

    /// The name at the end of a path of keys from the root, such as `["gallery", "list"]`.
    #[must_use]
    pub fn from_path<K: Hash>(path: &[K]) -> WidgetId {
        path.iter().fold(WidgetId::ROOT, WidgetId::with)
    }
}

/// The 64-bit hash of Fowler, Noll and Vo, variant 1a: the same bytes give the same hash on every machine and in every
/// run, unlike the hasher of the standard library, whose output is not promised to stay the same.
///
/// Fowler, Noll and Vo, "The FNV Non-Cryptographic Hash Algorithm", IETF draft, 2019.
#[derive(Clone, Copy, Debug)]
pub(crate) struct StableHasher {
    /// The hash of the bytes so far.
    state: u64,
}

/// The prime of the 64-bit hash.
const PRIME: u64 = 0x0000_0100_0000_01B3;

impl StableHasher {
    /// A hasher that has seen nothing.
    pub(crate) fn new() -> StableHasher {
        StableHasher {
            state: WidgetId::ROOT.0,
        }
    }

    /// A hasher that goes on from a hash already made.
    pub(crate) fn starting_at(state: u64) -> StableHasher {
        StableHasher { state }
    }

    /// Adds a float, by its bits.
    pub(crate) fn write_f32(&mut self, value: f32) {
        self.write_u32(value.to_bits());
    }
}

impl Hasher for StableHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.state ^= u64::from(*byte);
            self.state = self.state.wrapping_mul(PRIME);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_stable_and_depend_on_the_whole_path() {
        let list = WidgetId::ROOT.with("gallery").with("list");
        assert_eq!(list, WidgetId::from_path(&["gallery", "list"]));
        assert_ne!(list, WidgetId::ROOT.with("list").with("gallery"));
        assert_ne!(WidgetId::ROOT.with(1_usize), WidgetId::ROOT.with(2_usize));
        let mut hasher = StableHasher::new();
        hasher.write(b"a");
        assert_eq!(hasher.finish(), 0xAF63_DC4C_8601_EC8C);
    }
}
