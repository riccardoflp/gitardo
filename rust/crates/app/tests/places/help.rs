//! The Ask Gitardo page (`help.page`): opened from Quick search with a question typed there, the answer as it comes
//! with **Stop**, **New conversation**, the page with no AI provider, and its snapshots; against a test server on
//! 127.0.0.1 that answers as an OpenAI-compatible provider would, or with the answers given to the store.

use std::fmt::Write as _;

use app::store::{Effect, Message, State};
use app::work::ai::{AiFailed, AiReply, AiWork, AskId};
use http::test_server::{Answer, TestServer, answer};
use model::ai::AiProvider;
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::help::HelpIntent;
use views::page::Page;
use views::search::{QuickCommand, SearchIntent};
use views::text::help as words;

use super::pointer::apply;
use super::support::{AiDriver, Window, has, run, says, says_in, typed};

/// The size of the window.
pub(super) const SIZE: (u32, u32) = (1100, 720);

/// A repository of one commit.
pub(super) fn one_commit() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    repository
}

/// A window showing a repository, with an AI provider set up at a port, or none.
pub(super) fn window_with(repository: &Repository, port: Option<u16>) -> State {
    let mut state = super::showing(repository.path().to_path_buf());
    if let Some(port) = port {
        let ai = &mut state.settings.features.ai;
        ai.provider = Some(AiProvider::OpenAiCompatible);
        ai.endpoint = format!("http://127.0.0.1:{port}");
        ai.model = String::from("test-model");
    }
    state
}

/// Draws the window twice, so that what it shows is laid out and scrolled, and gives its tree.
pub(super) fn settled(window: &mut Window, state: &State) -> TreeUpdate {
    let _ = window.draw(state, &[]);
    window.draw(state, &[]).1
}

/// Reduces an intent of the page, and gives the question it asked of the AI provider, if it asked one.
pub(super) fn help(state: &mut State, intent: HelpIntent) -> (Vec<Effect>, Option<AskId>) {
    let effects = run(state, Message::Intent(Intent::Help(intent)));
    let asked = effects.iter().find_map(|effect| match effect {
        Effect::Ask(question) => Some(question.id),
        _ => None,
    });
    (effects, asked)
}

/// The questions before the one an effect asks, as it sends them; none when it asks nothing.
fn earlier_of(effects: &[Effect]) -> Option<Vec<String>> {
    effects.iter().find_map(|effect| match effect {
        Effect::Ask(question) => match &question.work {
            AiWork::Help(request) => Some(
                request
                    .input
                    .earlier
                    .iter()
                    .map(|asked| asked.question.clone())
                    .collect(),
            ),
            _ => None,
        },
        _ => None,
    })
}

/// The whole answer of a question, read as the engine reads it.
pub(super) fn answered(state: &mut State, ask: AskId, text: &str) {
    let answer = model::help::read_answer(text, &model::help::guide(model::help::App::Desktop));
    let result = Ok(AiReply::Help {
        answer,
        text: text.to_owned(),
    });
    let _ = run(state, Message::AiAnswered { ask, result });
}

/// A provider that streams its answer in pieces.
fn streamed(pieces: &[&str]) -> Answer {
    let mut stream = String::new();
    for piece in pieces {
        let text = json::to_compact_text(&json::Value::String((*piece).to_owned()));
        let _ = write!(
            stream,
            "data: {{\"choices\":[{{\"delta\":{{\"content\":{text}}}}}]}}\n\n"
        );
    }
    stream.push_str("data: [DONE]\n\n");
    Answer::Bytes(answer(
        "200 OK",
        &[("Content-Type", "text/event-stream")],
        stream.as_bytes(),
    ))
}

#[test]
fn help_page__stash_finds_stash_changes_and_a_question_that_finds_no_command_is_asked_by_the_last_entry() {
    let repository = one_commit();
    let server = TestServer::plain(Vec::new());
    let mut state = window_with(&repository, Some(server.port()));
    let mut window = Window::new(ThemeName::Dark, SIZE);
    apply(&mut state, vec![Intent::Search(SearchIntent::OpenQuick(String::new()))]);
    let _ = settled(&mut window, &state);
    let _ = window.draw(&state, &[typed("stash")]);
    let tree = settled(&mut window, &state);
    assert!(has(&tree, Role::ListBoxOption, "Stash Changes"));
    assert!(!says_in(&tree, "Ask Gitardo: "));
    apply(&mut state, vec![Intent::Search(SearchIntent::CloseQuick)]);
    let _ = settled(&mut window, &state);
    apply(&mut state, vec![Intent::Search(SearchIntent::OpenQuick(String::new()))]);
    let _ = settled(&mut window, &state);
    let _ = window.draw(&state, &[typed("how do I undo a push")]);
    let tree = settled(&mut window, &state);
    let options: Vec<&str> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::ListBoxOption)
        .filter_map(|(_, node)| node.label())
        .collect();
    let commands: Vec<&str> = QuickCommand::ALL
        .into_iter()
        .map(views::text::search::command_name)
        .collect();
    assert!(options.iter().all(|option| !commands.contains(option)), "{options:?}");
    let asked = words::ask_about("how do I undo a push");
    assert_eq!(options.last().copied(), Some(asked.as_str()), "{options:?}");
    let chosen = window.click(&state, Role::ListBoxOption, &asked);
    apply(&mut state, chosen);
    assert_eq!(state.page_shown(), Some(Page::Help));
    assert_eq!(state.help.exchanges.len(), 1);
    assert_eq!(state.help.exchanges[0].question, "how do I undo a push");
    assert!(state.help.asking.is_some());
}

#[test]
fn help_page__asking_shows_the_question_looking_for_the_features_then_the_answer_as_it_comes_with_stop() {
    let repository = one_commit();
    let mut state = window_with(&repository, Some(1));
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let (_, ask) = help(
        &mut state,
        HelpIntent::AskAbout(String::from("how do I put my changes aside")),
    );
    let ask = ask.expect("the question is asked");
    let tree = settled(&mut window, &state);
    assert!(says(&tree, "how do I put my changes aside"));
    assert!(says(&tree, words::LOOKING));
    assert!(has(&tree, Role::Button, words::STOP));
    assert!(!has(&tree, Role::Button, words::ASK));
    let piece = String::from("Choose **Stash changes…** in the context menu of the [[");
    let _ = run(&mut state, Message::AiPiece { ask, text: piece });
    let tree = settled(&mut window, &state);
    assert!(!says(&tree, words::LOOKING));
    assert!(
        says_in(&tree, "in the context menu of the"),
        "the answer so far is drawn"
    );
    assert!(!says_in(&tree, "[["), "a [[ whose end has not come is not shown");
    assert!(has(&tree, Role::Button, words::STOP));
}

#[test]
fn help_page__stop_keeps_what_came_of_the_answer_and_gives_the_box_back() {
    let repository = one_commit();
    let mut state = window_with(&repository, Some(1));
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let (_, ask) = help(&mut state, HelpIntent::AskAbout(String::from("how do I stash")));
    let ask = ask.expect("the question is asked");
    let text = String::from("Open Quick search and choose it.");
    let _ = run(&mut state, Message::AiPiece { ask, text });
    let _ = settled(&mut window, &state);
    let stopped = window.click(&state, Role::Button, words::STOP);
    let effects: Vec<Effect> = stopped
        .into_iter()
        .flat_map(|intent| run(&mut state, Message::Intent(intent)))
        .collect();
    assert!(effects.iter().any(|effect| *effect == Effect::StopAsking(vec![ask])));
    let tree = settled(&mut window, &state);
    assert!(says_in(&tree, "Open Quick search and choose it."));
    assert!(has(&tree, Role::Button, words::ASK));
    let intents = window.draw(&state, &[typed("and then?")]).0;
    apply(&mut state, intents);
    assert_eq!(
        state.help.document.buffer().text(),
        "and then?",
        "the box takes what is typed again"
    );
}

#[test]
fn help_page__new_conversation_empties_the_page_and_the_next_question_is_sent_without_the_earlier_ones() {
    let repository = one_commit();
    let mut state = window_with(&repository, Some(1));
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let (_, ask) = help(&mut state, HelpIntent::AskAbout(String::from("how do I fetch")));
    answered(
        &mut state,
        ask.expect("the first is asked"),
        "Choose **Fetch** in **Sync**.",
    );
    let (effects, ask) = help(&mut state, HelpIntent::AskAbout(String::from("and push?")));
    assert_eq!(earlier_of(&effects), Some(vec![String::from("how do I fetch")]));
    answered(&mut state, ask.expect("the second is asked"), "Choose **Push**.");
    let _ = settled(&mut window, &state);
    let intents = window.click(&state, Role::Button, words::NEW_CONVERSATION);
    apply(&mut state, intents);
    let tree = settled(&mut window, &state);
    assert!(says(&tree, words::EMPTY));
    assert!(!says_in(&tree, "how do I fetch"));
    let (effects, _) = help(&mut state, HelpIntent::AskAbout(String::from("how do I pull")));
    assert_eq!(earlier_of(&effects), Some(Vec::new()));
}

#[test]
fn help_page__with_no_provider_it_says_so_its_box_is_disabled_and_open_the_settings_shows_ai_provider() {
    let repository = one_commit();
    let mut state = window_with(&repository, None);
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let (_, ask) = help(&mut state, HelpIntent::AskAbout(String::from("how do I stash")));
    assert_eq!(ask, None, "nothing is asked");
    assert_eq!(state.help.exchanges.len(), 0);
    let tree = settled(&mut window, &state);
    assert!(says(&tree, model::help::NO_PROVIDER));
    let intents = window.draw(&state, &[typed("anything")]).0;
    apply(&mut state, intents);
    assert_eq!(state.help.document.buffer().text(), "", "the box takes nothing");
    let intents = window.click(&state, Role::Button, words::OPEN_SETTINGS);
    apply(&mut state, intents);
    assert_eq!(state.page_shown(), Some(Page::Settings));
    let title = views::text::settings::settings()
        .find(|info| info.key == "ai.provider")
        .map(|info| info.title)
        .expect("ai.provider is a setting");
    let tree = settled(&mut window, &state);
    assert!(
        tree.nodes.iter().any(|(_, node)| node.role() == Role::Group
            && node.label() == Some(title)
            && node.is_selected() == Some(true)),
        "the row of ai.provider is marked"
    );
}

#[test]
fn help_page__closing_the_page_forgets_the_conversation() {
    let repository = one_commit();
    let mut state = window_with(&repository, Some(1));
    let (effects, ask) = help(&mut state, HelpIntent::AskAbout(String::from("how do I stash")));
    assert_ne!(effects.len(), 0);
    let ask = ask.expect("the question is asked");
    let closed = run(&mut state, Message::Intent(Intent::ClosePage));
    assert!(closed.contains(&Effect::StopAsking(vec![ask])));
    assert_eq!(state.help.exchanges.len(), 0);
    apply(&mut state, vec![Intent::Help(HelpIntent::Open)]);
    assert_eq!(state.page_shown(), Some(Page::Help));
    assert_eq!(state.help.exchanges.len(), 0);
}

#[test]
fn help_page__a_question_is_answered_by_the_provider_in_two_requests_and_read_as_it_comes() {
    let repository = one_commit();
    let server = TestServer::plain(vec![
        streamed(&["stash.push\n"]),
        streamed(&["Choose **Stash changes…**, or ", "[[stash.push]] in Quick search."]),
    ]);
    let mut state = window_with(&repository, Some(server.port()));
    let driver = AiDriver::new(engine::MemorySecrets::default());
    let _effects = driver.intent(
        &mut state,
        Intent::Help(HelpIntent::AskAbout(String::from("how do I put my changes aside"))),
    );
    let _answered = driver.until_answered(&mut state);
    assert_eq!(server.received().len(), 2);
    let exchange = &state.help.exchanges[0];
    assert!(exchange.has_words && exchange.failed.is_none());
    assert_eq!(state.help.asking, None);
    assert_eq!(
        state.help.answers[0], "Choose **Stash changes…**, or [[stash.push]] in Quick search.",
        "the answer is kept as the model wrote it, to be sent with the next question"
    );
}

#[test]
fn help_page__a_failure_shows_under_its_question_with_try_again_which_asks_it_again() {
    let repository = one_commit();
    let mut state = window_with(&repository, Some(1));
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let (_, ask) = help(&mut state, HelpIntent::AskAbout(String::from("how do I stash")));
    let result = Err(AiFailed::saying(String::from("The provider refused: 401 Unauthorized")));
    let _ = run(
        &mut state,
        Message::AiAnswered {
            ask: ask.expect("asked"),
            result,
        },
    );
    let tree = settled(&mut window, &state);
    assert!(says_in(&tree, "401 Unauthorized"));
    let intents = window.click(&state, Role::Button, words::TRY_AGAIN);
    let effects: Vec<Effect> = intents
        .into_iter()
        .flat_map(|intent| run(&mut state, Message::Intent(intent)))
        .collect();
    assert!(effects.iter().any(|effect| matches!(effect, Effect::Ask(_))));
    assert_eq!(state.help.exchanges.len(), 1);
    assert_eq!(state.help.exchanges[0].failed, None);
}

#[test]
fn help_page__is_drawn_empty_with_a_question_and_its_answer_and_with_an_error_in_the_dark_and_the_light_theme() {
    let repository = one_commit();
    let answer = "Put your changes aside with a stash:\n\n1. Open **Quick search** with `Ctrl+K`.\n2. Choose \
                  [[stash.push]].\n\nThe stash is then listed in **Kept aside**, in the branch rail.";
    for theme in [ThemeName::Dark, ThemeName::Light] {
        let suffix = if theme == ThemeName::Dark { "dark" } else { "light" };
        let mut state = window_with(&repository, Some(1));
        let _ = help(&mut state, HelpIntent::Open);
        let mut window = Window::new(theme, SIZE);
        let _ = settled(&mut window, &state);
        window.assert_snapshot(&format!("help-page-empty-{suffix}"));
        let (_, ask) = help(
            &mut state,
            HelpIntent::AskAbout(String::from("How do I put my changes aside?")),
        );
        answered(&mut state, ask.expect("asked"), answer);
        let mut window = Window::new(theme, SIZE);
        let _ = settled(&mut window, &state);
        window.assert_snapshot(&format!("help-page-answer-{suffix}"));
        let mut state = window_with(&repository, Some(1));
        let (_, ask) = help(
            &mut state,
            HelpIntent::AskAbout(String::from("How do I squash my last three commits?")),
        );
        let result = Err(AiFailed::saying(String::from(
            "The AI provider refused the request: 401 Unauthorized.",
        )));
        let _ = run(
            &mut state,
            Message::AiAnswered {
                ask: ask.expect("asked"),
                result,
            },
        );
        let mut window = Window::new(theme, SIZE);
        let _ = settled(&mut window, &state);
        window.assert_snapshot(&format!("help-page-error-{suffix}"));
    }
}
