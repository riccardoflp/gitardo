//! The JSON of what version 1.14 adds for milestone M36 (D81): `read/range-diff.read-pairs`, the pairs of what a rewrite
//! changed, commit by commit, for the tab of `range-diff.page`; and the lines compared, which the follow-up that opens
//! that tab carries ([`crate::feature::FollowUpJson::OpenRangeDiff`]).

use model::CommitId;
use model::range_diff::{Line, Pair, PairKind, RangeDiff, Ranges};
use serde::{Deserialize, Serialize};

/// The names of the methods of what a rewrite changed.
pub mod methods {
    /// `range-diff.read-pairs`: the pairs of two lines of commits (version 1.14).
    pub const RANGE_DIFF_PAIRS: &str = "read/range-diff.read-pairs";
}

/// One line of commits: from the commit it starts after to the one it ends at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineJson {
    /// The commit it starts after, as git names it.
    pub from: String,
    /// The commit it ends at.
    pub to: String,
}

/// The two lines compared, and the arguments of `git range-diff` that compare them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RangesJson {
    /// The arguments after `git range-diff`.
    pub arguments: Vec<String>,
    /// The line before.
    pub before: LineJson,
    /// The line now.
    pub now: LineJson,
}

impl From<&Ranges> for RangesJson {
    fn from(ranges: &Ranges) -> RangesJson {
        let line = |line: &Line| LineJson {
            from: line.from.clone(),
            to: line.to.clone(),
        };
        RangesJson {
            arguments: ranges.arguments.clone(),
            before: line(&ranges.before),
            now: line(&ranges.now),
        }
    }
}

impl From<&RangesJson> for Ranges {
    fn from(ranges: &RangesJson) -> Ranges {
        let line = |line: &LineJson| Line {
            from: line.from.clone(),
            to: line.to.clone(),
        };
        Ranges {
            arguments: ranges.arguments.clone(),
            before: line(&ranges.before),
            now: line(&ranges.now),
        }
    }
}

/// The parameters of `read/range-diff.read-pairs`: the repository, and the lines its follow-up gave.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RangeDiffParams {
    /// The folder of the top of its working tree.
    pub repository: String,
    /// The lines compared.
    pub ranges: RangesJson,
}

/// The kind of a pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PairKindJson {
    /// Made again unchanged.
    Same,
    /// Made again with changes.
    Changed,
    /// Only in the line now.
    Added,
    /// Only in the line before.
    Dropped,
}

/// One pair of commits.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairJson {
    /// Its kind.
    pub kind: PairKindJson,
    /// The commit before, none for one added.
    pub before: Option<String>,
    /// The commit now, none for one dropped.
    pub now: Option<String>,
    /// The subject now, or before for one dropped.
    pub subject: String,
    /// Whether its message changed, from the headings of its diff of diffs.
    pub message_changed: bool,
    /// Whether its changes changed.
    pub changes_changed: bool,
    /// Its diff of diffs, each line with its outer mark; empty unless it changed.
    pub diff: Vec<String>,
}

/// The answer of `read/range-diff.read-pairs`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RangeDiffJson {
    /// The pairs, in the order of the commits now, a dropped one where it was.
    pub pairs: Vec<PairJson>,
    /// How many commits the line before has.
    pub before_count: usize,
    /// How many commits the line now has.
    pub now_count: usize,
    /// The count of each kind of pair, in words: `2 the same · 1 changed · 1 dropped`.
    pub counts: String,
}

fn commit_text(commit: Option<&CommitId>) -> Option<String> {
    commit.map(|commit| commit.as_str().to_owned())
}

impl From<&Pair> for PairJson {
    fn from(pair: &Pair) -> PairJson {
        let parts = pair.changed_parts();
        PairJson {
            kind: match pair.kind {
                PairKind::Same => PairKindJson::Same,
                PairKind::Changed => PairKindJson::Changed,
                PairKind::Added => PairKindJson::Added,
                PairKind::Dropped => PairKindJson::Dropped,
            },
            before: commit_text(pair.before.as_ref()),
            now: commit_text(pair.now.as_ref()),
            subject: pair.subject.clone(),
            message_changed: parts.message,
            changes_changed: parts.changes,
            diff: pair.diff.clone(),
        }
    }
}

impl From<&RangeDiff> for RangeDiffJson {
    fn from(read: &RangeDiff) -> RangeDiffJson {
        RangeDiffJson {
            pairs: read.pairs.iter().map(PairJson::from).collect(),
            before_count: read.before_count,
            now_count: read.now_count,
            counts: read.counts(),
        }
    }
}
