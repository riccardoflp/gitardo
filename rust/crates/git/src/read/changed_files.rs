//! The files a diff lists: with their status and names from `git diff --name-status -z`, or only their names from
//! `--name-only -z` and `ls-tree --name-only -z`.

use model::state::{ChangeKind, FileChange};

use crate::GitError;
use crate::read::fields::unexpected;

/// What is being read, for the errors.
const WHAT: &str = "the files of a diff";

/// The files that `git diff --name-status -z` printed, in the order printed: each a status and a path, a rename or a
/// copy its score and both paths, the old one first.
///
/// # Errors
///
/// When a status is not one git documents, or a path is missing after it.
pub fn parse_name_status(output: &[u8]) -> Result<Vec<FileChange>, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut fields = text.split('\0').filter(|field| !field.is_empty());
    let mut files = Vec::new();
    while let Some(status) = fields.next() {
        let kind = change_kind(status).ok_or_else(|| unexpected(WHAT, status))?;
        let mut path = || fields.next().map(str::to_owned).ok_or_else(|| unexpected(WHAT, status));
        let file = match kind {
            ChangeKind::Renamed | ChangeKind::Copied => {
                let original = path()?;
                FileChange {
                    path: path()?,
                    original_path: Some(original),
                    kind,
                }
            }
            ChangeKind::Added | ChangeKind::Modified | ChangeKind::Deleted | ChangeKind::TypeChanged => FileChange {
                path: path()?,
                original_path: None,
                kind,
            },
        };
        files.push(file);
    }
    Ok(files)
}

/// The names that `--name-only -z` printed, in the order printed.
#[must_use]
pub fn parse_name_only(output: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(output)
        .split('\0')
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
}

/// What the letter of a status says, the score after it left aside: `R087` is a rename. A file still in conflict, `U`,
/// is changed as far as a diff goes.
fn change_kind(status: &str) -> Option<ChangeKind> {
    Some(match status.get(..1)? {
        "A" => ChangeKind::Added,
        "C" => ChangeKind::Copied,
        "D" => ChangeKind::Deleted,
        "M" | "U" => ChangeKind::Modified,
        "R" => ChangeKind::Renamed,
        "T" => ChangeKind::TypeChanged,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_name_only, parse_name_status};
    use model::state::ChangeKind;

    #[test]
    fn reads_renames_with_both_names_and_refuses_what_it_does_not_know() {
        let files = parse_name_status(b"M\0a b.txt\0R087\0old\0new\0T\0link\0").unwrap();
        assert_eq!(files.len(), 3);
        assert_eq!(files[1].original_path.as_deref(), Some("old"));
        assert_eq!(files[1].path, "new");
        assert_eq!(files[1].kind, ChangeKind::Renamed);
        assert_eq!(files[2].kind, ChangeKind::TypeChanged);
        assert!(parse_name_status(b"X\0what\0").is_err());
        assert!(parse_name_status(b"R100\0only one\0").is_err());
        assert_eq!(parse_name_status(b"").unwrap(), Vec::new());
        assert_eq!(parse_name_only(b"a\0b c\0"), ["a", "b c"]);
    }
}
