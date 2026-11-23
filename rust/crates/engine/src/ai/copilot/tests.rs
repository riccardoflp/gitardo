#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::thread;
use std::time::{Duration, Instant};

use http::Cancel;
use model::ai::{AiModels, AiProvider, AiSettings, CopilotNeed, Effort};
use model::commit::DraftInput;
use testkit::{CopilotScript, FakeCopilot};

use super::{CopilotCli, CopilotError, CopilotModel, CopilotStreams, find_copilot, program_extensions};
use crate::ai::{AiError, ConflictRequest, ask_conflict, draft, failed_models};

/// The CLI of a window, started as the fake.
fn cli_of(fake: &FakeCopilot) -> CopilotCli {
    let fake = fake.clone();
    CopilotCli::over(move || {
        let (output, input) = fake.start();
        Ok(CopilotStreams { output, input })
    })
}

/// The settings with GitHub Copilot as the provider, a model and an effort.
fn settings(model: &str, effort: Option<Effort>) -> AiSettings {
    AiSettings {
        provider: Some(AiProvider::Copilot),
        endpoint: String::new(),
        model: model.to_owned(),
        effort,
    }
}

/// A resolution as Copilot writes it, in pieces.
fn resolution() -> Vec<String> {
    [
        "<resolution>\nboth",
        "();\n</resolution>",
        "<reason>Kept both.</reason>",
    ]
    .map(str::to_owned)
    .to_vec()
}

/// Waits until a condition holds, for at most ten seconds.
fn until(condition: impl Fn() -> bool) -> bool {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        if condition() {
            return true;
        }
        thread::sleep(Duration::from_millis(10));
    }
    false
}

#[test]
fn ai_copilot__resolve_with_ai_proposes_a_resolution_named_after_the_copilot_model_asked() {
    let fake = FakeCopilot::new(CopilotScript {
        pieces: resolution(),
        ..CopilotScript::default()
    });
    let cli = cli_of(&fake);
    let (settings, cancel) = (settings("", Some(Effort::Max)), Cancel::new());
    let mut model = CopilotModel {
        cli: &cli,
        settings: &settings,
        cancel: &cancel,
    };
    let request = ConflictRequest {
        question: String::from("Resolve one merge conflict in the file a.rs."),
        before: Vec::new(),
        after: Vec::new(),
        has_side_lines: true,
    };
    let mut seen = Vec::new();
    let reply = ask_conflict(&mut model, &request, &mut |piece| seen.push(piece.to_owned())).unwrap();
    // With no model chosen, the first of the subscription is asked, and named.
    assert_eq!(reply.model, "gpt-test");
    assert_eq!(reply.answer.lines, ["both();"]);
    assert_eq!(seen, resolution());
    assert_eq!(
        fake.methods(),
        [
            "connect",
            "auth.getStatus",
            "models.list",
            "auth.getStatus",
            "session.create",
            "session.options.update",
            "session.send",
            "session.detach"
        ]
    );
    let requests = fake.requests();
    let created = requests
        .iter()
        .find(|request| request.starts_with("session.create"))
        .unwrap();
    for member in [
        "\"model\":\"gpt-test\"",
        "\"reasoningEffort\":\"xhigh\"",
        "\"streaming\":true",
        "\"availableTools\":[]",
    ] {
        assert!(created.contains(member), "{created}");
    }
    let sent = requests
        .iter()
        .find(|request| request.starts_with("session.send"))
        .unwrap();
    assert!(sent.contains("Resolve one merge conflict in the file a.rs."), "{sent}");
}

#[test]
fn ai_copilot__draft_the_commit_message_writes_a_draft_from_github_copilot() {
    let fake = FakeCopilot::new(CopilotScript {
        pieces: vec![String::from("Add the "), String::from("parser")],
        ..CopilotScript::default()
    });
    let cli = cli_of(&fake);
    let (settings, cancel) = (settings("claude-test", None), Cancel::new());
    let mut model = CopilotModel {
        cli: &cli,
        settings: &settings,
        cancel: &cancel,
    };
    let input = DraftInput {
        diff: "diff --git a/a.txt b/a.txt\n",
        new_files: &[],
        subjects: &[],
    };
    let message = draft(&mut model, &input, &mut |_| {}).unwrap();
    assert_eq!(message, "Add the parser");
    // The model chosen is asked, without listing the others first, and with no effort when none is set.
    assert!(!fake.methods().contains(&String::from("models.list")));
    let requests = fake.requests();
    let created = requests
        .iter()
        .find(|request| request.starts_with("session.create"))
        .unwrap();
    assert!(created.contains("\"model\":\"claude-test\""), "{created}");
    assert!(!created.contains("reasoningEffort"), "{created}");
}

#[test]
fn ai_copilot__choose_the_model_lists_the_models_of_the_subscription_and_the_one_chosen_is_asked_next() {
    let fake = FakeCopilot::new(CopilotScript::default());
    let cli = cli_of(&fake);
    let cancel = Cancel::new();
    let models = cli.models(&cancel).unwrap();
    let listed: Vec<(&str, &str)> = models
        .iter()
        .map(|model| (model.id.as_str(), model.name.as_str()))
        .collect();
    assert_eq!(listed, [("gpt-test", "GPT Test"), ("claude-test", "Claude Test")]);
    let settings = settings("claude-test", None);
    let mut model = CopilotModel {
        cli: &cli,
        settings: &settings,
        cancel: &cancel,
    };
    let asked = crate::ai::LanguageModel::ask(
        &mut model,
        crate::ai::Purpose::CommitMessage,
        &model::ai::AiRequest {
            prompt: String::from("Q"),
        },
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(asked.model, "claude-test");
    // One CLI answers both: it is started once for the window.
    assert_eq!(fake.starts(), 1);
}

#[test]
fn ai_copilot__a_permission_the_cli_asks_is_refused_and_the_answer_still_comes() {
    let fake = FakeCopilot::new(CopilotScript {
        asks_permission: true,
        ..CopilotScript::default()
    });
    let cli = cli_of(&fake);
    let answer = cli
        .ask(&settings("gpt-test", None), "Q", &Cancel::new(), &mut |_| {})
        .unwrap();
    assert_eq!(answer, "Fixed");
    let answers = fake.permission_answers();
    assert_eq!(answers.len(), 1);
    assert!(answers[0].contains("\"kind\":\"reject\""), "{answers:?}");
}

#[test]
fn ai_copilot__without_the_cli_on_the_path_asking_refuses_with_how_to_install() {
    let folder = std::env::temp_dir().join(format!("gitardo-no-copilot-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = std::env::join_paths([&folder]).unwrap();
    assert_eq!(find_copilot(&path, &program_extensions(None)), None);
    let error = AiError::from(CopilotError::Needs(CopilotNeed::Install));
    assert_eq!(
        error.to_string(),
        "The GitHub Copilot CLI is not installed. Install it, then try again."
    );
    assert_eq!(failed_models(error), AiModels::CopilotNeeds(CopilotNeed::Install));
    // Once it is there, it is found.
    let name = format!("copilot{}", program_extensions(None)[0]);
    std::fs::write(folder.join(name), "").unwrap();
    assert!(find_copilot(&path, &program_extensions(None)).is_some());
    std::fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn ai_copilot__signed_out_asking_says_so_and_once_signed_in_it_is_answered() {
    let fake = FakeCopilot::new(CopilotScript {
        is_signed_in: false,
        ..CopilotScript::default()
    });
    let cli = cli_of(&fake);
    let settings = settings("gpt-test", None);
    let refused = cli.ask(&settings, "Q", &Cancel::new(), &mut |_| {}).unwrap_err();
    assert!(
        matches!(refused, CopilotError::Needs(CopilotNeed::SignIn)),
        "{refused:?}"
    );
    assert_eq!(refused.to_string(), "GitHub Copilot is not signed in.");
    assert!(!fake.methods().contains(&String::from("session.create")));
    // `copilot login` ran in a terminal, and the same CLI answers.
    fake.sign_in(true);
    assert_eq!(cli.ask(&settings, "Q", &Cancel::new(), &mut |_| {}).unwrap(), "Fixed");
    assert_eq!(fake.starts(), 1);
}

#[test]
fn ai_copilot__a_refusal_of_copilot_is_said_in_its_words() {
    let fake = FakeCopilot::new(CopilotScript {
        refusal: Some(String::from("You have no requests left this month.")),
        ..CopilotScript::default()
    });
    let refused = cli_of(&fake)
        .ask(&settings("gpt-test", None), "Q", &Cancel::new(), &mut |_| {})
        .unwrap_err();
    assert_eq!(
        refused.to_string(),
        "GitHub Copilot refused: You have no requests left this month."
    );
}

#[test]
fn ai_copilot__cancelling_a_question_ends_its_session_and_not_the_cli_and_closing_the_window_ends_the_cli() {
    let fake = FakeCopilot::new(CopilotScript {
        holds_answer: true,
        ..CopilotScript::default()
    });
    let cli = cli_of(&fake);
    let settings = settings("gpt-test", None);
    let cancel = Cancel::new();
    let canceller = cancel.clone();
    let asking = {
        let (cli, settings) = (cli.clone(), settings.clone());
        thread::spawn(move || {
            cli.ask(&settings, "Q", &cancel, &mut |_| {
                canceller.cancel();
            })
        })
    };
    let asked = asking.join().unwrap();
    assert!(matches!(asked, Err(CopilotError::Cancelled)), "{asked:?}");
    let methods = fake.methods();
    assert_eq!(methods[methods.len() - 2..], ["session.abort", "session.detach"]);
    assert!(cli.is_running() && fake.is_running());
    // The next question is answered by the same CLI.
    fake.answer_with(CopilotScript::default());
    assert_eq!(cli.ask(&settings, "Q", &Cancel::new(), &mut |_| {}).unwrap(), "Fixed");
    drop(cli);
    // The window closed: the last of its CLI is dropped, and the CLI ends.
    assert!(until(|| !fake.is_running()));
    assert_eq!(fake.starts(), 1);
}

#[test]
fn ai_copilot__the_tests_speak_to_a_fake_copilot_that_answers_the_protocol() {
    let fake = FakeCopilot::new(CopilotScript::default());
    let cli = cli_of(&fake);
    assert!(!cli.is_running());
    assert_eq!(
        cli.ask(&settings("gpt-test", None), "Q", &Cancel::new(), &mut |_| {})
            .unwrap(),
        "Fixed"
    );
    assert!(cli.is_running());
    assert!(fake.requests().iter().all(|request| !request.contains("Authorization")));
}

#[test]
fn ai_copilot__a_cli_too_old_for_the_sdk_is_refused_with_how_to_install() {
    let fake = FakeCopilot::new(CopilotScript {
        protocol_version: 3,
        ..CopilotScript::default()
    });
    let cli = cli_of(&fake);
    let refused = cli.models(&Cancel::new()).unwrap_err();
    assert!(
        matches!(refused, CopilotError::Needs(CopilotNeed::Update)),
        "{refused:?}"
    );
    assert_eq!(
        refused.to_string(),
        "The GitHub Copilot CLI is too old for Gitardo. Update it, then try again."
    );
    assert_eq!(CopilotNeed::Update.button(), "How to install");
    assert_eq!(
        failed_models(AiError::from(refused)),
        AiModels::CopilotNeeds(CopilotNeed::Update)
    );
    // Nothing more is asked of it, and it is not kept: once updated, it is started again.
    assert_eq!(fake.methods(), ["connect"]);
    assert!(!cli.is_running());
    fake.answer_with(CopilotScript::default());
    assert_eq!(cli.models(&Cancel::new()).unwrap().len(), 2);
    assert_eq!(fake.starts(), 2);
}

#[test]
fn ai_copilot__a_cli_too_new_for_the_sdk_says_to_update_gitardo() {
    let fake = FakeCopilot::new(CopilotScript {
        protocol_version: 5,
        ..CopilotScript::default()
    });
    let refused = cli_of(&fake)
        .ask(&settings("gpt-test", None), "Q", &Cancel::new(), &mut |_| {})
        .unwrap_err();
    assert!(matches!(refused, CopilotError::TooNew), "{refused:?}");
    assert_eq!(
        refused.to_string(),
        "The GitHub Copilot CLI is too new for Gitardo. Update Gitardo, then try again."
    );
    assert_eq!(fake.methods(), ["connect"]);
}
