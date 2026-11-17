//! The repositories of a workspace: how they are called in lists, which one a file is in, which changes ask for a read
//! of the one shown and when it is read.

mod group;
mod hidden;
mod schedule;
mod summary;
pub(crate) mod watch;

use std::path::{Path, PathBuf};

pub use group::{TabGroup, group_tabs};
pub use hidden::{HiddenTab, schedule_hidden};
pub use schedule::{ReadAction, ReadEvent, ReadSchedule, StartedRead, schedule_reads};
pub use summary::{SummaryHead, WorktreeSummary};
pub use watch::sort_tasks_change;
pub use watch::{
    Change, FOCUS_AFTER, FilesChange, FocusChange, LATER, ReadCause, ReadRequest, SOON, TreeChange, nested_worktrees,
    sort_tree_change, watch_changes,
};

/// A repository a window can show: one a folder of the workspace is in or holds, or a submodule of one of these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repository {
    /// The top folder of its working tree, as git names it.
    pub folder: PathBuf,
    /// The top folder of the repository it is a submodule of; none for a repository of its own.
    pub superproject: Option<PathBuf>,
    /// The git folder its worktrees share (`repo.common-folder`); none for a submodule, and when git could not say.
    pub common_folder: Option<PathBuf>,
}

/// Whether the case of a letter counts when two paths are compared: not on Windows, where `C:\Work` and `c:\work` are
/// one folder.
const CASE_COUNTS: bool = !cfg!(windows);

/// The names of folders in the lists of the toolbar, the picker and the status bar: the path of each from the first
/// workspace folder it is inside, after the name of that folder (`work/app` for `app` inside the folder `work`), or its
/// own name when it is a workspace folder itself or inside none. Slashes are forward slashes on every system.
///
/// Feature: repo.name-repositories
#[must_use]
pub fn name_repositories(roots: &[PathBuf], folders: &[PathBuf]) -> Vec<String> {
    folders.iter().map(|folder| repository_name(roots, folder)).collect()
}

/// The name of one folder, from the workspace folders.
fn repository_name(roots: &[PathBuf], folder: &Path) -> String {
    let folder_segments = segments(folder);
    let own_name = folder_segments.last().cloned().unwrap_or_default();
    for root in roots {
        let root_segments = segments(root);
        let Some(inside) = strip_segments(&folder_segments, &root_segments) else {
            continue;
        };
        let (Some(root_name), false) = (root_segments.last(), inside.is_empty()) else {
            return own_name;
        };
        let mut words = vec![root_name.as_str()];
        words.extend(inside.iter().map(String::as_str));
        return words.join("/");
    }
    own_name
}

/// The repository a file is in: the innermost repository of the workspace whose folder holds it, so that a file in a
/// submodule is the submodule's. The folder of a repository itself is that repository, for the history of a folder. A
/// file in no repository of the workspace has no owner.
///
/// Feature: repo.owner-of-file
#[must_use]
pub fn owner_of_file<'a>(path: &Path, repositories: &'a [Repository]) -> Option<&'a Repository> {
    let file = segments(path);
    repositories
        .iter()
        .filter_map(|repository| {
            let folder = segments(&repository.folder);
            strip_segments(&file, &folder).map(|_| (folder.len(), repository))
        })
        .max_by_key(|(depth, _)| *depth)
        .map(|(_, repository)| repository)
}

/// The path of a file or a folder from a folder that holds it, with `/` between names: empty for the folder itself,
/// none when the folder does not hold it. Case counts in the names as it does on the system.
#[must_use]
pub fn path_in_folder(path: &Path, folder: &Path) -> Option<String> {
    let path = segments(path);
    let rest = strip_segments(&path, &segments(folder))?;
    Some(rest.join("/"))
}

/// The names of the folders of a path, from its root. Both slashes separate them, so that a path written on Windows
/// reads the same on every system; `.` is left out.
fn segments(path: &Path) -> Vec<String> {
    path.to_string_lossy()
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .map(str::to_owned)
        .collect()
}

/// What is left of a path after the folder it starts with, none when it does not start with it.
fn strip_segments<'a>(path: &'a [String], folder: &[String]) -> Option<&'a [String]> {
    let starts = path.len() >= folder.len()
        && path
            .iter()
            .zip(folder)
            .all(|(segment, folder_segment)| same_name(segment, folder_segment));
    if starts { path.get(folder.len()..) } else { None }
}

/// Whether two names of folders are the same folder.
fn same_name(first: &str, second: &str) -> bool {
    if CASE_COUNTS {
        first == second
    } else {
        first.to_lowercase() == second.to_lowercase()
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{Repository, name_repositories, owner_of_file};

    fn paths(texts: &[&str]) -> Vec<PathBuf> {
        texts.iter().map(PathBuf::from).collect()
    }

    fn repository(folder: &str, superproject: Option<&str>) -> Repository {
        Repository {
            folder: PathBuf::from(folder),
            superproject: superproject.map(PathBuf::from),
            common_folder: None,
        }
    }

    #[test]
    fn repo_name_repositories__a_repository_inside_a_workspace_folder_is_named_from_that_folder() {
        let names = name_repositories(&paths(&["/home/me/work"]), &paths(&["/home/me/work/app"]));
        assert_eq!(names, ["work/app"]);
    }

    #[test]
    fn repo_name_repositories__a_repository_that_is_a_workspace_folder_is_named_by_its_folder() {
        let roots = paths(&["/home/me/app", "/home/me/other"]);
        let names = name_repositories(&roots, &paths(&["/home/me/app", "/srv/outside"]));
        assert_eq!(names, ["app", "outside"]);
    }

    #[test]
    fn repo_name_repositories__a_submodule_on_windows_is_named_with_forward_slashes() {
        let names = name_repositories(&paths(&[r"C:\work"]), &paths(&[r"C:\work\app\lib\core"]));
        assert_eq!(names, ["work/app/lib/core"]);
    }

    #[test]
    fn repo_name_repositories__the_first_workspace_folder_that_holds_it_names_it() {
        let roots = paths(&["/home/me", "/home/me/work"]);
        let names = name_repositories(&roots, &paths(&["/home/me/work/app"]));
        assert_eq!(names, ["me/work/app"]);
    }

    #[test]
    fn repo_owner_of_file__a_file_in_a_submodule_is_the_submodule_s() {
        let repositories = [
            repository("/w/app", None),
            repository("/w/app/lib/core", Some("/w/app")),
        ];
        let owner = owner_of_file(Path::new("/w/app/lib/core/x.rs"), &repositories);
        assert_eq!(owner, Some(&repositories[1]));
    }

    #[test]
    fn repo_owner_of_file__a_file_of_the_repository_is_the_repository_s() {
        let repositories = [
            repository("/w/app", None),
            repository("/w/app/lib/core", Some("/w/app")),
        ];
        let owner = |path: &str| owner_of_file(Path::new(path), &repositories);
        assert_eq!(owner("/w/app/README.md"), Some(&repositories[0]));
        assert_eq!(owner("/w/app/lib/core"), Some(&repositories[1]));
        assert_eq!(owner("/w/app/lib/core-other/x"), Some(&repositories[0]));
    }

    #[test]
    fn repo_owner_of_file__a_file_outside_every_repository_has_no_owner() {
        let repositories = [repository("/w/app", None)];
        assert_eq!(owner_of_file(Path::new("/w/other/x.rs"), &repositories), None);
        assert_eq!(owner_of_file(Path::new("/w"), &repositories), None);
    }
}
