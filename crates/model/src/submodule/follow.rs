//! What happens in the submodules, in their own windows or in a terminal, followed here (`submodule.follow-changes`).

use crate::repo::Change;
use crate::repo::watch::{is_watched, parts};

/// Whether a change in the git folder of the repository is a commit or a checkout in one of its submodules, at any
/// depth of submodules inside submodules: the `HEAD` of a clone under `modules/`, or a file under its `refs/`. Lock
/// files and the files of the file system monitor of git mean nothing. A file written in a submodule's folder is a
/// change of the working tree, read after the delay those have (`repo.watch-changes`).
///
/// Feature: submodule.follow-changes
#[must_use]
pub fn follow_changes(change: &Change) -> bool {
    let Change::GitFolder(path) = change else {
        return false;
    };
    let parts = parts(path);
    let names: Vec<&str> = parts.iter().map(String::as_str).collect();
    is_watched(&parts) && is_reference_of_submodule(&names)
}

/// Whether the parts of a path of the git folder name the `HEAD` or a reference of the clone of a submodule:
/// `modules/lib/HEAD`, `modules/lib/refs/heads/main`, `modules/vendor/a/modules/b/HEAD`.
pub(crate) fn is_reference_of_submodule(names: &[&str]) -> bool {
    match names {
        ["modules", inside @ .., last] if !inside.is_empty() => {
            *last == "HEAD" || inside.iter().skip(1).any(|part| *part == "refs")
        }
        _ => false,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use super::follow_changes;
    use crate::repo::{Change, watch_changes};

    fn git(path: &str) -> Change {
        Change::GitFolder(PathBuf::from(path))
    }

    #[test]
    fn submodule_follow_changes__a_lock_file_appearing_in_the_clone_of_lib_reads_nothing_again() {
        for path in [
            "modules/lib/index.lock",
            "modules/lib/refs/heads/main.lock",
            "modules/lib/HEAD.lock",
        ] {
            assert!(!follow_changes(&git(path)), "{path}");
            assert_eq!(watch_changes(&git(path)), None, "{path}");
        }
        assert!(!follow_changes(&git("modules/lib/fsmonitor--daemon/cookies/1")));
    }

    #[test]
    fn submodule_follow_changes__a_commit_or_a_checkout_in_a_submodule_at_any_depth_reads_again() {
        for path in [
            "modules/lib/HEAD",
            "modules/lib/refs/heads/main",
            "modules/vendor/a/modules/b/HEAD",
            "modules/vendor/a/modules/b/refs/heads/feature",
        ] {
            assert!(follow_changes(&git(path)), "{path}");
            assert!(watch_changes(&git(path)).is_some(), "{path}");
        }
        assert!(!follow_changes(&git("modules/lib/objects/ab/cdef")));
        assert!(
            !follow_changes(&git("refs/heads/main")),
            "a branch of the repository itself is not a submodule's"
        );
    }
}
