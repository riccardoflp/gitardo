//! What the operations on a change of a Diff page share: the change they were given, the versions of its file read
//! again, whether the change is still where the page showed it, and the plan that gives its patch to `git apply`.

use std::borrow::Cow;
use std::ffi::OsString;
use std::ops::Range;
use std::path::PathBuf;

use model::changes::pathspec;
use model::contract::{AnswerKey, Answers, Context, Danger, Outcome, Plan, Query, Refusal, Step, Target, Text, Undo};
use model::diff::{DiffChange, DiffSide, FileVersions};

/// The key under which the versions of the file come back.
pub const VERSIONS: AnswerKey = AnswerKey("versions");

/// Said when an operation on a change is given none.
const CHOOSE_A_CHANGE: &str = "Choose a change of a Diff page.";
/// Said when the change is no longer where the page showed it.
pub(super) const CHANGED: &str = "The file changed since the diff was shown. Try again.";
/// Said when a patch would carry characters of a file that is not UTF-8.
const NEEDS_UTF8: &str = "Staging a change needs UTF-8 text: stage the whole file.";
/// Said when a patch would touch the first line of a file that starts with a byte order mark.
const BYTE_ORDER_MARK: &str =
    "The first line of a file with a byte order mark cannot be staged as a change: stage the whole file.";

/// The bytes a file starts with when it has a byte order mark in UTF-8.
const UTF8_BYTE_ORDER_MARK: [u8; 3] = [0xef, 0xbb, 0xbf];

/// The change an operation was invoked on.
pub(super) fn target_change(context: &Context) -> Result<&DiffChange, Refusal> {
    match &context.target {
        Target::DiffChange(change) => Ok(change),
        _ => Err(Refusal::saying(CHOOSE_A_CHANGE)),
    }
}

/// The versions of the file of a change read again, when they are.
pub(super) fn versions(answers: &Answers) -> Option<&FileVersions> {
    answers.file_versions(VERSIONS)
}

/// The read that asks for the versions of the file of a change, with a commit's.
pub(super) fn read_versions(change: &DiffChange, commit: Option<&model::CommitId>) -> Outcome {
    Outcome::Read(
        VERSIONS,
        Query::FileVersions {
            path: change.path.clone(),
            commit: commit.cloned(),
        },
    )
}

/// The bytes of the version a side shows, among those read; none where the file is not, and for **Empty**.
pub(super) fn side_text<'a>(side: &DiffSide, versions: &'a FileVersions) -> Option<&'a [u8]> {
    match side {
        DiffSide::Head | DiffSide::Commit(_) => versions.commit.as_deref(),
        DiffSide::Index => versions.index.as_deref(),
        DiffSide::WorkingTree => versions.working_tree.as_deref(),
        DiffSide::Empty => None,
    }
}

/// A version as text, with what is not UTF-8 replaced, as the page shows it.
pub(super) fn text_of(bytes: Option<&[u8]>) -> Cow<'_, str> {
    String::from_utf8_lossy(bytes.unwrap_or_default())
}

/// Whether both sides of a change still have, at its lines, the text the page showed there.
pub(super) fn is_where_shown(change: &DiffChange, versions: &FileVersions) -> bool {
    let left = side_text(&change.left, versions);
    let right = side_text(&change.right, versions);
    has_lines(left, &change.left_lines, &change.left_text) && has_lines(right, &change.right_lines, &change.right_text)
}

/// Whether a text has these lines at this place.
fn has_lines(bytes: Option<&[u8]>, place: &Range<usize>, shown: &[String]) -> bool {
    let text = text_of(bytes);
    let lines = text::diff::lines(&text);
    lines
        .get(place.clone())
        .is_some_and(|found| found.iter().eq(shown.iter()))
}

/// Why a patch cannot be given to `git apply`, if it cannot: it carries characters of a file that is not UTF-8, or it
/// starts at the first line of a file that starts with a byte order mark.
pub(super) fn patch_refusal(patch: &str, texts: &[Option<&[u8]>]) -> Option<Refusal> {
    let is_utf8 = texts.iter().flatten().all(|bytes| std::str::from_utf8(bytes).is_ok());
    if !is_utf8 && !patch.is_ascii() {
        return Some(Refusal::saying(NEEDS_UTF8));
    }
    let has_mark = texts
        .iter()
        .flatten()
        .any(|bytes| bytes.starts_with(&UTF8_BYTE_ORDER_MARK));
    if has_mark && first_old_start(patch).is_some_and(|start| start <= 1) {
        return Some(Refusal::saying(BYTE_ORDER_MARK));
    }
    None
}

/// The line of the index the first difference of a patch starts at: the `12` of `@@ -12,0 +13,2 @@`.
fn first_old_start(patch: &str) -> Option<usize> {
    let header = patch.lines().find_map(|line| line.strip_prefix("@@ -"))?;
    let digits: String = header.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// The plan that gives a patch to `git apply --cached --unidiff-zero --whitespace=nowarn -`, which changes the index
/// alone; Undo does not take it back, since the index has no reflog: `by` says what does.
pub(super) fn apply_plan(context: &Context, title: String, (patch, by): (String, &str)) -> Plan {
    let arguments = ["apply", "--cached", "--unidiff-zero", "--whitespace=nowarn", "-"];
    let step = Step::Git {
        arguments: arguments.into_iter().map(OsString::from).collect(),
        directory: context.repository.folder.clone(),
        input: Some(patch.into_bytes()),
    };
    index_plan(title, vec![step], by)
}

/// A plan that changes the index alone, which Undo does not take back: `by` says what does.
pub(super) fn index_plan(title: String, steps: Vec<Step>, by: &str) -> Plan {
    Plan {
        title: Text::from(title),
        steps,
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(by)),
        after: Vec::new(),
        on_failure: Vec::new(),
    }
}

/// A git command with one file after `--`, named as itself (`changes.pathspec`).
pub(super) fn with_file(folder: PathBuf, arguments: &[&str], path: &str) -> Step {
    let mut all: Vec<OsString> = arguments.iter().map(OsString::from).collect();
    all.push(OsString::from("--"));
    all.push(pathspec(path));
    Step::git(folder, all)
}
