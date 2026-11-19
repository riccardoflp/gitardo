//! `note.fetch`: the notes of a remote brought into the reference of notes, which `git fetch` does not do by itself;
//! merged with the notes here when each side has notes the other has not.

use model::contract::{
    AnswerKey, Answers, Context, Danger, FailureOffer, FollowUp, Outcome, Plan, Query, Refusal, Step, Text,
};
use model::note::Notes;

/// Whether the remote has the reference of notes, read before the plan.
pub const HAS_NOTES: AnswerKey = AnswerKey("has-notes");

/// What git says when it refuses to move the reference of notes because each side has notes the other has not.
const NOT_FORWARD: &str = "non-fast-forward";

/// Fetches the reference of notes of a remote into the one of the repository, `git fetch <remote>
/// <reference>:<reference>`, which moves it only forward; when git refuses, offers to merge the two, keeping the lines of
/// both notes of a commit once (Q1214).
///
/// Feature: note.fetch
#[must_use]
pub fn fetch(context: &Context, answers: &Answers) -> Outcome {
    let remote = match super::remote_of(context) {
        Ok(remote) => remote,
        Err(refusal) => return Outcome::Refuse(refusal),
    };
    let notes = &context.repository.notes;
    match answers.flag(HAS_NOTES) {
        None => {
            let query = Query::RemoteNotes {
                remote: remote.to_owned(),
                reference: notes.reference.clone(),
            };
            return Outcome::Read(HAS_NOTES, query);
        }
        Some(false) => return Outcome::Refuse(Refusal::saying(&format!("{remote} has no notes."))),
        Some(true) => {}
    }
    let folder = context.repository.folder.clone();
    let reference = &notes.reference;
    let fetched = format!("{reference}:{reference}");
    Outcome::Run(Plan {
        title: Text::from(format!("Fetch the notes of {remote}")),
        steps: vec![Step::git(folder, ["fetch", remote, fetched.as_str()])],
        danger: Danger::None,
        undo: super::undo(context),
        // How many commits have a note is not said: a plan cannot read once its steps ended (Q1458).
        after: vec![FollowUp::Say(Text::from(format!("Fetched the notes of {remote}.")))],
        on_failure: vec![FollowUp::OfferWhenFailed(Box::new(FailureOffer {
            when: String::from(NOT_FORWARD),
            plan: merge_plan(context, remote, notes),
        }))],
    })
}

/// **Merge the notes**: the notes of the remote fetched into `refs/notes/remotes/<remote>/<name>`, then merged into the
/// reference of notes with `cat_sort_uniq`, which puts the two notes of a commit one after the other, without a line
/// twice.
fn merge_plan(context: &Context, remote: &str, notes: &Notes) -> Plan {
    let folder = context.repository.folder.clone();
    let theirs = format!("refs/notes/remotes/{remote}/{}", notes.short_name());
    let fetched = format!("+{}:{theirs}", notes.reference);
    Plan {
        title: Text::from(format!("Merge the notes of {remote}")),
        steps: vec![
            Step::git(folder.clone(), ["fetch", remote, fetched.as_str()]),
            Step::git(folder, ["notes", "merge", "--strategy=cat_sort_uniq", theirs.as_str()]),
        ],
        danger: Danger::Confirm(Text::from(format!(
            "Both {remote} and this repository have notes the other has not. Merge them, keeping both notes of a commit \
             one after the other?"
        ))),
        undo: super::undo(context),
        after: vec![FollowUp::Say(Text::from(format!("Merged the notes of {remote}.")))],
        on_failure: Vec::new(),
    }
}
