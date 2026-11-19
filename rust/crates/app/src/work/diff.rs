//! The diff of a file on a worker (`diff.page`): its two versions read (`diff.read-sides`), the lines that changed
//! (`diff.line-diff`) within the time a page gives it, the two sides in rows with the words changed (`diff.align`,
//! `diff.word-diff`), the unchanged rows that fold (`diff.collapse-unchanged`) and, against `HEAD`, which changes are
//! staged already (`diff.staged-changes`).

use std::path::Path;
use std::time::{Duration, Instant};

use git::read::diff::{DiffRequest, FileVersion, VersionRequest, read_sides};
use git::{CancelToken, Git};
use model::diff::{AlignInput, DiffSides, LineChange, StagedInput, align, staged_changes, unchanged_folds};
use render::image::{ImageFormat, image_format, read_image};
use text::diff::{LineDiffInput, TooComplex, WordDiffInput, line_diff, line_ranges, word_diff};
use views::diff::{DiffImages, DiffPageId, DiffText};

/// The longest a diff is computed for a page before it says it has too many differences.
pub const TIME_LIMIT: Duration = Duration::from_secs(5);
/// More lines than this on the two sides together, and the page asks before computing the diff.
pub const LARGE_LINES: usize = 100_000;

/// The diff of a file of a page, as a job asks for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffJob {
    /// The page.
    pub page: DiffPageId,
    /// The file, by its place in the page.
    pub file: usize,
    /// Its two versions.
    pub request: DiffRequest,
    /// Whether the index is read too, to tell which changes are staged: the diff of the working tree against `HEAD`.
    pub reads_index: bool,
    /// Whether a very large file was asked about and the person went on.
    pub is_confirmed: bool,
}

/// What the diff of a file came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffRead {
    /// The two sides in rows.
    Text(Box<DiffText>),
    /// The two versions of an image, and for an SVG the diff of its lines (`diff.image`).
    Image(Box<DiffImages>),
    /// A side has a NUL byte.
    Binary,
    /// It took longer than its time.
    TooComplex,
    /// The two sides have more than 100,000 lines together, and the person was not asked yet.
    Large,
    /// Git could not read a side: what it said.
    Failed(String),
}

/// Reads the two versions of a file and computes their diff, stopped by its token or after its time.
#[must_use]
pub fn read_diff(folder: &Path, job: &DiffJob, cancel: &CancelToken) -> DiffRead {
    let git = Git::new(folder).with_cancel(cancel.clone());
    let sides = match read_sides(&git, &job.request) {
        Ok(sides) => sides,
        Err(error) => return DiffRead::Failed(error.to_string()),
    };
    if let Some(images) = images_of(&git, job, &sides, cancel) {
        return DiffRead::Image(Box::new(images));
    }
    if sides.looks_binary() {
        return DiffRead::Binary;
    }
    text_diff(&git, job, &sides, cancel)
}

/// The two versions of a file that is an image on either side, drawn, and for an SVG the diff of its lines; none for
/// a file that is an image on neither side.
fn images_of(git: &Git, job: &DiffJob, sides: &DiffSides, cancel: &CancelToken) -> Option<DiffImages> {
    let paths = (&job.request.left.path, &job.request.right.path);
    let format = |bytes: &Option<Vec<u8>>, path: &str| bytes.as_deref().and_then(|bytes| image_format(bytes, path));
    let formats = (format(&sides.left, paths.0), format(&sides.right, paths.1));
    if formats == (None, None) {
        return None;
    }
    let read = |bytes: &Option<Vec<u8>>, path: &str| {
        bytes
            .as_deref()
            .map(|bytes| read_image(bytes, path, super::file_view::DRAW_SCALE))
    };
    let is_svg = |side_format: Option<ImageFormat>, side: &Option<Vec<u8>>| {
        side_format == Some(ImageFormat::Svg) || side.is_none()
    };
    let text = if is_svg(formats.0, &sides.left) && is_svg(formats.1, &sides.right) {
        match text_diff(git, job, sides, cancel) {
            DiffRead::Text(text) => Some(text),
            _ => None,
        }
    } else {
        None
    };
    Some(DiffImages {
        left: read(&sides.left, paths.0),
        right: read(&sides.right, paths.1),
        text,
        is_same_bytes: sides.left == sides.right,
    })
}

/// The diff of the lines of two versions of a file that is text.
fn text_diff(git: &Git, job: &DiffJob, sides: &DiffSides, cancel: &CancelToken) -> DiffRead {
    let has = (sides.left.is_some(), sides.right.is_some());
    let left = String::from_utf8_lossy(sides.left.as_deref().unwrap_or_default()).into_owned();
    let right = String::from_utf8_lossy(sides.right.as_deref().unwrap_or_default()).into_owned();
    let (left_lines, right_lines) = (line_ranges(&left), line_ranges(&right));
    if !job.is_confirmed && left_lines.len() + right_lines.len() > LARGE_LINES {
        return DiffRead::Large;
    }
    let started = Instant::now();
    let keep_going = || started.elapsed() < TIME_LIMIT && !cancel.is_cancelled();
    let texts = Texts {
        left: lines_of(&left, &left_lines),
        right: lines_of(&right, &right_lines),
    };
    let Ok(changes) = changes_between(&texts.left, &texts.right, &keep_going) else {
        return DiffRead::TooComplex;
    };
    let aligned = align(&AlignInput {
        left: &texts.left,
        right: &texts.right,
        changes: &changes,
        word_marks: &|left, right| word_diff(&WordDiffInput { left, right }),
    });
    let staged = if job.reads_index {
        staged_of(git, job, (&texts.right, &changes), &keep_going)
    } else {
        Vec::new()
    };
    let folds = unchanged_folds(&aligned);
    let left_colors = crate::store::file_page::colors_of(&job.request.left.path, &left);
    let right_colors = crate::store::file_page::colors_of(&job.request.right.path, &right);
    DiffRead::Text(Box::new(DiffText {
        left_colors,
        right_colors,
        sides: has,
        left_lines,
        right_lines,
        aligned,
        staged,
        folds,
        left,
        right,
        changes,
    }))
}

/// The lines of both sides.
struct Texts<'a> {
    /// The lines of the left side.
    left: Vec<&'a str>,
    /// The lines of the right side.
    right: Vec<&'a str>,
}

/// The lines of a text, from where they are in it.
fn lines_of<'a>(text: &'a str, ranges: &[std::ops::Range<usize>]) -> Vec<&'a str> {
    ranges
        .iter()
        .map(|range| text.get(range.clone()).unwrap_or_default())
        .collect()
}

/// The changes between two lists of lines, by the lines each takes on each side.
fn changes_between(
    left: &[&str],
    right: &[&str],
    keep_going: &dyn Fn() -> bool,
) -> Result<Vec<LineChange>, TooComplex> {
    let changes = line_diff(&LineDiffInput {
        left,
        right,
        keep_going,
    })?;
    Ok(changes
        .iter()
        .map(|change| LineChange {
            left: change.left.lines(),
            right: change.right.lines(),
        })
        .collect())
}

/// Which changes of the working tree against `HEAD` are staged: the index read, and its differences with the working
/// tree compared with them. None is staged when the index cannot be read or has no such file.
fn staged_of(
    git: &Git,
    job: &DiffJob,
    (right, changes): (&[&str], &[LineChange]),
    keep_going: &dyn Fn() -> bool,
) -> Vec<bool> {
    let request = DiffRequest {
        left: VersionRequest {
            version: FileVersion::Index,
            path: job.request.right.path.clone(),
        },
        right: VersionRequest {
            version: FileVersion::Absent,
            path: job.request.right.path.clone(),
        },
    };
    let index = read_sides(git, &request).ok().and_then(|sides| sides.left);
    let Some(index) = index else {
        return staged_changes(&StagedInput {
            changes,
            unstaged: None,
        });
    };
    let index = String::from_utf8_lossy(&index).into_owned();
    let index_lines = lines_of(&index, &line_ranges(&index));
    match changes_between(&index_lines, right, keep_going) {
        Ok(unstaged) => staged_changes(&StagedInput {
            changes,
            unstaged: Some(&unstaged),
        }),
        Err(TooComplex) => vec![false; changes.len()],
    }
}
