//! The Changes page: the one list of the changed files with their boxes, the commit box and the identity under it, and
//! the operations run from them on the thread of the operations.

use app::store::{Message, State};
use model::contract::{Answer, CommandPreview};
use platform::accesskit::{Role, Toggled, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::changes::ChangesIntent;
use views::feature::FeatureIntent;
use views::page::Page;

use super::showing;
use super::support::{OperationsDriver, Window, find, has, says, typed};

/// The size of the window of the pictures.
const WIDE: (u32, u32) = (1000, 520);

/// A repository with one commit of `a.txt`, `b.txt` and `gone.txt`, and the identity `Ada <ada@example.com>` of its
/// own.
fn committed() -> Repository {
    let mut repository = Repository::new();
    with_identity(&repository, "Ada", "ada@example.com");
    let _ = repository.commit(
        "Add the files",
        &[("a.txt", "a\n"), ("b.txt", "b\n"), ("gone.txt", "gone\n")],
    );
    repository
}

/// Writes a file of a repository.
fn write(repository: &Repository, path: &str, text: &str) {
    let file = repository.path().join(path);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(file, text).unwrap();
}

/// Sets the identity of a repository in its own configuration: the window reads git with the configuration of whoever
/// runs the tests, which the pictures must not show.
fn with_identity(repository: &Repository, name: &str, email: &str) {
    let _ = repository.git(&["config", "user.name", name]);
    let _ = repository.git(&["config", "user.email", email]);
}

/// A window showing a repository with its Changes page open.
fn changes_page(repository: &Repository) -> (State, OperationsDriver) {
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    driver.intent(&mut state, Intent::Changes(ChangesIntent::Open));
    (state, driver)
}

/// Reduces the intents of a frame.
fn apply(driver: &OperationsDriver, state: &mut State, intents: Vec<Intent>) {
    for intent in intents {
        driver.intent(state, intent);
    }
}

/// Clicks the widget of a role and a name, and reduces what the click asks.
fn click(driver: &OperationsDriver, state: &mut State, window: &mut Window, (role, name): (Role, &str)) {
    let intents = window.click(state, role, name);
    apply(driver, state, intents);
}

/// Waits for the operation running to end, the questions the store answers itself answered meanwhile.
fn until_ended(driver: &OperationsDriver, state: &mut State) {
    for _ in 0..8 {
        if matches!(driver.next(state), Message::FeatureEnded { .. }) {
            return;
        }
    }
    panic!("the operation does not end");
}

/// `Ctrl+Enter`.
fn ctrl_enter() -> ui::InputEvent {
    ui::InputEvent::KeyPressed(ui::KeyEvent::press(
        ui::Key::Named(ui::NamedKey::Enter),
        ui::Modifiers::CONTROL,
    ))
}

/// How a check box of the tree is ticked.
fn ticked(tree: &TreeUpdate, name: &str) -> Option<Toggled> {
    find(tree, Role::CheckBox, name).toggled()
}

/// The top of the node of a role and a name.
fn top(tree: &TreeUpdate, role: Role, name: &str) -> f64 {
    find(tree, role, name).bounds().unwrap().y0
}

#[test]
fn changes_pane__a_file_staged_one_partly_staged_and_one_untracked_are_three_rows_ticked_half_ticked_and_empty() {
    let repository = committed();
    write(&repository, "a.txt", "a, staged\n");
    let _ = repository.git(&["add", "a.txt"]);
    write(&repository, "b.txt", "b, staged\n");
    let _ = repository.git(&["add", "b.txt"]);
    write(&repository, "b.txt", "b, staged, then changed\n");
    write(&repository, "c.txt", "new\n");
    let (state, _driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(ticked(&tree, "a.txt"), Some(Toggled::True));
    assert_eq!(ticked(&tree, "b.txt"), Some(Toggled::Mixed));
    assert_eq!(ticked(&tree, "c.txt"), Some(Toggled::False));
    let (a, b, c) = (
        top(&tree, Role::CheckBox, "a.txt"),
        top(&tree, Role::CheckBox, "b.txt"),
        top(&tree, Role::CheckBox, "c.txt"),
    );
    assert!(a < b && b < c, "the rows are in the order of their paths");
    assert_eq!(ticked(&tree, "Changes 3"), Some(Toggled::Mixed));
    window.assert_snapshot("changes-page-dark");
}

#[test]
fn changes_pane__ctrl_enter_in_the_message_box_commits_and_does_nothing_while_the_button_is_disabled() {
    let repository = committed();
    write(&repository, "a.txt", "a, staged\n");
    let _ = repository.git(&["add", "a.txt"]);
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (focused, _) = window.draw(&state, &[]);
    apply(&driver, &mut state, focused);
    let (pressed, _) = window.draw(&state, &[ctrl_enter()]);
    assert!(
        !pressed.contains(&Intent::Changes(ChangesIntent::Commit)),
        "with no message the button is disabled, and Ctrl+Enter does nothing"
    );
    let (typed_intents, _) = window.draw(&state, &[typed("Fix the parser")]);
    apply(&driver, &mut state, typed_intents);
    let (pressed, _) = window.draw(&state, &[ctrl_enter()]);
    assert!(pressed.contains(&Intent::Changes(ChangesIntent::Commit)));
    apply(&driver, &mut state, pressed);
    until_ended(&driver, &mut state);
    assert_eq!(repository.git(&["log", "-1", "--format=%s"]).trim(), "Fix the parser");
}

#[test]
fn commit_create__after_the_commit_the_message_box_is_empty() {
    let repository = committed();
    write(&repository, "a.txt", "a, staged\n");
    let _ = repository.git(&["add", "a.txt"]);
    let (mut state, driver) = changes_page(&repository);
    driver.intent(
        &mut state,
        Intent::Changes(ChangesIntent::Type(String::from("Fix the parser"))),
    );
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click(&driver, &mut state, &mut window, (Role::Button, "Commit"));
    until_ended(&driver, &mut state);
    let lines: Vec<String> = state.log.entries().map(|entry| entry.line.clone()).collect();
    assert_eq!(lines, ["git commit -m \"Fix the parser\""]);
    let (_, tree) = window.draw(&state, &[]);
    let message = find(&tree, Role::TextInput, "Message (Ctrl+Enter to commit on main)");
    assert_eq!(message.value(), Some(""));
    assert!(
        state.persisted.commit_messages.is_empty(),
        "the message kept is forgotten"
    );
}

#[test]
fn commit_create__with_a_message_of_spaces_and_blank_lines_the_button_is_disabled() {
    let repository = committed();
    write(&repository, "a.txt", "a, changed\n");
    let (mut state, driver) = changes_page(&repository);
    driver.intent(&mut state, Intent::Changes(ChangesIntent::Type(String::from("  \n  "))));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(find(&tree, Role::Button, "Commit").is_disabled());
    driver.intent(&mut state, Intent::Changes(ChangesIntent::Type(String::from("Fix"))));
    let (_, tree) = window.draw(&state, &[]);
    assert!(!find(&tree, Role::Button, "Commit").is_disabled());
}

#[test]
fn commit_amend__in_a_repository_with_no_commit_the_box_cannot_be_ticked() {
    let repository = Repository::new();
    write(&repository, "first.txt", "first\n");
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(find(&tree, Role::CheckBox, "Amend last commit").is_disabled());
    driver.intent(&mut state, Intent::Changes(ChangesIntent::Amend(true)));
    assert!(!state.changes.is_amend, "nothing can tick it");
}

#[test]
fn changes_pane__view_as_tree_is_kept_when_the_page_is_opened_again_and_after_the_window_is_closed() {
    let repository = committed();
    write(&repository, "src/main.rs", "fn main() {}\n");
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click(&driver, &mut state, &mut window, (Role::Button, "View as tree"));
    apply(&driver, &mut state, vec![Intent::ClosePage]);
    apply(&driver, &mut state, vec![Intent::ShowPage(Page::Changes)]);
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::Button, "View as list"), "the tree is kept");
    assert!(has(&tree, Role::Button, "src"), "the file is under its directory");
    let again = app::persisted::Persisted::from_text(&state.persisted.to_text());
    assert_eq!(again.file_layouts.changes, views::details::FileLayout::Tree);
}

#[test]
fn changes_pane__a_deleted_file_has_no_open_file_button() {
    let repository = committed();
    std::fs::remove_file(repository.path().join("gone.txt")).unwrap();
    let (state, _driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::Button, "Open changes"));
    assert!(!has(&tree, Role::Button, "Open file"));
}

#[test]
fn changes_pane__with_no_repository_in_the_workspace_the_page_says_so_with_check_again() {
    let folder = Repository::new();
    let empty = folder.path().parent().unwrap().join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let mut state = showing(empty);
    let driver = OperationsDriver::new();
    driver.intent(&mut state, Intent::Changes(ChangesIntent::Open));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, "No Git repository in this workspace"));
    assert_eq!(window.click(&state, Role::Button, "Check again"), [Intent::CheckAgain]);
}

#[test]
fn changes_pane__the_identity_line_reads_name_and_email_or_no_identity_configured() {
    let repository = committed();
    write(
        &repository,
        "a.txt",
        "a, changed
",
    );
    let (state, _driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::Button, "Ada <ada@example.com>"));
    // The window reads git with the global configuration of whoever runs the tests: the page is drawn alone for a
    // repository with no identity.
    let list = model::changes::ChangesList::default();
    let page = views::changes::ChangesState::Repository(views::changes::ChangesRepository {
        list: &list,
        files: &[],
        tree: &[],
        layout: views::details::FileLayout::List,
        folded: &std::collections::BTreeSet::new(),
        commit: views::changes::CommitBox::default(),
        identity: &model::state::Identity::default(),
        is_running: false,
        wants_focus: false,
        on_click: model::changes::OpenOnClick::File,
        banner: None,
        worktree: None,
        marked: None,
        serial: 0,
    });
    let (_, tree) = window.frame(&[], |ui| views::changes::pane(&page, ui));
    assert!(has(&tree, Role::Button, "No identity configured"));
}

#[test]
fn changes_pane__stash_only_the_staged_changes_is_there_only_while_something_is_staged() {
    let repository = committed();
    write(&repository, "a.txt", "a, changed\n");
    let (state, _driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::Button, "Stash only the staged changes\u{2026}"));
    assert!(!has(&tree, Role::Button, "Save all"), "no file has edits not saved");
    let _ = repository.git(&["add", "a.txt"]);
    let (state, _driver) = changes_page(&repository);
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::Button, "Stash only the staged changes\u{2026}"));
}

#[test]
fn changes_stage_all__with_no_conflict_ticking_the_header_box_runs_git_add_all_untracked_included() {
    let repository = committed();
    write(&repository, "a.txt", "a, changed\n");
    write(&repository, "new.txt", "new\n");
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click(&driver, &mut state, &mut window, (Role::CheckBox, "Changes 2"));
    until_ended(&driver, &mut state);
    let lines: Vec<String> = state.log.entries().map(|entry| entry.line.clone()).collect();
    assert_eq!(lines, ["git add --all"]);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(ticked(&tree, "new.txt"), Some(Toggled::True));
    assert_eq!(ticked(&tree, "Changes 2"), Some(Toggled::True));
}

#[test]
fn changes_unstage__unticking_a_box_unstages_the_file() {
    let repository = committed();
    write(&repository, "a.txt", "a, staged\n");
    let _ = repository.git(&["add", "a.txt"]);
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click(&driver, &mut state, &mut window, (Role::CheckBox, "a.txt"));
    until_ended(&driver, &mut state);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(ticked(&tree, "a.txt"), Some(Toggled::False));
}

#[test]
fn preview_confirm__with_command_preview_all_staging_a_file_shows_git_add_with_its_literal_name_before_it_runs() {
    let repository = committed();
    write(&repository, "pages/[id].tsx", "export default 1;\n");
    let (mut state, driver) = changes_page(&repository);
    state.settings.features.command_preview = CommandPreview::All;
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click(&driver, &mut state, &mut window, (Role::CheckBox, "pages/[id].tsx"));
    let Message::FeatureConfirms(plan) = driver.next(&mut state) else {
        panic!("the plan is shown first");
    };
    assert_eq!(
        views::feature::plan_lines(&plan),
        ["git add -- :(literal)pages/[id].tsx"]
    );
    let _ = window.draw(&state, &[]);
    window.assert_snapshot("changes-stage-preview-dark");
    assert_eq!(
        repository.git(&["diff", "--cached", "--name-only"]).trim(),
        "",
        "nothing ran yet"
    );
    click(&driver, &mut state, &mut window, (Role::Button, "Run"));
    until_ended(&driver, &mut state);
    assert_eq!(
        repository.git(&["diff", "--cached", "--name-only"]).trim(),
        "pages/[id].tsx"
    );
}

#[test]
fn identity_switch__the_identity_under_the_commit_box_shows_the_new_one_once_read_again() {
    let repository = committed();
    write(&repository, "a.txt", "a, changed\n");
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click(
        &driver,
        &mut state,
        &mut window,
        (Role::Button, "Ada <ada@example.com>"),
    );
    for answer in ["enter", "Ada", "ada@home.example"] {
        let Message::FeatureAsks(_) = driver.next(&mut state) else {
            panic!("the switch asks");
        };
        let answer = if answer == "enter" {
            Answer::Choice(answer.to_owned())
        } else {
            Answer::Text(answer.to_owned())
        };
        driver.intent(&mut state, Intent::Feature(FeatureIntent::Answer(Some(answer))));
    }
    until_ended(&driver, &mut state);
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::Button, "Ada <ada@home.example>"));
}

#[test]
fn identity_use_global__unsets_the_name_and_email_of_the_repository_and_the_global_identity_is_shown_after() {
    let repository = committed();
    write(&repository, "a.txt", "a, changed\n");
    let (mut state, driver) = changes_page(&repository);
    driver.intent(
        &mut state,
        Intent::Feature(FeatureIntent::Run(
            model::contract::FeatureId("identity.use-global"),
            model::contract::Target::Head,
        )),
    );
    until_ended(&driver, &mut state);
    let lines: Vec<String> = state.log.entries().map(|entry| entry.line.clone()).collect();
    assert_eq!(
        lines,
        ["git config --unset-all user.name", "git config --unset-all user.email"]
    );
    // The global identity is the one of whoever runs the tests: the line is the identity read again, no longer the
    // repository's own.
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::Button, "Ada <ada@example.com>"));
    let identity = &state.shown_read().unwrap().state.identity;
    let line = match (&identity.name, &identity.email) {
        (Some(name), Some(email)) => format!("{name} <{email}>"),
        (Some(name), None) => name.clone(),
        (None, Some(email)) => email.clone(),
        (None, None) => String::from("No identity configured"),
    };
    assert!(
        has(&tree, Role::Button, &line),
        "the line shows the identity read again"
    );
}

#[test]
fn a_file_partly_staged_opens_the_index_against_the_file_on_disk_from_open_changes_and_from_a_click_set_to_diff() {
    use views::diff::{DiffVersion, DiffVersion::Index, DiffVersion::WorkingTree};
    let repository = committed();
    write(&repository, "a.txt", "a, staged\n");
    let _ = repository.git(&["add", "a.txt"]);
    write(&repository, "a.txt", "a, staged, then changed\n");
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let sides = |intents: &[Intent]| -> Vec<(DiffVersion, DiffVersion, String)> {
        super::diff_links::opened(intents)
            .into_iter()
            .map(|target| (target.left, target.right, target.files[0].right.clone()))
            .collect()
    };
    let from_button = window.click(&state, Role::Button, "Open changes");
    assert_eq!(sides(&from_button), [(Index, WorkingTree, String::from("a.txt"))]);
    let (_, tree) = window.draw(&state, &[]);
    // The box of the file is also named by its path: the name is the last node that says it, right of the box.
    let name = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.label() == Some("a.txt") || node.value() == Some("a.txt"))
        .filter_map(|(_, node)| node.bounds())
        .max_by(|one, other| one.x0.total_cmp(&other.x0))
        .expect("the name of the file");
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a window is far smaller than an f32 counts"
    )]
    let on_name = render::Point::new(name.x0 as f32 + 4.0, f64::midpoint(name.y0, name.y1) as f32);
    let from_click = super::pointer::click_at(&mut window, &state, on_name, 1);
    assert_eq!(sides(&from_click), [], "a click opens the File page");
    state.settings.open_on_click = model::changes::OpenOnClick::Diff;
    let from_click = super::pointer::click_at(&mut window, &state, on_name, 1);
    assert_eq!(sides(&from_click), sides(&from_button));
    apply(&driver, &mut state, from_click);
    assert_eq!(state.page_shown(), Some(Page::Changes), "the diff is beside the list");
    assert!(
        state
            .diff_pages
            .pages
            .iter()
            .any(|page| page.is_in_changes && page.is_open)
    );
}

#[test]
fn the_stash_buttons_of_the_changes_page_run_the_stash_of_a_file_of_all_changes_and_of_the_staged_ones() {
    use model::contract::{FeatureId, Target};
    let repository = committed();
    write(&repository, "a.txt", "a, staged\n");
    let _ = repository.git(&["add", "a.txt"]);
    let (state, _driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let run = |feature: &'static str, target: Target| Intent::Feature(FeatureIntent::Run(FeatureId(feature), target));
    let cases = [
        (
            "Stash this file\u{2026}",
            run("stash.push-file", Target::Files(vec![String::from("a.txt")])),
        ),
        ("Stash all changes\u{2026}", run("stash.push", Target::Head)),
        (
            "Stash only the staged changes\u{2026}",
            run("stash.push-staged", Target::Head),
        ),
    ];
    for (button, asked) in cases {
        let intents = window.click(&state, Role::Button, button);
        assert!(intents.contains(&asked), "{button}: {intents:?}");
    }
}
