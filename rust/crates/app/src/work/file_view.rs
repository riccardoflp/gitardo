//! The read of the file of a File page on a worker (`file-view.page`): from the disk for the working tree, or from git
//! at a revision (`file-view.read-revision`).

use std::path::{Path, PathBuf};

use git::read::file_view::{RevisionFileRequest, read_revision};
use git::{CancelToken, Git};
use model::CommitId;
use model::lfs::{LfsContent, LfsPointer, SideOfPage, StoredSide, StoredWithLfs};
use views::file_view::FilePageId;

/// How many pixels the picture of an SVG has for each logical pixel: two, so that it stays sharp on a screen that has
/// as many, and is drawn smoothly scaled down on one that has fewer, as a diagram of Mermaid is.
pub const DRAW_SCALE: f32 = 2.0;

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

/// What a read of the file of a File page gives: its bytes as [`read_file`] reads them, and the file as stored with
/// Git LFS when its content is not shown (`lfs.show-in-pages`).
pub type ShownRead = (Result<Option<Vec<u8>>, String>, Option<Box<StoredWithLfs>>);

/// Reads the file of a File page as [`read_file`] does, and, when its bytes are a pointer of Git LFS at a path the
/// repository stores with it (`lfs.show-in-pages`): at a revision, the content from the store of the repository when it
/// is there (`lfs.read-content`); in the working tree, or when the content is not there, the file as stored with Git
/// LFS, which the page shows in place of the lines of its pointer.
pub fn read_shown(folder: &Path, job: &FileJob, cancel: &CancelToken) -> ShownRead {
    let result = read_file(folder, job, cancel);
    let Ok(Some(bytes)) = &result else {
        return (result, None);
    };
    let git = Git::new(folder).with_cancel(cancel.clone());
    match stored_content(&git, &job.path, bytes) {
        None => (result, None),
        Some((_, LfsContent::Downloaded(content))) if job.revision.is_some() => (Ok(Some(content)), None),
        Some((pointer, content)) => {
            let stored = StoredWithLfs {
                path: job.path.clone(),
                commit: job.revision.clone(),
                sides: vec![StoredSide {
                    side: SideOfPage::Only,
                    size: pointer.size,
                    oid: pointer.oid,
                    is_downloaded: matches!(content, LfsContent::Downloaded(_)),
                }],
            };
            (result, Some(Box::new(stored)))
        }
    }
}

/// The pointer some bytes are and the content it stands for in the store, when they are a pointer of Git LFS at a path
/// the repository stores with it; none for any other file, or when git cannot say (`git::read::lfs::stored_content`).
#[must_use]
pub fn stored_content(git: &Git, path: &str, bytes: &[u8]) -> Option<(LfsPointer, LfsContent)> {
    git::read::lfs::stored_content(git, path, bytes)
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
