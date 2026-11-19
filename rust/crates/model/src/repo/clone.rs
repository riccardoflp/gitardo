//! The folder of a clone: its name from the URL of the repository (`repo.folder-from-url`), and what a name typed for
//! it must be (`repo.clone`).

use std::path::{Path, PathBuf};

use crate::contract::Text;

/// The name the folder of a clone gets by default: the last part of the URL or the path, after its last `/`, its last
/// `\` or the `:` of `host:path`, once the slashes at its end are taken away, without `.git` or `.bundle` at its end;
/// for a path that ends with the folder `.git`, the folder above it. None when nothing is left.
///
/// Feature: repo.folder-from-url
#[must_use]
pub fn folder_from_url(url: &str) -> Option<String> {
    let trimmed = url.trim().trim_end_matches(['/', '\\']);
    let trimmed = trimmed.strip_suffix(".git").map_or(trimmed, |without| {
        // A path that ends with the folder `.git` names the repository by the folder above it.
        if without.ends_with(['/', '\\']) {
            without.trim_end_matches(['/', '\\'])
        } else {
            without
        }
    });
    let last = trimmed.rsplit(['/', '\\', ':']).next().unwrap_or(trimmed);
    let name = last
        .strip_suffix(".git")
        .or_else(|| last.strip_suffix(".bundle"))
        .unwrap_or(last);
    // A URL whose last part is its host, as `https://example.com/`, names no repository.
    let is_host = url.contains("://") && trimmed.matches('/').count() <= 2;
    (!name.is_empty() && !is_host).then(|| name.to_owned())
}

/// What a name typed for the folder of a clone must be: the name of one new folder, or of one that is empty, in the
/// folder that holds the clone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloneFolderRule {
    /// The folder that holds the clone.
    pub parent: PathBuf,
    /// The names of what is in it and is not an empty folder: a file, or a folder with something in it.
    pub taken: Vec<String>,
}

impl CloneFolderRule {
    /// What is wrong with a name typed, or nothing when the clone can go there: **Enter a name**; **Enter the name of
    /// one folder** for a name with `/`, `\` or `..`, or one that starts with `-`; **`<full path>` is not empty**.
    #[must_use]
    pub fn problem(&self, typed: &str) -> Option<Text> {
        let name = typed.trim();
        if name.is_empty() {
            return Some(Text::from("Enter a name"));
        }
        if name.contains(['/', '\\']) || name.contains("..") || name.starts_with('-') {
            return Some(Text::from("Enter the name of one folder"));
        }
        let is_taken = self.taken.iter().any(|taken| same_name(taken, name));
        is_taken.then(|| Text::from(format!("{} is not empty", self.parent.join(name).display())))
    }

    /// Where the clone goes, said under the box while a name it can take is typed: **In `<full path>`**.
    #[must_use]
    pub fn hint(&self, typed: &str) -> Option<Text> {
        let name = typed.trim();
        (self.problem(typed).is_none()).then(|| Text::from(format!("In {}", self.parent.join(name).display())))
    }
}

/// Whether two names of folders are the same folder: whatever the case on Windows and macOS, whose disks do not tell
/// the cases apart, exactly elsewhere.
fn same_name(left: &str, right: &str) -> bool {
    if cfg!(any(windows, target_os = "macos")) {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    }
}

/// The full path of the folder of a clone.
#[must_use]
pub fn clone_folder(parent: &Path, name: &str) -> PathBuf {
    parent.join(name.trim())
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

    use std::path::PathBuf;

    use super::{CloneFolderRule, folder_from_url};

    #[test]
    fn repo_folder_from_url__urls_ssh_and_paths_on_disk_give_the_name_of_the_repository() {
        for url in [
            "https://github.com/owner/lib.git",
            "git@github.com:owner/lib.git",
            "ssh://host:2222/srv/lib.git/",
            "C:\\repos\\lib\\",
        ] {
            assert_eq!(folder_from_url(url).as_deref(), Some("lib"), "{url}");
        }
    }

    #[test]
    fn repo_folder_from_url__a_git_folder_gives_the_folder_above_it_and_a_bundle_its_name() {
        assert_eq!(folder_from_url("C:\\repos\\lib\\.git").as_deref(), Some("lib"));
        assert_eq!(folder_from_url("/srv/repos/lib/.git/").as_deref(), Some("lib"));
        assert_eq!(
            folder_from_url("https://example.com/acme/tools.bundle").as_deref(),
            Some("tools")
        );
    }

    #[test]
    fn repo_folder_from_url__a_url_with_no_path_gives_none() {
        assert_eq!(folder_from_url("https://example.com/"), None);
        assert_eq!(folder_from_url(""), None);
    }

    #[test]
    fn a_name_of_a_folder_with_something_in_it_is_refused_and_one_free_says_where_the_clone_goes() {
        let rule = CloneFolderRule {
            parent: PathBuf::from("work"),
            taken: vec![String::from("lib")],
        };
        let full = PathBuf::from("work").join("lib");
        assert_eq!(
            rule.problem("lib").map(|text| text.to_string()),
            Some(format!("{} is not empty", full.display()))
        );
        assert_eq!(rule.problem("tools"), None);
        assert_eq!(
            rule.hint("tools").map(|text| text.to_string()),
            Some(format!("In {}", PathBuf::from("work").join("tools").display()))
        );
        for (typed, said) in [
            ("", "Enter a name"),
            ("a/b", "Enter the name of one folder"),
            ("..", "Enter the name of one folder"),
            ("-lib", "Enter the name of one folder"),
        ] {
            assert_eq!(
                rule.problem(typed).map(|text| text.to_string()).as_deref(),
                Some(said),
                "{typed}"
            );
        }
    }
}
