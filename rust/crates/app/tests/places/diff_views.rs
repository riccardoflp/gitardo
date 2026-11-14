//! The parts of the Diff page drawn from the store: the unchanged rows folded, the line between the two sides dragged,
//! the changes already staged, the pictures of the page in each theme, and how long a very large diff takes.

use std::time::{Duration, Instant};

use app::persisted::Persisted;
use app::settings::Settings;
use app::store::{Message, State};
use platform::accesskit::{Role, TreeUpdate};
use render::{Appearance, Point, Rect, Theme, ThemeName};
use testkit::Repository;
use ui::Modifiers;
use views::Intent;
use views::diff::{DiffIntent, PageLine};

use super::diff::{DIFF, edited, f7, head_to_working_tree, open, press, shown, text};
use super::inspector::ask;
use super::pointer::{apply, bounds, click_at, drag};
use super::showing;
use super::support::{self, Window, find};

/// A repository with `app.rs` of 100 lines committed, and its line 50 changed in the working tree.
fn repository_with_one_change() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the app", &[("app.rs", &edited(100, &[]))]);
    std::fs::write(repository.path().join("app.rs"), edited(100, &[50])).unwrap();
    repository
}

/// The folds of the page shown, by how many rows each hides.
fn folds(state: &State) -> Vec<usize> {
    shown(state)
        .lines
        .iter()
        .filter_map(|line| match line {
            PageLine::Folded(_, rows) => Some(rows.len()),
            _ => None,
        })
        .collect()
}

#[test]
fn diff_collapse_unchanged__with_the_setting_off_a_new_page_shows_every_line_and_the_fold_button_folds_them() {
    let repository = repository_with_one_change();
    let settings = Settings {
        collapse_unchanged: false,
        ..Settings::default()
    };
    let mut state = State::new(
        Persisted {
            has_started: true,
            ..Persisted::default()
        },
        settings,
        None,
    );
    let _ = support::run(
        &mut state,
        Message::Started {
            folders: vec![repository.path().to_path_buf()],
        },
    );
    open(&mut state, head_to_working_tree("app.rs"));
    assert_eq!(folds(&state), []);
    assert_eq!(
        shown(&state).lines.len(),
        1 + 100,
        "the header, 99 unchanged rows and the row of the change"
    );
    let mut window = Window::new(ThemeName::Dark, DIFF);
    let intents = window.click(&state, Role::Button, "Collapse unchanged regions");
    apply(&mut state, intents);
    assert_eq!(folds(&state), [46, 47]);
}

#[test]
fn diff_collapse_unchanged__clicking_a_bar_shows_its_lines_on_both_sides() {
    let repository = repository_with_one_change();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("app.rs"));
    assert_eq!(folds(&state), [46, 47]);
    let mut window = Window::new(ThemeName::Dark, DIFF);
    let intents = window.click(&state, Role::Button, "\u{22ef} 46 unchanged lines");
    apply(&mut state, intents);
    assert_eq!(folds(&state), [47]);
    let read = text(shown(&state));
    let first = &read.aligned.rows[0];
    let (Some(left), Some(right)) = (&first.left, &first.right) else {
        panic!("the first row has a line on both sides");
    };
    assert_eq!(
        (read.left_line(left.line), read.right_line(right.line)),
        ("line 1", "line 1")
    );
    assert!(shown(&state).lines.contains(&PageLine::Row(0, 0)));
}

/// The sizes of the window of the tests.
fn sizes() -> render::Sizes {
    Theme::new(ThemeName::Dark, Appearance::Dark).sizes
}

/// The Diff page drawn, and where the line between its two sides is, low in its rows.
fn line_between(window: &mut Window, state: &State) -> (Rect, Point, TreeUpdate) {
    let (_, tree) = window.draw(state, &[]);
    let page = bounds(&tree, Role::Main, "Diff");
    let sizes = sizes();
    let room = page.width - sizes.scrollbar;
    let left = shown(state)
        .left_width
        .map_or((room - sizes.border - sizes.control_height) * 0.5, pixels);
    (
        page,
        Point::new(page.x + left + sizes.border * 0.5, page.bottom() - 40.0),
        tree,
    )
}

/// A width in whole pixels, as a page keeps it.
#[allow(
    clippy::cast_precision_loss,
    reason = "a width is far smaller than an f32 counts exactly"
)]
fn pixels(width: u32) -> f32 {
    width as f32
}

/// Where the name over the right side starts.
fn right_name_x(tree: &TreeUpdate) -> f32 {
    super::pointer::said_at(tree, "Working tree", Rect::new(0.0, 0.0, 10_000.0, 10_000.0)).x
}

#[test]
fn diff_resize_sides__dragging_the_gutter_100_pixels_right_widens_the_left_side_and_narrows_the_right_by_100() {
    let repository = repository_with_one_change();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("app.rs"));
    let mut window = Window::new(ThemeName::Dark, DIFF);
    let (page, line, tree) = line_between(&mut window, &state);
    let room = page.width - sizes().scrollbar;
    let before = (room - sizes().border - sizes().control_height) * 0.5;
    let name_before = right_name_x(&tree);
    let intents = drag(&mut window, &state, line, Point::new(line.x + 100.0, line.y));
    apply(&mut state, intents);
    let width = pixels(shown(&state).left_width.unwrap());
    assert!((width - before - 100.0).abs() <= 1.0, "{width} after {before}");
    let (_, tree) = window.draw(&state, &[]);
    assert!((right_name_x(&tree) - name_before - 100.0).abs() <= 1.0);
    assert_eq!(
        state.diff_pages.last_width,
        shown(&state).left_width,
        "the next page opens with it"
    );
}

#[test]
fn diff_resize_sides__dragging_past_the_edge_stops_with_80_pixels_left_to_the_narrower_side() {
    let repository = repository_with_one_change();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("app.rs"));
    let mut window = Window::new(ThemeName::Dark, DIFF);
    let (page, line, _) = line_between(&mut window, &state);
    let intents = drag(&mut window, &state, line, Point::new(page.right() + 200.0, line.y));
    apply(&mut state, intents);
    let room = page.width - sizes().scrollbar;
    let right = room - pixels(shown(&state).left_width.unwrap()) - sizes().border - sizes().control_height;
    assert!((right - 80.0).abs() <= 1.0, "the right side is {right} wide");
    let (_, line, _) = line_between(&mut window, &state);
    let intents = drag(&mut window, &state, line, Point::new(page.x - 200.0, line.y));
    apply(&mut state, intents);
    assert_eq!(shown(&state).left_width, Some(80));
}

#[test]
fn diff_resize_sides__a_double_click_after_dragging_gives_two_equal_sides() {
    let repository = repository_with_one_change();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("app.rs"));
    let mut window = Window::new(ThemeName::Dark, DIFF);
    let (_, line, tree) = line_between(&mut window, &state);
    let name_at_first = right_name_x(&tree);
    let intents = drag(&mut window, &state, line, Point::new(line.x - 150.0, line.y));
    apply(&mut state, intents);
    assert!(shown(&state).left_width.is_some());
    let (_, line, _) = line_between(&mut window, &state);
    window.now += Duration::from_secs(1);
    let intents = click_at(&mut window, &state, line, 2);
    apply(&mut state, intents);
    assert_eq!(shown(&state).left_width, None);
    let (_, tree) = window.draw(&state, &[]);
    assert!((right_name_x(&tree) - name_at_first).abs() <= 0.5);
}

#[test]
fn diff_staged_changes__a_change_staged_of_two_is_drawn_dimmed_and_the_other_is_not() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the app", &[("app.rs", &edited(30, &[]))]);
    std::fs::write(repository.path().join("app.rs"), edited(30, &[5])).unwrap();
    let _ = repository.git(&["add", "app.rs"]);
    std::fs::write(repository.path().join("app.rs"), edited(30, &[5, 25])).unwrap();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("app.rs"));
    assert_eq!(text(shown(&state)).staged, [true, false]);
}

/// A repository whose working tree changes a file of 60 lines in three places: a word, two lines removed, a line added.
fn repository_for_pictures() -> Repository {
    let mut repository = Repository::new();
    let mut lines: Vec<String> = (1..=60)
        .map(|line| format!("let value_{line} = compute({line});"))
        .collect();
    let _ = repository.commit("Add the app", &[("src/app.rs", &(lines.join("\n") + "\n"))]);
    *lines.get_mut(4).unwrap() = String::from("let value_5 = measure(5);");
    let _ = lines.drain(19..21);
    lines.insert(40, String::from("let added = compute(0);"));
    std::fs::write(repository.path().join("src/app.rs"), lines.join("\n") + "\n").unwrap();
    repository
}

#[test]
fn the_diff_page_is_drawn_in_each_theme() {
    let repository = repository_for_pictures();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("src/app.rs"));
    for (theme, name) in [
        (ThemeName::Dark, "diff-page-dark"),
        (ThemeName::Light, "diff-page-light"),
    ] {
        let mut window = Window::new(theme, DIFF);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(name);
    }
    let mut window = Window::new(ThemeName::Dark, DIFF);
    let _ = press(&mut window, &mut state, &[f7(Modifiers::NONE), f7(Modifiers::NONE)]);
    window.now += Duration::from_secs(2);
    let _ = window.draw(&state, &[]);
    window.assert_snapshot("diff-page-current-dark");
}

/// Opens a very large diff, says **Continue**, and gives how long the read took.
#[allow(clippy::print_stdout, reason = "the measure is printed")]
fn measure_large(state: &mut State) -> Duration {
    let started = Instant::now();
    open(state, head_to_working_tree("log.txt"));
    ask(state, Intent::Diff(DiffIntent::AnswerLarge(true)));
    let took = started.elapsed();
    println!("two files of 60,000 lines, 600 changes: {took:?}");
    took
}

#[test]
#[ignore = "a measure of speed, run in release"]
fn a_diff_of_two_files_of_60000_lines_with_600_changes_is_shown_within_its_time() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the log", &[("log.txt", &edited(60_000, &[]))]);
    let changed: Vec<usize> = (1..=600).map(|change| change * 100).collect();
    std::fs::write(repository.path().join("log.txt"), edited(60_000, &changed)).unwrap();
    let mut state = showing(repository.path().to_path_buf());
    let took = measure_large(&mut state);
    assert_eq!(text(shown(&state)).aligned.changes.len(), 600);
    assert!(took < Duration::from_secs(2));
    let mut window = Window::new(ThemeName::Dark, DIFF);
    let started = Instant::now();
    let (_, tree) = window.draw(&state, &[]);
    assert!(started.elapsed() < Duration::from_millis(100));
    let _ = find(&tree, Role::Button, "Next change (F7)");
}
