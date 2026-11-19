//! **Show me** (`help.show-me`): the buttons under an answer of Ask Gitardo, one for each feature it named, at most 4 in
//! the order named, which run an operation as Quick search runs it, open a page, or show where a feature is and say in
//! the status bar what to choose there; nothing runs until one is pressed.

use app::store::{Effect, Message, State};
use app::work::ai::AiReply;
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use views::Intent;
use views::graph::GraphIntent;
use views::help::HelpIntent;
use views::page::Page;
use views::search::{QuickChoice, QuickCommand, SearchIntent};
use views::text::header::{HIDE_REFERENCES, SHOW_REFERENCES};
use views::text::help as words;

use super::help::{SIZE, help, one_commit, settled, window_with};
use super::pointer::apply;
use super::support::{Window, has, run};

/// A window whose Ask Gitardo page shows a question answered with a text, and the effects of the answer.
fn answered_with(state: &mut State, text: &str) -> Vec<Effect> {
    let (_, Some(ask)) = help(state, HelpIntent::AskAbout(String::from("how do I do it"))) else {
        return Vec::new();
    };
    let answer = model::help::read_answer(text, &model::help::guide(model::help::App::Desktop));
    let result = Ok(AiReply::Help {
        answer,
        text: text.to_owned(),
    });
    run(state, Message::AiAnswered { ask, result })
}

/// The labels of the buttons under the answers, in the order drawn.
fn buttons(state: &State, tree: &TreeUpdate) -> Vec<String> {
    let labels: Vec<&str> = state
        .help
        .exchanges
        .iter()
        .flat_map(|exchange| exchange.buttons.iter().map(|button| button.label.as_str()))
        .collect();
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button)
        .filter_map(|(_, node)| node.label())
        .filter(|label| labels.contains(label))
        .map(str::to_owned)
        .collect()
}

#[test]
fn help_show_me__stash_changes_under_an_answer_that_names_stash_push_asks_what_quick_search_asks() {
    let repository = one_commit();
    std::fs::write(repository.path().join("README.md"), "Gitardo, changed\n").expect("the file is written");
    let mut state = window_with(&repository, Some(1));
    let mut from_quick_search = state.clone();
    let _ = answered_with(&mut state, "Choose [[stash.push]] in Quick search.");
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = settled(&mut window, &state);
    assert_eq!(buttons(&state, &tree), ["Stash Changes"]);
    let pressed = window.click(&state, Role::Button, "Stash Changes");
    let effects: Vec<Effect> = pressed
        .into_iter()
        .flat_map(|intent| run(&mut state, Message::Intent(intent)))
        .collect();
    let chosen = Intent::Search(SearchIntent::ChooseQuick(QuickChoice::Command(
        QuickCommand::StashChanges,
    )));
    let expected = run(&mut from_quick_search, Message::Intent(chosen));
    assert!(!expected.is_empty(), "Quick search runs the stash");
    assert_eq!(effects, expected);
    assert_eq!(state.operations.running, from_quick_search.operations.running);
}

#[test]
fn help_show_me__open_search_under_an_answer_that_names_the_search_page_opens_it() {
    let repository = one_commit();
    let mut state = window_with(&repository, Some(1));
    let _ = answered_with(&mut state, "Lines of text are found on [[file-search.page]].");
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = settled(&mut window, &state);
    let open = words::open_page(views::text::file_search::PAGE);
    assert_eq!(open, "Open Search");
    assert_eq!(buttons(&state, &tree), std::slice::from_ref(&open));
    let pressed = window.click(&state, Role::Button, &open);
    apply(&mut state, pressed);
    assert_eq!(state.page_shown(), Some(Page::Search));
}

#[test]
fn help_show_me__show_where_of_a_branch_renamed_shows_the_branch_rail_and_says_what_to_choose_there() {
    let repository = one_commit();
    let mut state = window_with(&repository, Some(1));
    apply(&mut state, vec![Intent::Graph(GraphIntent::ToggleReferences)]);
    let _ = answered_with(&mut state, "Right-click the branch: [[branch.rename]].");
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = settled(&mut window, &state);
    assert_eq!(buttons(&state, &tree), [words::SHOW_WHERE]);
    let pressed = window.click(&state, Role::Button, words::SHOW_WHERE);
    apply(&mut state, pressed);
    assert_eq!(state.page_shown(), None, "the timeline is shown");
    let tree = settled(&mut window, &state);
    assert!(has(&tree, Role::Button, HIDE_REFERENCES), "the branch rail is shown");
    assert!(!has(&tree, Role::Button, SHOW_REFERENCES));
    let said = state.operations.said.as_ref().map(|said| said.text.as_str());
    assert_eq!(said, Some("Right-click a branch in the branch rail."));
}

#[test]
fn help_show_me__an_answer_that_names_six_features_has_four_buttons_in_the_order_they_were_named() {
    let repository = one_commit();
    let mut state = window_with(&repository, Some(1));
    let named = "First [[file-search.page]], then [[stash.push]], [[branch.rename]], [[welcome.page]], \
                 [[sync.fetch]] and [[sync.push]].";
    let _ = answered_with(&mut state, named);
    let shown: Vec<&str> = state.help.exchanges[0]
        .buttons
        .iter()
        .map(|button| button.feature.0)
        .collect();
    assert_eq!(
        shown,
        ["file-search.page", "stash.push", "branch.rename", "welcome.page"]
    );
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = settled(&mut window, &state);
    assert_eq!(
        buttons(&state, &tree),
        ["Open Search", "Stash Changes", words::SHOW_WHERE, "Open Welcome"]
    );
}

#[test]
fn help_show_me__nothing_runs_until_a_button_is_pressed() {
    let repository = one_commit();
    let mut state = window_with(&repository, Some(1));
    let effects = answered_with(&mut state, "Choose [[stash.push]], or [[sync.fetch]] first.");
    assert_eq!(effects, Vec::new(), "the answer asks nothing of git");
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let _ = settled(&mut window, &state);
    let _ = settled(&mut window, &state);
    assert_eq!(state.operations.running, None);
    assert_eq!(state.page_shown(), Some(Page::Help));
    assert_eq!(state.help.exchanges[0].buttons.len(), 2);
}
