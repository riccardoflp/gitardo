//! `lfs.download`: downloads the content of files left as pointers, into the working tree for the commit checked out,
//! or into the store of the repository for another commit (D84).

use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Refusal, Step, Target, Text, Undo};
use model::lfs::LfsDownload;

/// The characters `--include` reads as patterns or as the end of one, so that a path holding them cannot be named alone.
const PATTERN_CHARACTERS: [char; 4] = [',', '*', '?', '['];
/// Said when the target is not files stored with Git LFS.
const NO_FILES: &str = "Choose the files stored with Git LFS to download.";
/// Said when another commit is asked of a repository with no remote.
const NO_REMOTE: &str = "This repository has no remote to download the files of another commit from.";
/// Why Undo has nothing to take back.
const NOTHING_TO_TAKE_BACK: &str = "Nothing to take back: the files become what is committed.";

/// Pulls the files of the commit checked out with `git lfs pull --include=<path>,…`, a folder as `<folder>/**`, or
/// every file with `git lfs pull`; fetches a file of another commit into the store with
/// `git lfs fetch --include=<path> <remote> <commit>`, the remote being the upstream's of the branch checked out or the
/// default one. Refuses without Git LFS, and a path `--include` cannot name alone (Q1192).
///
/// Feature: lfs.download
#[must_use]
pub fn download(context: &Context, _answers: &Answers) -> Outcome {
    let Target::LfsFiles(download) = &context.target else {
        return Outcome::Refuse(Refusal::saying(NO_FILES));
    };
    if let Some(refusal) = super::not_installed(context) {
        return Outcome::Refuse(refusal);
    }
    if let Some(path) = download
        .paths
        .iter()
        .find(|path| path.path.contains(PATTERN_CHARACTERS))
    {
        return Outcome::Refuse(Refusal::saying(&format!(
            "{} cannot be named to Git LFS on its own: download all the files instead.",
            path.path
        )));
    }
    let repository = &context.repository;
    let is_checked_out = download
        .commit
        .as_ref()
        .is_none_or(|commit| repository.head.commit.as_ref() == Some(commit));
    let include = include(download);
    let (arguments, title) = if is_checked_out {
        let mut arguments = vec![String::from("lfs"), String::from("pull")];
        arguments.extend(include.clone().map(|include| format!("--include={include}")));
        (arguments, title_of(download, "Download"))
    } else {
        let Some(remote) = remote_of(context) else {
            return Outcome::Refuse(Refusal::saying(NO_REMOTE));
        };
        let commit = download.commit.as_ref().map(ToString::to_string).unwrap_or_default();
        let mut arguments = vec![String::from("lfs"), String::from("fetch")];
        arguments.extend(include.clone().map(|include| format!("--include={include}")));
        arguments.extend([remote, commit]);
        (arguments, title_of(download, "Fetch"))
    };
    Outcome::Run(Plan {
        title: Text::from(title),
        steps: vec![Step::git(repository.folder.clone(), arguments)],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(NOTHING_TO_TAKE_BACK)),
        after: Vec::<FollowUp>::new(),
        on_failure: Vec::new(),
    })
}

/// The value of `--include`: each path, a folder as `<folder>/**`, joined by commas; none for every file.
fn include(download: &LfsDownload) -> Option<String> {
    (!download.paths.is_empty()).then(|| {
        download
            .paths
            .iter()
            .map(|path| {
                if path.is_folder {
                    format!("{}/**", path.path.trim_end_matches('/'))
                } else {
                    path.path.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(",")
    })
}

/// The remote another commit is fetched from: the upstream's of the branch checked out, or the default remote.
fn remote_of(context: &Context) -> Option<String> {
    let repository = &context.repository;
    let checked_out = repository.checked_out_branch();
    repository
        .branches
        .iter()
        .find(|branch| Some(branch.name.as_str()) == checked_out)
        .and_then(|branch| branch.upstream.as_ref())
        .map(|upstream| upstream.remote.clone())
        .or_else(|| model::remote::default(&repository.remotes))
}

/// The title of the plan: **Download art/logo.psd with Git LFS**, or of every file.
fn title_of(download: &LfsDownload, verb: &str) -> String {
    match download.paths.as_slice() {
        [] => format!("{verb} every file stored with Git LFS"),
        [one] => format!("{verb} {} with Git LFS", one.path),
        many => format!("{verb} {} files and folders with Git LFS", many.len()),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
