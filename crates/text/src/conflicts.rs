//! The conflicts git marks in a file: where they are (`conflicts.parse-markers`), the key that finds the same conflict
//! again wherever it moves (`review.fingerprint`), and those that need no choice (`conflicts.resolve-by-rules`).

mod fingerprint;
mod markers;
mod rules;
mod sha1;

pub use fingerprint::{Fingerprint, fingerprint};
pub use markers::{Line, MarkedConflict, parse_markers, split_lines};
pub use rules::{
    LeftAlone, RuleCase, RuleConflict, RuleResolution, RuleResolutions, RulesInput, Stretch, resolve_by_rules,
};
