//! What the File page does beyond typing, on files of repositories made for each test: `Ctrl+S` run through the
//! operations thread, shown first with `commandPreview` set to `all` and asking when the file changed on disk; the box
//! that finds; **Go to line**; the file opened in the external editor; and the colors of the code in each theme.

use app::store::feature::Ended;
use app::store::{FileContent, Message, State};
use model::contract::CommandPreview;
use platform::accesskit::Role;
use render::{Appearance, Theme, ThemeName};
use ui::{Modifiers, NamedKey};
use views::Intent;
use views::feature::FeatureIntent;
use views::file_view::FileTarget;
use views::text::file_view as words;

use super::diff::{head_to_working_tree, open as open_diff, shown as diff_shown, text as diff_text};
use super::file_view::{FILE, key, numbered, open, repository_with, shown, step, text, type_text};
use super::showing;
use super::support::{OperationsDriver, Window, says, shortcut, typed};

/// A page of `src/app.rs` with a text, drawn once so that its editor has the keyboard.
fn page_of(text: &str) -> (testkit::Repository, State, Window) {
    let repository = repository_with(text);
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, FileTarget::working_tree("src/app.rs"));
    let mut window = Window::new(ThemeName::Dark, FILE);
    let _ = step(&mut window, &mut state, &[]);
    (repository, state, window)
}

/// Draws a frame with some input and runs the operation it asks through the operations thread: the plan shown first
/// is confirmed, and the question asked is answered by `answer`. Gives how it ended, the lines it logged, and whether
/// a plan or a question was shown.
fn run_operation(
    window: &mut Window,
    state: &mut State,
    (events, answer): (&[ui::InputEvent], Option<model::contract::Answer>),
) -> (Ended, Vec<String>, Vec<&'static str>) {
    let driver = OperationsDriver::new();
    let (intents, _) = window.draw(state, events);
    for intent in intents {
        driver.intent(state, intent);
    }
    assert!(state.operations.running.is_some(), "an operation runs");
    let mut shown = Vec::new();
    loop {
        match driver.next(state) {
            Message::FeatureConfirms(_) => {
                shown.push("plan");
                driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true)));
            }
            Message::FeatureAsks(_) => {
                shown.push("question");
                driver.intent(state, Intent::Feature(FeatureIntent::Answer(answer.clone())));
            }
            Message::FeatureEnded { ended, log } => {
                return (
                    ended,
                    log.into_iter()
                        .filter(super::support::is_ran)
                        .map(|entry| entry.line)
                        .collect(),
                    shown,
                );
            }
            _ => {}
        }
    }
}

#[test]
fn file_view_save__ctrl_s_after_typing_writes_the_file_and_the_log_has_exactly_one_write() {
    let (repository, mut state, mut window) = page_of("fn main() {}\n");
    type_text(&mut window, &mut state, "pub ");
    let (ended, log, shown_first) =
        run_operation(&mut window, &mut state, (&[shortcut('s', Modifiers::CONTROL)], None));
    assert!(matches!(ended, Ended::Ran { .. }), "{ended:?}");
    let path = repository.path().join("src").join("app.rs");
    assert_eq!(log, [format!("write {}", path.display())]);
    assert_eq!(shown_first.len(), 0);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "pub fn main() {}\n");
    assert!(!shown(&state).is_unsaved(), "the dot goes away");
}

#[test]
fn file_view_save__with_command_preview_all_the_save_is_shown_first() {
    let (repository, mut state, mut window) = page_of("fn main() {}\n");
    state.settings.features.command_preview = CommandPreview::All;
    type_text(&mut window, &mut state, "pub ");
    let (ended, _, shown_first) = run_operation(&mut window, &mut state, (&[shortcut('s', Modifiers::CONTROL)], None));
    assert!(matches!(ended, Ended::Ran { .. }), "{ended:?}");
    assert_eq!(shown_first, ["plan"]);
    let on_disk = std::fs::read_to_string(repository.path().join("src/app.rs")).unwrap();
    assert_eq!(on_disk, "pub fn main() {}\n");
}

#[test]
fn file_view_save__a_file_changed_on_disk_meanwhile_asks_and_cancel_writes_nothing() {
    let (repository, mut state, mut window) = page_of("fn main() {}\n");
    type_text(&mut window, &mut state, "pub ");
    let path = repository.path().join("src/app.rs");
    std::fs::write(&path, "fn main() { theirs(); }\n").unwrap();
    let (ended, log, shown_first) =
        run_operation(&mut window, &mut state, (&[shortcut('s', Modifiers::CONTROL)], None));
    assert_eq!(shown_first, ["question"]);
    assert!(matches!(ended, Ended::BackedOut), "{ended:?}");
    assert_eq!(log.len(), 0, "{log:?}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "fn main() { theirs(); }\n");
    assert!(shown(&state).is_unsaved());
}

/// A file with `parse` on 12 of its lines and `Parse` on one.
fn parse_twelve_times() -> String {
    let mut text = String::new();
    for line in 1..=12 {
        text.push_str("let a");
        text.push_str(&line.to_string());
        text.push_str(" = parse(input);\n");
    }
    text.push_str("struct Parse;\n");
    text
}

#[test]
fn file_view_find__typing_parse_marks_12_matches_reads_1_of_12_and_f3_goes_to_2_of_12() {
    let (_repository, mut state, mut window) = page_of(&parse_twelve_times());
    let _ = step(&mut window, &mut state, &[shortcut('f', Modifiers::CONTROL)]);
    let _ = step(&mut window, &mut state, &[]);
    let _ = step(&mut window, &mut state, &[typed("parse(")]);
    let find = shown(&state).find.clone().unwrap();
    assert_eq!(find.marks.len(), 12);
    let tree = step(&mut window, &mut state, &[]);
    assert!(says(&tree, "1 of 12"));
    let _ = step(&mut window, &mut state, &[key(NamedKey::Function(3), Modifiers::NONE)]);
    let tree = step(&mut window, &mut state, &[]);
    assert!(says(&tree, "2 of 12"));
}

#[test]
fn file_view_find__with_match_case_on_parse_with_a_capital_does_not_match_parse() {
    let (_repository, mut state, mut window) = page_of(&parse_twelve_times());
    let _ = step(&mut window, &mut state, &[shortcut('f', Modifiers::CONTROL)]);
    let _ = step(&mut window, &mut state, &[]);
    let _ = step(&mut window, &mut state, &[typed("Parse")]);
    assert_eq!(shown(&state).find.as_ref().unwrap().marks.len(), 13);
    let intents = window.click(&state, Role::CheckBox, words::MATCH_CASE);
    super::pointer::apply(&mut state, intents);
    assert_eq!(shown(&state).find.as_ref().unwrap().marks.len(), 1);
}

#[test]
fn file_view_find__an_invalid_regular_expression_says_so_and_marks_nothing() {
    let (_repository, mut state, _) = page_of(&parse_twelve_times());
    // Wide enough for every switch of the bar beside the side column.
    let mut window = Window::new(ThemeName::Dark, (1260, 600));
    let _ = step(&mut window, &mut state, &[]);
    let _ = step(&mut window, &mut state, &[shortcut('f', Modifiers::CONTROL)]);
    let _ = step(&mut window, &mut state, &[]);
    let _ = step(&mut window, &mut state, &[typed("parse(")]);
    assert_eq!(shown(&state).find.as_ref().unwrap().marks.len(), 12);
    let intents = window.click(&state, Role::CheckBox, words::REGULAR_EXPRESSION);
    super::pointer::apply(&mut state, intents);
    let find = shown(&state).find.clone().unwrap();
    assert!(find.is_invalid);
    assert_eq!(find.marks.len(), 0);
    let tree = step(&mut window, &mut state, &[]);
    assert!(super::support::says_in(&tree, words::INVALID_PATTERN));
}

/// The letter of Go to line with Ctrl: G, and L on macOS, as its card says.
const GO_TO_LINE: char = if cfg!(target_os = "macos") { 'l' } else { 'g' };

#[test]
fn file_view_go_to_line__ctrl_g_and_120_put_the_cursor_at_the_start_of_line_120_centered() {
    let (_repository, mut state, mut window) = page_of(&numbered(300));
    let _ = step(&mut window, &mut state, &[shortcut(GO_TO_LINE, Modifiers::CONTROL)]);
    assert_eq!(shown(&state).go_to_line.as_deref(), Some("1"));
    let _ = step(&mut window, &mut state, &[]);
    let _ = step(
        &mut window,
        &mut state,
        &[shortcut('a', Modifiers::CONTROL), typed("120")],
    );
    let _ = step(&mut window, &mut state, &[key(NamedKey::Enter, Modifiers::NONE)]);
    let page = shown(&state);
    assert_eq!(page.go_to_line, None);
    let FileContent::Text(document) = &page.content else {
        panic!("text")
    };
    assert_eq!(page.cursor.head, document.buffer().line_range(119).start);
    assert_eq!(page.reveal.map(|(_, line)| line), Some(119));
    let tree = step(&mut window, &mut state, &[]);
    assert!(super::support::says_in(&tree, "line 120"));
}

#[test]
fn file_view_go_to_line__typing_abc_is_refused_while_typed() {
    let (_repository, mut state, mut window) = page_of(&numbered(300));
    let _ = step(&mut window, &mut state, &[shortcut(GO_TO_LINE, Modifiers::CONTROL)]);
    let _ = step(&mut window, &mut state, &[]);
    let _ = step(
        &mut window,
        &mut state,
        &[shortcut('a', Modifiers::CONTROL), typed("abc")],
    );
    let tree = step(&mut window, &mut state, &[]);
    let refusal = words::line_between(300);
    let first_words = refusal.split(" from").next().unwrap();
    assert!(super::support::says_in(&tree, first_words), "{refusal}");
    let _ = step(&mut window, &mut state, &[key(NamedKey::Enter, Modifiers::NONE)]);
    assert_eq!(shown(&state).go_to_line.as_deref(), Some("abc"), "Go does not go");
    assert_eq!(shown(&state).cursor.head, 0);
}

#[test]
fn file_view_open_external__the_command_of_the_setting_is_run_with_the_absolute_path_and_the_line() {
    let (repository, mut state, mut window) = page_of(&numbered(300));
    state.settings.features.external_editor = String::from("echo ${file} ${line}");
    super::file_view::open(
        &mut state,
        FileTarget {
            line: Some(42),
            ..FileTarget::working_tree("src/app.rs")
        },
    );
    let _ = step(&mut window, &mut state, &[]);
    let (_, tree) = window.draw(&state, &[]);
    assert!(super::support::has(&tree, Role::Button, words::OPEN_EXTERNAL));
    let point = super::pointer::bounds(&tree, Role::Button, words::OPEN_EXTERNAL).center();
    let driver = OperationsDriver::new();
    for intent in window.click_at(&state, point, 1) {
        driver.intent(&mut state, intent);
    }
    let Message::FeatureEnded { ended, log } = driver.next(&mut state) else {
        panic!("the command runs at once")
    };
    assert!(matches!(ended, Ended::Ran { .. }), "{ended:?}");
    let path = repository.path().join("src").join("app.rs");
    let lines: Vec<String> = log
        .into_iter()
        .filter(super::support::is_ran)
        .map(|entry| entry.line)
        .collect();
    assert_eq!(lines, [format!("run echo {} 42", path.display())]);
}

/// How many pixels of a frame are near a color.
fn pixels_near(window: &Window, color: render::Color) -> usize {
    let pixels = window.frame.to_rgba();
    let distance = |channel: u8, wanted: u8| u32::from(channel.abs_diff(wanted));
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|[red, green, blue, _]| {
            distance(*red, color.red) + distance(*green, color.green) + distance(*blue, color.blue) < 24
        })
        .count()
}

#[test]
fn file_view_highlight__switching_from_the_dark_to_the_light_theme_recolors_an_open_file_page() {
    let repository = repository_with("fn main() { let answer = 42; }\n");
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, FileTarget::working_tree("src/app.rs"));
    let dark = Theme::new(ThemeName::Dark, Appearance::Dark).code.keyword;
    let light = Theme::new(ThemeName::Light, Appearance::Dark).code.keyword;
    let mut window = Window::new(ThemeName::Dark, FILE);
    let _ = step(&mut window, &mut state, &[]);
    let _ = step(&mut window, &mut state, &[]);
    assert!(
        pixels_near(&window, dark) > 0,
        "the keywords are in the color of the dark theme"
    );
    let mut window = Window::new(ThemeName::Light, FILE);
    let _ = step(&mut window, &mut state, &[]);
    assert!(
        pixels_near(&window, light) > 0,
        "the keywords are in the color of the light theme"
    );
    assert_eq!(pixels_near(&window, dark), 0);
    assert_eq!(text(&state), "fn main() { let answer = 42; }\n");
}

#[test]
fn file_view_highlight__a_line_in_a_block_comment_of_10_lines_is_a_comment_also_as_the_first_line_of_a_change() {
    let mut lines = vec![String::from("/* one")];
    lines.extend((2..=9).map(|line| format!("   line {line}")));
    lines.push(String::from("*/ fn main() {}"));
    let before = lines.join("\n") + "\n";
    let mut repository = testkit::Repository::new();
    let _ = repository.commit("Add the app", &[("src/app.rs", before.as_str())]);
    let after = before.replace("   line 5", "   line five");
    std::fs::write(repository.path().join("src/app.rs"), &after).unwrap();
    let mut state = showing(repository.path().to_path_buf());
    open_diff(&mut state, head_to_working_tree("src/app.rs"));
    let diff = diff_text(diff_shown(&state));
    let comment = |colors: &[ui::CodeSpan], line: &str| {
        colors.len() == 1
            && colors
                .first()
                .is_some_and(|span| span.kind == ui::CodeKind::Comment && span.range == (0..line.len()))
    };
    let line = 4;
    assert_eq!(diff.right_line(line), "   line five");
    assert!(
        comment(&diff.right_colors[line], "   line five"),
        "{:?}",
        diff.right_colors[line]
    );
    assert!(
        comment(&diff.left_colors[line], "   line 5"),
        "{:?}",
        diff.left_colors[line]
    );
    let file = repository_with(&before);
    let mut state = showing(file.path().to_path_buf());
    open(&mut state, FileTarget::working_tree("src/app.rs"));
    assert!(
        comment(&shown(&state).colors.spans[line], "   line 5"),
        "the File page colors it so too"
    );
}
