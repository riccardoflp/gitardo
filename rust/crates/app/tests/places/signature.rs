//! The line **Signature** of the inspector and of the page of a commit (`details.signature`), with commits signed by an
//! SSH key made for each test: what it says, that it waits for the details, and that a read no one waits for is
//! cancelled.

use app::store::{Effect, JobKind, Message, SignatureCheck, State};
use git::CancelToken;
use model::details::{CommitSignature, SignatureKind, SignatureVerdict};
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::details::{SignatureRead, SignatureState, signature_view};
use views::graph::{GraphIntent, Selected};

use super::commit_page::PAGE;
use super::inspector::{drawn, row_id, select};
use super::showing;
use super::support::{self, Window, says, says_in};

/// A repository with a commit not signed, then one signed with an SSH key whose allowed signers are set, or not.
fn signed_repository(is_allowed: bool) -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let key = repository.ssh_signing_key();
    if is_allowed {
        repository.allow_signer(&key);
    }
    let _ = repository.commit_signed("Sign the parser");
    repository
}

/// The jobs an effect list starts, by their kinds.
fn started(effects: &[Effect]) -> Vec<(app::store::JobId, JobKind)> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Start(job) => Some((job.id, job.kind.clone())),
            _ => None,
        })
        .collect()
}

/// Reduces a message, without running the jobs it starts.
fn reduce(state: &mut State, message: Message) -> Vec<Effect> {
    app::store::reduce(state, message)
}

/// Runs a job the store started, and reduces what it answered, without running the jobs that starts.
fn finish(state: &mut State, effects: &[Effect], wanted: fn(&JobKind) -> bool) -> Vec<Effect> {
    let Some(job) = effects.iter().find_map(|effect| match effect {
        Effect::Start(job) if wanted(&job.kind) => Some(job.clone()),
        _ => None,
    }) else {
        panic!("the job was started");
    };
    let mut after = Vec::new();
    for message in app::work::jobs::messages(&job, &CancelToken::new()) {
        after.extend(reduce(state, message));
    }
    after
}

#[test]
fn details_signature__a_good_signature_says_good_with_the_signer_in_the_color_of_an_addition() {
    let repository = signed_repository(true);
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Sign the parser");
    let Some(SignatureCheck::Read(signature)) = &state.inspector.signature else {
        panic!("the signature is read: {:?}", state.inspector.signature);
    };
    assert_eq!(signature.verdict, SignatureVerdict::Good);
    let tooltip = views::text::details::signature_tooltip(signature);
    assert!(
        tooltip.contains(&signature.fingerprint) && !signature.fingerprint.is_empty(),
        "{tooltip}"
    );
    let tree = drawn(&state);
    assert!(says(&tree, "Signature"));
    assert!(says(&tree, "Good, author@example.com"));
}

#[test]
fn details_signature__a_commit_not_signed_has_no_line() {
    let repository = signed_repository(true);
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Add the readme");
    assert!(
        matches!(&state.inspector.signature, Some(SignatureCheck::Read(read)) if read.verdict == SignatureVerdict::NotSigned)
    );
    assert!(!says(&drawn(&state), "Signature"));
}

#[test]
fn details_signature__without_allowed_signers_a_signed_commit_says_signed_not_checked() {
    let repository = signed_repository(false);
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Sign the parser");
    assert!(says_in(&drawn(&state), "Signed, not checked"));
}

#[test]
fn details_signature__the_message_and_files_are_shown_before_the_signature_is_read() {
    let repository = signed_repository(true);
    let mut state = showing(repository.path().to_path_buf());
    let commit = row_id(&state, "Sign the parser");
    let effects = reduce(
        &mut state,
        Message::Intent(Intent::Graph(GraphIntent::Select(Selected::Commit(commit)))),
    );
    assert!(
        started(&effects)
            .iter()
            .all(|(_, kind)| !matches!(kind, JobKind::ReadSignature { .. }))
    );
    let after = finish(&mut state, &effects, |kind| matches!(kind, JobKind::ReadDetails { .. }));
    assert!(matches!(
        started(&after).as_slice(),
        [(_, JobKind::ReadSignature { .. })]
    ));
    let tree = drawn(&state);
    assert!(says(&tree, "Sign the parser"), "the message is shown");
    assert!(!says(&tree, "Signature"), "no line until the signature is read");
    let _ = finish(&mut state, &after, |kind| matches!(kind, JobKind::ReadSignature { .. }));
    assert!(says(&drawn(&state), "Signature"));
}

#[test]
fn details_signature__going_through_ten_commits_reads_the_signature_of_the_last_the_reads_before_cancelled() {
    let mut repository = Repository::new();
    let key = repository.ssh_signing_key();
    repository.allow_signer(&key);
    let subjects: Vec<String> = (1..=10).map(|number| format!("Sign commit {number}")).collect();
    for subject in &subjects {
        let _ = repository.commit_signed(subject);
    }
    let mut state = showing(repository.path().to_path_buf());
    let first = row_id(&state, "Sign commit 1");
    let effects = reduce(
        &mut state,
        Message::Intent(Intent::Graph(GraphIntent::Select(Selected::Commit(first)))),
    );
    let after = finish(&mut state, &effects, |kind| matches!(kind, JobKind::ReadDetails { .. }));
    let [(first_signature, _)] = started(&after)[..] else {
        panic!("one read of the signature");
    };
    let mut cancelled = Vec::new();
    let mut last = Vec::new();
    for subject in subjects.iter().skip(1) {
        let commit = row_id(&state, subject);
        last = reduce(
            &mut state,
            Message::Intent(Intent::Graph(GraphIntent::Select(Selected::Commit(commit)))),
        );
        cancelled.extend(last.iter().filter_map(|effect| match effect {
            Effect::Cancel(job) => Some(*job),
            _ => None,
        }));
        assert!(
            started(&last)
                .iter()
                .all(|(_, kind)| !matches!(kind, JobKind::ReadSignature { .. }))
        );
    }
    assert!(
        cancelled.contains(&first_signature),
        "the first read of a signature is cancelled"
    );
    let after = finish(&mut state, &last, |kind| matches!(kind, JobKind::ReadDetails { .. }));
    let tenth = row_id(&state, "Sign commit 10");
    assert!(matches!(
        started(&after).as_slice(),
        [(_, JobKind::ReadSignature { request, .. })] if request.commit == tenth
    ));
}

#[test]
fn details_signature__the_tab_of_a_commit_signed_shows_the_same_line() {
    let repository = signed_repository(true);
    let mut state = showing(repository.path().to_path_buf());
    let commit = row_id(&state, "Sign the parser");
    let _effects = support::run(&mut state, Message::Intent(Intent::Graph(GraphIntent::Open(commit))));
    let tree = Window::new(ThemeName::Dark, PAGE).draw(&state, &[]).1;
    assert!(says(&tree, "Signature"));
    assert!(says(&tree, "Good, author@example.com"));
}

/// A signature as git says it, the same at every run.
fn fixed(verdict: SignatureVerdict) -> CommitSignature {
    CommitSignature {
        verdict,
        kind: Some(SignatureKind::Ssh),
        signer: String::from("Ada <ada@example.com>"),
        key: String::from("SHA256:1Fp3XnXkPp6Lx2C9mRrj0bJ5fJmJ0lRV3vGQq5mZr7Y"),
        fingerprint: String::from("SHA256:1Fp3XnXkPp6Lx2C9mRrj0bJ5fJmJ0lRV3vGQq5mZr7Y"),
        trust: String::new(),
    }
}

/// Draws the line of a signature alone, in a theme, and gives the window and its tree.
fn line(theme: ThemeName, signature: &CommitSignature) -> (Window, platform::accesskit::TreeUpdate) {
    let mut window = Window::new(theme, (420, 28));
    let state = SignatureState {
        read: SignatureRead(signature),
        key_width: 84.0,
    };
    let (_, tree) = window.frame(&[], |ui| signature_view(&state, ui));
    (window, tree)
}

#[test]
fn details_signature__a_bad_signature_says_bad_in_the_color_of_a_removal() {
    for (theme, name) in [(ThemeName::Dark, "dark"), (ThemeName::Light, "light")] {
        let (window, tree) = line(theme, &fixed(SignatureVerdict::Bad));
        assert!(says(&tree, "Bad"));
        window.assert_snapshot(&format!("details-signature-bad-{name}"));
    }
}

#[test]
fn details_signature__the_line_of_a_good_signature_matches_its_snapshots_in_each_theme() {
    for (theme, name) in [(ThemeName::Dark, "dark"), (ThemeName::Light, "light")] {
        let (window, tree) = line(theme, &fixed(SignatureVerdict::Good));
        assert!(says(&tree, "Good, Ada <ada@example.com>"));
        window.assert_snapshot(&format!("details-signature-good-{name}"));
    }
}
