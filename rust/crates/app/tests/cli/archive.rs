//! **Save as archive…**: the files of a commit, or of one folder of it, written by `git archive` in a zip or a tar.gz
//! saved inside the throwaway repository's `.git` folder, where nothing else looks.

use std::path::{Path, PathBuf};
use std::process::Command;

use testkit::Repository;

use crate::{cli, cli_done};

/// A repository whose commit tagged `v1.2.0` has `readme.txt` and `docs/guide.md`.
fn tagged() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit(
        "Add the readme and the guide",
        &[("readme.txt", "readme\n"), ("docs/guide.md", "# Guide\n")],
    );
    repository.tag("v1.2.0");
    repository
}

/// The name of the repository's folder, which the archive's names start with.
fn repository_name(repository: &Repository) -> String {
    repository
        .path()
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// A file of the repository's `.git` folder.
fn saved_at(repository: &Repository, name: &str) -> PathBuf {
    repository.path().join(".git").join(name)
}

/// Whether the bytes of a zip hold a file of that name: a zip keeps its names uncompressed.
fn zip_holds(path: &Path, name: &str) -> bool {
    let bytes = std::fs::read(path).unwrap_or_default();
    bytes.windows(name.len()).any(|window| window == name.as_bytes())
}

/// What `tar -tzf` lists of a tar.gz, run in its folder so that no tar reads a drive letter as a host.
fn tar_listing(path: &Path) -> String {
    let folder = path.parent().unwrap_or(path);
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    Command::new("tar")
        .args(["-tzf", &name])
        .current_dir(folder)
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default()
}

#[test]
fn archive_save__the_tag_proposes_its_name_and_the_zip_holds_its_files_under_the_folder_inside() {
    let repository = tagged();
    let name = format!("{}-v1.2.0", repository_name(&repository));
    let base = ["--target", "tag:v1.2.0", "archive.save", "holds=every", "format=zip"];
    let asked = cli(&repository, &base);
    assert_eq!(asked.code, Some(2));
    assert!(asked.errors.contains(&format!("{name}/")), "{}", asked.errors);
    let inside = format!("inside={name}/");
    let asked = cli(&repository, &[&base[..], &[inside.as_str()]].concat());
    assert!(asked.errors.contains(&format!("{name}.zip")), "{}", asked.errors);
    let path = saved_at(&repository, "release.zip");
    let answer = format!("path={}", path.display());
    let ended = cli_done(&repository, &[&base[..], &[inside.as_str(), answer.as_str()]].concat());
    let command = format!("git archive --format=zip --prefix={name}/ -o");
    assert!(ended.out.contains(&command), "{}", ended.out);
    assert!(ended.out.contains("refs/tags/v1.2.0"), "{}", ended.out);
    assert!(zip_holds(&path, &format!("{name}/readme.txt")));
    assert!(zip_holds(&path, &format!("{name}/docs/guide.md")));
}

#[test]
fn archive_save__one_folder_adds_it_after_two_dashes_and_the_archive_holds_only_its_files() {
    let repository = tagged();
    let base = ["--target", "tag:v1.2.0", "archive.save", "holds=folder"];
    let asked = cli(&repository, &base);
    assert_eq!(asked.code, Some(2));
    assert!(asked.errors.to_lowercase().contains("folder"), "{}", asked.errors);
    let path = saved_at(&repository, "docs.zip");
    let answer = format!("path={}", path.display());
    let answers = ["folder=docs", "format=zip", "inside=", answer.as_str()];
    let ended = cli_done(&repository, &[&base[..], &answers[..]].concat());
    assert!(ended.out.contains("-- docs"), "{}", ended.out);
    assert!(zip_holds(&path, "docs/guide.md"));
    assert!(!zip_holds(&path, "readme.txt"));
}

#[test]
fn archive_save__a_tar_gz_is_a_file_tar_lists() {
    let repository = tagged();
    let path = saved_at(&repository, "release.tar.gz");
    let answer = format!("path={}", path.display());
    let answers = ["holds=every", "format=tar.gz", "inside=release/", answer.as_str()];
    let ended = cli_done(
        &repository,
        &[&["--target", "tag:v1.2.0", "archive.save"][..], &answers[..]].concat(),
    );
    assert!(ended.out.contains("--format=tar.gz"), "{}", ended.out);
    let listing = tar_listing(&path);
    assert!(listing.contains("release/readme.txt"), "{listing}");
    assert!(listing.contains("release/docs/guide.md"), "{listing}");
}

#[test]
fn archive_save__an_empty_folder_inside_runs_without_prefix_and_the_files_are_at_the_top() {
    let repository = tagged();
    let path = saved_at(&repository, "top.tar.gz");
    let answer = format!("path={}", path.display());
    let answers = ["holds=every", "format=tar.gz", "inside=", answer.as_str()];
    let ended = cli_done(
        &repository,
        &[&["--target", "branch:main", "archive.save"][..], &answers[..]].concat(),
    );
    assert!(!ended.out.contains("--prefix"), "{}", ended.out);
    assert!(ended.out.contains("refs/heads/main"), "{}", ended.out);
    let listing = tar_listing(&path);
    assert!(listing.lines().any(|line| line == "readme.txt"), "{listing}");
}

#[test]
fn archive_save__cancelling_the_dialog_writes_nothing_and_the_log_has_the_command_once_saved() {
    let repository = tagged();
    let answers = ["holds=every", "format=zip", "inside="];
    let asked = cli(
        &repository,
        &[&["--target", "tag:v1.2.0", "archive.save"][..], &answers[..]].concat(),
    );
    assert_eq!(asked.code, Some(2), "it stops at the save dialog");
    assert!(!asked.out.contains("git archive"), "{}", asked.out);
    let written = std::fs::read_dir(repository.path().join(".git")).is_ok_and(|entries| {
        entries
            .flatten()
            .any(|entry| entry.path().extension().is_some_and(|extension| extension == "zip"))
    });
    assert!(!written, "no archive was written");
    let path = saved_at(&repository, "saved.zip");
    let answer = format!("path={}", path.display());
    let ended = cli_done(
        &repository,
        &[
            &["--target", "tag:v1.2.0", "archive.save"][..],
            &answers[..],
            &[answer.as_str()],
        ]
        .concat(),
    );
    assert_eq!(ended.out.matches("git archive").count(), 1, "{}", ended.out);
    assert!(
        ended.out.contains(&format!("Saved {}", path.display())),
        "{}",
        ended.out
    );
}
