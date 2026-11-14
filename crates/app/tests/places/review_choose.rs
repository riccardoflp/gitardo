//! The choices of a card of the review: a side, both in either order, or lines written in the box **Edit** opens;
//! accepted at once, kept while the window is open, and never asked about to the AI provider.

use app::store::State;
use model::review::{ChoiceKind, ProposalSource};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use ui::{Key, Modifiers, NamedKey};
use views::Intent;
use views::review::{CardProposal, ConflictCard, ReviewIntent};

use super::compare::until_ended;
use super::diff::{key as pressed, press};
use super::pointer::apply;
use super::review::{NEXT_TO_EACH_OTHER, SAME_LINE, cards, drawn, file, key, merged, read_again, review, reviewed};
use super::support::{OperationsDriver, Window, has, typed};

/// A file whose one line, a version, was changed in two ways, in Windows line endings.
const VERSIONS: (&str, &str, &str) = (
    "{\r\n  \"version\": \"1.2.0\"\r\n}\r\n",
    "{\r\n  \"version\": \"1.2.1\"\r\n}\r\n",
    "{\r\n  \"version\": \"1.3.0\"\r\n}\r\n",
);

/// The only card of the page.
fn card(state: &State) -> ConflictCard {
    cards(state).into_iter().next().unwrap()
}

/// What the only card proposes.
fn proposal(state: &State) -> CardProposal {
    card(state).proposal.unwrap()
}

/// The texts of the lines the only card proposes.
fn proposed(state: &State) -> Vec<String> {
    proposal(state).lines.into_iter().map(|line| line.line.text).collect()
}

/// Makes a choice on the conflict of a file at a place among its conflicts.
fn choose(state: &mut State, (path, index): (&str, usize), kind: ChoiceKind) {
    let key = key(state, path, index);
    review(state, ReviewIntent::Choose(key, kind));
}

/// The text of the box **Edit** opened, while it is open.
fn in_the_box(state: &State) -> Option<String> {
    state
        .reviews
        .lines
        .as_ref()
        .map(|lines| lines.document.buffer().text().to_owned())
}

/// A window wide enough for the list beside the cards.
fn window() -> Window {
    Window::new(ThemeName::Dark, (1400, 1000))
}

/// Clicks **Edit** on the only card, and draws the box once so that it takes the keyboard.
fn edit(window: &mut Window, state: &mut State) {
    let intents = window.click(state, Role::Button, "Edit");
    apply(state, intents);
    let _ = window.draw(state, &[]);
}

/// Writes what is accepted from the page, confirming what is asked, and gives the thread of the operations that ran it.
fn write(state: &mut State) -> OperationsDriver {
    let driver = OperationsDriver::new();
    driver.intent(state, Intent::Review(ReviewIntent::Write));
    until_ended(&driver, state);
    driver
}

/// The text of a file of the repository.
fn on_disk(repository: &Repository, path: &str) -> String {
    std::fs::read_to_string(repository.path().join(path)).unwrap()
}

#[test]
fn review_choose__a_choice_takes_the_place_of_the_rules_is_accepted_at_once_and_changes_no_file() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER)]);
    let mut state = reviewed(&repository);
    assert!(matches!(proposal(&state).source, ProposalSource::Rules(_)));
    let before = on_disk(&repository, "one.txt");
    let logged = state.log.entries().count();
    choose(&mut state, ("one.txt", 0), ChoiceKind::IncomingOnly);
    assert_eq!(
        proposal(&state).source,
        ProposalSource::Choice(ChoiceKind::IncomingOnly)
    );
    assert!(card(&state).is_accepted);
    assert_eq!(on_disk(&repository, "one.txt"), before);
    assert_eq!(state.log.entries().count(), logged, "nothing ran");
}

#[test]
fn review_choose__both_current_first_chosen_offers_the_other_choices_and_not_itself() {
    let repository = merged(&[file("one.txt", SAME_LINE)]);
    let mut state = reviewed(&repository);
    choose(&mut state, ("one.txt", 0), ChoiceKind::BothCurrentFirst);
    assert_eq!(
        card(&state).choices,
        [
            ChoiceKind::CurrentOnly,
            ChoiceKind::IncomingOnly,
            ChoiceKind::BothIncomingFirst,
            ChoiceKind::Written
        ]
    );
    review(&mut state, ReviewIntent::FoldFile(String::from("one.txt"), false));
    let tree = drawn(&state);
    assert!(has(&tree, Role::Button, "Both, incoming first"));
    assert!(!has(&tree, Role::Button, "Both, current first"));
}

#[test]
fn review_choose__the_sides_of_a_version_are_proposed_alone_or_one_after_the_other_in_the_line_endings_of_the_file() {
    let repository = merged(&[file("package.json", VERSIONS)]);
    let mut state = reviewed(&repository);
    choose(&mut state, ("package.json", 0), ChoiceKind::CurrentOnly);
    assert_eq!(proposed(&state), ["  \"version\": \"1.2.1\""]);
    choose(&mut state, ("package.json", 0), ChoiceKind::IncomingOnly);
    assert_eq!(proposed(&state), ["  \"version\": \"1.3.0\""]);
    choose(&mut state, ("package.json", 0), ChoiceKind::BothCurrentFirst);
    assert_eq!(
        proposed(&state),
        ["  \"version\": \"1.2.1\"", "  \"version\": \"1.3.0\""]
    );
    let _driver = write(&mut state);
    assert_eq!(
        on_disk(&repository, "package.json"),
        "{\r\n  \"version\": \"1.2.1\"\r\n  \"version\": \"1.3.0\"\r\n}\r\n"
    );
}

#[test]
fn review_choose__edit_opens_the_lines_proposed_and_ctrl_enter_takes_them_with_the_new_line_counted() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER)]);
    let mut state = reviewed(&repository);
    let lines = proposed(&state).join("\n");
    let mut window = window();
    edit(&mut window, &mut state);
    assert_eq!(in_the_box(&state).as_deref(), Some(lines.as_str()));
    let _ = press(
        &mut window,
        &mut state,
        &[typed("x"), pressed(Key::Named(NamedKey::Enter), Modifiers::NONE)],
    );
    let enter = pressed(Key::Named(NamedKey::Enter), Modifiers::CONTROL);
    let _ = press(&mut window, &mut state, &[enter]);
    assert_eq!(in_the_box(&state), None, "the box closed");
    let proposal = proposal(&state);
    assert_eq!(proposal.source, ProposalSource::Choice(ChoiceKind::Written));
    assert_eq!(proposed(&state).first().map(String::as_str), Some("x"));
    assert_eq!(proposal.counts.new, 1);
    assert!(card(&state).is_accepted);
}

#[test]
fn review_choose__edit_with_nothing_proposed_opens_both_sides_and_escape_or_cancel_leave_them() {
    let repository = merged(&[file("one.txt", SAME_LINE)]);
    let mut state = reviewed(&repository);
    let mut window = window();
    edit(&mut window, &mut state);
    assert_eq!(in_the_box(&state).as_deref(), Some("B1\nB2"));
    let _ = press(
        &mut window,
        &mut state,
        &[pressed(Key::Named(NamedKey::Escape), Modifiers::NONE)],
    );
    assert_eq!(in_the_box(&state), None);
    assert_eq!(card(&state).proposal, None);
    edit(&mut window, &mut state);
    let intents = window.click(&state, Role::Button, "Use these lines");
    assert_eq!(intents, [Intent::Review(ReviewIntent::UseLines)]);
    let intents = window.click(&state, Role::Button, "Cancel");
    assert_eq!(intents, [Intent::Review(ReviewIntent::CancelLines)]);
    apply(&mut state, intents);
    assert_eq!(in_the_box(&state), None);
    assert!(!card(&state).is_accepted);
}

#[test]
fn review_choose__the_ai_is_not_asked_about_a_conflict_with_a_choice() {
    let repository = merged(&[file("one.txt", SAME_LINE), file("two.txt", SAME_LINE)]);
    let mut state = reviewed(&repository);
    state.settings.features.conflicts_ai = true;
    choose(&mut state, ("one.txt", 0), ChoiceKind::CurrentOnly);
    review(&mut state, ReviewIntent::AskAi(None));
    let asked: Vec<&str> = state
        .reviews
        .questions
        .failed
        .iter()
        .map(|(_, (path, _), _)| path.as_str())
        .collect();
    assert_eq!(
        asked,
        ["two.txt"],
        "with no provider set up, each question asked fails at once"
    );
}

#[test]
fn review_choose__a_choice_is_kept_when_the_page_closes_and_after_undo_and_gone_once_the_conflict_is_edited() {
    let repository = merged(&[file("one.txt", SAME_LINE)]);
    let mut state = reviewed(&repository);
    choose(&mut state, ("one.txt", 0), ChoiceKind::IncomingOnly);
    let chosen = Some(ProposalSource::Choice(ChoiceKind::IncomingOnly));
    let _ = super::support::run(&mut state, app::store::Message::Intent(Intent::ClosePage));
    super::review::open(&mut state);
    assert_eq!(card(&state).proposal.map(|proposal| proposal.source), chosen);
    review(&mut state, ReviewIntent::AcceptAll);
    let driver = write(&mut state);
    assert_eq!(on_disk(&repository, "one.txt"), "a\nB2\nc\n");
    driver.intent(&mut state, Intent::Review(ReviewIntent::Undo));
    until_ended(&driver, &mut state);
    read_again(&mut state);
    assert_eq!(card(&state).proposal.map(|proposal| proposal.source), chosen);
    let edited = on_disk(&repository, "one.txt").replacen("B1\n", "B1 by hand\n", 1);
    std::fs::write(repository.path().join("one.txt"), edited).unwrap();
    read_again(&mut state);
    assert_eq!(card(&state).proposal, None);
}
