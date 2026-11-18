//! **Run Task…** (`tasks.quick-run`): the picker of the tasks opened with `Ctrl+Shift+T` from the timeline and from
//! Quick search, a task typed and run from it, and a task that runs chosen in it. Each test runs real commands in the
//! terminals of the tasks.

use app::store::State;
use model::tasks::TaskReference;
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::Intent;
use views::panel::PanelTab;
use views::search::{QuickChoice, QuickCommand, SearchIntent};
use views::tasks::TasksIntent;
use views::terminal::TerminalIntent;

use super::pointer::{middle, rect};
use super::support::{Window, shortcut, typed};
use super::tasks_groups::{group, write_groups};
use super::tasks_tab::{Runner, WIDE, entry, printed_line, shell_task, tab, waiting_task, with_tasks};

/// `Ctrl+Shift+T`.
fn the_key() -> InputEvent {
    shortcut('T', Modifiers::CONTROL | Modifiers::SHIFT)
}

/// The names of the rows of the picker, from its top.
fn rows(tree: &TreeUpdate) -> Vec<String> {
    let rows = tree.nodes.iter().filter(|(_, node)| {
        let is_row = node.role() == Role::ListBoxOption;
        is_row && node.label().is_some_and(|label| label.contains(", "))
    });
    let mut rows: Vec<(f32, String)> = rows
        .map(|(_, node)| (middle(rect(node.bounds().unwrap())).y, node.label().unwrap().to_owned()))
        .collect();
    rows.sort_by(|one, other| one.0.total_cmp(&other.0));
    rows.into_iter().map(|(_, name)| name).collect()
}

/// Opens the picker with its key, and draws it once the tasks were read.
fn open(runner: &Runner, state: &mut State, window: &mut Window) -> TreeUpdate {
    let _ = runner.draw(state, window, &[the_key()]);
    assert!(state.tasks_tab.quick_run.is_some(), "the key opens the picker");
    runner.until(state, "the tasks read", |state| {
        tab(state).is_some_and(|tab| tab.read.is_some() && tab.reading.is_none())
    });
    let _ = runner.draw(state, window, &[]);
    runner.draw(state, window, &[])
}

#[test]
fn tasks_quick_run__ctrl_shift_t_from_the_timeline_opens_the_picker_with_the_groups_first() {
    let build = shell_task("build", ("echo building", "echo building"), "");
    let lint = shell_task("lint", ("echo linting", "echo linting"), "");
    let (repository, mut state) = with_tasks(&[build, lint]);
    write_groups(&repository, &[group("Checks", &["lint", "build"])]);
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(
        rows(&tree).is_empty() && !state.panel.look.is_shown,
        "the timeline alone"
    );
    let tree = open(&runner, &mut state, &mut window);
    assert_eq!(
        rows(&tree),
        [
            "Checks, lint, build, Groups",
            "build, echo building, Workspace",
            "lint, echo linting, Workspace"
        ],
        "the groups first, then the tasks by source, each with its command"
    );
    assert!(
        !state.panel.look.is_shown,
        "nothing else is shown until a task is chosen"
    );
    window.assert_snapshot("tasks-quick-run-dark");
    let escape = InputEvent::KeyPressed(KeyEvent::press(Key::Named(NamedKey::Escape), Modifiers::NONE));
    let _ = runner.draw(&mut state, &mut window, &[escape]);
    assert!(state.tasks_tab.quick_run.is_none(), "Escape closes it");
    // **Run Task…** of Quick search opens it too.
    let chosen = Intent::Search(SearchIntent::ChooseQuick(QuickChoice::Command(QuickCommand::RunTask)));
    runner.intents(&mut state, vec![chosen]);
    assert!(state.tasks_tab.quick_run.is_some() && !state.search.quick.is_open);
    assert!(entry(&state, "build").is_none() && entry(&state, "lint").is_none());
}

#[test]
fn tasks_quick_run__build_typed_then_enter_runs_build_and_shows_its_terminal_in_the_tasks_tab() {
    let build = shell_task("build", ("echo building", "echo building"), "");
    let lint = shell_task("lint", ("echo linting", "echo linting"), "");
    let (_repository, mut state) = with_tasks(&[lint, build]);
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.ask(&mut state, TasksIntent::Terminal(TerminalIntent::Resize(100, 20)));
    let _ = open(&runner, &mut state, &mut window);
    let _ = runner.draw(&mut state, &mut window, &[typed("build")]);
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert_eq!(
        rows(&tree),
        ["build, echo building, Workspace"],
        "what is typed filters"
    );
    let enter = InputEvent::KeyPressed(KeyEvent::press(Key::Named(NamedKey::Enter), Modifiers::NONE));
    let _ = runner.draw(&mut state, &mut window, &[enter]);
    assert!(state.tasks_tab.quick_run.is_none(), "the picker closes");
    runner.until(&mut state, "what build prints", |state| {
        printed_line(state, "build", "building")
    });
    assert!(state.panel.look.is_shown && state.panel.look.tab == PanelTab::Tasks);
    assert_eq!(tab(&state).unwrap().selected, Some(TaskReference::workspace("build")));
    assert!(entry(&state, "lint").is_none(), "only the task chosen ran");
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(
        tree.nodes.iter().any(|(_, node)| node.role() == Role::TextInput),
        "its terminal is shown"
    );
}

#[test]
fn tasks_quick_run__a_task_running_chosen_in_the_picker_shows_its_terminal_and_runs_nothing() {
    let other = shell_task("build", ("echo building", "echo building"), "");
    let (_repository, mut state) = with_tasks(&[other, waiting_task("serve", "serving")]);
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.open(&mut state);
    runner.run_task(&mut state, "serve");
    runner.until(&mut state, "what serve prints", |state| {
        printed_line(state, "serve", "serving")
    });
    runner.run_task(&mut state, "build");
    runner.until(&mut state, "the end of build", |state| {
        entry(state, "build").is_some_and(|entry| !entry.is_running())
    });
    let first = entry(&state, "serve").unwrap().id;
    runner.intents(&mut state, vec![Intent::Panel(views::panel::PanelIntent::Hide)]);
    let tree = open(&runner, &mut state, &mut window);
    let serve = rows(&tree).into_iter().find(|row| row.starts_with("serve, ")).unwrap();
    runner.click(&mut state, &mut window, Role::ListBoxOption, &serve);
    runner.for_a_while(&mut state, std::time::Duration::from_millis(300));
    assert!(state.tasks_tab.quick_run.is_none(), "the picker closes");
    assert!(state.panel.look.is_shown && state.panel.look.tab == PanelTab::Tasks);
    assert_eq!(tab(&state).unwrap().selected, Some(TaskReference::workspace("serve")));
    let running = entry(&state, "serve").unwrap();
    assert!(running.id == first && running.is_running(), "the same run goes on");
    assert!(state.operations.running.is_none(), "nothing is asked about it");
    let log = runner.log.borrow().clone();
    let starts = log.iter().filter(|line| line.starts_with("start ")).count();
    assert_eq!(starts, 2, "serve was not started again: {log:?}");
    runner.ask(&mut state, TasksIntent::Stop(TaskReference::workspace("serve")));
    runner.until(&mut state, "the end of serve", |state| {
        entry(state, "serve").is_some_and(|entry| !entry.is_running())
    });
}
