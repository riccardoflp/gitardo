//! `lfs.store-committed`: stages again, as pointers, the files a pattern of Git LFS takes that were committed before
//! the pattern was there; the commits before keep them as they are (Q1193).

use model::changes::pathspec;
use model::contract::{AnswerKey, Answers, Context, Danger, FollowUp, Outcome, Plan, Query, Refusal, Step, Text, Undo};

/// The files to store, read before the plan or given by the offer of `lfs.track`.
pub const FILES: AnswerKey = AnswerKey("files");
/// The words of the offer, after `lfs.track` and in the menu of a pattern.
pub const OFFER: &str = "Store them with Git LFS in the next commit";
/// The pathspec that takes the files every pattern of Git LFS takes.
const EVERY_PATTERN: &str = ":(attr:filter=lfs)";
/// Said when there is nothing to store.
const NOTHING: &str = "Every file the patterns of Git LFS take is stored with it already.";
/// Why Undo has nothing to take back.
const NOT_TAKEN_BACK: &str = "Not taken back by Undo: unstaging the files does.";

/// Runs `git add --renormalize -- <path>…` on the files git tracks that a pattern of Git LFS takes and that Git LFS
/// does not store yet, so that the next commit stores them with it. Refuses without Git LFS, and when every such file
/// is stored already.
///
/// Feature: lfs.store-committed
#[must_use]
pub fn store_committed(context: &Context, answers: &Answers) -> Outcome {
    if let Some(refusal) = super::not_installed(context) {
        return Outcome::Refuse(refusal);
    }
    let Some(files) = answers.names(FILES) else {
        return Outcome::Read(
            FILES,
            Query::LfsToStore {
                pathspecs: vec![String::from(EVERY_PATTERN)],
            },
        );
    };
    let title = match files {
        [] => return Outcome::Refuse(Refusal::saying(NOTHING)),
        [one] => format!("Store {one} with Git LFS in the next commit"),
        many => format!("Store {} files with Git LFS in the next commit", many.len()),
    };
    let mut arguments = vec![
        std::ffi::OsString::from("add"),
        std::ffi::OsString::from("--renormalize"),
        std::ffi::OsString::from("--"),
    ];
    arguments.extend(files.iter().map(|file| pathspec(file)));
    Outcome::Run(Plan {
        title: Text::from(title),
        steps: vec![Step::git(context.repository.folder.clone(), arguments)],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(NOT_TAKEN_BACK)),
        after: vec![FollowUp::Say(Text::from(
            "Staged as pointers: the next commit stores them with Git LFS, and the commits before keep them as they are.",
        ))],
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
