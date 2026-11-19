//! The read of the file of a File page on a worker (`file-view.page`): from the disk for the working tree, or from git
//! at a revision (`file-view.read-revision`).

use std::path::{Path, PathBuf};

use git::read::file_view::{RevisionFileRequest, read_revision};
use git::{CancelToken, Git};
use model::CommitId;
use views::file_view::FilePageId;

/// What a read of a file is given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileJob {
    /// The page it is for.
    pub page: FilePageId,
    /// The path from the top of the working tree, with `/`.
    pub path: String,
    /// The commit whose version is read; none for the file on disk.
    pub revision: Option<CommitId>,
}

/// Reads the bytes of a file: none when it is not there, on disk or at the revision.
///
/// # Errors
///
/// What the system or git said when the file is there and could not be read.
pub fn read_file(folder: &Path, job: &FileJob, cancel: &CancelToken) -> Result<Option<Vec<u8>>, String> {
    match &job.revision {
        Some(commit) => {
            let request = RevisionFileRequest {
                commit: commit.clone(),
                path: job.path.clone(),
            };
            read_revision(&Git::new(folder).with_cancel(cancel.clone()), &request).map_err(|error| error.to_string())
        }
        None => match std::fs::read(on_disk(folder, &job.path)) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        },
    }
}

/// Reads an image of the repository and draws it (`file-view.read-image`); none when it is not there, is not an image or
/// could not be read, which the preview shows as its text in its place. An SVG is drawn one pixel to a logical pixel,
/// which the preview lays out an image by.
#[must_use]
pub fn read_image(folder: &Path, job: &FileJob, cancel: &CancelToken) -> Option<render::Image> {
    // A read that fails is not said: the preview shows the text of the image in its place, as for any image not drawn.
    let bytes = read_file(folder, job, cancel).ok().flatten()?;
    render::image::read_image(&bytes, &job.path, 1.0)
        .ok()
        .map(|read| read.image)
}

/// Where a file of the working tree is on disk; a whole path, such as that of the user's settings file, is itself.
fn on_disk(folder: &Path, path: &str) -> PathBuf {
    if Path::new(path).is_absolute() {
        return PathBuf::from(path);
    }
    let mut file = folder.to_path_buf();
    file.extend(path.split('/').filter(|part| !part.is_empty()));
    file
}
