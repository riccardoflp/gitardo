//! The questions to the AI provider about the conflicts of the review: which conflicts are asked about, two at a time,
//! **Stop**, the cards while the AI is asked and when it failed, and the answers kept while the window is open. The
//! provider is never reached here but by a test server on 127.0.0.1, and the answers are made by the tests.

use app::store::{Effect, Message, State};
use app::work::ai::{AiFailed, AiQuestion, AiReply, AiWork, AskId};
use engine::ai::ConflictReply;
use model::ai::{AiProvider, Confidence, ConflictAnswer};
use model::review::ProposalSource;
use platform::accesskit::Role;
use testkit::Repository;
use views::Intent;
use views::review::{CardAi, ReviewIntent};

use super::review::{NEXT_TO_EACH_OTHER, SAME_LINE, cards, drawn, file, merged, open, read_again};
use super::support::{has, run, says};

/// A window showing the repository, with the rules and the AI switched on and a provider set up at an address nothing
/// answers: the tests answer the questions themselves.
pub fn with_ai(repository: &Repository) -> State {
    let mut state = super::showing(repository.path().to_path_buf());
    state.settings.features.conflicts_auto_resolve = true;
    state.settings.features.conflicts_ai = true;
    state.settings.features.ai.provider = Some(AiProvider::OpenAiCompatible);
    state.settings.features.ai.endpoint = String::from("http://127.0.0.1:9");
    state
}

/// The questions among some effects.
pub fn questions(effects: &[Effect]) -> Vec<AiQuestion> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Ask(question) => Some((**question).clone()),
            _ => None,
        })
        .collect()
}

/// The text of a question about a conflict.
fn text_of(question: &AiQuestion) -> &str {
    match &question.work {
        AiWork::Conflict(request) => &request.question,
        AiWork::Draft(_) => "",
    }
}

/// Gives the page an intent, with the jobs it starts, and gives the effects.
fn reduce(state: &mut State, intent: ReviewIntent) -> Vec<Effect> {
    run(state, Message::Intent(Intent::Review(intent)))
}

/// The answer of a model that writes one line.
pub fn answer(ask: AskId, line: &str) -> Message {
    Message::AiAnswered {
        ask,
        result: Ok(AiReply::Conflict(ConflictReply {
            model: String::from("test-model"),
            answer: ConflictAnswer {
                lines: vec![line.to_owned()],
                reason: String::from("Kept both."),
                confidence: Confidence::High,
            },
        })),
    }
}

/// The files of the tests: `three.txt` and on, each with one conflict the rules leave.
fn same_line_files(count: usize) -> Repository {
    let names: Vec<String> = (0..count).map(|index| format!("file{index}.txt")).collect();
    let files: Vec<_> = names.iter().map(|name| file(name, SAME_LINE)).collect();
    merged(&files)
}

#[test]
fn ai_resolve_conflicts__with_5_conflicts_of_which_3_are_resolved_by_the_rules_2_questions_are_sent() {
    let base = "a\nb\nc\nd\n1\n2\n3\n4\n5\n6\nw\nx\ny\nz\n";
    let main = "a\nB\nc\nd\n1\n2\n3\n4\n5\n6\nw\nX\ny\nz\n";
    let feature = "a\nb\nC\nd\n1\n2\n3\n4\n5\n6\nw\nx\nY\nz\n";
    let repository = merged(&[
        file("one.txt", (base, main, feature)),
        file("two.txt", NEXT_TO_EACH_OTHER),
        file("three.txt", SAME_LINE),
        file("four.txt", SAME_LINE),
    ]);
    let mut state = with_ai(&repository);
    let effects = reduce(&mut state, ReviewIntent::ResolveWithAi(None));
    assert_eq!(cards(&state).len(), 5);
    let asked = questions(&effects);
    let mut files: Vec<&str> = asked
        .iter()
        .map(|question| text_of(question).lines().next().unwrap_or_default())
        .collect();
    files.sort_unstable();
    assert_eq!(
        files,
        [
            "Resolve one merge conflict in the file four.txt.",
            "Resolve one merge conflict in the file three.txt."
        ]
    );
}

#[test]
fn ai_resolve_conflicts__with_6_conflicts_no_more_than_2_questions_are_under_way() {
    let repository = same_line_files(6);
    let mut state = with_ai(&repository);
    let mut asked = questions(&reduce(&mut state, ReviewIntent::ResolveWithAi(None)));
    assert_eq!(asked.len(), 2);
    let mut answered = 0;
    while let Some(question) = asked.pop() {
        let next = questions(&run(&mut state, answer(question.id, "both();")));
        answered += 1;
        assert!(next.len() <= 1, "{} sent at once", next.len());
        asked.extend(next);
        assert!(asked.len() <= 2, "{} under way", asked.len());
    }
    assert_eq!(answered, 6);
    assert!(cards(&state).iter().all(|card| card.proposal.is_some()));
}

#[test]
fn ai_resolve_conflicts__stop_ends_the_2_under_way_and_the_4_waiting() {
    let repository = same_line_files(6);
    let mut state = with_ai(&repository);
    let asked = questions(&reduce(&mut state, ReviewIntent::ResolveWithAi(None)));
    assert!(says(&drawn(&state), "Asking the AI: 0 of 6 conflicts answered"));
    let effects = reduce(&mut state, ReviewIntent::StopAi);
    let mut stopped: Vec<AskId> = asked.iter().map(|question| question.id).collect();
    stopped.sort_unstable();
    assert_eq!(effects, [Effect::StopAsking(stopped)]);
    let late = run(&mut state, answer(asked[0].id, "both();"));
    assert_eq!(questions(&late).len(), 0);
    for card in cards(&state) {
        assert_eq!((card.proposal.is_none(), &card.ai), (true, &CardAi::Idle));
    }
    assert!(!says(&drawn(&state), "Asking the AI: 0 of 6 conflicts answered"));
}

#[test]
fn ai_resolve_conflicts__with_no_ai_provider_each_card_says_so_and_offers_ask_again() {
    let repository = same_line_files(2);
    let mut state = with_ai(&repository);
    state.settings.features.ai.provider = None;
    let effects = reduce(&mut state, ReviewIntent::ResolveWithAi(None));
    assert_eq!(questions(&effects).len(), 0);
    for card in cards(&state) {
        assert_eq!(card.ai, CardAi::Failed(String::from("No AI provider is set up.")));
    }
    let tree = drawn(&state);
    assert!(says(&tree, "No AI provider is set up."));
    assert!(has(&tree, Role::Button, "Ask again"));
    assert!(has(&tree, Role::Button, "No language model"));
}

#[test]
fn ai_resolve_conflicts__closing_and_reopening_the_review_page_shows_the_answers_again_without_asking() {
    let repository = same_line_files(1);
    let mut state = with_ai(&repository);
    let asked = questions(&reduce(&mut state, ReviewIntent::ResolveWithAi(None)));
    let _effects = run(&mut state, answer(asked[0].id, "both();"));
    let _closed = run(&mut state, Message::Intent(Intent::ClosePage));
    assert!(state.reviews.page.is_none());
    let choose = views::search::SearchIntent::ChooseQuick(views::search::QuickChoice::Command(
        views::search::QuickCommand::ReviewConflicts,
    ));
    let effects = run(&mut state, Message::Intent(Intent::Search(choose)));
    assert_eq!(questions(&effects).len(), 0);
    let card = cards(&state).remove(0);
    let proposal = card.proposal.unwrap();
    assert!(matches!(&proposal.source, ProposalSource::Ai { model, .. } if model == "test-model"));
    assert_eq!(proposal.lines[0].line.text, "both();");
}

#[test]
fn ai_resolve_conflicts__editing_a_line_inside_an_answered_conflict_drops_its_answer() {
    let repository = same_line_files(1);
    let mut state = with_ai(&repository);
    let asked = questions(&reduce(&mut state, ReviewIntent::ResolveWithAi(None)));
    let _effects = run(&mut state, answer(asked[0].id, "both();"));
    assert!(cards(&state)[0].proposal.is_some());
    let path = repository.path().join("file0.txt");
    let text = std::fs::read_to_string(&path).unwrap().replace("B1", "B3");
    std::fs::write(&path, text).unwrap();
    read_again(&mut state);
    let card = cards(&state).remove(0);
    assert!(card.proposal.is_none(), "{:?}", card.proposal);
}

#[test]
fn review_card__a_card_the_ai_failed_on_shows_the_error_and_ask_again() {
    let repository = same_line_files(1);
    let mut state = with_ai(&repository);
    let asked = questions(&reduce(&mut state, ReviewIntent::ResolveWithAi(None)));
    let tree = drawn(&state);
    assert!(says(&tree, "The AI is reading the conflict\u{2026}"));
    let _piece = run(
        &mut state,
        Message::AiPiece {
            ask: asked[0].id,
            text: String::from("<resolution>\nbo"),
        },
    );
    assert!(says(&drawn(&state), "The AI is writing a resolution\u{2026}"));
    let failed = Message::AiAnswered {
        ask: asked[0].id,
        result: Err(AiFailed::saying(String::from(
            "The language model did not answer with a resolution.",
        ))),
    };
    let _effects = run(&mut state, failed);
    let tree = drawn(&state);
    assert!(says(&tree, "The language model did not answer with a resolution."));
    assert!(has(&tree, Role::Button, "Ask again"));
    let key = cards(&state)[0].key.clone();
    let again = questions(&reduce(&mut state, ReviewIntent::AskAi(Some(key))));
    assert_eq!(again.len(), 1);
}

#[test]
fn a_model_that_is_not_there_asks_no_other() {
    let repository = same_line_files(4);
    let mut state = with_ai(&repository);
    let asked = questions(&reduce(&mut state, ReviewIntent::ResolveWithAi(None)));
    let missing = Message::AiAnswered {
        ask: asked[0].id,
        result: Err(engine::ai::AiError::ModelMissing(String::from("gone")).into()),
    };
    let effects = run(&mut state, missing);
    assert_eq!(questions(&effects).len(), 0);
    assert!(says(
        &drawn(&state),
        "The model chosen to resolve conflicts, gone, is not available: choose another one."
    ));
}

#[test]
fn review_page__opening_it_from_the_sparkle_of_one_file_shows_only_that_file_and_asks_about_it() {
    let repository = merged(&[file("one.txt", SAME_LINE), file("two.txt", SAME_LINE)]);
    let mut state = with_ai(&repository);
    let _changes = run(
        &mut state,
        Message::Intent(Intent::Changes(views::changes::ChangesIntent::Open)),
    );
    let mut window = super::support::Window::new(render::ThemeName::Dark, (1400, 1000));
    let _ = super::changes_menu::menu_of(&mut window, &state, "one.txt");
    let intents = window.click(
        &state,
        Role::MenuItem,
        "Resolve with AI, after looking at what is proposed",
    );
    let resolve = Intent::Review(ReviewIntent::ResolveWithAi(Some(vec![String::from("one.txt")])));
    assert!(intents.contains(&resolve), "{intents:?}");
    let asked = questions(&run(&mut state, Message::Intent(resolve)));
    let page = state.reviews.page.as_ref().unwrap();
    assert_eq!(page.only, Some(vec![String::from("one.txt")]));
    assert_eq!(asked.len(), 1);
    assert!(text_of(&asked[0]).starts_with("Resolve one merge conflict in the file one.txt."));
}

#[test]
fn with_conflicts_ai_off_resolve_with_ai_refuses_with_open_settings() {
    let repository = same_line_files(1);
    let mut state = with_ai(&repository);
    state.settings.features.conflicts_ai = false;
    let effects = reduce(&mut state, ReviewIntent::ResolveWithAi(None));
    assert_eq!(questions(&effects).len(), 0);
    let said = state.operations.said.as_ref().unwrap();
    assert_eq!(
        said.text,
        "Resolving conflicts with AI is experimental, and it is switched off in the settings."
    );
    assert_eq!(said.offer.as_ref().map(|(feature, _)| feature.0), Some("settings.edit"));
}

#[test]
fn conflicts_restore__after_undo_a_conflict_the_ai_had_answered_shows_the_same_proposal_without_asking_again() {
    let repository = same_line_files(1);
    let mut state = with_ai(&repository);
    let asked = questions(&reduce(&mut state, ReviewIntent::ResolveWithAi(None)));
    let _effects = run(&mut state, answer(asked[0].id, "both();"));
    let _accepted = reduce(&mut state, ReviewIntent::AcceptAll);
    let driver = super::support::OperationsDriver::new();
    driver.intent(&mut state, Intent::Review(ReviewIntent::Write));
    super::compare::until_ended(&driver, &mut state);
    let written = std::fs::read_to_string(repository.path().join("file0.txt")).unwrap();
    assert!(written.contains("both();"), "{written}");
    driver.intent(&mut state, Intent::Review(ReviewIntent::Undo));
    super::compare::until_ended(&driver, &mut state);
    let effects = run(&mut state, Message::Intent(Intent::Refresh));
    assert_eq!(questions(&effects).len(), 0);
    let card = cards(&state).remove(0);
    let proposal = card.proposal.unwrap();
    assert!(matches!(&proposal.source, ProposalSource::Ai { model, .. } if model == "test-model"));
}

#[test]
fn ai_choose_model__the_button_of_the_review_page_reads_the_model_and_its_effort() {
    let repository = same_line_files(1);
    let mut state = with_ai(&repository);
    state.settings.features.ai.model = String::from("test-model");
    state.settings.features.ai.effort = Some(model::ai::Effort::High);
    open(&mut state);
    let tree = drawn(&state);
    assert!(has(&tree, Role::Button, "test-model \u{b7} high effort"));
}
