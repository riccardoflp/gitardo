//! Many files merged by git at once: each file's base, current and incoming versions are committed on two branches of a
//! throwaway repository, and `git merge-tree` merges them in each style of markers.

use std::collections::BTreeMap;

use testkit::Repository;

/// The most files merged in one repository, so that the command that stages them stays short enough for Windows.
const FILES_IN_A_REPOSITORY: usize = 300;

/// A file to merge: its base, absent when both sides added it, and its two sides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Case {
    /// The base.
    pub base: Option<String>,
    /// The current side, committed on `main`.
    pub current: String,
    /// The incoming side, committed on `incoming`.
    pub incoming: String,
}

/// The style of git's markers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// `merge`, git's default.
    Merge,
    /// `diff3`, with the base between the sides.
    Diff3,
    /// `zdiff3`, with the base and the lines both sides share outside the markers.
    Zdiff3,
}

impl Style {
    /// Every style.
    pub const ALL: [Style; 3] = [Style::Merge, Style::Diff3, Style::Zdiff3];

    /// The value of `merge.conflictStyle`.
    fn name(self) -> &'static str {
        match self {
            Style::Merge => "merge",
            Style::Diff3 => "diff3",
            Style::Zdiff3 => "zdiff3",
        }
    }
}

/// What git wrote for each file, in the order of the cases, merged in a style.
pub fn merged(cases: &[Case], style: Style) -> Vec<String> {
    merged_in_styles(cases, &[style]).into_iter().next().unwrap_or_default()
}

/// What git wrote for each file, in the order of the cases, for each style in order.
pub fn merged_in_styles(cases: &[Case], styles: &[Style]) -> Vec<Vec<String>> {
    let mut written = vec![Vec::with_capacity(cases.len()); styles.len()];
    for some in cases.chunks(FILES_IN_A_REPOSITORY) {
        for (all, more) in written.iter_mut().zip(merged_together(some, styles)) {
            all.extend(more);
        }
    }
    written
}

/// The name of the file of a case.
fn name(index: usize) -> String {
    format!("case-{index:05}.txt")
}

/// Commits a version of each file that has one.
fn commit(repository: &mut Repository, message: &str, files: &[(String, &str)]) {
    let files: Vec<(&str, &str)> = files.iter().map(|(name, text)| (name.as_str(), *text)).collect();
    let _ = repository.commit(message, &files);
}

/// The files of some cases merged in one repository.
fn merged_together(cases: &[Case], styles: &[Style]) -> Vec<Vec<String>> {
    let mut repository = Repository::new();
    let version = |pick: fn(&Case) -> Option<&str>| -> Vec<(String, &str)> {
        cases
            .iter()
            .enumerate()
            .filter_map(|(index, case)| pick(case).map(|text| (name(index), text)))
            .collect()
    };
    commit(&mut repository, "Add the base", &version(|case| case.base.as_deref()));
    repository.branch("incoming");
    commit(&mut repository, "Change on main", &version(|case| Some(&case.current)));
    repository.checkout("incoming");
    commit(
        &mut repository,
        "Change on incoming",
        &version(|case| Some(&case.incoming)),
    );
    repository.checkout("main");
    let trees: Vec<String> = styles.iter().map(|style| merge_tree(&repository, *style)).collect();
    let objects = objects(&repository);
    trees
        .iter()
        .map(|tree| {
            let blobs = blobs(&repository, tree);
            (0..cases.len())
                .map(|index| objects[&blobs[&name(index)]].clone())
                .collect()
        })
        .collect()
}

/// Merges `incoming` into `main` without a working tree, and returns the tree git wrote, conflicts and all.
fn merge_tree(repository: &Repository, style: Style) -> String {
    let setting = format!("merge.conflictStyle={}", style.name());
    let output = repository
        .command("git")
        .args([
            "-c",
            &setting,
            "merge-tree",
            "--write-tree",
            "--no-messages",
            "main",
            "incoming",
        ])
        .output()
        .expect("git runs");
    assert!(
        output.status.code().is_some_and(|code| code <= 1),
        "git merge-tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let printed = String::from_utf8(output.stdout).expect("git prints UTF-8");
    printed.lines().next().expect("a tree").to_owned()
}

/// The object of each file of a tree, by name.
fn blobs(repository: &Repository, tree: &str) -> BTreeMap<String, String> {
    repository
        .git(&["ls-tree", "-r", tree])
        .lines()
        .map(|line| {
            let (about, name) = line.split_once('\t').expect("a tab before the name");
            let object = about.split(' ').nth(2).expect("an object");
            (name.to_owned(), object.to_owned())
        })
        .collect()
}

/// Every object of the repository that is text, by name.
fn objects(repository: &Repository) -> BTreeMap<String, String> {
    let output = repository
        .command("git")
        .args(["cat-file", "--batch-all-objects", "--batch"])
        .output()
        .expect("git runs");
    assert!(output.status.success(), "git cat-file failed");
    let mut found = BTreeMap::new();
    let mut rest = output.stdout.as_slice();
    while let Some(end) = rest.iter().position(|byte| *byte == b'\n') {
        let header = String::from_utf8_lossy(&rest[..end]).into_owned();
        let mut words = header.split(' ');
        let (object, size) = (words.next().expect("an object"), words.nth(1).expect("a size"));
        let size: usize = size.parse().expect("a size is a number");
        let contents = &rest[end + 1..end + 1 + size];
        if let Ok(text) = std::str::from_utf8(contents) {
            let _ = found.insert(object.to_owned(), text.to_owned());
        }
        rest = &rest[end + 2 + size..];
    }
    found
}
