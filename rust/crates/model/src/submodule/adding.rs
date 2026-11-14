//! What adding a submodule proposes and refuses while it is typed (`submodule.add`).

use crate::contract::Text;

/// The folder proposed for a submodule: the last part of its URL or path, without `.git`.
/// `https://github.com/owner/lib.git`, `git@github.com:owner/lib.git` and `../lib/` all give `lib`. Backslashes count
/// as slashes.
#[must_use]
pub fn folder_of_url(url: &str) -> String {
    let url = url.trim().replace('\\', "/");
    let last = url
        .trim_end_matches('/')
        .rsplit(['/', ':'])
        .next()
        .unwrap_or_default()
        .to_owned();
    last.strip_suffix(".git").map_or(last.clone(), str::to_owned)
}

/// A folder typed from the root of the repository, with `/` between its parts and none at its ends.
#[must_use]
pub fn typed_folder(text: &str) -> String {
    text.trim().replace('\\', "/").trim_matches('/').to_owned()
}

/// What is wrong with a folder typed for a new submodule: **Enter a folder** when there is none; **Enter a folder
/// inside the repository** for one starting with `-` or `/`, a drive letter, or one with `..`; **A submodule is there
/// already** for the folder of one. Backslashes count as slashes.
#[must_use]
pub fn folder_problem(text: &str, submodules: &[String]) -> Option<Text> {
    let raw = text.trim().replace('\\', "/");
    let folder = typed_folder(text);
    let has_drive =
        raw.as_bytes().get(1) == Some(&b':') && raw.chars().next().is_some_and(|first| first.is_ascii_alphabetic());
    let is_outside = raw.starts_with(['-', '/']) || has_drive || folder.split('/').any(|part| part == "..");
    if folder.is_empty() {
        Some(Text::from("Enter a folder"))
    } else if is_outside {
        Some(Text::from("Enter a folder inside the repository"))
    } else if submodules.contains(&folder) {
        Some(Text::from("A submodule is there already"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{folder_of_url, folder_problem};

    #[test]
    fn proposes_the_last_part_of_the_url_without_git() {
        for url in [
            "https://github.com/owner/lib.git",
            "git@github.com:owner/lib.git",
            "../lib/",
            "..\\lib",
            "https://example.com/acme/lib",
        ] {
            assert_eq!(folder_of_url(url), "lib", "{url}");
        }
    }

    #[test]
    fn refuses_folders_outside_the_repository_and_those_taken() {
        let taken = [String::from("vendor/a")];
        let cases = [
            ("", Some("Enter a folder")),
            ("../lib", Some("Enter a folder inside the repository")),
            ("/tmp/lib", Some("Enter a folder inside the repository")),
            ("C:/lib", Some("Enter a folder inside the repository")),
            ("c:\\lib", Some("Enter a folder inside the repository")),
            ("-lib", Some("Enter a folder inside the repository")),
            ("vendor/../../lib", Some("Enter a folder inside the repository")),
            ("vendor\\a", Some("A submodule is there already")),
            ("vendor/lib", None),
            ("lib", None),
        ];
        for (typed, expected) in cases {
            let problem = folder_problem(typed, &taken);
            assert_eq!(problem.as_ref().map(crate::contract::Text::as_str), expected, "{typed}");
        }
    }
}
