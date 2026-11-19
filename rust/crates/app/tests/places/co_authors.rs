//! The co-authors of the next commit (`commit.co-authors`, `commit.options`, `commit.option-arguments`): the picker of
//! the authors of the history, the co-authors in the menu of the options and under the commit box, and the commits
//! that name them, on repositories made for each test.

use app::store::State;
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::changes::{ChangesIntent, CoAuthorsIntent, OptionsIntent};

use super::commit_options::{changes_page, commit, line_under_the_box, open_menu, stage, staged};
use super::support::{OperationsDriver, Window, find, has};

/// The size of the window.
const WIDE: (u32, u32) = (1000, 720);

/// A repository with `a.txt` staged, whose history has three commits of Ben, one of Cy and two of Ada, the identity
/// that commits.
fn with_authors() -> Repository {
    let repository = staged();
    for (author, message) in [
        ("Cy <cy@example.com>", "Add the readme"),
        ("Ben <ben@example.com>", "Fix the parser"),
        ("Ada <ada@example.com>", "Fix the lexer"),
        ("Ben <ben@example.com>", "Fix the printer"),
        ("Ben <ben@example.com>", "Fix the checker"),
    ] {
        let _ = repository.git(&["commit", "--quiet", "--allow-empty", "-m", message, "--author", author]);
    }
    stage(&repository, "a, changed by the authors\n");
    repository
}

/// Something asked of the picker.
fn picker(driver: &OperationsDriver, state: &mut State, intent: CoAuthorsIntent) {
    driver.intent(state, Intent::Changes(ChangesIntent::CoAuthors(intent)));
}

/// A window on the Changes page of a repository with the picker of the co-authors open and its authors read.
fn picker_open(repository: &Repository) -> (State, OperationsDriver) {
    let (mut state, driver) = changes_page(vec![repository.path().to_path_buf()]);
    driver.intent(
        &mut state,
        Intent::Changes(ChangesIntent::Options(OptionsIntent::ChooseCoAuthors)),
    );
    (state, driver)
}

/// The tree of a frame of the window.
fn drawn(state: &State) -> TreeUpdate {
    let mut window = Window::new(ThemeName::Dark, WIDE);
    window.draw(state, &[]).1
}

/// Whether the tree has a check box whose name starts with **Add**.
fn offers_to_add(tree: &TreeUpdate) -> bool {
    tree.nodes
        .iter()
        .any(|(_, node)| node.role() == Role::CheckBox && node.label().is_some_and(|label| label.starts_with("Add ")))
}

/// Ticks Ben and Cy in the picker.
fn tick_ben_and_cy(driver: &OperationsDriver, state: &mut State) {
    for (name, email) in [("Ben", "ben@example.com"), ("Cy", "cy@example.com")] {
        let person = model::commit::CoAuthor {
            name: String::from(name),
            email: String::from(email),
        };
        picker(driver, state, CoAuthorsIntent::Tick(person));
    }
}

#[test]
fn commit_co_authors__the_picker_lists_the_authors_with_the_most_commits_first_without_the_identity() {
    let repository = with_authors();
    let (state, _driver) = picker_open(&repository);
    let tree = drawn(&state);
    assert!(super::support::says(&tree, "Co-authors of this commit"));
    let top = |name: &str| find(&tree, Role::CheckBox, name).bounds().unwrap().y0;
    assert!(top("Ben") < top("Cy"), "Ben has more commits");
    assert!(
        !has(&tree, Role::CheckBox, "Ada"),
        "the identity that commits is left out"
    );
}

#[test]
fn commit_co_authors__typing_ben_keeps_ben_by_his_name_or_his_email() {
    let repository = with_authors();
    let (mut state, driver) = picker_open(&repository);
    for typed in ["ben", "BEN@example"] {
        picker(&driver, &mut state, CoAuthorsIntent::Typed(String::from(typed)));
        let tree = drawn(&state);
        assert!(has(&tree, Role::CheckBox, "Ben"), "{typed}");
        assert!(!has(&tree, Role::CheckBox, "Cy"), "{typed}");
    }
}

#[test]
fn commit_co_authors__typing_a_name_and_an_email_offers_to_add_them_and_chosen_they_are_a_co_author() {
    let repository = with_authors();
    let (mut state, driver) = picker_open(&repository);
    picker(
        &driver,
        &mut state,
        CoAuthorsIntent::Typed(String::from("Dee <dee@example.com>")),
    );
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::CheckBox, "Add Dee <dee@example.com>");
    for intent in intents {
        driver.intent(&mut state, intent);
    }
    picker(&driver, &mut state, CoAuthorsIntent::Keep);
    assert_eq!(line_under_the_box(&state).as_deref(), Some("With Dee"));
}

#[test]
fn commit_co_authors__typing_a_name_alone_offers_nothing_to_add() {
    let repository = with_authors();
    let (mut state, driver) = picker_open(&repository);
    picker(&driver, &mut state, CoAuthorsIntent::Typed(String::from("Dee")));
    assert!(!offers_to_add(&drawn(&state)));
}

#[test]
fn commit_co_authors__escape_after_ticking_two_authors_leaves_the_co_authors_as_they_were() {
    let repository = with_authors();
    let (mut state, driver) = picker_open(&repository);
    tick_ben_and_cy(&driver, &mut state);
    let escape = ui::InputEvent::KeyPressed(ui::KeyEvent::press(
        ui::Key::Named(ui::NamedKey::Escape),
        ui::Modifiers::NONE,
    ));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = window.draw(&state, &[]);
    let (intents, _) = window.draw(&state, &[escape]);
    assert!(
        intents.contains(&Intent::Changes(ChangesIntent::CoAuthors(CoAuthorsIntent::Cancel))),
        "{intents:?}"
    );
    for intent in intents {
        driver.intent(&mut state, intent);
    }
    assert_eq!(line_under_the_box(&state), None);
    assert!(state.changes.co_authors.is_none(), "the picker is closed");
}

#[test]
fn commit_options__two_co_authors_chosen_are_listed_in_the_menu_and_under_the_box_and_gone_after_the_commit() {
    let repository = with_authors();
    let (mut state, driver) = picker_open(&repository);
    tick_ben_and_cy(&driver, &mut state);
    picker(&driver, &mut state, CoAuthorsIntent::Keep);
    assert_eq!(line_under_the_box(&state).as_deref(), Some("With Ben, +1"));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    open_menu(&driver, &mut state, &mut window);
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::MenuItem, "Ben <ben@example.com>"));
    assert!(has(&tree, Role::MenuItem, "Cy <cy@example.com>"));
    let _ = window.draw(
        &state,
        &[ui::InputEvent::KeyPressed(ui::KeyEvent::press(
            ui::Key::Named(ui::NamedKey::Escape),
            ui::Modifiers::NONE,
        ))],
    );
    let ran = commit(&driver, &mut state, "Fix the parser");
    assert_eq!(
        ran,
        "git commit --trailer \"Co-authored-by: Ben <ben@example.com>\" --trailer \"Co-authored-by: Cy <cy@example.com>\" \
         -m \"Fix the parser\""
    );
    assert_eq!(line_under_the_box(&state), None);
}

#[test]
fn commit_option_arguments__an_amend_with_an_empty_message_sign_off_and_a_co_author_ends_the_message_with_the_two_lines()
 {
    let repository = with_authors();
    let (mut state, driver) = picker_open(&repository);
    tick_ben_and_cy(&driver, &mut state);
    let cy = model::commit::CoAuthor {
        name: String::from("Cy"),
        email: String::from("cy@example.com"),
    };
    picker(&driver, &mut state, CoAuthorsIntent::Tick(cy));
    picker(&driver, &mut state, CoAuthorsIntent::Keep);
    driver.intent(
        &mut state,
        Intent::Changes(ChangesIntent::Options(OptionsIntent::SignOff(true))),
    );
    driver.intent(&mut state, Intent::Changes(ChangesIntent::Amend(true)));
    let ran = commit(&driver, &mut state, "");
    assert_eq!(
        ran,
        "git commit --amend --signoff --trailer \"Co-authored-by: Ben <ben@example.com>\" --no-edit"
    );
    let message = repository.git(&["log", "-1", "--format=%B"]);
    let lines: Vec<&str> = message.trim_end().lines().collect();
    assert_eq!(
        lines[lines.len() - 2..],
        [
            "Signed-off-by: Ada <ada@example.com>",
            "Co-authored-by: Ben <ben@example.com>"
        ],
        "{message}"
    );
}
