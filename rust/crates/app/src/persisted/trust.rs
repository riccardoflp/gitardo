//! The folders the user trusts, under `trustedFolders`, as the system writes them and in the order they were trusted
//! (`trust.ask`, `trust.manage`). Every folder trusted is kept: the list is not cut as the recent ones are.

use std::path::{Path, PathBuf};

use json::Value;

use super::same_folder;

/// Trusts a folder from now on, after those trusted before; a folder trusted already stays where it is.
pub fn trust(list: &mut Vec<PathBuf>, folder: &Path) {
    if !list.iter().any(|trusted| same_folder(trusted, folder)) {
        list.push(folder.to_path_buf());
    }
}

/// Stops trusting some folders; the others keep their order.
pub fn forget(list: &mut Vec<PathBuf>, folders: &[PathBuf]) {
    list.retain(|trusted| !folders.iter().any(|forgotten| same_folder(trusted, forgotten)));
}

/// The folders trusted, as written.
pub(super) fn folders_value(list: &[PathBuf]) -> Value {
    Value::Array(
        list.iter()
            .map(|folder| Value::String(folder.to_string_lossy().into_owned()))
            .collect(),
    )
}

/// The folders trusted, read back: the strings of the list, none when it is not there.
pub(super) fn trusted_folders(document: &Value) -> Vec<PathBuf> {
    document
        .get("trustedFolders")
        .and_then(Value::as_array)
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(PathBuf::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{forget, trust};
    use crate::persisted::Persisted;

    #[test]
    fn the_folders_trusted_are_kept_once_each_in_their_order_and_read_back() {
        let mut persisted = Persisted::default();
        for number in 0..30 {
            trust(&mut persisted.trusted_folders, Path::new(&format!("/work/app{number}")));
        }
        trust(&mut persisted.trusted_folders, Path::new("/work/app3"));
        assert_eq!(persisted.trusted_folders.len(), 30, "none is cut, none is there twice");
        assert_eq!(persisted.trusted_folders.first(), Some(&PathBuf::from("/work/app0")));
        assert_eq!(Persisted::from_text(&persisted.to_text()), persisted);
        forget(
            &mut persisted.trusted_folders,
            &[PathBuf::from("/work/app0"), PathBuf::from("/work/app29")],
        );
        assert_eq!(persisted.trusted_folders.len(), 28);
        assert_eq!(persisted.trusted_folders.first(), Some(&PathBuf::from("/work/app1")));
        assert_eq!(Persisted::from_text(&persisted.to_text()), persisted);
    }

    #[test]
    fn a_file_without_trusted_folders_trusts_none() {
        assert_eq!(Persisted::from_text("{}").trusted_folders, Vec::<PathBuf>::new());
        assert!(!Persisted::default().to_text().contains("trustedFolders"));
    }
}
