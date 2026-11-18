//! The commands of a new worktree in the Tasks tab (`worktree-setup.command-steps`): tasks of the source **New
//! worktrees**, run one after the other once the worktree is made, each in a terminal of its own where what it asks is
//! answered, with what the operation does after them waiting for the last one. The tests but the last run real
//! commands, with the shell the tasks run in.

use std::time::Duration;

use app::store::feature::Ended;
use app::store::tasks_tab::{Entry, TaskRunMessage, Waiting, task_list};
use app::store::{Message, State};
use model::contract::{Answer, Answers, FeatureId, FollowUp, Step, Target, Text};
use model::tasks::TaskReference;
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::Intent;
use views::feature::FeatureIntent;
use views::panel::PanelTab;
use views::tasks::{Row, RowStatus, TasksIntent};

use super::showing;
use super::support::{Window, find, has, says, typed};
use super::tasks_tab::{Runner, WIDE, shell_task, tab, write_tasks};

/// A repository with a commit, a branch `feature/login`, and commands for new worktrees in its settings, shown in a
/// window of a trusted folder.
fn with_commands(commands: &[&str]) -> (Repository, State) {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    repository.branch("feature/login");
    let listed: Vec<String> = commands.iter().map(|command| format!("{command:?}")).collect();
    let settings = format!("{{ \"worktrees.setupCommands\": [{}] }}", listed.join(", "));
    let folder = repository.path().join(".gitardo");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("settings.json"), settings).unwrap();
    let mut state = showing(repository.path().to_path_buf());
    state.persisted.trusted_folders.push(repository.path().to_path_buf());
    (repository, state)
}

/// Runs an operation from the window until it ended; its commands, handed to the Tasks tab, have not ended yet.
fn run_operation(runner: &Runner, state: &mut State, (feature, target, answers): (&'static str, Target, Answers)) {
    let asked = FeatureIntent::RunWith(FeatureId(feature), target, answers);
    runner.intents(state, vec![Intent::Feature(asked)]);
    runner.ended(state);
}

/// Creates the worktree `fix` from `main`.
fn create_fix(runner: &Runner, state: &mut State) {
    let answers = Answers::new()
        .with(features::worktree::START, Answer::Choice(String::from("main")))
        .with(features::worktree::NAME, Answer::Text(String::from("fix")));
    run_operation(runner, state, ("worktree.create", Target::Head, answers));
}

/// Waits until the commands of every new worktree are over, and what they wrote in the Command log came.
fn until_over(runner: &Runner, state: &mut State) {
    runner.until(state, "the end of the commands", |state| {
        tab(state)
            .is_some_and(|tab| !tab.commands.is_empty() && tab.commands.iter().all(|commands| !commands.is_running()))
    });
    runner.for_a_while(state, Duration::from_millis(200));
}

/// The lines of the Command log.
fn logged(state: &State) -> Vec<String> {
    state.log.entries().map(|entry| entry.line.clone()).collect()
}

/// The run of a command of the worktree `fix`, by the name its row has.
fn entry<'a>(state: &'a State, name: &str) -> Option<&'a Entry> {
    tab(state)?.entry(&TaskReference::new_worktree("fix", name))
}

/// What the terminal of a command of the worktree `fix` shows, as text.
fn printed(state: &State, name: &str) -> String {
    let screen = entry(state, name).and_then(|entry| entry.screen.as_ref());
    screen.map(model::terminal::Screen::text).unwrap_or_default()
}

/// The rows of the commands of new worktrees, each with how it stands.
fn standing(state: &State) -> Vec<(String, RowStatus)> {
    let rows = task_list(state).rows.into_iter();
    rows.filter_map(|row| match row {
        Row::Task(task) if task.reference.is_of_new_worktree() => Some((task.name, task.status)),
        _ => None,
    })
    .collect()
}

#[test]
fn worktree_setup_command_steps__the_tasks_tab_lists_both_commands_under_new_worktrees_and_runs_one_after_the_other() {
    let (_repository, mut state) = with_commands(&["echo one", "echo two"]);
    let runner = Runner::new();
    create_fix(&runner, &mut state);
    assert!(
        state.panel.look.is_shown && state.panel.look.tab == PanelTab::Tasks,
        "the bottom panel is shown at Tasks on the commands"
    );
    let first = TaskReference::new_worktree("fix", "echo one");
    assert_eq!(tab(&state).unwrap().selected, Some(first), "the first is selected");
    until_over(&runner, &mut state);
    let log = logged(&state);
    let ran: Vec<&String> = log.iter().filter(|line| line.starts_with("run ")).collect();
    assert_eq!(ran, ["run echo one", "run echo two"], "{log:?}");
    let succeeded = |name: &str| (name.to_owned(), RowStatus::Succeeded);
    assert_eq!(standing(&state), [succeeded("echo one"), succeeded("echo two")]);
    assert!(printed(&state, "echo one").lines().any(|line| line.trim() == "one"));
    assert!(printed(&state, "echo two").lines().any(|line| line.trim() == "two"));
    let (one, two) = (entry(&state, "echo one").unwrap(), entry(&state, "echo two").unwrap());
    for entry in [one, two] {
        let folder = &entry.task.folder;
        assert!(
            folder.ends_with("fix") && folder.join("README.md").exists(),
            "{folder:?}"
        );
    }
    let told = runner.log.borrow().clone();
    let place = |line: String| told.iter().position(|said| *said == line).unwrap();
    assert!(
        place(format!("ended {}", one.id)) < place(format!("start {}", two.id)),
        "the second starts once the first ended: {told:?}"
    );
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = runner.draw(&mut state, &mut window, &[]);
    let tree = runner.draw(&mut state, &mut window, &[]);
    for heading in ["New worktrees", "fix"] {
        assert!(has(&tree, Role::Button, heading), "the heading {heading}");
    }
    for row in ["echo one", "echo two"] {
        assert!(has(&tree, Role::ListBoxOption, row), "the row of {row}");
    }
    let shown = find(&tree, Role::TextInput, "Terminal of the task")
        .value()
        .unwrap_or_default();
    assert!(shown.contains("one"), "the terminal of the command selected: {shown}");
    assert!(!has(&tree, Role::Button, "Run"), "a command is not run again");
}

#[test]
fn worktree_setup_command_steps__when_the_first_exits_with_1_the_next_does_not_run_and_the_warning_offers_to_open() {
    let (_repository, mut state) = with_commands(&["exit 1", "echo two"]);
    let runner = Runner::new();
    create_fix(&runner, &mut state);
    until_over(&runner, &mut state);
    assert_eq!(
        standing(&state),
        [
            (String::from("exit 1"), RowStatus::Failed(1)),
            (String::from("echo two"), RowStatus::DidNotRun)
        ]
    );
    assert!(entry(&state, "echo two").is_none(), "no terminal was started for it");
    let log = logged(&state);
    assert!(!log.contains(&String::from("run echo two")), "{log:?}");
    assert!(log.contains(&String::from("It exited with 1.")), "{log:?}");
    assert!(runner.opened.borrow().is_empty(), "the worktree is not opened");
    let said = state.operations.said.clone().unwrap();
    assert!(said.is_error);
    assert!(
        said.text.starts_with("`exit 1` failed with exit code 1 in `")
            && said.text.ends_with("fix`, so the command after it did not run."),
        "{}",
        said.text
    );
    let offered: Vec<&str> = said.runs.iter().map(|offer| offer.feature.0).collect();
    assert_eq!(offered.first(), Some(&"worktree.open-in-new-window"));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = runner.draw(&mut state, &mut window, &[]);
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(has(&tree, Role::Button, "Open in new window"));
    assert!(has(&tree, Role::ListBoxOption, "exit 1, 1"));
    assert!(has(&tree, Role::ListBoxOption, "echo two, did not run"));
    runner.click(&mut state, &mut window, Role::ListBoxOption, "echo two, did not run");
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(
        says(&tree, "It did not run: `exit 1` ended with exit code 1."),
        "the command selected says why it did not run"
    );
    assert!(!has(&tree, Role::Button, "Run"), "and is not run from there");
}

/// A command that asks a name, reads a line, and says hello to it.
fn asking_command() -> &'static str {
    if cfg!(windows) {
        "Write-Host 'Name?'; $name = Read-Host; Write-Host \"Hello $name\""
    } else {
        "echo 'Name?'; read name; echo \"Hello $name\""
    }
}

#[test]
fn worktree_setup_command_steps__a_command_that_waits_for_input_is_answered_in_its_terminal_in_the_tasks_tab() {
    let command = asking_command();
    let (_repository, mut state) = with_commands(&[command]);
    let runner = Runner::new();
    create_fix(&runner, &mut state);
    runner.until(&mut state, "what the command asks", |state| {
        printed(state, command).contains("Name?")
    });
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = runner.draw(&mut state, &mut window, &[]);
    runner.click(&mut state, &mut window, Role::TextInput, "Terminal of the task");
    let enter = InputEvent::KeyPressed(KeyEvent::press(Key::Named(NamedKey::Enter), Modifiers::NONE));
    for events in [vec![typed("Ada")], vec![enter]] {
        let _ = runner.draw(&mut state, &mut window, &events);
    }
    until_over(&runner, &mut state);
    let printed = printed(&state, command);
    assert!(printed.contains("Hello Ada"), "{printed}");
    assert_eq!(standing(&state), [(command.to_owned(), RowStatus::Succeeded)]);
}

#[test]
fn worktree_setup_command_steps__open_in_worktree_opens_it_only_once_both_commands_exit_with_0() {
    let open = |commands: &[&str]| {
        let (repository, mut state) = with_commands(commands);
        let runner = Runner::new();
        let target = Target::Branch(String::from("feature/login"));
        run_operation(
            &runner,
            &mut state,
            ("worktree.open-in-worktree", target, Answers::new()),
        );
        assert!(
            runner.opened.borrow().is_empty(),
            "the worktree does not open before its commands ran"
        );
        until_over(&runner, &mut state);
        let opened = runner.opened.borrow().clone();
        drop(repository);
        opened
    };
    let opened = open(&["echo one", "echo two"]);
    assert_eq!(opened.len(), 1, "{opened:?}");
    assert!(opened.first().is_some_and(|folder| folder.ends_with("feature/login")));
    assert_eq!(open(&["exit 1", "echo two"]), Vec::<std::path::PathBuf>::new());
}

/// A repository with two tasks whose worktree `fix` was just made by a plan that handed `npm ci` and `npm run build`
/// over, with the folder of the worktree, which the plan opens after them; no process is started.
fn with_a_new_worktree() -> (Repository, State, Runner, std::path::PathBuf) {
    let (repository, mut state) = with_commands(&[]);
    let plain = |label: &str, command: &str| shell_task(label, (command, command), "");
    write_tasks(
        &repository,
        &[
            plain("build", "cargo build --workspace"),
            plain("test", "cargo test --workspace"),
        ],
    );
    let runner = Runner::new();
    runner.pretend.set(true);
    runner.open(&mut state);
    let folder = repository.path().with_file_name("repository.worktrees").join("fix");
    let task = |command: &str| Step::Task {
        command: command.to_owned(),
        directory: folder.clone(),
        name: String::from("fix"),
    };
    let waiting = Waiting {
        steps: vec![task("npm ci"), task("npm run build")],
        on_failure: Vec::new(),
    };
    let ended = Ended::Ran {
        title: Text::from("Create worktree fix"),
        after: vec![FollowUp::OpenWindow(folder.clone())],
        can_undo: false,
        prepared_message: None,
        waiting: Some(Box::new(waiting)),
    };
    runner.message(&mut state, Message::FeatureEnded { ended, log: Vec::new() });
    (repository, state, runner, folder)
}

#[test]
fn worktree_setup_command_steps__the_commands_are_drawn_in_the_tasks_tab_in_each_theme() {
    let (_repository, mut state, runner, folder) = with_a_new_worktree();
    let id = entry(&state, "npm ci").expect("the first command starts").id;
    let lines = ["ok added 182 packages in 3s", "", "24 packages are looking for funding"];
    for told in [
        TaskRunMessage::Started(id),
        TaskRunMessage::Screen(id, Box::new(super::tasks_view::screen(&lines)), false),
    ] {
        runner.message(&mut state, Message::TaskRun(told));
    }
    assert_eq!(
        standing(&state),
        [
            (String::from("npm ci"), RowStatus::Running { is_ready: false }),
            (String::from("npm run build"), RowStatus::Waiting)
        ],
        "the second waits for the first"
    );
    assert!(runner.opened.borrow().is_empty());
    for (theme, name) in super::activity_tab::THEMES {
        let mut window = Window::new(theme, WIDE);
        let _ = runner.draw(&mut state, &mut window, &[]);
        let tree = runner.draw(&mut state, &mut window, &[]);
        assert!(has(&tree, Role::Button, "Tasks (1)"));
        assert!(has(&tree, Role::ListBoxOption, "npm run build, waits"));
        window.assert_snapshot(&format!("tasks-tab-new-worktree-{name}"));
    }
    // Once both ended with exit code 0, what the plan does after them is done.
    runner.message(
        &mut state,
        Message::TaskRun(TaskRunMessage::Ended(id, Some(0), Duration::from_secs(3))),
    );
    let id = entry(&state, "npm run build").expect("the second starts").id;
    for told in [
        TaskRunMessage::Started(id),
        TaskRunMessage::Ended(id, Some(0), Duration::from_secs(8)),
    ] {
        runner.message(&mut state, Message::TaskRun(told));
    }
    assert_eq!(runner.opened.borrow().as_slice(), [folder]);
}

#[test]
fn worktree_setup_command_steps__stop_stops_the_command_and_those_after_it_and_a_command_is_not_run_again() {
    let (_repository, mut state, runner, _folder) = with_a_new_worktree();
    let first = TaskReference::new_worktree("fix", "npm ci");
    let second = TaskReference::new_worktree("fix", "npm run build");
    let id = entry(&state, "npm ci").expect("the first command starts").id;
    runner.message(&mut state, Message::TaskRun(TaskRunMessage::Started(id)));
    runner.ask(&mut state, TasksIntent::Stop(first.clone()));
    runner.ended(&mut state);
    assert!(
        runner.log.borrow().contains(&format!("stop {id}")),
        "its terminal is asked to stop"
    );
    runner.message(
        &mut state,
        Message::TaskRun(TaskRunMessage::Ended(id, None, Duration::from_secs(2))),
    );
    assert_eq!(
        standing(&state),
        [
            (String::from("npm ci"), RowStatus::Stopped),
            (String::from("npm run build"), RowStatus::DidNotRun)
        ]
    );
    let said = state.operations.said.clone().unwrap();
    assert!(
        said.is_error
            && said.text.starts_with("`npm ci` did not finish in `")
            && said.text.ends_with("fix`, so the command after it did not run."),
        "{}",
        said.text
    );
    assert!(runner.opened.borrow().is_empty(), "the worktree is not opened");
    // **Run** asked of a command, as a double click on its row would, only selects it.
    let starts = |runner: &Runner| {
        let log = runner.log.borrow();
        log.iter().filter(|line| line.starts_with("start ")).count()
    };
    let before = starts(&runner);
    for command in [second, first] {
        runner.ask(&mut state, TasksIntent::Run(command.clone()));
        assert_eq!(tab(&state).unwrap().selected, Some(command));
    }
    assert!(state.operations.running.is_none() && starts(&runner) == before);
}
