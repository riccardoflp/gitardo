//! `note.push`: the reference of notes sent to a remote, which `git push` does not do by itself.

use model::contract::{
    Answers, Context, Danger, FeatureId, FollowUp, OfferedRun, Outcome, Plan, Refusal, Step, Target, Text, Undo,
};

/// Pushes the reference of notes git uses to a remote; when the remote refuses because it has notes this repository has
/// not, offers **Fetch notes**.
///
/// Feature: note.push
#[must_use]
pub fn push(context: &Context, _answers: &Answers) -> Outcome {
    let remote = match super::remote_of(context) {
        Ok(remote) => remote,
        Err(refusal) => return Outcome::Refuse(refusal),
    };
    let notes = &context.repository.notes;
    if !notes.has_reference() {
        return Outcome::Refuse(Refusal::saying("This repository has no notes to push."));
    }
    let fetch = OfferedRun {
        label: Text::from("Fetch notes"),
        feature: FeatureId("note.fetch"),
        target: Target::Remote(remote.to_owned()),
        answers: Answers::new(),
        when_said: vec![String::from("fetch first"), String::from("non-fast-forward")],
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Push the notes to {remote}")),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["push", remote, notes.reference.as_str()],
        )],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Not taken back by Undo: the notes are on the remote.")),
        after: vec![FollowUp::Say(Text::from(format!("Pushed the notes to {remote}.")))],
        // What git said of the refusal is shown with the failure, and **Fetch notes** beside it (Q1458).
        on_failure: vec![FollowUp::OfferRun(Box::new(fetch))],
    })
}
