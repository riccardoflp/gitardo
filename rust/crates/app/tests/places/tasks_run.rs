//! A task run from the Tasks tab (`tasks.run`): what a shell task and a process task run, with their environment and
//! what is typed to them, a task that runs asked to run again, and a folder not trusted. Each test runs real commands
//! in the terminal of a task.

use model::tasks::TaskReference;
use model::terminal::CellColor;
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use views::tasks::{RowStatus, TasksIntent};

use super::support::{Window, find, typed};
use super::tasks_tab::{Runner, WIDE, entry, printed_line, quoted, shell_task, standing, waiting_task, with_tasks};

/// A process task of `tasks.json`: a command and its arguments, run with no shell.
fn process_task(label: &str, command: &str, arguments: &[&str]) -> String {
    let arguments: Vec<String> = arguments.iter().map(|argument| quoted(argument)).collect();
    format!(
        "    {{ \"label\": {}, \"type\": \"process\", \"command\": {}, \"args\": [{}] }}",
        quoted(label),
        quoted(command),
        arguments.join(", ")
    )
}

/// Whether something drawn is named so.
fn named(tree: &TreeUpdate, name: &str) -> bool {
    tree.nodes.iter().any(|(_, node)| node.label() == Some(name))
}

/// Runs a task until it ended, and gives how it ended on its row.
fn run_to_its_end(runner: &Runner, state: &mut app::store::State, label: &str) -> Option<RowStatus> {
    runner.run_task(state, label);
    runner.until(state, &format!("the end of {label}"), |state| {
        !entry(state, label).unwrap().is_running()
    });
    standing(state, label)
}

#[test]
fn tasks_run__a_shell_task_git_status_prints_in_color_in_its_terminal_and_its_row_shows_a_check_once_it_ended() {
    let status = shell_task("status", ("git status --short", "git status --short"), "");
    let (_repository, mut state) = with_tasks(&[status]);
    let runner = Runner::new();
    runner.open(&mut state);
    assert_eq!(
        run_to_its_end(&runner, &mut state, "status"),
        Some(RowStatus::Succeeded)
    );
    runner.until(&mut state, "what git printed", |state| {
        super::tasks_tab::printed(state, "status").contains(".vscode/")
    });
    // The folder of the tasks is not tracked: git names it in red, since it prints to a terminal.
    let screen = entry(&state, "status").unwrap().screen.clone().unwrap();
    let mut spans = screen.rows.iter().flatten();
    let colored = spans.find(|span| span.text.contains("??") || span.text.contains(".vscode/"));
    let colored = colored.expect("the line of the folder not tracked");
    assert_ne!(colored.style.foreground, CellColor::Default, "{colored:?}");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let tree = runner.draw(&mut state, &mut window, &[]);
    // The check of its row is drawn, not said: the row says nothing after the command of a task that succeeded.
    let row = find(&tree, Role::ListBoxOption, "status, git status --short");
    assert!(row.bounds().is_some());
}

#[test]
fn tasks_run__a_process_task_runs_its_command_with_its_arguments_as_given_a_space_in_one_included() {
    let log = process_task("last", "git", &["log", "-1", "--format=one two %s"]);
    let (_repository, mut state) = with_tasks(&[log]);
    let runner = Runner::new();
    runner.open(&mut state);
    assert_eq!(run_to_its_end(&runner, &mut state, "last"), Some(RowStatus::Succeeded));
    runner.until(&mut state, "what git printed", |state| {
        printed_line(state, "last", "one two Add the readme")
    });
}

#[test]
fn tasks_run__options_env_foo_1_is_in_the_environment_of_the_task() {
    let commands = ("echo \"FOO=$env:FOO\"", "echo \"FOO=$FOO\"");
    let print = shell_task("print", commands, "\"options\": { \"env\": { \"FOO\": \"1\" } }");
    let (_repository, mut state) = with_tasks(&[print]);
    let runner = Runner::new();
    runner.open(&mut state);
    assert_eq!(run_to_its_end(&runner, &mut state, "print"), Some(RowStatus::Succeeded));
    runner.until(&mut state, "what it printed", |state| {
        printed_line(state, "print", "FOO=1")
    });
}

#[test]
fn tasks_run__a_task_that_reads_a_line_from_its_input_gets_the_line_typed_in_its_terminal() {
    let (_repository, mut state) = with_tasks(&[waiting_task("ask", "alpha")]);
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.open(&mut state);
    runner.run_task(&mut state, "ask");
    runner.until(&mut state, "what it prints", |state| {
        printed_line(state, "ask", "alpha")
    });
    // The keys go to the terminal of the task, which has the focus once the task was run.
    let _ = runner.draw(&mut state, &mut window, &[]);
    runner.click(&mut state, &mut window, Role::TextInput, "Terminal of the task");
    let _ = runner.draw(&mut state, &mut window, &[typed("hello")]);
    let enter = ui::KeyEvent::press(ui::Key::Named(ui::NamedKey::Enter), ui::Modifiers::NONE);
    let _ = runner.draw(&mut state, &mut window, &[ui::InputEvent::KeyPressed(enter)]);
    runner.until(&mut state, "the line it got", |state| {
        printed_line(state, "ask", "got hello")
    });
    runner.until(&mut state, "its end", |state| {
        !entry(state, "ask").unwrap().is_running()
    });
    assert_eq!(standing(&state, "ask"), Some(RowStatus::Succeeded));
}

#[test]
fn tasks_run__running_a_task_that_runs_asks_with_restart_and_leave_it_running_and_restart_ends_the_first_run_first() {
    let (_repository, mut state) = with_tasks(&[waiting_task("ask", "alpha")]);
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.open(&mut state);
    runner.run_task(&mut state, "ask");
    runner.until(&mut state, "what it prints", |state| {
        printed_line(state, "ask", "alpha")
    });
    let first = entry(&state, "ask").unwrap().id;
    runner.ask(&mut state, TasksIntent::Run(TaskReference::workspace("ask")));
    runner.until(&mut state, "the question", |state| {
        state
            .operations
            .running
            .as_ref()
            .is_some_and(|running| running.asking.is_some())
    });
    let tree = runner.draw(&mut state, &mut window, &[]);
    for words in ["`ask` is running.", "Restart", "Leave it running"] {
        assert!(named(&tree, words), "the question says {words}");
    }
    runner.choose(&mut state, features::tasks::run::LEAVE);
    runner.ended(&mut state);
    assert_eq!(
        *runner.log.borrow(),
        [format!("start {first}"), format!("started {first}")]
    );
    assert!(entry(&state, "ask").unwrap().is_running(), "left running");
    runner.ask(&mut state, TasksIntent::Run(TaskReference::workspace("ask")));
    runner.choose(&mut state, features::tasks::run::RESTART);
    runner.until(&mut state, "the second run", |state| {
        entry(state, "ask").is_some_and(|entry| entry.id != first && entry.is_running())
    });
    let second = entry(&state, "ask").unwrap().id;
    let log = runner.log.borrow().clone();
    let place = |said: String| {
        log.iter()
            .position(|line| *line == said)
            .unwrap_or_else(|| panic!("{said}: {log:?}"))
    };
    assert!(
        place(format!("stop {first}")) < place(format!("ended {first}")),
        "{log:?}"
    );
    assert!(
        place(format!("ended {first}")) < place(format!("start {second}")),
        "{log:?}"
    );
    runner.until(&mut state, "what the second run prints", |state| {
        printed_line(state, "ask", "alpha")
    });
    runner.ask(&mut state, TasksIntent::Stop(TaskReference::workspace("ask")));
    runner.until(&mut state, "its end", |state| {
        !entry(state, "ask").unwrap().is_running()
    });
}

#[test]
fn tasks_run__in_a_folder_not_trusted_nothing_runs() {
    let (repository, mut state) = with_tasks(&[shell_task("touch", ("echo ran > ran.txt", "echo ran > ran.txt"), "")]);
    state.persisted.trusted_folders.clear();
    let runner = Runner::new();
    runner.open(&mut state);
    runner.ask(&mut state, TasksIntent::Run(TaskReference::workspace("touch")));
    runner.until(&mut state, "the refusal", |state| state.operations.said.is_some());
    let said = state.operations.said.clone().unwrap();
    assert_eq!(said.text, features::tasks::run::NOT_TRUSTED);
    assert!(said.is_error);
    assert!(
        entry(&state, "touch").is_none() && runner.log.borrow().is_empty(),
        "{:?}",
        runner.log.borrow()
    );
    assert!(!repository.path().join("ran.txt").exists(), "the command did not run");
}
