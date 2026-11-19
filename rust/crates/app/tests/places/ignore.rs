//! Ignoring a file and stopping tracking it from the menu of its row in the changes and from the context menu of the
//! Files page (`changes.row-menu`, `changes.ignore`, `changes.stop-tracking`).

use app::store::{Message, State};
use model::contract::{Answer, FeatureId, Target};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::feature::FeatureIntent;
use views::text::changes as words;

use super::changes_beside::{changes_page, three_changed};
use super::changes_menu::menu_of;
use super::files_page::{choose, files_page};
use super::support::{OperationsDriver, Window};

/// A window wide enough for the whole Changes page.
const WIDE: (u32, u32) = (1200, 640);

/// Gives what was asked, answers each question of the operation by the choices given in turn, accepts its
/// confirmation, and waits for it to end.
fn answer_until_ended(driver: &OperationsDriver, state: &mut State, intents: Vec<Intent>, choices: &[&str]) {
    for intent in intents {
        driver.intent(state, intent);
    }
    let mut choices = choices.iter();
    loop {
        match driver.next(state) {
            Message::FeatureAsks(_) => {
                let Some(choice) = choices.next() else {
                    panic!("a choice for each question");
                };
                let answer = Answer::Choice((*choice).to_owned());
                driver.intent(state, Intent::Feature(FeatureIntent::Answer(Some(answer))));
            }
            Message::FeatureConfirms(_) => driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureEnded { .. } => return,
            _ => {}
        }
    }
}

/// The text of a file of the working tree.
fn text(repository: &Repository, name: &str) -> String {
    std::fs::read_to_string(repository.path().join(name)).unwrap_or_default()
}

#[test]
fn changes_row_menu__an_untracked_file_has_ignore_and_a_tracked_one_stop_tracking_too() {
    let repository = three_changed();
    std::fs::write(repository.path().join("debug.log"), "started\n").unwrap();
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let untracked = menu_of(&mut window, &state, "debug.log");
    assert_eq!(
        untracked.last().map(String::as_str),
        Some(words::IGNORE),
        "{untracked:?}"
    );
    assert!(!untracked.iter().any(|item| item == words::STOP_TRACKING));
    let tracked = menu_of(&mut window, &state, "a.txt");
    assert!(tracked.iter().any(|item| item == words::STOP_TRACKING), "{tracked:?}");
    let _ = menu_of(&mut window, &state, "debug.log");
    let intents = window.click(&state, Role::MenuItem, words::IGNORE);
    let asked = Intent::Feature(FeatureIntent::Run(
        FeatureId("changes.ignore"),
        Target::File(String::from("debug.log")),
    ));
    assert!(intents.contains(&asked), "{intents:?}");
    answer_until_ended(&driver, &mut state, intents, &["1", "0"]);
    assert_eq!(text(&repository, ".gitignore"), "*.log\n");
    let status = repository.git(&["status", "--porcelain=v1"]);
    assert!(
        !status.contains("debug.log") && status.contains("?? .gitignore"),
        "{status}"
    );
}

#[test]
fn changes_ignore__this_folder_on_a_folder_of_the_files_page_writes_it_with_a_slash_at_its_end() {
    let mut repository = Repository::new();
    let _first = repository.commit(
        "Add the sources",
        &[
            (
                "src/main.rs",
                "fn main() {}
",
            ),
            (
                "generated/a.rs",
                "a
",
            ),
            (
                "generated/b.rs",
                "b
",
            ),
        ],
    );
    let (mut state, mut window, driver) = files_page(&repository, &[]);
    let intents = choose(&mut window, &state, ("generated", words::IGNORE));
    answer_until_ended(&driver, &mut state, intents, &["0", "0", "only-write"]);
    assert_eq!(
        text(&repository, ".gitignore"),
        "/generated/
"
    );
}

#[test]
fn changes_stop_tracking__from_the_files_page_the_file_stays_on_disk_and_its_deletion_is_staged() {
    let mut repository = Repository::new();
    let _first = repository.commit("Add the settings", &[("config/local.json", "{}\n")]);
    let (mut state, mut window, driver) = files_page(&repository, &["config"]);
    let intents = choose(&mut window, &state, ("local.json", words::STOP_TRACKING));
    answer_until_ended(&driver, &mut state, intents, &[]);
    assert_eq!(text(&repository, "config/local.json"), "{}\n");
    let status = repository.git(&["status", "--porcelain=v1"]);
    assert!(status.contains("D  config/local.json"), "{status}");
}
