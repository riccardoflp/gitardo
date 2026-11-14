//! What the Diff page changes in the repository: one change staged, unstaged or reverted from the buttons of its
//! gutter, and the whole file staged or discarded from its title buttons, each run through the engine as an operation,
//! and the page drawn again with what the repository then has.

use app::store::feature::Ended;
use app::store::{Message, State};
use platform::accesskit::{Role, TreeUpdate};
use render::{Point, ThemeName};
use testkit::Repository;
use views::Intent;
use views::diff::{DiffFile, DiffTarget, DiffVersion};
use views::feature::FeatureIntent;
use views::text::diff as words;

use super::diff::{DIFF, edited, head_to_working_tree, open, repository_with_three_changes, shown, text};
use super::showing;
use super::support::{OperationsDriver, Window};

/// The middle of the buttons of a name, from the top of the window down.
fn buttons(tree: &TreeUpdate, name: &str) -> Vec<Point> {
    let mut found: Vec<Point> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(name))
        .filter_map(|(_, node)| node.bounds())
        .map(|bounds| {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "a window is far smaller than an f32 counts"
            )]
            Point::new(
                f64::midpoint(bounds.x0, bounds.x1) as f32,
                f64::midpoint(bounds.y0, bounds.y1) as f32,
            )
        })
        .collect();
    found.sort_by(|one, other| one.y.total_cmp(&other.y));
    found
}

/// Clicks the button of a name, the first from the top being 0, and gives what the click asks.
fn click_nth(window: &mut Window, state: &State, (name, nth): (&str, usize)) -> Vec<Intent> {
    let (_, tree) = window.draw(state, &[]);
    let at = buttons(&tree, name);
    let Some(point) = at.get(nth) else {
        panic!("no button {name} number {nth} among {} of them", at.len());
    };
    window.click_at(state, *point, 1)
}

/// Runs what a click asked through the operations thread, confirming the plan when it is shown first, and gives how
/// the operation ended and the lines it logged.
fn run(state: &mut State, intents: Vec<Intent>) -> (Ended, Vec<String>) {
    let driver = OperationsDriver::new();
    let asked = intents
        .iter()
        .any(|intent| matches!(intent, Intent::Feature(FeatureIntent::Run(..))));
    assert!(asked, "the click runs an operation: {intents:?}");
    for intent in intents {
        driver.intent(state, intent);
    }
    loop {
        match driver.next(state) {
            Message::FeatureConfirms(_) => driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureEnded { ended, log } => {
                return (ended, log.into_iter().map(|entry| entry.line).collect());
            }
            _ => {}
        }
    }
}

/// The page of a file between `HEAD` and the working tree, with the operations of its buttons ready.
fn page_of(repository: &Repository, path: &str) -> (State, Window) {
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree(path));
    (state, Window::new(ThemeName::Dark, DIFF))
}

/// What `git diff --cached` says of the repository.
fn staged(repository: &Repository) -> String {
    repository.git(&["diff", "--cached", "--unified=0"])
}

const APPLY: &str = "git apply --cached --unidiff-zero --whitespace=nowarn -";

#[test]
fn diff_stage_change__staging_the_second_of_three_changes_leaves_only_it_staged_and_logs_git_apply() {
    let repository = repository_with_three_changes();
    let (mut state, mut window) = page_of(&repository, "app.rs");
    let intents = click_nth(&mut window, &state, (words::STAGE_CHANGE, 1));
    let (ended, log) = run(&mut state, intents);
    assert!(matches!(ended, Ended::Ran { .. }), "{ended:?}");
    assert_eq!(log, [APPLY]);
    let cached = staged(&repository);
    assert!(cached.contains("-line 15\n+line 15 changed\n"), "{cached}");
    assert!(
        !cached.contains("line 5 changed") && !cached.contains("line 25 changed"),
        "{cached}"
    );
    assert_eq!(
        text(shown(&state)).staged,
        [false, true, false],
        "the page shows the change staged without being opened again"
    );
}

#[test]
fn diff_stage_change__with_command_preview_all_the_command_and_the_patch_are_shown_before_it_runs() {
    let repository = repository_with_three_changes();
    let (mut state, mut window) = page_of(&repository, "app.rs");
    state.settings.features.command_preview = model::contract::CommandPreview::All;
    let driver = OperationsDriver::new();
    for intent in click_nth(&mut window, &state, (words::STAGE_CHANGE, 0)) {
        driver.intent(&mut state, intent);
    }
    let Message::FeatureConfirms(plan) = driver.next(&mut state) else {
        panic!("the plan is shown first");
    };
    let shown = views::feature::plan_lines(&plan);
    assert_eq!(
        shown.iter().take(5).map(String::as_str).collect::<Vec<_>>(),
        [
            APPLY,
            "  diff --git a/app.rs b/app.rs",
            "  --- a/app.rs",
            "  +++ b/app.rs",
            "  @@ -5,1 +5,1 @@"
        ]
    );
    assert_eq!(staged(&repository), "", "nothing ran yet");
    driver.intent(&mut state, Intent::Feature(FeatureIntent::Confirm(false)));
    let _ended = driver.next(&mut state);
}

#[test]
fn diff_stage_change__a_file_edited_in_another_program_after_the_page_was_drawn_refuses_and_stages_nothing() {
    let repository = repository_with_three_changes();
    let (mut state, mut window) = page_of(&repository, "app.rs");
    let intents = click_nth(&mut window, &state, (words::STAGE_CHANGE, 1));
    std::fs::write(
        repository.path().join("app.rs"),
        format!("added on top\n{}", edited(30, &[5, 15, 25])),
    )
    .unwrap();
    let (ended, log) = run(&mut state, intents);
    assert_eq!(
        ended,
        Ended::Refused("The file changed since the diff was shown. Try again.".into(), None)
    );
    assert_eq!(log, Vec::<String>::new());
    assert_eq!(staged(&repository), "");
}

#[test]
fn diff_change_patch__with_an_earlier_change_staged_git_apply_takes_the_patch_of_a_later_one() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the app", &[("app.rs", &edited(30, &[]))]);
    let mut lines: Vec<String> = edited(30, &[]).lines().map(str::to_owned).collect();
    lines.insert(3, String::from("inserted early"));
    lines.insert(20, String::from("inserted late"));
    std::fs::write(repository.path().join("app.rs"), lines.join("\n") + "\n").unwrap();
    let (mut state, mut window) = page_of(&repository, "app.rs");
    let intents = click_nth(&mut window, &state, (words::STAGE_CHANGE, 0));
    let _ = run(&mut state, intents);
    // The first change is staged: its button is now the one that unstages it, and the late one is the only to stage.
    let intents = click_nth(&mut window, &state, (words::STAGE_CHANGE, 0));
    let (ended, _) = run(&mut state, intents);
    assert!(matches!(ended, Ended::Ran { .. }), "{ended:?}");
    assert_eq!(repository.git(&["diff"]), "", "both changes are in the index");
    assert_eq!(text(shown(&state)).staged, [true, true]);
}

#[test]
fn diff_resize_sides__pressing_a_plus_button_in_the_gutter_stages_its_change_and_moves_nothing() {
    let repository = repository_with_three_changes();
    let (mut state, mut window) = page_of(&repository, "app.rs");
    let intents = click_nth(&mut window, &state, (words::STAGE_CHANGE, 0));
    assert!(
        !intents.iter().any(|intent| matches!(intent, Intent::Diff(_))),
        "the press asks nothing of the page: {intents:?}"
    );
    let _ = run(&mut state, intents);
    assert_eq!(shown(&state).left_width, None, "the sides keep their widths");
    assert!(staged(&repository).contains("+line 5 changed\n"));
}

#[test]
fn diff_unstage_change__in_the_diff_of_the_index_against_head_unstaging_the_only_change_removes_it_from_the_page() {
    let repository = repository_with_three_changes();
    std::fs::write(repository.path().join("app.rs"), edited(30, &[15])).unwrap();
    let _ = repository.git(&["add", "app.rs"]);
    let mut state = showing(repository.path().to_path_buf());
    let target = DiffTarget::file(DiffVersion::Head, DiffVersion::Index, DiffFile::same("app.rs"));
    open(&mut state, target);
    assert_eq!(text(shown(&state)).aligned.changes.len(), 1);
    let mut window = Window::new(ThemeName::Dark, DIFF);
    let intents = click_nth(&mut window, &state, (words::UNSTAGE_CHANGE, 0));
    let (ended, log) = run(&mut state, intents);
    assert!(matches!(ended, Ended::Ran { .. }), "{ended:?}");
    assert_eq!(log, [APPLY]);
    assert_eq!(staged(&repository), "");
    assert_eq!(text(shown(&state)).aligned.changes.len(), 0);
}

#[test]
fn diff_unstage_change__unstaging_one_of_two_staged_changes_leaves_the_other_staged_and_the_working_tree_as_it_was() {
    let repository = repository_with_three_changes();
    std::fs::write(repository.path().join("app.rs"), edited(30, &[5, 25])).unwrap();
    let _ = repository.git(&["add", "app.rs"]);
    let (mut state, mut window) = page_of(&repository, "app.rs");
    assert_eq!(text(shown(&state)).staged, [true, true]);
    let intents = click_nth(&mut window, &state, (words::STAGED_UNSTAGE_CHANGE, 0));
    let _ = run(&mut state, intents);
    let cached = staged(&repository);
    assert!(
        !cached.contains("line 5 changed") && cached.contains("line 25 changed"),
        "{cached}"
    );
    assert_eq!(
        std::fs::read_to_string(repository.path().join("app.rs")).unwrap(),
        edited(30, &[5, 25])
    );
    assert_eq!(text(shown(&state)).staged, [false, true]);
}

#[test]
fn diff_stage_file__stage_file_runs_exactly_git_add_and_every_change_is_drawn_dimmed() {
    let repository = repository_with_three_changes();
    let (mut state, mut window) = page_of(&repository, "app.rs");
    let intents = window.click(&state, Role::Button, words::STAGE_FILE);
    let (_, log) = run(&mut state, intents);
    assert_eq!(log, ["git add -- app.rs"]);
    assert_eq!(text(shown(&state)).staged, [true, true, true]);
}

#[test]
fn diff_discard_file__on_an_untracked_file_it_runs_git_clean_and_the_page_closes_with_the_file() {
    let repository = repository_with_three_changes();
    std::fs::write(repository.path().join("notes.txt"), "a note\n").unwrap();
    let (mut state, mut window) = page_of(&repository, "notes.txt");
    let intents = window.click(&state, Role::Button, words::DISCARD_FILE);
    let (_, log) = run(&mut state, intents);
    assert_eq!(log, ["git clean -f -- notes.txt"]);
    assert!(!repository.path().join("notes.txt").exists());
    assert!(
        !matches!(state.page_shown(), Some(views::page::Page::Diff(_))),
        "the page closed"
    );
    assert_eq!(state.diff_pages.pages.len(), 0, "the page is forgotten");
}
