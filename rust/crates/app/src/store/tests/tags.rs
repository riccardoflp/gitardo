//! The marks of the tags a remote lacks, as the window keeps them (`tag.remote-marks`): asked of the remote only when
//! **Check tags on `<remote>`** is chosen, and changed by Gitardo's own operations without asking it again.

use model::contract::{FollowUp, Text};
use model::state::Tag;
use model::tag::{RemoteTags, TagObject, TagOnRemote, TagsOnRemote};
use model::{CommitId, Timestamp};

use super::{folder, open, window};
use crate::store::feature::Ended;
use crate::store::state::Shown;
use crate::store::{Effect, JobKind, Message, State, reduce};

fn id(digit: char) -> CommitId {
    CommitId::parse(&digit.to_string().repeat(40)).unwrap_or_else(|| panic!("a hash"))
}

/// A window showing a repository whose tags are `v1` and `v2`.
fn showing() -> State {
    let mut state = window();
    let app = folder("app");
    let _effects = open(&mut state, &app, &[(&app, std::slice::from_ref(&app))]);
    if let Some(Shown::Read(read)) = state.workspace.as_mut().map(|workspace| &mut workspace.shown) {
        let repository = std::sync::Arc::make_mut(&mut read.state);
        repository.remotes = vec![String::from("origin")];
        repository.tags = ["v1", "v2"]
            .iter()
            .zip(['1', '2'])
            .map(|(name, digit)| Tag {
                name: (*name).to_owned(),
                commit: id(digit),
                date: Timestamp::default(),
            })
            .collect();
    }
    state
}

/// The marks of the repository shown, for each of its tags.
fn marks(state: &State) -> Option<Vec<TagOnRemote>> {
    let read = state.shown_read()?;
    let marks = state.rail.tag_marks.get(&read.folder)?;
    Some(read.state.tags.iter().map(|tag| marks.of(tag)).collect())
}

fn object(name: &str, digit: char) -> TagObject {
    TagObject {
        name: name.to_owned(),
        object: digit.to_string().repeat(40),
    }
}

#[test]
fn tag_remote_marks__nothing_is_asked_of_the_remote_and_nothing_marked_before_the_check() {
    let state = showing();
    assert!(
        !state
            .jobs
            .values()
            .any(|kind| matches!(kind, JobKind::ReadRemoteTags { .. }))
    );
    assert_eq!(marks(&state), None);
}

#[test]
fn tag_remote_marks__the_check_asks_origin_marks_v2_and_push_tag_takes_the_mark_away() {
    let mut state = showing();
    let effects = reduce(
        &mut state,
        Message::Intent(views::Intent::Rail(views::RailIntent::CheckTags(String::from(
            "origin",
        )))),
    );
    let Some(Effect::Start(job)) = effects.first() else {
        panic!("no job started: {effects:?}");
    };
    let JobKind::ReadRemoteTags { request, .. } = &job.kind else {
        panic!("another job: {:?}", job.kind);
    };
    assert_eq!(request.remote, "origin");
    let read = RemoteTags {
        remote: String::from("origin"),
        there: vec![object("v1", '1')],
        here: vec![object("v1", '1'), object("v2", '2')],
    };
    let _done = reduce(&mut state, Message::RemoteTagsRead(job.id, Ok(read)));
    assert_eq!(marks(&state), Some(vec![TagOnRemote::Pushed, TagOnRemote::NotPushed]));
    let pushed = Ended::Ran {
        title: Text::from("Push tag v2 to origin"),
        after: vec![FollowUp::TagsOnRemote(Box::new(TagsOnRemote::pushed(
            "origin",
            vec![String::from("v2")],
        )))],
        can_undo: false,
        prepared_message: None,
        waiting: None,
    };
    let _ended = reduce(
        &mut state,
        Message::FeatureEnded {
            ended: pushed,
            log: Vec::new(),
        },
    );
    assert_eq!(marks(&state), Some(vec![TagOnRemote::Pushed, TagOnRemote::Pushed]));
    assert!(
        !state
            .jobs
            .values()
            .any(|kind| matches!(kind, JobKind::ReadRemoteTags { .. })),
        "the remote is not asked again"
    );
}
