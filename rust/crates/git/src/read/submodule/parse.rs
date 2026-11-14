//! What the reads of the submodules ask git, and how their answers are read.

use std::ffi::OsString;

use model::CommitId;
use model::submodule::Distance;

use crate::GitError;
use crate::read::fields::{arguments, commit_id, optional_commit_id, unexpected};
use crate::read::repo::parse_status;

/// What is being read, for the errors.
const WHAT: &str = "the submodules";

/// The mode git gives a submodule in a tree and in the index.
const SUBMODULE_MODE: &str = "160000";

/// A submodule as `.gitmodules` declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredSubmodule {
    /// Its name.
    pub name: String,
    /// Its folder from the top of the repository, with `/`.
    pub folder: String,
    /// Its URL.
    pub url: Option<String>,
    /// The branch it follows.
    pub branch: Option<String>,
}

/// `git config -z --file .gitmodules --get-regexp ^submodule\.`: every key of every submodule, each key and its value
/// with a line break between them and a NUL after; the exit code 1 when there is none.
#[must_use]
pub fn declared_arguments() -> Vec<OsString> {
    arguments(&["config", "-z", "--file", ".gitmodules", "--get-regexp", r"^submodule\."])
}

/// The submodules that `git config --get-regexp` printed, in the order their first key comes. A key is
/// `submodule.<name>.<key>`, and a name may hold dots. One with no folder is left out.
#[must_use]
pub fn parse_declared(output: &[u8]) -> Vec<DeclaredSubmodule> {
    let text = String::from_utf8_lossy(output);
    // A submodule whose folder is not read yet has an empty one.
    let mut declared: Vec<DeclaredSubmodule> = Vec::new();
    for record in text.split('\0').filter(|record| !record.is_empty()) {
        let (key, value) = record.split_once('\n').unwrap_or((record, ""));
        let Some((name, key)) = key.strip_prefix("submodule.").and_then(|rest| rest.rsplit_once('.')) else {
            continue;
        };
        if !declared.iter().any(|submodule| submodule.name == name) {
            declared.push(DeclaredSubmodule {
                name: name.to_owned(),
                folder: String::new(),
                url: None,
                branch: None,
            });
        }
        let Some(submodule) = declared.iter_mut().find(|submodule| submodule.name == name) else {
            continue;
        };
        match key {
            "path" => value.trim_end_matches('/').clone_into(&mut submodule.folder),
            "url" => submodule.url = Some(value.to_owned()),
            "branch" => submodule.branch = Some(value.to_owned()),
            // `update`, `ignore`, `shallow` and the others say nothing the list shows.
            _ => {}
        }
    }
    declared.retain(|submodule| !submodule.folder.is_empty());
    declared
}

/// `git ls-tree -z HEAD -- <folders>`: the commit recorded for each folder in the last commit. `ls-tree` takes paths,
/// not patterns with magic, so the folders are given as they are.
#[must_use]
pub fn recorded_arguments(folders: &[String]) -> Vec<OsString> {
    let mut words = arguments(&["ls-tree", "-z", "HEAD", "--"]);
    words.extend(folders.iter().map(OsString::from));
    words
}

/// The submodules that `git ls-tree -z` printed, `<mode> <type> <object>\t<path>`, with their commits. What is not a
/// submodule is left out.
///
/// # Errors
///
/// When a record is not of that form, or its object is not a hash.
pub fn parse_recorded(output: &[u8]) -> Result<Vec<(String, CommitId)>, GitError> {
    entries(output, ObjectField::Third)
}

/// `git ls-files -s -z -- <folders>`, each as `:(literal)<folder>`: the commit staged for each.
#[must_use]
pub fn staged_arguments(folders: &[String]) -> Vec<OsString> {
    let mut words = arguments(&["ls-files", "-s", "-z", "--"]);
    words.extend(
        folders
            .iter()
            .map(|folder| OsString::from(format!(":(literal){folder}"))),
    );
    words
}

/// The submodules that `git ls-files -s -z` printed, `<mode> <object> <stage>\t<path>`, with their commits.
///
/// # Errors
///
/// When a record is not of that form, or its object is not a hash.
pub fn parse_staged(output: &[u8]) -> Result<Vec<(String, CommitId)>, GitError> {
    entries(output, ObjectField::Second)
}

/// Where the object is among the three fields before the tab.
#[derive(Clone, Copy)]
enum ObjectField {
    /// Second, after the mode: `ls-files -s`.
    Second,
    /// Third, after the mode and the type: `ls-tree`.
    Third,
}

/// The submodules among the records of `ls-tree` or `ls-files -s`: three fields, a tab and the path.
fn entries(output: &[u8], object_field: ObjectField) -> Result<Vec<(String, CommitId)>, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut found = Vec::new();
    for record in text.split('\0').filter(|record| !record.is_empty()) {
        let (fields, path) = record.split_once('\t').ok_or_else(|| unexpected(WHAT, record))?;
        let fields: Vec<&str> = fields.split(' ').collect();
        let object = match object_field {
            ObjectField::Second => fields.get(1),
            ObjectField::Third => fields.get(2),
        };
        let (3, Some(mode), Some(object)) = (fields.len(), fields.first(), object) else {
            return Err(unexpected(WHAT, record));
        };
        if *mode == SUBMODULE_MODE {
            found.push((path.to_owned(), commit_id(WHAT, object)?));
        }
    }
    Ok(found)
}

/// `git status --porcelain=v2 --branch -z --untracked-files=all`, run in a submodule: its commit, its branch and what
/// is not committed in it.
#[must_use]
pub fn checkout_arguments() -> Vec<OsString> {
    arguments(&["status", "--porcelain=v2", "--branch", "-z", "--untracked-files=all"])
}

/// The commit checked out (none before the first commit), the branch (none when detached) and the number of files not
/// committed that `git status --porcelain=v2 --branch -z` printed.
///
/// # Errors
///
/// When a line is of no kind git documents, or the commit is not a hash.
pub fn parse_checkout(output: &[u8]) -> Result<(Option<CommitId>, Option<String>, usize), GitError> {
    let text = String::from_utf8_lossy(output);
    let mut commit = None;
    let mut branch = None;
    for field in text.split('\0') {
        if let Some(oid) = field.strip_prefix("# branch.oid ") {
            commit = if oid == "(initial)" {
                None
            } else {
                optional_commit_id(WHAT, oid)?
            };
        } else if let Some(head) = field.strip_prefix("# branch.head ") {
            branch = (head != "(detached)").then(|| head.to_owned());
        }
    }
    let uncommitted = parse_status(output)?.file_count();
    Ok((commit, branch, uncommitted))
}

/// `git rev-list --left-right --count <recorded>...<checked out> --`: how many commits each has that the other has not.
#[must_use]
pub fn distance_arguments(recorded: &CommitId, checked_out: &CommitId) -> Vec<OsString> {
    let mut words = arguments(&["rev-list", "--left-right", "--count"]);
    words.push(OsString::from(format!("{recorded}...{checked_out}")));
    words.push(OsString::from("--"));
    words
}

/// How far apart the two commits are, from the two counts `git rev-list --left-right --count` printed: the recorded
/// commit's first, then the checked out one's.
///
/// # Errors
///
/// When it did not print two numbers.
pub fn parse_distance(output: &[u8]) -> Result<Distance, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut counts = text.split_whitespace().map(str::parse::<u32>);
    match (counts.next(), counts.next(), counts.next()) {
        (Some(Ok(behind)), Some(Ok(ahead)), None) => Ok(Distance { ahead, behind }),
        _ => Err(unexpected(WHAT, &text)),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{parse_checkout, parse_declared, parse_distance, parse_recorded, parse_staged};

    #[test]
    fn submodule_read_list__reads_what_gitmodules_declares_names_with_dots_included() {
        let output = "submodule.lib.path\nlib\0submodule.lib.url\n../lib.git\0submodule.v1.2.path\nvendor/a b/\0\
                      submodule.v1.2.branch\nstable\0submodule.nofolder.url\nx\0";
        let declared = parse_declared(output.as_bytes());
        assert_eq!(declared.len(), 2);
        assert_eq!(declared[0].folder, "lib");
        assert_eq!(declared[0].url.as_deref(), Some("../lib.git"));
        assert_eq!(declared[1].name, "v1.2");
        assert_eq!(declared[1].folder, "vendor/a b");
        assert_eq!(declared[1].branch.as_deref(), Some("stable"));
    }

    #[test]
    fn submodule_read_list__keeps_only_the_submodules_of_a_tree_and_the_index() {
        let commit = "a".repeat(40);
        let blob = "b".repeat(40);
        let tree = format!("160000 commit {commit}\tlib\0100644 blob {blob}\tREADME.md\0");
        assert_eq!(parse_recorded(tree.as_bytes()).unwrap()[0].0, "lib");
        assert_eq!(parse_recorded(tree.as_bytes()).unwrap().len(), 1);
        let index = format!("160000 {commit} 0\tvendor/a b\0");
        assert_eq!(parse_staged(index.as_bytes()).unwrap()[0].0, "vendor/a b");
        assert!(parse_staged(b"160000 x\tlib\0").is_err());
    }

    #[test]
    fn submodule_read_list__reads_the_commit_the_branch_the_count_and_the_distance() {
        let commit = "c".repeat(40);
        let status = format!("# branch.oid {commit}\0# branch.head (detached)\0? new.txt\0");
        let (read_commit, branch, uncommitted) = parse_checkout(status.as_bytes()).unwrap();
        assert_eq!(read_commit.unwrap().as_str(), commit);
        assert_eq!((branch, uncommitted), (None, 1));
        let (_, branch, _) = parse_checkout(b"# branch.oid (initial)\0# branch.head main\0").unwrap();
        assert_eq!(branch.as_deref(), Some("main"));
        let distance = parse_distance(b"1\t2\n").unwrap();
        assert_eq!((distance.ahead, distance.behind), (2, 1));
        assert!(parse_distance(b"1\n").is_err());
    }
}
