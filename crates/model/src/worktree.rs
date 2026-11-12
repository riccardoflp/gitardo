//! Worktrees: where a new one goes, and how the views name one.

use std::path::{Path, PathBuf};

/// The folder of the worktrees when `worktrees.folder` is empty.
pub const DEFAULT_FOLDER: &str = "${repo}.worktrees";

/// The most characters a part of the folder of a worktree keeps.
const PART_LENGTH: usize = 60;

/// What [`folder_path`] looks at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FolderInput<'a> {
    /// `worktrees.folder`, as the setting has it.
    pub setting: &'a str,
    /// The folder of the main worktree, whichever worktree asks.
    pub main_worktree: &'a Path,
    /// The home folder of the user, for `~`.
    pub home: &'a Path,
    /// The name of the worktree: the name of its branch, slashes and all.
    pub name: &'a str,
}

/// A path with forward slashes and none at its end.
fn slashed(path: &str) -> String {
    let path = path.replace('\\', "/");
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() && path.starts_with('/') {
        String::from("/")
    } else {
        trimmed.to_owned()
    }
}

/// Whether a path, with forward slashes, is absolute: `/srv`, `D:/trees`, `//server/share`.
fn is_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    path.starts_with('/') || matches!(bytes, [letter, b':', ..] if letter.is_ascii_alphabetic())
}

/// The folder of the worktrees: the setting, or its default, with `${repo}` and `~` filled, from the folder that holds
/// the main worktree when it is relative.
#[must_use]
pub fn worktrees_folder(setting: &str, main_worktree: &Path, home: &Path) -> String {
    let setting = if setting.trim().is_empty() {
        DEFAULT_FOLDER
    } else {
        setting.trim()
    };
    let main = slashed(&main_worktree.to_string_lossy());
    let (parent, repo) = main.rsplit_once('/').unwrap_or(("", main.as_str()));
    let filled = slashed(&setting.replace("${repo}", repo));
    let folder = if let Some(rest) = filled.strip_prefix('~') {
        format!("{}{rest}", slashed(&home.to_string_lossy()))
    } else if is_absolute(&filled) {
        filled
    } else {
        format!("{parent}/{filled}")
    };
    normalized(&folder)
}

/// A path without `.` and with each `..` taking away the part before it.
fn normalized(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "." => {}
            ".." if parts.last().is_some_and(|last| !last.is_empty() && *last != "..") => {
                let _ = parts.pop();
            }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}

/// The part of a folder made of a part of a name: runs of other characters than ASCII letters, digits, `_`, `.` and `-`
/// become one `-`, `-` and `.` are dropped from both ends, and it is cut to 60 characters.
fn folder_part(part: &str) -> String {
    let mut folder = String::new();
    for character in part.chars() {
        if character.is_ascii_alphanumeric() || "_.-".contains(character) {
            folder.push(character);
        } else if !folder.ends_with('-') {
            folder.push('-');
        }
    }
    let folder: String = folder.trim_matches(['-', '.']).chars().take(PART_LENGTH).collect();
    folder.trim_end_matches(['-', '.']).to_owned()
}

/// The folder of a new worktree: the folder of the worktrees, then the name part by part between slashes, each made a
/// safe folder name; `worktree` when nothing is left.
///
/// Feature: worktree.folder-path
#[must_use]
pub fn folder_path(input: &FolderInput<'_>) -> PathBuf {
    let folder = worktrees_folder(input.setting, input.main_worktree, input.home);
    let parts: Vec<String> = input
        .name
        .replace('\\', "/")
        .split('/')
        .map(folder_part)
        .filter(|part| !part.is_empty())
        .collect();
    let inside = if parts.is_empty() {
        String::from("worktree")
    } else {
        parts.join("/")
    };
    PathBuf::from(format!("{folder}/{inside}"))
}

/// How the views name a worktree: its path from the folder of the worktrees when it is inside it, compared without
/// regard to case, and the name of its folder otherwise.
#[must_use]
pub fn worktree_label(worktrees_folder: &str, worktree: &Path) -> String {
    let path = slashed(&worktree.to_string_lossy());
    let folder = slashed(worktrees_folder);
    let inside = path
        .get(..folder.len())
        .filter(|start| start.eq_ignore_ascii_case(&folder))
        .and_then(|_| path.get(folder.len()..))
        .and_then(|rest| rest.strip_prefix('/'))
        .filter(|rest| !rest.is_empty());
    match inside {
        Some(rest) => rest.to_owned(),
        None => path.rsplit('/').next().unwrap_or_default().to_owned(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::Path;

    use super::{FolderInput, folder_path, worktree_label, worktrees_folder};

    fn folder(setting: &str, name: &str) -> String {
        folder_path(&FolderInput {
            setting,
            main_worktree: Path::new("/home/someone/code/app"),
            home: Path::new("/home/someone"),
            name,
        })
        .to_string_lossy()
        .replace('\\', "/")
    }

    #[test]
    fn worktree_folder_path__every_example_of_the_table() {
        let cases = [
            ("", "fix-login", "/home/someone/code/app.worktrees/fix-login"),
            ("", "feature/login", "/home/someone/code/app.worktrees/feature/login"),
            (
                "../trees/${repo}",
                "feature/login",
                "/home/someone/trees/app/feature/login",
            ),
            ("~/wt", "fix: the #42 bug", "/home/someone/wt/fix-the-42-bug"),
            ("D:/wt", "../escape", "D:/wt/escape"),
            ("", "...", "/home/someone/code/app.worktrees/worktree"),
            ("D:\\wt\\", "a", "D:/wt/a"),
        ];
        for (setting, name, expected) in cases {
            assert_eq!(folder(setting, name), expected, "{setting} {name}");
        }
    }

    #[test]
    fn worktree_folder_path__from_another_worktree_it_is_beside_the_main_one() {
        // The main worktree is given whichever worktree asks: a worktree's own folder is never the base.
        let from_main = worktrees_folder("", Path::new("/home/someone/code/app"), Path::new("/home/someone"));
        assert_eq!(from_main, "/home/someone/code/app.worktrees");
    }

    #[test]
    fn worktree_folder_path__no_name_leaves_the_folder_of_the_worktrees() {
        let base = "/home/someone/code/app.worktrees/";
        for name in [
            "../../etc",
            "..",
            "a/../../b",
            "/abs/olute",
            "C:\\x",
            "~/home",
            "a/./b",
            "\u{e9}t\u{e9}/..",
            "",
        ] {
            let found = folder("", name);
            assert!(found.starts_with(base), "{name} gave {found}");
            assert!(
                !found[base.len()..].split('/').any(|part| part == ".." || part == "."),
                "{name} gave {found}"
            );
        }
        assert_eq!(folder("", &"x".repeat(80)).len(), base.len() + 60);
    }

    #[test]
    fn worktree_folder_path__the_label_is_the_path_inside_the_folder_of_the_worktrees() {
        let worktrees = "D:/Code/App.worktrees";
        assert_eq!(
            worktree_label(worktrees, Path::new("d:/code/app.worktrees/feature/login")),
            "feature/login"
        );
        assert_eq!(worktree_label(worktrees, Path::new("D:\\elsewhere\\hotfix")), "hotfix");
        assert_eq!(worktree_label(worktrees, Path::new("D:/Code/App.worktrees-old/x")), "x");
    }
}
