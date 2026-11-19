//! `lfs.set-up`: sets up the filter and the hooks of Git LFS for one repository, never the user's own configuration
//! (Q1194).

use model::contract::{
    AnswerKey, Answers, Context, Danger, FeatureId, FollowUp, OfferedRun, Outcome, Plan, Query, Refusal, Step, Target,
    Text, Undo,
};

/// The files stored with Git LFS left as pointers in the working tree, read before the plan.
pub const POINTERS: AnswerKey = AnswerKey("pointers");
/// Said when the repository is set up already.
const SET_UP_ALREADY: &str = "Git LFS is set up in this repository already.";
/// Why Undo has nothing to take back.
const NOT_TAKEN_BACK: &str = "Not taken back by Undo: git lfs uninstall --local, in a terminal, does.";

/// Runs `git lfs install --local`, which writes the configuration of the repository and its hooks; offers **Download
/// all** once it ran while files are left as pointers. Refuses without Git LFS, and once it is set up.
///
/// Feature: lfs.set-up
#[must_use]
pub fn set_up(context: &Context, answers: &Answers) -> Outcome {
    if let Some(refusal) = super::not_installed(context) {
        return Outcome::Refuse(refusal);
    }
    let lfs = &context.repository.lfs;
    if lfs.is_set_up {
        return Outcome::Refuse(Refusal::saying(SET_UP_ALREADY));
    }
    let Some(pointers) = answers.names(POINTERS) else {
        return Outcome::Read(
            POINTERS,
            Query::LfsPointers {
                version: lfs.version.clone().unwrap_or_default(),
            },
        );
    };
    let mut after = vec![FollowUp::Say(Text::from("Git LFS is set up in this repository."))];
    if !pointers.is_empty() {
        after.push(FollowUp::OfferRun(Box::new(OfferedRun {
            label: Text::from(download_all(pointers.len())),
            feature: FeatureId("lfs.download"),
            target: Target::LfsFiles(Box::default()),
            answers: Answers::new(),
            when_said: Vec::new(),
        })));
    }
    Outcome::Run(Plan {
        title: Text::from("Set up Git LFS in this repository"),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["lfs", "install", "--local"],
        )],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(NOT_TAKEN_BACK)),
        after,
        on_failure: Vec::new(),
    })
}

/// The words of **Download all** with the count of the files left as pointers.
fn download_all(count: usize) -> String {
    format!("Download all ({count})")
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
