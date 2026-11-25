//! Which repositories of a workspace share a tab (`repo.group-tabs`): a repository, its worktrees and its submodules.

use std::path::Path;

use super::Repository;

/// The repositories that share a tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabGroup {
    /// The repository the tab is named after: the main worktree of the group when it is a repository of the
    /// workspace, otherwise the first of the group found.
    pub repository: Repository,
    /// The other repositories of the group, in the order they were found: its other worktrees, and its submodules
    /// with theirs, each after the repository it is in.
    pub members: Vec<Repository>,
}

impl TabGroup {
    /// Whether a folder is the repository of the group or one of its members.
    #[must_use]
    pub fn holds(&self, folder: &Path) -> bool {
        same_folder(&self.repository.folder, folder)
            || self.members.iter().any(|member| same_folder(&member.folder, folder))
    }
}

/// The repositories of a workspace grouped by tab, in the order they were found: a worktree in the group of the
/// repository whose common git folder it shares, a submodule in the group of the repository around it, and so a
/// submodule of a submodule; any other repository is a group of its own.
///
/// Feature: repo.group-tabs
#[must_use]
pub fn group_tabs(repositories: &[Repository]) -> Vec<TabGroup> {
    let mut groups: Vec<TabGroup> = Vec::new();
    for repository in repositories {
        match groups.iter_mut().find(|group| belongs(group, repository)) {
            Some(group) => group.members.push(repository.clone()),
            None => groups.push(TabGroup {
                repository: repository.clone(),
                members: Vec::new(),
            }),
        }
    }
    for group in &mut groups {
        let main = group.members.iter().position(is_main_worktree);
        if let Some(main) = main.filter(|_| !is_main_worktree(&group.repository)) {
            let first = std::mem::replace(&mut group.repository, group.members.remove(main));
            group.members.insert(0, first);
        }
    }
    groups
}

/// Whether a repository goes in a group: a submodule of one of its repositories, or a worktree that shares its common
/// git folder.
fn belongs(group: &TabGroup, repository: &Repository) -> bool {
    if let Some(superproject) = &repository.superproject {
        return group.holds(superproject);
    }
    let Some(common) = &repository.common_folder else {
        return false;
    };
    std::iter::once(&group.repository)
        .chain(&group.members)
        .filter(|other| other.superproject.is_none())
        .any(|other| {
            other
                .common_folder
                .as_deref()
                .is_some_and(|other| same_folder(other, common))
        })
}

/// Whether a repository is the main worktree of its repository: the one whose `.git` is the common git folder.
fn is_main_worktree(repository: &Repository) -> bool {
    repository.superproject.is_none()
        && repository
            .common_folder
            .as_deref()
            .is_some_and(|common| same_folder(common, &repository.folder.join(".git")))
}

/// Whether two folders are the same: on Windows the case and the slashes do not count.
pub(super) fn same_folder(first: &Path, second: &Path) -> bool {
    let key = |folder: &Path| {
        let text = folder.to_string_lossy().replace('\\', "/");
        let text = text.trim_end_matches('/').to_owned();
        if super::CASE_COUNTS { text } else { text.to_lowercase() }
    };
    key(first) == key(second)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use super::{Repository, group_tabs};

    /// A repository of its own, with its common git folder.
    fn repository(folder: &str, common: &str) -> Repository {
        Repository {
            folder: PathBuf::from(folder),
            superproject: None,
            common_folder: Some(PathBuf::from(common)),
        }
    }

    /// A submodule of a repository.
    fn submodule(folder: &str, superproject: &str) -> Repository {
        Repository {
            folder: PathBuf::from(folder),
            superproject: Some(PathBuf::from(superproject)),
            common_folder: None,
        }
    }

    /// The folders of each group, its repository first.
    fn folders(repositories: &[Repository]) -> Vec<Vec<String>> {
        group_tabs(repositories)
            .iter()
            .map(|group| {
                std::iter::once(&group.repository)
                    .chain(&group.members)
                    .map(|member| member.folder.to_string_lossy().into_owned())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn repo_group_tabs__a_worktree_shares_the_tab_of_its_repository() {
        let repositories = [
            repository("/w/api", "/w/api/.git"),
            repository("/w/api-login", "/w/api/.git"),
        ];
        assert_eq!(folders(&repositories), [["/w/api", "/w/api-login"]]);
    }

    #[test]
    fn repo_group_tabs__a_submodule_and_its_own_submodule_have_no_tab_of_their_own() {
        let repositories = [
            repository("/w/api", "/w/api/.git"),
            submodule("/w/api/lib/core", "/w/api"),
            submodule("/w/api/lib/core/vendor", "/w/api/lib/core"),
        ];
        assert_eq!(
            folders(&repositories),
            [["/w/api", "/w/api/lib/core", "/w/api/lib/core/vendor"]]
        );
    }

    #[test]
    fn repo_group_tabs__repositories_that_share_no_common_folder_and_are_not_inside_each_other_are_two_tabs() {
        let repositories = [repository("/w/api", "/w/api/.git"), repository("/w/web", "/w/web/.git")];
        assert_eq!(folders(&repositories), [["/w/api"], ["/w/web"]]);
    }

    #[test]
    fn repo_group_tabs__the_main_worktree_names_the_tab_when_a_worktree_was_found_first() {
        let repositories = [
            repository("/w/api-login", "/w/api/.git"),
            repository("/w/api", "/w/api/.git"),
        ];
        assert_eq!(folders(&repositories), [["/w/api", "/w/api-login"]]);
    }

    #[test]
    fn repo_group_tabs__a_repository_with_no_other_worktree_and_no_submodule_is_alone_in_its_group() {
        let groups = group_tabs(&[repository("/w/api", "/w/api/.git")]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].members, []);
    }

    #[test]
    fn repo_group_tabs__on_windows_the_case_and_the_slashes_of_the_common_folder_do_not_count() {
        if !cfg!(windows) {
            return;
        }
        let repositories = [
            repository(r"C:\w\api", "C:/w/api/.git"),
            repository(r"C:\w\api-login", r"c:\W\API\.git"),
        ];
        assert_eq!(folders(&repositories).len(), 1);
    }
}
