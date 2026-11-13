//! The blame beside the lines of a file in the File page, drawn from the store on repositories made for each test: the
//! column before the text, the lines of the commit of the cursor brought out, the lines typed, the commits made outside,
//! showing and hiding it, blaming the file as it was before a commit, ignoring whitespace and showing it in every file.

use std::time::Duration;

use app::store::{Effect, JobKind, Message, State, reduce};
use model::CommitId;
use model::blame::BlameLine;
use platform::accesskit::{Role, TreeUpdate};
use render::{Appearance, Theme, ThemeName};
use testkit::Repository;
use ui::editor::Command;
use views::Intent;
use views::file_view::{FilePageIntent, FileTarget, FileViewIntent};
use views::search::QuickCommand;
use views::text::blame as words;

use super::blame::{at_line, give, page, page_of_file, quick, settings_read, three_commits};
use super::file_view::{open, shown, step};
use super::support::{self, OperationsDriver, Window, has, says_in};

/// What the column says beside the first line of each run, in the default format, before the padding of the author.
const SAID: &str = "2025-01-01 Test Author";

/// What the column of the blame says, from the top, with where each is.
fn column(tree: &TreeUpdate) -> Vec<(String, ui::Rect)> {
    let mut said: Vec<(String, ui::Rect)> = tree
        .nodes
        .iter()
        .filter_map(|(_, node)| {
            let value = node.value()?;
            let bounds = node.bounds()?;
            #[allow(
                clippy::cast_possible_truncation,
                reason = "a window is far smaller than an f32 counts"
            )]
            let rect = ui::Rect::new(
                bounds.x0 as f32,
                bounds.y0 as f32,
                (bounds.x1 - bounds.x0) as f32,
                (bounds.y1 - bounds.y0) as f32,
            );
            (value.starts_with(SAID) || value == model::blame::NOT_COMMITTED).then(|| (value.to_owned(), rect))
        })
        .collect();
    said.sort_by(|first, second| first.1.y.total_cmp(&second.1.y));
    said
}

/// Clicks a button of the title of the File page, and waits for the operation it runs.
fn click(window: &mut Window, driver: &OperationsDriver, state: &mut State, name: &str) {
    let intents = window.click(state, Role::Button, name);
    give(driver, state, intents);
}

/// Who last changed each line of the page shown, as the store keeps it while the file shows its blame.
fn blamed(state: &State) -> Vec<Option<CommitId>> {
    let id = shown(state).id;
    let lines = state
        .blames
        .pages
        .get(&id)
        .and_then(|page| page.lines.clone())
        .unwrap_or_default();
    lines
        .into_iter()
        .map(|line| match line {
            Some(BlameLine::Committed(commit)) => Some(commit.id.clone()),
            _ => None,
        })
        .collect()
}

/// The color of the pixel of the window at a point.
fn pixel(window: &Window, (x, y): (f32, f32)) -> Option<render::Color> {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a window is far smaller than an i32 counts"
    )]
    window.frame.pixel(x.round() as i32, y.round() as i32)
}

#[test]
fn blame_file__a_file_of_three_commits_says_the_date_and_author_beside_each_run_and_its_text_starts_in_one_column() {
    let (repository, _commits) = three_commits();
    let (mut state, mut window, driver) = page(&repository);
    click(&mut window, &driver, &mut state, words::SHOW_FILE_BLAME);
    let said = column(&step(&mut window, &mut state, &[]));
    assert_eq!(
        said.len(),
        4,
        "lines 1, 2, 3 and 4 start runs, line 5 says nothing: {said:?}"
    );
    let left = said.first().unwrap().1.x;
    assert!(said.iter().all(|(_, rect)| (rect.x - left).abs() < 0.5), "{said:?}");
    let page = shown(&state).id;
    let gutter = state
        .blames
        .pages
        .get(&page)
        .and_then(|blame| blame.lines.as_ref())
        .map(Vec::len);
    assert_eq!(gutter, Some(5));
}

#[test]
fn blame_file__the_lines_of_the_commit_of_the_cursor_are_brought_out_with_one_mark_per_run() {
    let mut repository = Repository::new();
    let lines: Vec<String> = (1..=60).map(|line| format!("line {line}\n")).collect();
    let first = repository.commit("Write sixty lines", &[("src/app.rs", &lines.concat())]);
    let changed: Vec<String> = (1..=60)
        .map(|line| {
            if (20..=30).contains(&line) {
                format!("LINE {line}\n")
            } else {
                format!("line {line}\n")
            }
        })
        .collect();
    let _ = repository.commit("Shout lines 20 to 30", &[("src/app.rs", &changed.concat())]);
    let (mut state, mut window, driver) = page(&repository);
    click(&mut window, &driver, &mut state, words::SHOW_FILE_BLAME);
    at_line(&mut state, 5);
    let said = column(&step(&mut window, &mut state, &[]));
    let ((_, of_first), (_, of_second)) = (said.first().unwrap(), said.get(1).unwrap());
    let background = Theme::new(ThemeName::Dark, Appearance::Dark)
        .colors
        .list_inactive_selection;
    let beside = |rect: &ui::Rect| (rect.x - 2.0, rect.y + rect.height / 2.0);
    assert_eq!(
        pixel(&window, beside(of_first)),
        Some(background),
        "line 1 is of the commit of the cursor"
    );
    assert_ne!(pixel(&window, beside(of_second)), Some(background), "line 20 is not");
    let page = shown(&state).id;
    let lines = state
        .blames
        .pages
        .get(&page)
        .and_then(|blame| blame.lines.clone())
        .unwrap();
    let first = CommitId::parse(&first).unwrap();
    assert_eq!(views::blame::runs_of(&lines, &first), [0..19, 30..60]);
}

#[test]
fn blame_file__typing_says_not_committed_yet_at_once_and_the_blame_is_read_again_400_ms_after_with_the_text() {
    let (repository, _commits) = three_commits();
    let (mut state, mut window, driver) = page(&repository);
    click(&mut window, &driver, &mut state, words::SHOW_FILE_BLAME);
    at_line(&mut state, 2);
    let id = shown(&state).id;
    let typed = Intent::FileView(FileViewIntent::Page(
        id,
        FilePageIntent::Edit(vec![Command::Type(String::from("x"))]),
    ));
    let effects = reduce(&mut state, Message::Intent(typed));
    let lines = state
        .blames
        .pages
        .get(&id)
        .and_then(|blame| blame.lines.clone())
        .unwrap();
    assert_eq!(lines.get(1), Some(&Some(BlameLine::NotCommitted)));
    let asked = effects.iter().find_map(|effect| match effect {
        Effect::Start(job) => match &job.kind {
            JobKind::ReadBlame { job, .. } if job.line.is_none() => Some(job.clone()),
            _ => None,
        },
        _ => None,
    });
    let asked = asked.expect("the blame of the file is read again");
    assert_eq!(asked.delay, Duration::from_millis(400));
    assert_eq!(
        asked.unsaved.as_deref(),
        Some(b"one\nxTWO\nthree\nfour\nfive\n".as_slice())
    );
}

/// The colors of the pixels of the window along a band, at a height, from one x to another.
fn colors_along(window: &Window, y: f32, (from, to): (f32, f32)) -> Vec<render::Color> {
    let mut colors: Vec<render::Color> = Vec::new();
    let mut x = from;
    while x < to {
        if let Some(color) = pixel(window, (x, y))
            && !colors.contains(&color)
        {
            colors.push(color);
        }
        x += 1.0;
    }
    colors
}

#[test]
fn blame_file__with_word_wrap_on_a_long_line_wraps_with_its_second_part_in_the_text_column_not_under_the_blame() {
    let mut repository = Repository::new();
    let _ = repository.commit("Write the lines", &[("src/app.rs", "one\ntwo\nthree\n")]);
    let long = "the quick brown fox jumps over the lazy dog ".repeat(4);
    let text = format!("one\n{}\nthree\n", long.trim_end());
    let _ = repository.commit("Make the second line long", &[("src/app.rs", &text)]);
    let (mut state, mut window, driver) = page(&repository);
    state.settings.word_wrap = ui::LineWrap::Words;
    click(&mut window, &driver, &mut state, words::SHOW_FILE_BLAME);
    let said = column(&step(&mut window, &mut state, &[]));
    let [(_, first), (_, second), (_, third)] = said.as_slice() else {
        panic!("each of the three lines starts a run: {said:?}");
    };
    let line_height = second.y - first.y;
    let rows = (third.y - second.y) / line_height;
    assert!(
        rows >= 2.0 && (rows - rows.round()).abs() < 0.01,
        "the long line takes whole rows, two at least, and the blame of the next line comes after them: {rows}"
    );
    // The middle of the second row of the long line: nothing is drawn under the blame, and the text goes on after it.
    let y = second.y + line_height * 1.5;
    let under_the_blame = colors_along(&window, y, (second.x, second.right() - 1.0));
    assert_eq!(under_the_blame.len(), 1, "{under_the_blame:?}");
    let in_the_text = colors_along(&window, y, (second.right() + 1.0, second.right() + 200.0));
    assert!(
        in_the_text.len() > 1,
        "the second part of the line is drawn in the text column"
    );
}

#[test]
fn blame_file__a_commit_made_in_a_terminal_updates_the_column_without_closing_the_page() {
    let (mut repository, _commits) = three_commits();
    let (mut state, mut window, driver) = page(&repository);
    click(&mut window, &driver, &mut state, words::SHOW_FILE_BLAME);
    let id = shown(&state).id;
    let made = repository.commit(
        "Change the first line",
        &[("src/app.rs", "ONE\nTWO\nthree\nfour\nfive\n")],
    );
    let _effects = support::run(&mut state, Message::Intent(Intent::Refresh));
    assert_eq!(shown(&state).id, id, "the page stays");
    assert_eq!(blamed(&state).first(), Some(&Some(CommitId::parse(&made).unwrap())));
}

#[test]
fn blame_file__hidden_and_closed_the_file_opened_again_shows_no_blame() {
    let (repository, _commits) = three_commits();
    let (mut state, mut window, driver) = page(&repository);
    click(&mut window, &driver, &mut state, words::SHOW_FILE_BLAME);
    click(&mut window, &driver, &mut state, words::HIDE_FILE_BLAME);
    let intents = window.click(&state, Role::Button, "Close the page");
    give(&driver, &mut state, intents);
    open(&mut state, FileTarget::working_tree("src/app.rs"));
    assert_eq!(column(&step(&mut window, &mut state, &[])), []);
}

#[test]
fn blame_toggle_file__on_a_file_of_a_repository_it_shows_the_blame_and_again_hides_it() {
    let (repository, _commits) = three_commits();
    let (mut state, mut window, driver) = page(&repository);
    click(&mut window, &driver, &mut state, words::SHOW_FILE_BLAME);
    let tree = step(&mut window, &mut state, &[]);
    assert_eq!(column(&tree).len(), 4);
    assert!(has(&tree, Role::Button, words::HIDE_FILE_BLAME));
    click(&mut window, &driver, &mut state, words::HIDE_FILE_BLAME);
    let tree = step(&mut window, &mut state, &[]);
    assert_eq!(column(&tree), []);
    assert!(has(&tree, Role::Button, words::SHOW_FILE_BLAME));
}

#[test]
fn blame_toggle_file__on_a_file_never_committed_it_says_so_and_shows_nothing() {
    let (repository, _commits) = three_commits();
    std::fs::write(repository.path().join("src/new.rs"), "fn new() {}\n").unwrap();
    let (mut state, mut window, driver) = page_of_file(&repository, "src/new.rs");
    click(&mut window, &driver, &mut state, words::SHOW_FILE_BLAME);
    let tree = step(&mut window, &mut state, &[]);
    assert!(says_in(&tree, "new.rs is not in any commit yet"));
    assert_eq!(column(&tree), []);
}

#[test]
fn blame_previous__on_a_line_of_b_it_opens_the_file_as_of_its_parent_with_its_blame_and_the_trail_naming_it() {
    let (repository, [first, ..]) = three_commits();
    let (mut state, _window, driver) = page(&repository);
    at_line(&mut state, 2);
    quick(&driver, &mut state, QuickCommand::BlamePrevious);
    let opened = shown(&state);
    assert_eq!(opened.revision, Some(first.clone()));
    assert_eq!(opened.title, format!("app.rs ({})", first.short()));
    assert_eq!(blamed(&state), [Some(first.clone()), Some(first.clone()), Some(first)]);
}

#[test]
fn blame_previous__on_a_file_renamed_by_b_it_opens_the_file_under_its_old_name_as_of_a() {
    let mut repository = Repository::new();
    let first = repository.commit("Write the old file", &[("src/old.rs", "one\ntwo\nthree\n")]);
    let _ = repository.git(&["mv", "src/old.rs", "src/new.rs"]);
    let _ = repository.commit(
        "Rename the file and change a line",
        &[("src/new.rs", "one\nTWO\nthree\n")],
    );
    let (mut state, _window, driver) = page_of_file(&repository, "src/new.rs");
    at_line(&mut state, 2);
    quick(&driver, &mut state, QuickCommand::BlamePrevious);
    let opened = shown(&state);
    assert_eq!(opened.path, "src/old.rs");
    assert_eq!(opened.revision, Some(CommitId::parse(&first).unwrap()));
}

#[test]
fn blame_previous__on_a_line_of_the_commit_that_added_the_file_it_says_so_and_opens_nothing() {
    let (repository, [first, ..]) = three_commits();
    let (mut state, mut window, driver) = page(&repository);
    let id = shown(&state).id;
    at_line(&mut state, 1);
    quick(&driver, &mut state, QuickCommand::BlamePrevious);
    assert_eq!(shown(&state).id, id);
    let said = format!("{} is the commit that added this file", first.short());
    assert!(says_in(&step(&mut window, &mut state, &[]), &said));
}

/// `src/app.rs` with its second line written by a first commit and indented by a second.
fn indented() -> (Repository, CommitId) {
    let mut repository = Repository::new();
    let first = repository.commit("Write main", &[("src/app.rs", "fn main() {\nlet x = 1;\n}\n")]);
    let _ = repository.commit("Indent main", &[("src/app.rs", "fn main() {\n    let x = 1;\n}\n")]);
    (repository, CommitId::parse(&first).unwrap())
}

#[test]
fn blame_ignore_whitespace__on_a_line_reindented_by_b_and_written_by_a_is_said_to_be_a_s() {
    let (repository, first) = indented();
    let (mut state, mut window, driver) = page(&repository);
    click(&mut window, &driver, &mut state, words::SHOW_FILE_BLAME);
    assert_ne!(blamed(&state).get(1), Some(&Some(first.clone())));
    click(&mut window, &driver, &mut state, words::IGNORE_WHITESPACE);
    settings_read(&mut state, &repository);
    assert_eq!(blamed(&state).get(1), Some(&Some(first)));
}

#[test]
fn blame_ignore_whitespace__the_funnel_is_filled_while_the_setting_is_on() {
    let (repository, _first) = indented();
    let (mut state, mut window, driver) = page(&repository);
    click(&mut window, &driver, &mut state, words::SHOW_FILE_BLAME);
    click(&mut window, &driver, &mut state, words::IGNORE_WHITESPACE);
    settings_read(&mut state, &repository);
    let tree = step(&mut window, &mut state, &[]);
    assert!(has(&tree, Role::Button, words::STOP_IGNORING_WHITESPACE));
    assert!(state.settings.features.blame.ignores_whitespace);
}

/// A repository with two files of a commit each, and the window with the blame in every file switched on.
fn every_file() -> (Repository, State, Window, OperationsDriver) {
    let (mut repository, _commits) = three_commits();
    let _ = repository.commit("Add the library", &[("src/lib.rs", "pub mod app;\n")]);
    let (mut state, window, driver) = page(&repository);
    quick(&driver, &mut state, QuickCommand::ToggleEveryFile);
    settings_read(&mut state, &repository);
    (repository, state, window, driver)
}

#[test]
fn blame_every_file__with_it_on_opening_any_committed_file_shows_its_blame_without_asking() {
    let (_repository, mut state, mut window, _driver) = every_file();
    open(&mut state, FileTarget::working_tree("src/lib.rs"));
    let tree = step(&mut window, &mut state, &[]);
    assert_eq!(column(&tree).len(), 1);
}

#[test]
fn blame_every_file__a_file_hidden_while_it_is_on_stays_without_it_until_its_page_is_closed() {
    let (_repository, mut state, mut window, driver) = every_file();
    let _ = step(&mut window, &mut state, &[]);
    click(&mut window, &driver, &mut state, words::HIDE_FILE_BLAME);
    open(&mut state, FileTarget::working_tree("src/lib.rs"));
    open(&mut state, FileTarget::working_tree("src/app.rs"));
    assert_eq!(column(&step(&mut window, &mut state, &[])), []);
    let intents = window.click(&state, Role::Button, "Close the page");
    give(&driver, &mut state, intents);
    open(&mut state, FileTarget::working_tree("src/app.rs"));
    assert_eq!(column(&step(&mut window, &mut state, &[])).len(), 4);
}

#[test]
fn blame_every_file__turned_off_the_file_it_was_asked_for_keeps_it_and_the_others_lose_it() {
    let (mut repository, _commits) = three_commits();
    let _ = repository.commit("Add the library", &[("src/lib.rs", "pub mod app;\n")]);
    let (mut state, mut window, driver) = page(&repository);
    click(&mut window, &driver, &mut state, words::SHOW_FILE_BLAME);
    quick(&driver, &mut state, QuickCommand::ToggleEveryFile);
    settings_read(&mut state, &repository);
    open(&mut state, FileTarget::working_tree("src/lib.rs"));
    assert_eq!(column(&step(&mut window, &mut state, &[])).len(), 1);
    quick(&driver, &mut state, QuickCommand::ToggleEveryFile);
    settings_read(&mut state, &repository);
    assert_eq!(column(&step(&mut window, &mut state, &[])), []);
    open(&mut state, FileTarget::working_tree("src/app.rs"));
    assert_eq!(column(&step(&mut window, &mut state, &[])).len(), 4);
}
