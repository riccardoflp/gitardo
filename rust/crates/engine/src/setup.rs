//! The links and the copies a worktree gets of what git ignores in the checkout it comes from
//! (`worktree-setup.link-and-copy-steps`): nothing is made where the worktree has something already, nor under a folder
//! of the worktree that is a link or a file, and a copy keeps the links it meets as links.

use std::fs;
use std::io;
use std::path::Path;

use model::contract::Step;

/// Whether a link or a copy may be made, or why it is not.
pub(crate) enum Room {
    /// Nothing is in its way.
    Free,
    /// It is not made, for this reason, which the log says.
    Taken(String),
}

/// Whether the link or the copy of a step has room in the worktree: nothing at its path, a broken link included, and
/// no link or file in the way among the folders it goes in, from the top of the worktree.
pub(crate) fn room(step: &Step) -> Room {
    let (Some(made), Some(destination)) = (step.made_path(), destination(step)) else {
        return Room::Free;
    };
    let shown = slashed(&made);
    if fs::symlink_metadata(destination).is_ok() {
        return Room::Taken(format!("{shown} is in the worktree already."));
    }
    let depth = made.components().count();
    let top = destination.ancestors().nth(depth);
    let mut folder = top.map_or_default(Path::to_path_buf);
    for part in made.components().take(depth.saturating_sub(1)) {
        folder.push(part);
        let Ok(metadata) = fs::symlink_metadata(&folder) else {
            break; // A folder not there yet is made, and nothing under it can be in the way.
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            let in_the_way = folder.strip_prefix(top.unwrap_or(Path::new(""))).unwrap_or(&folder);
            return Room::Taken(format!(
                "{shown} is not made: {} in the worktree is a link or a file.",
                slashed(in_the_way)
            ));
        }
    }
    Room::Free
}

/// The path a link or a copy makes.
fn destination(step: &Step) -> Option<&Path> {
    match step {
        Step::Link { from, .. } => Some(from),
        Step::Copy { to, .. } => Some(to),
        _ => None, // Other steps make nothing from another place.
    }
}

/// A path with `/` between its parts, as the log says paths from the top of a worktree.
fn slashed(path: &Path) -> String {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Makes `from` a link to the folder `to`: a junction on Windows.
pub(crate) fn link(from: &Path, to: &Path) -> io::Result<()> {
    if let Some(folder) = from.parent() {
        fs::create_dir_all(folder)?;
    }
    system::link_folder(from, to).map_err(io::Error::other)
}

/// Copies a file, or a folder with all it has, keeping the links it meets as links.
pub(crate) fn copy(from: &Path, to: &Path) -> io::Result<()> {
    if let Some(folder) = to.parent() {
        fs::create_dir_all(folder)?;
    }
    copy_into(from, to)
}

fn copy_into(from: &Path, to: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(from)?;
    if metadata.file_type().is_symlink() {
        return copy_link(from, to, &metadata);
    }
    if !metadata.is_dir() {
        return fs::copy(from, to).map(|_bytes| ());
    }
    fs::create_dir(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        copy_into(&entry.path(), &to.join(entry.file_name()))?;
    }
    Ok(())
}

/// Makes `to` a link to what the link `from` points at, from where `from` is when it says a relative path.
fn copy_link(from: &Path, to: &Path, metadata: &fs::Metadata) -> io::Result<()> {
    let target = fs::read_link(from)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileTypeExt;
        if metadata.file_type().is_symlink_dir() {
            let absolute = from
                .parent()
                .map_or_else(|| target.clone(), |folder| folder.join(&target));
            return system::link_folder(to, &absolute).map_err(io::Error::other);
        }
        std::os::windows::fs::symlink_file(&target, to)
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        std::os::unix::fs::symlink(&target, to)
    }
}
