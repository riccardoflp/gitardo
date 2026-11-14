//! The reads of the blame of a File page on a worker (`blame.current-line`, `blame.file`): who last changed the line
//! under the cursor, or each line of the file. Each waits first, so that a cursor that moves on, or a key typed after,
//! asks again before git runs: the store cancels the read it no longer wants, and a read cancelled while it waits
//! runs nothing.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use git::read::blame::{BlameFileRequest, BlameLineRequest, Whitespace, read_file, read_line};
use git::{CancelToken, Git};
use model::CommitId;
use model::blame::BlameLine;

/// How often a read that waits looks whether it was cancelled.
const LOOK_EVERY: Duration = Duration::from_millis(10);

/// A read of the blame of a File page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlameJob {
    /// The file, from the top of the working tree, with `/`.
    pub path: String,
    /// The commit whose version the page shows; none for the file of the working tree.
    pub revision: Option<CommitId>,
    /// The text of the page when it has edits not saved, given to git on its standard input.
    pub unsaved: Option<Vec<u8>>,
    /// Whether whitespace and moved lines are left to who wrote them (`blame.ignoreWhitespaceAndMoves`).
    pub ignores_whitespace: bool,
    /// The line to blame, from 1; none for every line of the file.
    pub line: Option<usize>,
    /// How long the read waits before it runs.
    pub delay: Duration,
}

/// What a read of the blame gives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlameAnswer {
    /// Who last changed the line; none when git has no blame for it.
    Line(Option<BlameLine>),
    /// Who last changed each line; none when git has no blame for the file.
    File(Option<Vec<Option<BlameLine>>>),
}

/// Waits, then reads the blame a job asks for in the working tree of a folder.
///
/// # Errors
///
/// What git said when it could not be started, or the read was cancelled while it waited.
pub fn read_blame(folder: &Path, job: &BlameJob, cancel: &CancelToken) -> Result<BlameAnswer, String> {
    let started = Instant::now();
    while started.elapsed() < job.delay {
        if cancel.is_cancelled() {
            return Err(String::from("cancelled"));
        }
        std::thread::sleep(LOOK_EVERY.min(job.delay.saturating_sub(started.elapsed())));
    }
    let git = Git::new(folder).with_cancel(cancel.clone());
    let whitespace = if job.ignores_whitespace {
        Whitespace::IgnoredWithMoves
    } else {
        Whitespace::Counted
    };
    let path = PathBuf::from(&job.path);
    let answer = if let Some(line) = job.line {
        let request = BlameLineRequest {
            path,
            line,
            whitespace,
            contents: job.unsaved.clone(),
        };
        read_line(&git, &request).map(BlameAnswer::Line)
    } else {
        let request = BlameFileRequest {
            path,
            whitespace,
            contents: job.unsaved.clone(),
            revision: job.revision.as_ref().map(|commit| commit.as_str().to_owned()),
        };
        read_file(&git, &request).map(BlameAnswer::File)
    };
    answer.map_err(|error| error.to_string())
}
