//! More of the Tasks tab of the bottom panel (`tasks.view`): the terminal of each task kept apart, how a task that
//! failed is shown, a task opened in `tasks.json`, a folder not trusted, and the tab drawn in each theme.

use std::time::Duration;

use app::store::tasks_tab::TaskRunMessage;
use app::store::{FileContent, Message, State};
use model::tasks::TaskReference;
use model::terminal::{CellColor, CellStyle, Cursor, CursorShape, Screen, Span};
use platform::accesskit::Role;
use render::{Point, ThemeName};
use ui::{InputEvent, Modifiers, PointerButton, PointerButtonEvent};
use views::tasks::{RowStatus, TasksIntent};

use super::pointer::{middle, moved, rect};
use super::support::{Window, find, has, says, says_in};
use super::tasks_tab::{
    Runner, WIDE, entry, printed, printed_line, row_of, shell_task, standing, waiting_task, with_tasks,
};

/// The themes, each with its name in a snapshot.
const THEMES: [(ThemeName, &str); 4] = [
    (ThemeName::Dark, "dark"),
    (ThemeName::Light, "light"),
    (ThemeName::HighContrastDark, "high-contrast-dark"),
    (ThemeName::HighContrastLight, "high-contrast-light"),
];

/// Presses and lets go the button of context menus at a point, then draws the menu it opened.
fn right_click(window: &mut Window, state: &State, point: Point) {
    let _ = window.draw(state, &[moved(point)]);
    let press = PointerButtonEvent {
        button: PointerButton::Secondary,
        position: point,
        modifiers: Modifiers::NONE,
        click_count: 1,
    };
    let _ = window.draw(
        state,
        &[InputEvent::PointerPressed(press), InputEvent::PointerReleased(press)],
    );
    let _ = window.draw(state, &[]);
}

/// A task that ends with an exit code after printing a word.
fn exiting_task(label: &str, word: &str, code: i32) -> String {
    let command = format!("echo {word}; exit {code}");
    shell_task(label, (&command, &command), "")
}

#[test]
fn tasks_view__selecting_another_task_shows_its_terminal_and_coming_back_shows_the_first_as_it_was() {
    let (_repository, mut state) = with_tasks(&[waiting_task("first", "alpha"), waiting_task("second", "beta")]);
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.open(&mut state);
    for (label, word) in [("first", "alpha"), ("second", "beta")] {
        runner.run_task(&mut state, label);
        runner.until(&mut state, word, |state| printed_line(state, label, word));
    }
    let shown = |runner: &Runner, state: &mut State, window: &mut Window| {
        let tree = runner.draw(state, window, &[]);
        find(&tree, Role::TextInput, "Terminal of the task")
            .value()
            .unwrap_or_default()
            .trim_end()
            .to_owned()
    };
    let second = shown(&runner, &mut state, &mut window);
    assert!(second.contains("beta") && !second.contains("alpha"), "{second}");
    let tree = runner.draw(&mut state, &mut window, &[]);
    let intents = window.click_at(&state, row_of(&tree, "first"), 1);
    runner.intents(&mut state, intents);
    let first = shown(&runner, &mut state, &mut window);
    assert!(first.contains("alpha") && !first.contains("beta"), "{first}");
    // What is typed goes to the task shown, and the other keeps what it showed.
    let line = TasksIntent::Terminal(views::terminal::TerminalIntent::Input(b"one\r".to_vec()));
    runner.ask(&mut state, line);
    runner.until(&mut state, "the line it got", |state| {
        printed_line(state, "first", "got one")
    });
    let tree = runner.draw(&mut state, &mut window, &[]);
    let intents = window.click_at(&state, row_of(&tree, "second"), 1);
    runner.intents(&mut state, intents);
    assert_eq!(shown(&runner, &mut state, &mut window), second, "as it was");
    assert!(entry(&state, "second").unwrap().is_running());
}

#[test]
fn tasks_view__a_task_that_ended_with_exit_code_2_shows_2_on_its_row_and_says_so_under_its_output() {
    let (_repository, mut state) = with_tasks(&[exiting_task("lint", "linted", 2)]);
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.open(&mut state);
    runner.run_task(&mut state, "lint");
    runner.until(&mut state, "its end", |state| {
        !entry(state, "lint").unwrap().is_running() && printed_line(state, "lint", "linted")
    });
    assert_eq!(standing(&state, "lint"), Some(RowStatus::Failed(2)));
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(
        has(&tree, Role::ListBoxOption, "lint, echo linted; exit 2, 2"),
        "its row says the exit code"
    );
    assert!(says(&tree, "The task ended with exit code 2"));
    assert!(says_in(&tree, "It ran for "), "and how long it ran");
    assert!(printed(&state, "lint").contains("linted"), "its output stays");
}

#[test]
fn tasks_view__open_in_tasks_json_opens_the_file_page_at_the_line_of_its_label() {
    let build = shell_task("build", ("echo building", "echo building"), "");
    let watch = shell_task("watch", ("echo watching", "echo watching"), "");
    let (repository, mut state) = with_tasks(&[build, watch]);
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.open(&mut state);
    let tree = runner.draw(&mut state, &mut window, &[]);
    right_click(&mut window, &state, row_of(&tree, "watch"));
    let tree = runner.draw(&mut state, &mut window, &[]);
    for item in ["Run", "Stop", "Restart", "Open in tasks.json", "Copy command"] {
        assert!(has(&tree, Role::MenuItem, item), "the menu of a task has {item}");
    }
    runner.click(&mut state, &mut window, Role::MenuItem, "Open in tasks.json");
    let page = state.file_pages.pages.last().expect("a File page");
    assert_eq!(page.path, ".vscode/tasks.json");
    let text = std::fs::read_to_string(repository.path().join(".vscode/tasks.json")).unwrap();
    let line = text.lines().position(|line| line.contains("\"watch\"")).unwrap();
    let FileContent::Text(document) = &page.content else {
        panic!("the file is text")
    };
    assert_eq!(page.cursor.head, document.buffer().line_range(line).start);
}

#[test]
fn tasks_view__in_a_folder_not_trusted_run_is_dimmed_and_trust_this_folder_asks_to_trust_it() {
    let (_repository, mut state) = with_tasks(&[shell_task("build", ("echo building", "echo building"), "")]);
    state.persisted.trusted_folders.clear();
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.open(&mut state);
    runner.ask(&mut state, TasksIntent::Select(TaskReference::workspace("build")));
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(says(
        &tree,
        "Tasks run commands of the repository: trust this folder to run them."
    ));
    let runs: Vec<_> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some("Run"))
        .collect();
    assert_eq!(runs.len(), 2, "on the row and beside the command");
    assert!(runs.iter().all(|(_, node)| node.is_disabled()), "every Run is dimmed");
    runner.click(&mut state, &mut window, Role::Button, "Trust this folder");
    runner.until(&mut state, "the question", |state| {
        app::store::feature::question_state(state).is_some()
    });
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(says(&tree, "Trust this folder?"));
    assert!(
        state.persisted.trusted_folders.is_empty(),
        "nothing is trusted before the answer"
    );
    // The button of the question is over the one of the column, and after it in what is drawn.
    let mut buttons = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.label() == Some("Trust this folder"));
    let (_, accept) = buttons.next_back().unwrap();
    let intents = window.click_at(&state, middle(rect(accept.bounds().unwrap())), 1);
    runner.intents(&mut state, intents);
    runner.until(&mut state, "the folder trusted", |state| {
        !state.persisted.trusted_folders.is_empty()
    });
    assert_eq!(
        state.persisted.trusted_folders,
        [state.shown_folder().cloned().unwrap()]
    );
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(!find(&tree, Role::Button, "Run").is_disabled(), "Run runs now");
    assert!(!has(&tree, Role::Button, "Trust this folder"));
}

/// A screen of some lines in the default look, the cursor under them; a line that starts with `ok` has it in green.
pub(super) fn screen(lines: &[&str]) -> Screen {
    let span = |column: usize, text: &str, foreground: CellColor| Span {
        column,
        width: text.chars().count(),
        is_wide: false,
        text: text.to_owned(),
        style: CellStyle {
            foreground,
            ..CellStyle::default()
        },
    };
    let rows = lines.iter().map(|line| match line.strip_prefix("ok") {
        Some(rest) => vec![span(0, "ok", CellColor::Indexed(2)), span(2, rest, CellColor::Default)],
        None => vec![span(0, line, CellColor::Default)],
    });
    Screen {
        columns: 100,
        rows: rows.collect(),
        cursor: Some(Cursor {
            row: lines.len(),
            column: 0,
            shape: CursorShape::Block,
        }),
        ..Screen::default()
    }
}

/// Starts a task as the store asks, without a process: its terminal started and shows some lines.
fn pretend(runner: &Runner, state: &mut State, label: &str, lines: &[&str]) -> u64 {
    runner.run_task(state, label);
    let id = entry(state, label).unwrap().id;
    for told in [
        TaskRunMessage::Started(id),
        TaskRunMessage::Screen(id, Box::new(screen(lines)), false),
    ] {
        runner.message(state, Message::TaskRun(told));
    }
    id
}

/// A repository whose task `build` ended with exit code 0 and `lint` with 2, while `serve`, of the group `Servers`,
/// runs, none of them a process: the tests say what each printed.
fn some_ended_and_one_running() -> (testkit::Repository, State, Runner) {
    let plain = |label: &str, command: &str| shell_task(label, (command, command), "");
    let tasks = [
        plain("build", "cargo build --workspace"),
        plain("lint", "cargo clippy --workspace -- -D warnings"),
        plain("serve", "cargo run -p server -- --port 8080"),
        plain("test", "cargo test --workspace"),
    ];
    let (repository, mut state) = with_tasks(&tasks);
    std::fs::write(
        repository.path().join("package.json"),
        "{ \"name\": \"site\", \"scripts\": { \"dev\": \"vite\", \"build\": \"vite build\" } }\n",
    )
    .unwrap();
    super::tasks_groups::write_groups(&repository, &[super::tasks_groups::group("Servers", &["serve"])]);
    let runner = Runner::new();
    runner.pretend.set(true);
    runner.open(&mut state);
    let ended = |runner: &Runner, state: &mut State, id: u64, code: i32, seconds: u64| {
        let told = TaskRunMessage::Ended(id, Some(code), Duration::from_secs(seconds));
        runner.message(state, Message::TaskRun(told));
    };
    let build = pretend(
        &runner,
        &mut state,
        "build",
        &["   Compiling server v1.0.0", "ok Finished in 41.2 s"],
    );
    ended(&runner, &mut state, build, 0, 41);
    let lint = pretend(
        &runner,
        &mut state,
        "lint",
        &[
            "    Checking server v1.0.0",
            "error: unused variable: `port`",
            " --> src/main.rs:12:9",
            "error: could not compile `server` due to 1 previous error",
        ],
    );
    ended(&runner, &mut state, lint, 2, 3);
    let _serve = pretend(
        &runner,
        &mut state,
        "serve",
        &[
            "   Compiling server v1.0.0",
            "ok Listening on http://127.0.0.1:8080",
            "GET /health 200",
        ],
    );
    (repository, state, runner)
}

#[test]
fn tasks_view__the_tab_is_drawn_with_a_task_running_and_one_ended_in_each_theme() {
    let (_repository, mut state, runner) = some_ended_and_one_running();
    for (theme, name) in THEMES {
        let mut window = Window::new(theme, WIDE);
        runner.ask(&mut state, TasksIntent::Select(TaskReference::workspace("serve")));
        let tree = runner.draw(&mut state, &mut window, &[]);
        assert!(has(&tree, Role::Button, "Tasks (1)"));
        let _ = runner.draw(&mut state, &mut window, &[]);
        window.assert_snapshot(&format!("tasks-tab-running-{name}"));
        runner.ask(&mut state, TasksIntent::Select(TaskReference::workspace("lint")));
        let _ = runner.draw(&mut state, &mut window, &[]);
        let tree = runner.draw(&mut state, &mut window, &[]);
        assert!(says(&tree, "The task ended with exit code 2"));
        window.assert_snapshot(&format!("tasks-tab-ended-{name}"));
    }
}
