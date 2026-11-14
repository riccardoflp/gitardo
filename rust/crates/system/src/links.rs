//! Links to folders: what a new worktree gets in place of a folder of the checkout it comes from
//! (`worktree-setup.link-and-copy-steps`).

use std::path::Path;

use crate::error::SystemError;

/// Makes `link` a link to the folder `target`: a junction on Windows, which needs no rights, and a symbolic link
/// elsewhere. Nothing may be at `link` yet.
///
/// # Errors
///
/// When something is at `link` already, its folder is not there, or the system refuses.
pub fn link_folder(link: &Path, target: &Path) -> Result<(), SystemError> {
    #[cfg(windows)]
    return crate::windows::make_junction(link, target);
    #[cfg(not(windows))]
    return std::os::unix::fs::symlink(target, link).map_err(|error| SystemError::system("symlink", &error));
}

#[cfg(test)]
mod tests {
    use super::link_folder;
    use std::fs;

    #[test]
    fn links_a_folder_and_reads_through_the_link() {
        let base = std::env::temp_dir().join(format!("gitardo-link-{}", std::process::id()));
        let _left = fs::remove_dir_all(&base);
        let target = base.join("target folder");
        fs::create_dir_all(target.join("inner")).unwrap();
        fs::write(target.join("inner").join("file.txt"), "kept").unwrap();
        let link = base.join("link");
        link_folder(&link, &target).unwrap();
        assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_eq!(fs::read_to_string(link.join("inner").join("file.txt")).unwrap(), "kept");
        assert!(
            link_folder(&link, &target).is_err(),
            "a second link over the first is refused"
        );
        // Removing the link leaves the folder it pointed at with all it has.
        fs::remove_dir(&link).or_else(|_| fs::remove_file(&link)).unwrap();
        assert_eq!(
            fs::read_to_string(target.join("inner").join("file.txt")).unwrap(),
            "kept"
        );
        fs::remove_dir_all(&base).unwrap();
    }
}
