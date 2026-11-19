//! GitHub Copilot as the AI provider of the window (`ai.copilot`): the review and the draft of a message asked through
//! a fake GitHub Copilot CLI of `testkit`, and the buttons of its refusals.

use app::store::{Effect, State};
use engine::ai::{CopilotCli, CopilotError, CopilotStreams};
use model::ai::{AiProvider, COPILOT_INSTALL_PAGE, CopilotNeed};
use testkit::{CopilotScript, FakeCopilot, Repository};
use views::Intent;
use views::changes::ChangesIntent;
use views::feature::FeatureIntent;
use views::review::ReviewIntent;

use super::review::{SAME_LINE, file, merged};
use super::support::{AiDriver, run};

/// The CLI of the window, started as the fake.
fn cli_of(fake: &FakeCopilot) -> CopilotCli {
    let fake = fake.clone();
    CopilotCli::over(move || {
        let (output, input) = fake.start();
        Ok(CopilotStreams { output, input })
    })
}

/// A CLI that is not on the `PATH`.
fn missing() -> CopilotCli {
    CopilotCli::over(|| Err(CopilotError::Needs(CopilotNeed::Install)))
}

/// A window showing the repository with GitHub Copilot as the provider and the Changes page open, `a.txt` changed.
fn drafting(repository: &Repository) -> State {
    std::fs::write(repository.path().join("a.txt"), "a changed\n").unwrap();
    let mut state = super::showing(repository.path().to_path_buf());
    state.settings.features.ai.provider = Some(AiProvider::Copilot);
    let _effects = run(
        &mut state,
        app::store::Message::Intent(Intent::Changes(ChangesIntent::Open)),
    );
    state
}

/// A repository with `a.txt` committed.
fn one_file() -> Repository {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a\n")]);
    repository
}

/// The text of the message box of the repository.
fn message_box(state: &State) -> String {
    state
        .persisted
        .commit_messages
        .first()
        .map_or_default(|kept| kept.message.clone())
}

#[test]
fn commit_draft_message__with_github_copilot_the_draft_is_written_by_copilot() {
    let repository = one_file();
    let mut state = drafting(&repository);
    let fake = FakeCopilot::new(CopilotScript {
        pieces: vec![String::from("Change "), String::from("a")],
        ..CopilotScript::default()
    });
    let driver = AiDriver::with_copilot(engine::MemorySecrets::default(), &cli_of(&fake));
    let _effects = driver.intent(&mut state, Intent::Changes(ChangesIntent::DraftMessage));
    let _answered = driver.until_answered(&mut state);
    assert_eq!(message_box(&state), "Change a");
    let sent = fake.requests();
    let sent = sent.iter().find(|request| request.starts_with("session.send")).unwrap();
    assert!(sent.contains("diff --git a/a.txt b/a.txt"), "{sent}");
}

#[test]
fn ai_ask_conflict__with_github_copilot_the_proposal_is_named_after_the_copilot_model_asked() {
    let repository = merged(&[file("one.txt", SAME_LINE)]);
    let mut state = super::ai::with_ai(&repository);
    state.settings.features.ai.provider = Some(AiProvider::Copilot);
    state.settings.features.ai.endpoint = String::new();
    let fake = FakeCopilot::new(CopilotScript {
        pieces: vec![String::from(
            "<resolution>\nboth\n</resolution>\n<reason>Kept both.</reason>",
        )],
        ..CopilotScript::default()
    });
    let driver = AiDriver::with_copilot(engine::MemorySecrets::default(), &cli_of(&fake));
    let _effects = driver.intent(&mut state, Intent::Review(ReviewIntent::ResolveWithAi(None)));
    let _answered = driver.until_answered(&mut state);
    let answers = &state.reviews.questions.answers;
    assert_eq!(answers.len(), 1, "{:?}", state.reviews.questions.failed);
    assert_eq!(answers[0].2.model, "gpt-test");
}

#[test]
fn ai_copilot__signed_out_sign_in_opens_a_terminal_of_the_bottom_panel_running_copilot_login() {
    let repository = one_file();
    let mut state = drafting(&repository);
    let fake = FakeCopilot::new(CopilotScript {
        is_signed_in: false,
        ..CopilotScript::default()
    });
    let driver = AiDriver::with_copilot(engine::MemorySecrets::default(), &cli_of(&fake));
    let _effects = driver.intent(&mut state, Intent::Changes(ChangesIntent::DraftMessage));
    let _answered = driver.until_answered(&mut state);
    let said = state.operations.said.clone().unwrap();
    assert_eq!(said.text, "GitHub Copilot is not signed in.");
    let (feature, label) = said.offer.unwrap();
    assert_eq!(label, "Sign in");
    let effects = run(
        &mut state,
        app::store::Message::Intent(Intent::Feature(FeatureIntent::Run(
            feature,
            model::contract::Target::Head,
        ))),
    );
    let typed = effects.iter().find_map(|effect| match effect {
        Effect::Terminal(app::store::terminals::TerminalEffect::Write(_, bytes)) => Some(bytes.clone()),
        _ => None,
    });
    assert_eq!(typed.as_deref(), Some(&b"copilot login\r"[..]));
    // Once `copilot login` ended, asking again is answered.
    fake.sign_in(true);
    let _effects = driver.intent(&mut state, Intent::Changes(ChangesIntent::DraftMessage));
    let _answered = driver.until_answered(&mut state);
    assert_eq!(message_box(&state), "Fixed");
}

#[test]
fn ai_copilot__without_the_cli_how_to_install_opens_githubs_page() {
    let repository = one_file();
    let mut state = drafting(&repository);
    let driver = AiDriver::with_copilot(engine::MemorySecrets::default(), &missing());
    let _effects = driver.intent(&mut state, Intent::Changes(ChangesIntent::DraftMessage));
    let _answered = driver.until_answered(&mut state);
    let said = state.operations.said.clone().unwrap();
    assert_eq!(
        said.text,
        "The GitHub Copilot CLI is not installed. Install it, then try again."
    );
    assert_eq!(
        said.link,
        Some((String::from("How to install"), COPILOT_INSTALL_PAGE.to_owned()))
    );
    // The refusal of **Choose the model that resolves conflicts** offers the same page.
    let effects = run(
        &mut state,
        app::store::Message::Intent(Intent::Feature(FeatureIntent::Run(
            features::ai::COPILOT_INSTALL,
            model::contract::Target::Head,
        ))),
    );
    assert!(
        effects.contains(&Effect::OpenUrl(COPILOT_INSTALL_PAGE.to_owned())),
        "{effects:?}"
    );
}
