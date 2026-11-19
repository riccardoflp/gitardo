//! What a rewrite changed, commit by commit: where a branch was before its last rebase or amend, or a remote branch
//! before it was force pushed (`range-diff.read-last-rewrite`); the two lines of commits compared (`range-diff.ranges`);
//! and the commits of the two lines paired as `git range-diff` pairs them (`range-diff.read-pairs`).

use crate::CommitId;

/// The most commits a line may have to be compared: git pairs every commit of one side with every commit of the other
/// (Q1138).
pub const MOST_COMMITS: usize = 1_000;

/// What rewrote a branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewriteKind {
    /// A rebase, `git pull --rebase` included, onto a commit.
    Rebase {
        /// The commit it was onto.
        onto: CommitId,
    },
    /// An amend of the last commit.
    Amend {
        /// Whether the commit amended is the first, with no parent: there is nothing to compare it from.
        is_root: bool,
    },
    /// A fetch that found the remote branch force pushed.
    ForcePush,
}

/// The last rewrite of a branch, from its reflog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rewrite {
    /// What rewrote it.
    pub kind: RewriteKind,
    /// Its tip before.
    pub before: CommitId,
    /// Its tip after.
    pub after: CommitId,
}

/// A branch, by its name and its commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Side<'a> {
    /// Its name: `feature`, `origin/feature`.
    pub name: &'a str,
    /// Its commit.
    pub commit: &'a CommitId,
}

/// What is compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangesInput<'a> {
    /// A branch before and after its last rewrite, or a remote branch before and after a force push.
    Rewrite(&'a Rewrite),
    /// A local branch and its upstream.
    Upstream {
        /// The branch.
        branch: Side<'a>,
        /// Its upstream.
        upstream: Side<'a>,
        /// The last rewrite of the branch, if its reflog has one.
        last_rewrite: Option<&'a Rewrite>,
    },
}

/// One line of commits, from a commit, left out, to another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// Where it starts, left out.
    pub from: String,
    /// Where it ends.
    pub to: String,
}

impl Line {
    /// The range git reads: `<from>..<to>`.
    #[must_use]
    pub fn range(&self) -> String {
        format!("{}..{}", self.from, self.to)
    }
}

/// The two lines compared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ranges {
    /// The arguments of `git range-diff` after its options: `<base> <before> <after>`, `<a>..<b> <c>..<d>` or
    /// `<a>...<b>`.
    pub arguments: Vec<String>,
    /// The commits before.
    pub before: Line,
    /// The commits now.
    pub now: Line,
}

/// The two lines of commits compared, as the arguments of `git range-diff`; none when the two sides are the same
/// commit, or for an amend of the first commit.
///
/// Feature: range-diff.ranges
#[must_use]
pub fn ranges(input: &RangesInput<'_>) -> Option<Ranges> {
    match input {
        RangesInput::Rewrite(rewrite) => of_rewrite(rewrite),
        RangesInput::Upstream {
            branch,
            upstream,
            last_rewrite,
        } => {
            if branch.commit == upstream.commit {
                return None;
            }
            match last_rewrite {
                Some(Rewrite {
                    kind: RewriteKind::Rebase { onto },
                    before,
                    ..
                }) if before == upstream.commit => Some(three(onto.as_str(), upstream.name, branch.name)),
                _ => Some(symmetric(upstream.name, branch.name)),
            }
        }
    }
}

/// The ranges of a rewrite.
fn of_rewrite(rewrite: &Rewrite) -> Option<Ranges> {
    if rewrite.before == rewrite.after {
        return None;
    }
    let (before, after) = (rewrite.before.as_str(), rewrite.after.as_str());
    match &rewrite.kind {
        RewriteKind::Rebase { onto } => Some(three(onto.as_str(), before, after)),
        RewriteKind::Amend { is_root: true } => None,
        RewriteKind::Amend { is_root: false } => {
            let line = |tip: &str| Line {
                from: format!("{tip}^"),
                to: tip.to_owned(),
            };
            let (before, now) = (line(before), line(after));
            Some(Ranges {
                arguments: vec![before.range(), now.range()],
                before,
                now,
            })
        }
        RewriteKind::ForcePush => Some(symmetric(before, after)),
    }
}

/// `<base> <before> <after>`, which git reads as `<base>..<before>` and `<base>..<after>`.
fn three(base: &str, before: &str, after: &str) -> Ranges {
    Ranges {
        arguments: vec![base.to_owned(), before.to_owned(), after.to_owned()],
        before: Line {
            from: base.to_owned(),
            to: before.to_owned(),
        },
        now: Line {
            from: base.to_owned(),
            to: after.to_owned(),
        },
    }
}

/// `<before>...<after>`, which git reads as `<after>..<before>` and `<before>..<after>`.
fn symmetric(before: &str, after: &str) -> Ranges {
    Ranges {
        arguments: vec![format!("{before}...{after}")],
        before: Line {
            from: after.to_owned(),
            to: before.to_owned(),
        },
        now: Line {
            from: before.to_owned(),
            to: after.to_owned(),
        },
    }
}

/// How a commit of one line is paired with one of the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairKind {
    /// The same commit made again: `=`.
    Same,
    /// Made again with changes, to its message or to what it changes: `!`.
    Changed,
    /// Only now: `>`.
    Added,
    /// Only before: `<`.
    Dropped,
}

/// A commit before and its commit now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    /// How they are paired.
    pub kind: PairKind,
    /// The commit before, none for one added.
    pub before: Option<CommitId>,
    /// The commit now, none for one dropped.
    pub now: Option<CommitId>,
    /// The subject: of the commit now, or of the one before for one dropped.
    pub subject: String,
    /// What changed in it, the diff of its two diffs, one line each, as git prints it without its indent.
    pub diff: Vec<String>,
}

/// What changed in a pair, from the headings of its diff of diffs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChangedParts {
    /// Its message: `## Commit message ##`.
    pub message: bool,
    /// What it changes: `## <path> ##`.
    pub changes: bool,
}

impl Pair {
    /// What changed in it: its message, what it changes, or both, from the parts its lines of `-` and `+` are in. A
    /// part is named by a heading, `## Commit message ##` or `## <path> ##`, or by the hunk of the outer diff it is in,
    /// `@@ Commit message`; the lines before them are of `## Metadata ##`.
    #[must_use]
    pub fn changed_parts(&self) -> ChangedParts {
        let mut parts = ChangedParts::default();
        let mut part = "Metadata";
        for line in &self.diff {
            if let Some(heading) = heading_of(line).or_else(|| line.strip_prefix("@@ ")) {
                part = heading;
            } else if line.starts_with(['-', '+']) {
                match part {
                    "Metadata" => {}
                    "Commit message" => parts.message = true,
                    _ => parts.changes = true,
                }
            }
        }
        parts
    }
}

/// The heading a line of a diff of diffs is, without its marks: `Commit message` of `  ## Commit message ##`, its outer
/// mark then the line of the diff it is in.
#[must_use]
pub fn heading_of(line: &str) -> Option<&str> {
    line.get(1..)?.strip_prefix(" ## ")?.strip_suffix(" ##")
}

/// The pairs of two lines of commits, with how many commits each line has.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RangeDiff {
    /// The pairs, in the order of the commits now, a dropped commit where it was.
    pub pairs: Vec<Pair>,
    /// How many commits the line before has.
    pub before_count: usize,
    /// How many commits the line now has.
    pub now_count: usize,
}

impl RangeDiff {
    /// How many pairs there are of each kind, in words, those of none left out: `2 the same · 1 changed · 1 dropped`.
    #[must_use]
    pub fn counts(&self) -> String {
        let count = |kind: PairKind| self.pairs.iter().filter(|pair| pair.kind == kind).count();
        [
            (PairKind::Same, "the same"),
            (PairKind::Changed, "changed"),
            (PairKind::Added, "added"),
            (PairKind::Dropped, "dropped"),
        ]
        .iter()
        .filter_map(|(kind, words)| {
            let count = count(*kind);
            (count > 0).then(|| format!("{count} {words}"))
        })
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
    }
}

/// What the page of what a rewrite changed shows, as `range-diff.compare` opens it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeDiffTarget {
    /// The title: **What the rebase of `feature` changed**.
    pub title: String,
    /// The branch, which names the tab: `feature · what changed`.
    pub branch: String,
    /// The two lines compared.
    pub ranges: Ranges,
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
