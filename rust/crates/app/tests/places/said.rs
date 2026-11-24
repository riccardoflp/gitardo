//! What was said, in the status bar (`activity.said`): what an operation said when it ended, its links, how long it
//! stays, and that no bar is drawn under the header any more.

use std::time::Duration;

use app::store::feature::{Ended, Said};
use app::store::{Message, State, ViewData, reduce};
use engine::{LogEntry, LogKind};
use model::contract::FeatureId;
use platform::accesskit::{Role, TreeUpdate};
use render::{Rect, ThemeName};
use ui::{Appearance, TextKind, Theme};
use views::Intent;
use views::feature::FeatureIntent;
use views::panel::{PanelIntent, PanelTab};

use super::pointer::{apply, bounds, click_at, middle, moved};
use super::support::{self, Window, find};
use super::tabs::{WIDE, three_clones};

/// The height of the status bar.
const STATUS: f32 = 26.0;
/// What a pull that succeeded says.
const PULLED: &str = "Pulled 2 commits into main.";

/// Something said, with nothing beside it.
fn said(text: &str) -> Said {
    Said {
        text: text.to_owned(),
        is_error: false,
        can_undo: false,
        offers_stashes: false,
        found: None,
        offer: None,
        stopped: None,
        runs: Vec::new(),
        link: None,
        shown_at: None,
        log_at: None,
    }
}

/// The three clones with what a pull that succeeded said.
fn pulled() -> (impl Sized, State) {
    let (kept, clones, mut state) = three_clones();
    state.operations.said = Some(Said {
        can_undo: true,
        ..said(PULLED)
    });
    ((kept, clones), state)
}

/// The status bar of a window of the size of `WIDE`.
fn status_bar() -> Rect {
    #[allow(clippy::cast_precision_loss, reason = "the window is a few hundred pixels")]
    let (width, height) = (WIDE.0 as f32, WIDE.1 as f32);
    Rect::new(0.0, height - STATUS, width, STATUS)
}

/// Whether a rectangle is in the status bar.
fn in_status_bar(rect: Rect) -> bool {
    rect.y >= status_bar().y - 0.5 && rect.bottom() <= status_bar().bottom() + 0.5
}

/// The words said, as the button that opens the Command log at them.
fn words(tree: &TreeUpdate, text: &str) -> Rect {
    bounds(tree, Role::Button, text)
}

/// The timeline drawn.
fn timeline(tree: &TreeUpdate) -> Rect {
    bounds(tree, Role::Main, "Timeline")
}

#[test]
fn activity_said__after_a_pull_that_succeeded_the_status_bar_says_it_with_undo_and_is_as_high_as_before() {
    let (_kept, mut state) = pulled();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(in_status_bar(words(&tree, PULLED)), "{:?}", words(&tree, PULLED));
    let undo = bounds(&tree, Role::Button, "Undo");
    assert!(in_status_bar(undo), "{undo:?}");
    let with_said = timeline(&tree);
    let intents = window.click(&state, Role::Button, "Undo");
    state.operations.said = None;
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(timeline(&tree), with_said, "the status bar is as high as it was");
    assert!(
        intents
            .iter()
            .any(|intent| matches!(intent, Intent::Feature(FeatureIntent::Run(FeatureId("undo.last"), _)))),
        "{intents:?}"
    );
}

#[test]
fn activity_said__ten_seconds_later_without_the_pointer_on_it_it_is_gone() {
    let (_kept, mut state) = pulled();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (intents, _) = window.draw(&state, &[]);
    apply(&mut state, intents);
    assert!(
        state
            .operations
            .said
            .as_ref()
            .is_some_and(|said| said.shown_at.is_some())
    );
    window.now += Duration::from_secs(9);
    let (intents, _) = window.draw(&state, &[]);
    apply(&mut state, intents);
    assert!(state.operations.said.is_some(), "nine seconds later it is still said");
    window.now += Duration::from_secs(1);
    let (intents, _) = window.draw(&state, &[]);
    apply(&mut state, intents);
    assert!(state.operations.said.is_none(), "ten seconds later it is gone");
    let (_, tree) = window.draw(&state, &[]);
    assert!(!support::says(&tree, PULLED));
}

#[test]
fn activity_said__with_the_pointer_on_it_it_stays_and_goes_ten_seconds_after_the_pointer_left() {
    let (_kept, mut state) = pulled();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (intents, tree) = window.draw(&state, &[]);
    apply(&mut state, intents);
    let on = middle(words(&tree, PULLED));
    for _ in 0..12 {
        window.now += Duration::from_secs(1);
        let (intents, _) = window.draw(&state, &[moved(on)]);
        apply(&mut state, intents);
    }
    assert!(state.operations.said.is_some(), "it stays while the pointer is on it");
    let away = middle(timeline(&window.draw(&state, &[]).1));
    let (intents, _) = window.draw(&state, &[moved(away)]);
    apply(&mut state, intents);
    window.now += Duration::from_secs(10);
    let (intents, _) = window.draw(&state, &[]);
    apply(&mut state, intents);
    assert!(state.operations.said.is_none());
}

#[test]
fn activity_said__after_a_push_of_a_branch_to_github_it_offers_create_pull_request_which_opens_the_page() {
    let (_kept, _clones, mut state) = three_clones();
    let page = "https://github.com/octo/api/pull/new/feature/login";
    state.operations.said = Some(Said {
        link: Some(("Create pull request".to_owned(), page.to_owned())),
        ..said("Pushed feature/login to origin.")
    });
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(in_status_bar(bounds(&tree, Role::Button, "Create pull request")));
    let intents = window.click(&state, Role::Button, "Create pull request");
    assert!(intents.contains(&Intent::OpenUrl(page.to_owned())), "{intents:?}");
}

#[test]
fn activity_said__a_failed_push_says_why_in_the_color_of_errors_and_stays_until_dismiss() {
    let (_kept, _clones, mut state) = three_clones();
    let why = "Could not push main: the remote has commits you do not have.";
    state.operations.said = Some(Said {
        is_error: true,
        ..said(why)
    });
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (intents, _) = window.draw(&state, &[]);
    apply(&mut state, intents);
    let error = Theme::new(ThemeName::Dark, Appearance::Dark).colors.error;
    let pixels = window.frame.to_rgba();
    let width = window.frame.width() as usize;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "inside the window"
    )]
    let (top, bottom) = (status_bar().y as usize, status_bar().bottom() as usize);
    let near = |chunk: &[u8]| {
        chunk
            .iter()
            .zip([error.red, error.green, error.blue])
            .map(|(a, b)| u32::from(a.abs_diff(b)))
            .sum::<u32>()
            < 24
    };
    let in_error = (top..bottom)
        .flat_map(|y| {
            pixels
                .get(y * width * 4..(y + 1) * width * 4)
                .unwrap()
                .chunks(4)
                .collect::<Vec<_>>()
        })
        .filter(|chunk| near(chunk))
        .count();
    assert!(
        in_error > 20,
        "the status bar has {in_error} pixels in the color of errors"
    );
    window.now += Duration::from_secs(60);
    let (intents, _) = window.draw(&state, &[]);
    apply(&mut state, intents);
    assert!(state.operations.said.is_some(), "an error stays");
    let intents = window.click(&state, Role::Button, "Dismiss");
    assert_eq!(intents, [Intent::Feature(FeatureIntent::Dismiss)]);
    apply(&mut state, intents);
    assert!(state.operations.said.is_none());
}

#[test]
fn activity_said__words_longer_than_the_room_are_cut_and_the_tooltip_has_them_whole() {
    let (_kept, _clones, mut state) = three_clones();
    let long = "Rebased feature/login onto main: ".repeat(12) + "and the last of them too.";
    state.operations.said = Some(said(&long));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let place = words(&tree, &long);
    let (whole, _) = window.frame(&[], |ui| ui.measure(&long, ui.text_style(TextKind::Detail)));
    assert!(place.width < whole, "the words are cut: {place:?} for {whole}");
    let before = tooltip_pixels(&window);
    let _ = window.draw(&state, &[moved(middle(place))]);
    window.now += Duration::from_secs(1);
    let _ = window.draw(&state, &[]);
    assert!(
        tooltip_pixels(&window) > before + 200,
        "the tooltip with the whole words is drawn"
    );
}

/// How many pixels are of the background of tooltips.
fn tooltip_pixels(window: &Window) -> usize {
    let color = Theme::new(ThemeName::Dark, Appearance::Dark).colors.tooltip_background;
    window
        .frame
        .to_rgba()
        .chunks(4)
        .filter(|pixel| {
            pixel.first() == Some(&color.red) && pixel.get(1) == Some(&color.green) && pixel.get(2) == Some(&color.blue)
        })
        .count()
}

/// A line of the Command log, as a step of an operation or what git said of the step that failed.
fn logged(line: &str, kind: LogKind) -> LogEntry {
    LogEntry {
        time: model::Timestamp(1_700_000_000),
        kind,
        line: line.to_owned(),
    }
}

/// How many pixels of a rectangle of a window are of the color that marks a line of the Command log.
fn marked_pixels(window: &Window, within: Rect) -> usize {
    let color = Theme::new(ThemeName::Dark, Appearance::Dark)
        .colors
        .list_inactive_selection;
    let pixels = window.frame.to_rgba();
    let width = WIDE.0 as usize;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "inside the window"
    )]
    let (left, top, right, bottom) = (
        within.x as usize,
        within.y as usize,
        within.right() as usize,
        within.bottom() as usize,
    );
    (top..bottom)
        .flat_map(|y| (left..right).map(move |x| (y * width + x) * 4))
        .filter(|at| pixels.get(*at..*at + 3) == Some(&[color.red, color.green, color.blue][..]))
        .count()
}

#[test]
fn activity_said__a_click_on_the_words_opens_the_command_log_at_that_command() {
    let (_kept, _clones, mut state) = three_clones();
    let before: Vec<LogEntry> = (0..300)
        .map(|number| logged(&format!("git step {number}"), LogKind::Step))
        .collect();
    state.log.append(before);
    let log = vec![
        logged("git fetch origin", LogKind::Step),
        logged("git push origin main", LogKind::Step),
        logged("! [rejected] main -> main (non-fast-forward)", LogKind::Failure),
    ];
    let failed = Ended::Failed {
        title: String::from("Push main"),
        said: String::from("the remote has commits you do not have"),
        plan: String::from("Push main"),
        on_failure: Vec::new(),
    };
    let _effects = reduce(&mut state, Message::FeatureEnded { ended: failed, log });
    let after: Vec<LogEntry> = (0..50)
        .map(|number| logged(&format!("git later {number}"), LogKind::Step))
        .collect();
    state.log.append(after);
    let said = state.operations.said.as_ref().unwrap();
    assert_eq!(
        said.log_at,
        Some(301),
        "the step that failed, before what git said of it"
    );
    let text = said.text.clone();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let intents = click_at(&mut window, &state, middle(words(&tree, &text)), 1);
    assert_eq!(intents, [Intent::Panel(PanelIntent::ShowLogAt(301))]);
    apply(&mut state, intents);
    assert!(app::store::panel::is_log_shown(&state));
    let data = ViewData::new(&state, Vec::new(), support::NOW);
    let shown = data
        .window_state(&state)
        .panel
        .map(|panel| panel.log.at.map(|at| at.line));
    assert_eq!(shown, Some(Some(301)));
    let _ = window.draw(&state, &[]);
    let (_, tree) = window.draw(&state, &[]);
    let list = bounds(&tree, Role::ListBox, views::text::activity::COMMAND_LOG);
    let first_line = Rect::new(list.x, list.y, list.width, 18.0);
    assert!(
        marked_pixels(&window, first_line) > 100,
        "the command that said it is the first line in sight, marked"
    );
    let _ = reduce(
        &mut state,
        Message::Intent(Intent::Panel(PanelIntent::Show(PanelTab::CommandLog))),
    );
    let (_, tree) = window.draw(&state, &[]);
    let list = bounds(&tree, Role::ListBox, views::text::activity::COMMAND_LOG);
    assert_eq!(
        marked_pixels(&window, list),
        0,
        "Show Command Log follows the last line again"
    );
}

#[test]
fn activity_said__no_bar_is_drawn_between_the_header_and_the_timeline_after_an_operation() {
    let (_kept, mut state) = pulled();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, with_said) = window.draw(&state, &[]);
    state.operations.said = None;
    let (_, without) = window.draw(&state, &[]);
    assert_eq!(
        timeline(&with_said).y,
        timeline(&without).y,
        "the timeline starts where it did"
    );
    let undo = find(&with_said, Role::Button, "Undo").bounds().unwrap();
    assert!(
        undo.y0 >= f64::from(status_bar().y) - 0.5,
        "Undo is in the status bar: {undo:?}"
    );
}

#[test]
fn activity_said__a_step_given_a_patch_that_failed_opens_the_command_log_at_the_step_not_at_its_patch() {
    let (_kept, _clones, mut state) = three_clones();
    let log = vec![
        logged("git fetch origin", LogKind::Step),
        logged("From /remote", LogKind::Output),
        logged("git apply --cached -", LogKind::Step),
        logged("  --- a/a.txt", LogKind::Input),
        logged("  +++ b/a.txt", LogKind::Input),
        logged("error: patch failed: a.txt:1", LogKind::Failure),
    ];
    let failed = Ended::Failed {
        title: String::from("Stage a change of a.txt"),
        said: String::from("patch failed"),
        plan: String::from("Stage a change of a.txt"),
        on_failure: Vec::new(),
    };
    let _effects = reduce(&mut state, Message::FeatureEnded { ended: failed, log });
    let said = state.operations.said.as_ref().unwrap();
    assert_eq!(
        said.log_at,
        Some(2),
        "the step that failed, above its patch and what git said of it"
    );
}
