//! A file in conflict on a worker (`merge-view.page`): its versions read (`merge-view.read-sides`), its regions
//! computed (`merge-view.three-way`) within the time a page gives it, what each region offers, the choices that need no
//! thought, and what the rules propose for the conflicts git still marks (`conflicts.resolve-by-rules`).

#[cfg(test)]
mod tests;

use std::ops::Range;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use git::read::merge_view::{MergeSidesRequest, read_sides};
use git::{CancelToken, Git};
use model::diff::LineChange;
use model::merge_view::{
    Choice, MarkedBlock, MergeTexts, Proposal, RuleReason, ThreeWayInput, simple_choices, three_way,
};
use model::state::Operation;
use text::conflicts::{RuleCase, RuleResolutions, RulesInput, parse_markers, resolve_by_rules, split_lines};
use text::diff::{LineDiffInput, WordDiffInput, line_diff, word_diff};
use views::merge_view::{MergePageId, MergeText, Offers};

use super::diff::TIME_LIMIT;

/// More lines than this in the four texts together, and the page asks before showing them.
pub const LARGE_LINES: usize = 200_000;

/// The base and the two sides of a file in conflict, read once for a page.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct MergeSidesText {
    /// The base, as UTF-8 with what is not replaced.
    pub base: String,
    /// The current side.
    pub current: String,
    /// The incoming side.
    pub incoming: String,
    /// The operation that stopped on the conflict.
    pub operation: Option<Operation>,
}

/// The regions of a file of a Merge page, as a job asks for them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeJob {
    /// The page.
    pub page: MergePageId,
    /// The file, from the top of the working tree.
    pub file: String,
    /// The result to compute the regions for: the text the page holds when it has changes that are not saved; none
    /// to read the file on disk.
    pub result: Option<String>,
    /// The base and the two sides read before, so that only the regions are computed again; none to read them.
    pub sides: Option<Arc<MergeSidesText>>,
    /// `conflicts.autoResolve`: whether the rules propose what they can for the conflicts git still marks.
    pub auto_resolve: bool,
    /// Whether a very large file was asked about and the person went on.
    pub is_confirmed: bool,
    /// The number of the text of the result the regions are for.
    pub generation: u64,
}

/// What the read of a file of a Merge page came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MergeRead {
    /// The texts and their regions.
    Text {
        /// The texts and their regions.
        text: Box<MergeText>,
        /// The base and the two sides, kept for the next computation.
        sides: Arc<MergeSidesText>,
        /// Whether the result was read from the disk.
        is_from_disk: bool,
        /// The number of the text of the result the regions are for.
        generation: u64,
    },
    /// The index does not keep a version of the file on each side.
    NotInConflict,
    /// A text has a NUL byte.
    Binary,
    /// It took longer than its time.
    TooComplex,
    /// The four texts have more than 200,000 lines together, and the person was not asked yet.
    Large,
    /// Git could not read a version: what it said.
    Failed(String),
}

/// Reads the versions of a file in conflict, unless the job has them, and computes its regions, stopped by its token
/// or after its time.
#[must_use]
pub fn read_merge(folder: &Path, job: &MergeJob, cancel: &CancelToken) -> MergeRead {
    let (sides, result, is_from_disk) = match (&job.sides, &job.result) {
        (Some(sides), Some(result)) => (Arc::clone(sides), result.clone(), false),
        _ => match read_versions(folder, job, cancel) {
            Ok(read) => read,
            Err(refused) => return refused,
        },
    };
    let started = Instant::now();
    let keep_going = || started.elapsed() < TIME_LIMIT && !cancel.is_cancelled();
    let Some(text) = compute(&sides, result, (job.auto_resolve, &keep_going)) else {
        return MergeRead::TooComplex;
    };
    MergeRead::Text {
        text: Box::new(text),
        sides,
        is_from_disk,
        generation: job.generation,
    }
}

/// The versions of the file read with git, refused when it is not in conflict with a version on each side, looks
/// binary, or is very large and was not asked about.
fn read_versions(
    folder: &Path,
    job: &MergeJob,
    cancel: &CancelToken,
) -> Result<(Arc<MergeSidesText>, String, bool), MergeRead> {
    let git = Git::new(folder).with_cancel(cancel.clone());
    let request = MergeSidesRequest {
        file: job.file.clone(),
        unsaved: job.result.as_ref().map(|text| text.as_bytes().to_vec()),
    };
    let read = read_sides(&git, &request).map_err(|error| MergeRead::Failed(error.to_string()))?;
    if !read.has_both_sides {
        return Err(MergeRead::NotInConflict);
    }
    let texts = [&read.base, &read.current, &read.incoming, &read.result];
    if texts.iter().any(|bytes| bytes.contains(&0)) {
        return Err(MergeRead::Binary);
    }
    let lossy = |bytes: &[u8]| String::from_utf8_lossy(bytes).into_owned();
    let line_count: usize = texts
        .iter()
        .map(|bytes| split_lines(&String::from_utf8_lossy(bytes)).len())
        .sum();
    if !job.is_confirmed && line_count > LARGE_LINES {
        return Err(MergeRead::Large);
    }
    let sides = MergeSidesText {
        base: lossy(&read.base),
        current: lossy(&read.current),
        incoming: lossy(&read.incoming),
        operation: read.operation,
    };
    Ok((Arc::new(sides), lossy(&read.result), job.result.is_none()))
}

/// The regions of a result against the versions of its file, with what each region offers, the choices that need no
/// thought and what the rules propose; none when a diff takes longer than its time.
#[must_use]
pub fn compute(
    sides: &MergeSidesText,
    result: String,
    (auto_resolve, keep_going): (bool, &dyn Fn() -> bool),
) -> Option<MergeText> {
    let mut text = MergeText {
        base_lines: line_ranges(&sides.base),
        current_lines: line_ranges(&sides.current),
        incoming_lines: line_ranges(&sides.incoming),
        result_lines: line_ranges(&result),
        base: sides.base.clone(),
        current: sides.current.clone(),
        incoming: sides.incoming.clone(),
        result,
        ..MergeText::default()
    };
    let marked = marked_blocks(&text.result);
    let lines = text.lines();
    let differ = |left: &[&str], right: &[&str]| line_changes((left, right), keep_going);
    let regions = three_way(&ThreeWayInput {
        base: &lines.base,
        current: &lines.current,
        incoming: &lines.incoming,
        result: &lines.result,
        marked: &marked,
        line_diff: &differ,
        word_marks: &|left, right| word_diff(&WordDiffInput { left, right }),
    })?
    .regions;
    let texts = lines.texts();
    let offers = regions.iter().map(|region| offers_of(region, &texts)).collect();
    let simple = simple_choices(&regions, &texts);
    let proposals = if auto_resolve {
        proposals_of(sides, (&text.result, &regions), keep_going)
    } else {
        Vec::new()
    };
    drop(lines);
    text.regions = regions;
    text.offers = offers;
    text.simple = simple;
    text.proposals = proposals;
    Some(text)
}

/// The conflicts git still marks in a result.
fn marked_blocks(result: &str) -> Vec<MarkedBlock> {
    parse_markers(result)
        .unwrap_or_default()
        .into_iter()
        .map(|conflict| MarkedBlock {
            span: conflict.span,
            current: conflict.current,
            incoming: conflict.incoming,
        })
        .collect()
}

/// The changes of the minimal line diff from a text to another, by lines; none when it takes longer than its time.
fn line_changes((left, right): (&[&str], &[&str]), keep_going: &dyn Fn() -> bool) -> Option<Vec<LineChange>> {
    let changes = line_diff(&LineDiffInput {
        left,
        right,
        keep_going,
    })
    .ok()?;
    Some(
        changes
            .iter()
            .map(|change| LineChange {
                left: change.left.lines(),
                right: change.right.lines(),
            })
            .collect(),
    )
}

/// Which choices would change what the result has in a region.
fn offers_of(region: &model::merge_view::Region, texts: &MergeTexts<'_>) -> Offers {
    Offers {
        current: region.would_change(texts, Choice::Current),
        current_first: region.would_change(texts, Choice::CurrentFirst),
        incoming: region.would_change(texts, Choice::Incoming),
        incoming_first: region.would_change(texts, Choice::IncomingFirst),
    }
}

/// What the rules propose for the conflicts git still marks, each by the region of its conflict.
fn proposals_of(
    sides: &MergeSidesText,
    (result, regions): (&str, &[model::merge_view::Region]),
    keep_going: &dyn Fn() -> bool,
) -> Vec<Proposal> {
    let resolved = resolve_by_rules(&RulesInput {
        file: result.as_bytes(),
        base: sides.base.as_bytes(),
        current: sides.current.as_bytes(),
        incoming: sides.incoming.as_bytes(),
        keep_going,
    });
    let RuleResolutions::LookedInto(conflicts) = resolved else {
        return Vec::new();
    };
    conflicts
        .into_iter()
        .filter_map(|conflict| {
            let resolution = conflict.resolution?;
            let region = regions.iter().position(|region| {
                region.is_unresolved()
                    && region
                        .marked
                        .as_ref()
                        .is_some_and(|block| block.span == conflict.conflict.span)
            })?;
            let text = resolution
                .lines
                .iter()
                .flat_map(|line| [line.text, line.ending])
                .collect();
            Some(Proposal {
                region,
                replaced: resolution.replaced,
                text,
                reason: reason_of(resolution.case),
            })
        })
        .collect()
}

/// Why the rules could resolve a conflict, in the words of the page.
fn reason_of(case: RuleCase) -> RuleReason {
    match case {
        RuleCase::SpacesOrLineEndings => RuleReason::SpacesOrLineEndings,
        RuleCase::SameChange => RuleReason::SameChange,
        RuleCase::OneSide => RuleReason::OneSide,
        RuleCase::NextToEachOther => RuleReason::NextToEachOther,
    }
}

/// Where each line of a text is, without what ends it: `\n`, `\r\n` and `\r` end a line, as git's markers are read.
#[must_use]
pub fn line_ranges(text: &str) -> Vec<Range<usize>> {
    let mut start = 0;
    split_lines(text)
        .iter()
        .map(|line| {
            let range = start..start + line.text.len();
            start = range.end + line.ending.len();
            range
        })
        .collect()
}
