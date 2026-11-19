//! A file git resolved from a resolution it recorded (`review.recorded`): what it wrote in place of the conflicts, as
//! the changes from the file as git marked it to the file on disk.

use git::read::rerere::read_marked;
use git::{Git, GitError};
use model::conflicts::{Side, SideNames};
use model::rerere::MarkedRequest;
use model::review::RecordedFile;

use super::LARGEST_FILE;
use super::kept::kept_changes;

/// What git wrote in a file it resolved as before; none for a file that is not text or is larger than 2 MB.
pub(super) fn read(git: &Git, path: &str, sides: &SideNames) -> Result<Option<RecordedFile>, GitError> {
    let name = |side: &Side| side.name.clone().unwrap_or_else(|| side.role.words().to_owned());
    let request = MarkedRequest {
        path: path.to_owned(),
        current: name(&sides.current),
        incoming: name(&sides.incoming),
    };
    let marked = read_marked(git, &request)?;
    let Ok(on_disk) = std::fs::read(git.directory().join(path)) else {
        return Ok(None);
    };
    let is_text = |bytes: &[u8]| bytes.len() <= LARGEST_FILE && !bytes.contains(&0);
    if !is_text(&marked) || !is_text(&on_disk) {
        return Ok(None);
    }
    let (marked, on_disk) = (String::from_utf8_lossy(&marked), String::from_utf8_lossy(&on_disk));
    let marked: Vec<&str> = text::diff::lines(&marked);
    let on_disk: Vec<&str> = text::diff::lines(&on_disk);
    Ok(Some(RecordedFile {
        path: path.to_owned(),
        changes: kept_changes(&marked, &on_disk),
    }))
}
