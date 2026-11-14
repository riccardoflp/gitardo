//! The page that chooses what new worktrees get (`worktree-setup.page`), drawn from the store on a repository made for
//! each test: what git ignores in sections, the buttons that choose what a line gets, the patterns git is asked about
//! at **Look again**, the sections closed by hand, and the settings changed elsewhere.

use app::store::{Effect, Message, State, reduce};
use model::worktree_setup::Section;
use platform::accesskit::{Role, TreeUpdate};
use render::{Point, ThemeName};
use testkit::Repository;
use views::Intent;
use views::search::QuickCommand;
use views::text::worktree_setup as words;
use views::worktree_setup::SetupIntent;

use super::blame::quick;
use super::pointer::{middle, rect as rect_of};
use super::showing;
use super::support::{self, OperationsDriver, Window, says, says_in};

/// The size of the window of the page: high enough for every row.
const SIZE: (u32, u32) = (1100, 1400);

/// A checkout with `node_modules` at the top and in `packages/api`, `.env` and `dist` ignored.
fn checkout() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit(
        "Ignore what is made",
        &[
            (".gitignore", "node_modules/\n.env\ndist/\n"),
            ("packages/api/index.js", "export {};\n"),
        ],
    );
    for (path, text) in [
        ("node_modules/left-pad/index.js", "module.exports = 1;\n"),
        ("packages/api/node_modules/left-pad/index.js", "module.exports = 2;\n"),
        (".env", "TOKEN=test\n"),
        ("dist/app.js", "built\n"),
    ] {
        let file = repository.path().join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }
    repository
}

/// A window showing the page of a checkout, opened from Quick search.
fn page_of(repository: &Repository) -> (State, Window) {
    let mut state = showing(repository.path().to_path_buf());
    quick(
        &OperationsDriver::new(),
        &mut state,
        QuickCommand::ChooseWhatNewWorktreesGet,
    );
    (state, Window::new(ThemeName::Dark, SIZE))
}

/// Reduces what a frame asked, running the reads it starts.
fn give(state: &mut State, intents: Vec<Intent>) {
    for intent in intents {
        let _effects = support::run(state, Message::Intent(intent));
    }
}

/// The bounds of the lowest node that says a text, or whose name begins with it.
fn bounds_of(tree: &TreeUpdate, text: &str) -> ui::Rect {
    let found = tree
        .nodes
        .iter()
        .filter(|(_, node)| {
            [node.label(), node.value()]
                .into_iter()
                .flatten()
                .any(|said| said.starts_with(text))
        })
        .filter_map(|(_, node)| node.bounds().map(rect_of))
        .max_by(|first, second| first.y.total_cmp(&second.y));
    found.unwrap_or_else(|| panic!("nothing says {text}"))
}

/// The middle of the button of a name on the row of a text.
fn button_on_row(tree: &TreeUpdate, (text, button): (&str, &str)) -> Point {
    let row = bounds_of(tree, text);
    let row_middle = row.y + row.height * 0.5;
    let found = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(button))
        .filter_map(|(_, node)| node.bounds().map(rect_of))
        .find(|bounds| bounds.y <= row_middle && row_middle <= bounds.y + bounds.height);
    middle(found.unwrap_or_else(|| panic!("no {button} on the row of {text}")))
}

/// The patterns the page shows.
fn patterns(state: &State) -> (Vec<String>, Vec<String>) {
    let shown = app::store::new_worktrees::page_state(state).chosen.patterns.clone();
    (shown.link, shown.copy)
}

#[test]
fn worktree_setup_page__node_modules_in_two_places_is_one_line_of_dependencies_and_build_output_is_closed() {
    let repository = checkout();
    let (state, mut window) = page_of(&repository);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, &words::title("repository")), "the trail names the page");
    assert!(says(&tree, &words::ignored_here(4)), "four things are ignored");
    assert!(says_in(&tree, words::section_title(Section::Dependencies)));
    assert!(says(&tree, "node_modules/"), "node_modules is one line");
    assert!(says_in(&tree, &words::places(2)), "of 2 places");
    assert!(!says(&tree, "packages/api/node_modules"), "its places are folded");
    assert!(says_in(&tree, words::section_title(Section::Secrets)));
    assert!(says(&tree, ".env"));
    assert!(says_in(&tree, words::section_title(Section::BuildOutput)));
    assert!(!says(&tree, "dist"), "the Build output section is closed");
}

#[test]
fn worktree_setup_page__use_what_is_proposed_links_node_modules_and_copies_env_and_runs_no_git_command() {
    let repository = checkout();
    let (mut state, mut window) = page_of(&repository);
    let intents = window.click(&state, Role::Button, words::USE_PROPOSED);
    assert_eq!(intents.len(), 1, "{intents:?}");
    for intent in intents {
        let effects = reduce(&mut state, Message::Intent(intent));
        let started = effects
            .iter()
            .filter(|effect| matches!(effect, Effect::Start(_) | Effect::RunFeature { .. }))
            .count();
        assert_eq!(started, 0, "no git command runs: {effects:?}");
    }
    let (link, copy) = patterns(&state);
    assert_eq!(link, ["node_modules/"]);
    assert_eq!(copy, ["/.env"]);
    let (_, tree) = window.draw(&state, &[]);
    assert!(
        says(&tree, words::CHANGED_SINCE),
        "the page says git was not asked since"
    );
}

#[test]
fn worktree_setup_page__nothing_for_one_place_of_node_modules_keeps_the_other_place_linked_by_its_path() {
    let repository = checkout();
    let (mut state, mut window) = page_of(&repository);
    let intents = window.click(&state, Role::Button, words::USE_PROPOSED);
    give(&mut state, intents);
    give(
        &mut state,
        vec![Intent::WorktreeSetup(SetupIntent::Unfold(String::from(
            "node_modules/",
        )))],
    );
    let (_, tree) = window.draw(&state, &[]);
    let point = button_on_row(&tree, ("packages/api/node_modules", words::NOTHING));
    let intents = super::pointer::click_at(&mut window, &state, point, 1);
    give(&mut state, intents);
    let (link, _) = patterns(&state);
    assert_eq!(link, ["/node_modules/"]);
}

#[test]
fn worktree_setup_page__a_pattern_that_matches_nothing_says_takes_nothing_here_after_look_again() {
    let repository = checkout();
    let (mut state, mut window) = page_of(&repository);
    let mut with_vendor = app::store::new_worktrees::page_state(&state).chosen.patterns.clone();
    with_vendor.link.push(String::from("vendor/"));
    give(
        &mut state,
        vec![Intent::WorktreeSetup(SetupIntent::SetPatterns(with_vendor))],
    );
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, "vendor/"));
    assert!(!says(&tree, words::TAKES_NOTHING), "git was not asked yet");
    let intents = window.click(&state, Role::Button, words::LOOK_AGAIN);
    give(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, words::TAKES_NOTHING));
    assert!(!says(&tree, words::CHANGED_SINCE));
}

#[test]
fn worktree_setup_page__a_section_closed_by_hand_is_closed_when_the_page_is_opened_again() {
    let repository = checkout();
    let (mut state, mut window) = page_of(&repository);
    let (_, tree) = window.draw(&state, &[]);
    let point = middle(bounds_of(&tree, words::section_title(Section::Dependencies)));
    let intents = super::pointer::click_at(&mut window, &state, point, 1);
    assert_eq!(
        intents,
        [Intent::WorktreeSetup(SetupIntent::ToggleSection(
            Section::Dependencies,
            false
        ))]
    );
    give(&mut state, intents);
    give(&mut state, vec![Intent::ClosePage]);
    quick(
        &OperationsDriver::new(),
        &mut state,
        QuickCommand::ChooseWhatNewWorktreesGet,
    );
    let (_, tree) = window.draw(&state, &[]);
    assert!(says_in(&tree, words::section_title(Section::Dependencies)));
    assert!(!says(&tree, "node_modules/"), "the section is still closed");
    assert!(says(&tree, ".env"), "the others are as they were");
}

#[test]
fn worktree_setup_page__a_change_of_copy_files_in_the_settings_file_shows_without_closing_the_page() {
    let repository = checkout();
    let (mut state, mut window) = page_of(&repository);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says_in(&tree, &format!("{}: nothing", words::COPIED_NOW)));
    let settings = repository.path().join(".gitardo");
    std::fs::create_dir_all(&settings).unwrap();
    std::fs::write(
        settings.join("settings.json"),
        "{ \"worktrees.copyFiles\": [\"/.env\"] }\n",
    )
    .unwrap();
    let read_again = Box::new(state.settings.clone());
    let _effects = support::run(&mut state, Message::SettingsRead(read_again));
    let (_, tree) = window.draw(&state, &[]);
    let now = words::now(
        words::COPIED_NOW,
        &[String::from("/.env")],
        "the settings of repository",
    );
    assert!(says(&tree, &now), "{now}");
    let (_, copy) = patterns(&state);
    assert_eq!(copy, ["/.env"], "the page follows what applies");
}

#[test]
fn worktree_setup_page__the_page_reads_the_same_in_the_dark_and_the_light_theme() {
    let repository = checkout();
    let (mut state, mut window) = page_of(&repository);
    let intents = window.click(&state, Role::Button, words::USE_PROPOSED);
    give(&mut state, intents);
    give(
        &mut state,
        vec![
            Intent::WorktreeSetup(SetupIntent::Unfold(String::from("node_modules/"))),
            Intent::WorktreeSetup(SetupIntent::LookAgain),
        ],
    );
    for (theme, name) in [
        (ThemeName::Dark, "new-worktrees-dark"),
        (ThemeName::Light, "new-worktrees-light"),
    ] {
        let mut window = Window::new(theme, (1100, 1250));
        let (_, tree) = window.draw(&state, &[]);
        assert!(says(&tree, words::HEADER));
        window.assert_snapshot(name);
    }
}
