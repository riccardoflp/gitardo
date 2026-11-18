//! A task stopped from the Tasks tab (`tasks.stop`): the task and what it started ended, a task that ignores the
//! interrupt ended three seconds later, what its row and its terminal show then, and the window closed while a task
//! runs. Each test runs real commands in the terminal of a task; and the tasks read again when their file changes
//! (`tasks.read`).

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use app::persisted::WindowPlace;
use app::store::{Message, State};
use model::repo::Change;
use model::tasks::TaskReference;
use platform::accesskit::Role;
use render::ThemeName;
use views::tasks::{RowStatus, TasksIntent};

use super::support::{Window, find, has, says};
use super::tasks_tab::{
    PATIENCE, Runner, WIDE, entry, printed, printed_line, shell_task, standing, waiting_task, with_tasks, write_tasks,
};

/// A task that starts a process of its own, which no interrupt reaches, writes the number of that process in
/// `child.pid`, prints `started` and waits ten minutes.
fn task_with_a_child(label: &str) -> String {
    let windows = "$child = Start-Process powershell -PassThru -WindowStyle Hidden -ArgumentList '-NoProfile', \
                   '-Command', 'Start-Sleep 600'; Set-Content -Path child.pid -Value $child.Id; echo started; \
                   Start-Sleep 600";
    // A shell that keeps no jobs starts what it runs behind it with the interrupt ignored.
    let elsewhere = "sleep 600 & echo $! > child.pid; echo started; sleep 600";
    shell_task(label, (windows, elsewhere), "")
}

/// A task that goes on when it is sent an interrupt.
fn deaf_task(label: &str) -> String {
    let windows = "[Console]::TreatControlCAsInput = $true; echo deaf; while ($true) { Start-Sleep -Milliseconds 200 }";
    let elsewhere = "trap '' INT; echo deaf; while true; do sleep 1; done";
    shell_task(label, (windows, elsewhere), "")
}

/// Whether the system still has a process of a number.
fn is_alive(process: u32) -> bool {
    if cfg!(windows) {
        let filter = format!("PID eq {process}");
        let listed = Command::new("tasklist")
            .args(["/FI", &filter, "/NH", "/FO", "CSV"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&listed.stdout).contains(&format!("\"{process}\""))
    } else {
        let asked = Command::new("kill")
            .args(["-0", &process.to_string()])
            .output()
            .unwrap();
        asked.status.success()
    }
}

/// The number a task wrote in a file of the repository, once it wrote it.
fn number_in(file: &Path) -> u32 {
    let started = Instant::now();
    loop {
        let read = std::fs::read_to_string(file).unwrap_or_default();
        if let Ok(number) = read.trim().trim_start_matches('\u{feff}').parse() {
            return number;
        }
        assert!(started.elapsed() < PATIENCE, "{} was not written", file.display());
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Stops a task of `tasks.json` as **Stop** on its row does, and waits until it ended.
fn stop(runner: &Runner, state: &mut State, label: &str) {
    runner.ask(state, TasksIntent::Stop(TaskReference::workspace(label)));
    runner.until(state, &format!("the end of {label}"), |state| {
        !entry(state, label).unwrap().is_running()
    });
}

#[test]
fn tasks_stop__stop_on_a_task_ends_it_and_the_process_it_started() {
    let (repository, mut state) = with_tasks(&[task_with_a_child("serve")]);
    let runner = Runner::new();
    runner.open(&mut state);
    runner.run_task(&mut state, "serve");
    runner.until(&mut state, "what it prints", |state| {
        printed_line(state, "serve", "started")
    });
    let child = number_in(&repository.path().join("child.pid"));
    assert!(is_alive(child), "the process the task started runs");
    stop(&runner, &mut state, "serve");
    let started = Instant::now();
    while is_alive(child) {
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "the process {child} outlived its task"
        );
        runner.for_a_while(&mut state, Duration::from_millis(200));
    }
    assert_eq!(standing(&state, "serve"), Some(RowStatus::Stopped));
}

#[test]
fn tasks_stop__a_task_that_ignores_the_interrupt_is_ended_three_seconds_later() {
    let (_repository, mut state) = with_tasks(&[deaf_task("deaf")]);
    let runner = Runner::new();
    runner.open(&mut state);
    runner.run_task(&mut state, "deaf");
    runner.until(&mut state, "what it prints", |state| {
        printed_line(state, "deaf", "deaf")
    });
    let asked = Instant::now();
    runner.ask(&mut state, TasksIntent::Stop(TaskReference::workspace("deaf")));
    runner.for_a_while(&mut state, Duration::from_secs(2));
    assert!(
        entry(&state, "deaf").unwrap().is_running(),
        "it goes on after the interrupt"
    );
    runner.until(&mut state, "its end", |state| {
        !entry(state, "deaf").unwrap().is_running()
    });
    let took = asked.elapsed();
    assert!(took >= Duration::from_millis(2900), "ended after {took:?}");
    assert!(took < Duration::from_secs(20), "ended after {took:?}");
    assert_eq!(standing(&state, "deaf"), Some(RowStatus::Stopped));
}

#[test]
fn tasks_stop__its_row_says_stopped_and_its_terminal_keeps_its_output() {
    let (_repository, mut state) = with_tasks(&[waiting_task("ask", "alpha")]);
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.open(&mut state);
    runner.run_task(&mut state, "ask");
    runner.until(&mut state, "what it prints", |state| {
        printed_line(state, "ask", "alpha")
    });
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(has(&tree, Role::Button, "Stop"), "Stop on the row of a task that runs");
    runner.click(&mut state, &mut window, Role::Button, "Stop");
    runner.until(&mut state, "its end", |state| {
        !entry(state, "ask").unwrap().is_running()
    });
    assert_eq!(standing(&state, "ask"), Some(RowStatus::Stopped));
    assert!(printed(&state, "ask").contains("alpha"), "its output stays");
    let tree = runner.draw(&mut state, &mut window, &[]);
    let row = tree.nodes.iter().find(|(_, node)| {
        node.role() == Role::ListBoxOption && node.label().is_some_and(|label| label.starts_with("ask, "))
    });
    let label = row.and_then(|(_, node)| node.label()).unwrap_or_default();
    assert!(label.ends_with(", stopped"), "{label}");
    let terminal = find(&tree, Role::TextInput, "Terminal of the task");
    assert!(terminal.value().is_some_and(|text| text.contains("alpha")));
    assert!(says(&tree, "The task was stopped"));
    // A task that does not run is not stopped again: it is said.
    runner.ask(&mut state, TasksIntent::Stop(TaskReference::workspace("ask")));
    runner.until(&mut state, "the refusal", |state| state.operations.said.is_some());
    assert_eq!(state.operations.said.clone().unwrap().text, "`ask` is not running.");
}

#[test]
fn tasks_stop__closing_the_window_with_a_task_running_asks_first_and_cancel_leaves_it_running() {
    let (_repository, mut state) = with_tasks(&[waiting_task("ask", "alpha")]);
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.open(&mut state);
    runner.run_task(&mut state, "ask");
    runner.until(&mut state, "what it prints", |state| {
        printed_line(state, "ask", "alpha")
    });
    runner.message(&mut state, Message::CloseRequested(WindowPlace::default()));
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(says(&tree, "1 task is running."), "the question");
    assert!(has(&tree, Role::Button, "Stop them and close"));
    runner.click(&mut state, &mut window, Role::Button, "Cancel");
    runner.for_a_while(&mut state, Duration::from_millis(300));
    assert!(entry(&state, "ask").unwrap().is_running(), "Cancel leaves it running");
    assert!(!runner.log.borrow().contains(&String::from("quit")));
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(!says(&tree, "1 task is running."), "the question is put away");
    // Asked again and answered, the task is stopped as Stop stops it, and the window closes once it ended.
    runner.message(&mut state, Message::CloseRequested(WindowPlace::default()));
    let _ = runner.draw(&mut state, &mut window, &[]);
    runner.click(&mut state, &mut window, Role::Button, "Stop them and close");
    runner.until(&mut state, "the window closed", |_| {
        runner.log.borrow().contains(&String::from("quit"))
    });
    let log = runner.log.borrow().clone();
    let place = |said: &str| {
        log.iter()
            .position(|line| line == said)
            .unwrap_or_else(|| panic!("{said}: {log:?}"))
    };
    assert!(
        place("stop 0") < place("ended 0") && place("ended 0") < place("quit"),
        "{log:?}"
    );
}

#[test]
fn tasks_read__adding_a_task_to_tasks_json_lists_it_in_the_tab_without_refresh() {
    let build = shell_task("build", ("echo building", "echo building"), "");
    let (repository, mut state) = with_tasks(std::slice::from_ref(&build));
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.open(&mut state);
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(!has(&tree, Role::ListBoxOption, "test, echo testing"));
    write_tasks(
        &repository,
        &[build, shell_task("test", ("echo testing", "echo testing"), "")],
    );
    // What the watcher of the repository tells of the file written.
    let changes = vec![Change::WorkingTree(".vscode/tasks.json".into())];
    let at = Duration::from_secs(10);
    runner.message(&mut state, Message::Changed { changes, at });
    runner.until(&mut state, "the tasks read again", |state| {
        standing(state, "test").is_some()
    });
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(
        has(&tree, Role::ListBoxOption, "test, echo testing"),
        "listed without Refresh"
    );
}
