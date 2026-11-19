//! The sides of a diff stored with Git LFS (`lfs.show-in-pages`): each side that is a pointer at a path the repository
//! stores with Git LFS read from the store of the repository (`lfs.read-content`), never from the server.

use git::Git;
use git::read::diff::{DiffRequest, FileVersion, VersionRequest};
use model::diff::DiffSides;
use model::lfs::{LfsContent, SideOfPage, StoredSide, StoredWithLfs};

/// The sides with the contents of their pointers, and whether any was a pointer; or, when a side is a pointer whose
/// content is not in the store, the file as the page shows it with **Download**.
///
/// # Errors
///
/// The file stored with Git LFS, when a side is not downloaded.
pub fn contents(git: &Git, request: &DiffRequest, sides: DiffSides) -> Result<(DiffSides, bool), Box<StoredWithLfs>> {
    let left = side(git, &request.left, sides.left);
    let right = side(git, &request.right, sides.right);
    let stored: Vec<(StoredSide, &VersionRequest)> = [
        (&left, &request.left, SideOfPage::Before),
        (&right, &request.right, SideOfPage::After),
    ]
    .into_iter()
    .filter_map(|(read, version, place)| {
        read.pointer.as_ref().map(|(size, oid, is_downloaded)| {
            (
                StoredSide {
                    side: place,
                    size: *size,
                    oid: oid.clone(),
                    is_downloaded: *is_downloaded,
                },
                version,
            )
        })
    })
    .collect();
    if let Some((_, version)) = stored.iter().find(|(side, _)| !side.is_downloaded) {
        let commit = match &version.version {
            FileVersion::Commit(commit) => Some(commit.clone()),
            FileVersion::Absent
            | FileVersion::Index
            | FileVersion::Base
            | FileVersion::WorkingTree { .. }
            | FileVersion::Marked => None,
        };
        return Err(Box::new(StoredWithLfs {
            path: version.path.clone(),
            commit,
            sides: stored.into_iter().map(|(side, _)| side).collect(),
        }));
    }
    let is_stored = !stored.is_empty();
    Ok((
        DiffSides {
            left: left.bytes,
            right: right.bytes,
        },
        is_stored,
    ))
}

/// A side as read: its bytes, the content in place of a pointer when it is in the store, and the size, the id and
/// whether it is downloaded of a pointer.
struct Side {
    /// The bytes shown.
    bytes: Option<Vec<u8>>,
    /// The size, the id and whether the content is in the store, of a side that is a pointer.
    pointer: Option<(u64, String, bool)>,
}

/// A side with the content of its pointer, when it is one at a path the repository stores with Git LFS.
fn side(git: &Git, version: &VersionRequest, bytes: Option<Vec<u8>>) -> Side {
    let Some(found) = bytes
        .as_deref()
        .and_then(|bytes| super::file_view::stored_content(git, &version.path, bytes))
    else {
        return Side { bytes, pointer: None };
    };
    let (pointer, content) = found;
    match content {
        LfsContent::Downloaded(content) => Side {
            bytes: Some(content),
            pointer: Some((pointer.size, pointer.oid, true)),
        },
        LfsContent::NotDownloaded => Side {
            bytes,
            pointer: Some((pointer.size, pointer.oid, false)),
        },
    }
}
