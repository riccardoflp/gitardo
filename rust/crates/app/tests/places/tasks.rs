//! The Tasks of the Activity drawer: the commands of a new worktree run one after the other once it is made, what they
//! print shown as it comes, what they ask answered from the drawer, and what the operation does after them waiting for
//! the last one (`worktree-setup.command-steps`).

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use app::store::feature::Ended;
use app::store::tasks::{TaskEffect, TaskMessage, Waiting};
use app::store::{Effect, Message, State, ViewData};
use model::contract::{Answer, Answers, FeatureId, FollowUp, Step, Target, Text};
use model::tasks::{TaskEnd, TaskStatus};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::Intent;
use views::activity::TaskId;
use views::feature::FeatureIntent;

use super::showing;
use super::support::{NOW, OperationsDriver, Window, find, has, run, says_in, typed};

/// The size of the window.
const WIDE: (u32, u32) = (1000, 640);

/// The threads of the tasks of a window, run for a test: what the store asks of them is given to them, and what they
/// say comes back to the test, which reduces it.
struct TasksDriver {
    runners: RefCell<app::work::TaskRunners>,
    said: Receiver<Message>,
}

impl TasksDriver {
    fn new() -> TasksDriver {
        let (sender, said) = std::sync::mpsc::channel();
        let sender = std::sync::Mutex::new(sender);
        let deliver: app::work::Deliver = std::sync::Arc::new(move |message| {
            let _sent = sender.lock().unwrap().send(message);
        });
        TasksDriver {
            runners: RefCell::new(app::work::TaskRunners::new(&engine::Activities::new(), &deliver)),
            said,
        }
    }

    /// Gives the threads what the store asked of them among some effects.
    fn take(&self, effects: &[Effect]) {
        for effect in effects {
            let Effect::Tasks(asked) = effect else {
                continue;
            };
            let mut runners = self.runners.borrow_mut();
            match asked.clone() {
                TaskEffect::Run { batch, steps } => runners.run(batch, steps, engine::LogLevel::Info),
                TaskEffect::Answer { batch, line } => runners.answer(batch, line),
                TaskEffect::Stop(batch) => runners.stop(batch),
            }
        }
    }

    /// Reduces what the tasks say until `until` holds for a message, and gives the effects of that message.
    fn until(&self, state: &mut State, until: impl Fn(&State, &TaskMessage) -> bool) -> Vec<Effect> {
        loop {
            let message = self
                .said
                .recv_timeout(Duration::from_secs(60))
                .unwrap_or_else(|_| panic!("the tasks say nothing within 60 s"));
            let Message::Task(told) = &message else {
                panic!("{message:?}")
            };
            let told = told.clone();
            let effects = run(state, message);
            self.take(&effects);
            if until(state, &told) {
                return effects;
            }
        }
    }

    /// Reduces what the tasks say until a run of them ended, and gives the effects of its end.
    fn until_ended(&self, state: &mut State) -> Vec<Effect> {
        self.until(state, |_, told| matches!(told, TaskMessage::Ended { .. }))
    }
}

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

/// Runs an operation from the window until it ended, giving the tasks what it hands over, and gives the effects of
/// its end.
fn run_operation(
    state: &mut State,
    (feature, target, answers): (&'static str, Target, Answers),
    tasks: &TasksDriver,
) -> Vec<Effect> {
    let driver = OperationsDriver::new();
    driver.intent(
        state,
        Intent::Feature(FeatureIntent::RunWith(FeatureId(feature), target, answers)),
    );
    loop {
        let message = driver.raw_next();
        let is_ended = matches!(message, Message::FeatureEnded { .. });
        let effects = run(state, message);
        driver.take(state, &effects);
        tasks.take(&effects);
        if is_ended {
            return effects;
        }
    }
}

/// Creates the worktree `fix` from `main`.
fn create_fix(state: &mut State, tasks: &TasksDriver) -> Vec<Effect> {
    let answers = Answers::new()
        .with(features::worktree::START, Answer::Choice(String::from("main")))
        .with(features::worktree::NAME, Answer::Text(String::from("fix")));
    run_operation(state, ("worktree.create", Target::Head, answers), tasks)
}

/// The lines of the Command log.
fn logged(state: &State) -> Vec<String> {
    state.log.entries().map(|entry| entry.line.clone()).collect()
}

/// The statuses of the tasks of the first run.
fn statuses(state: &State) -> Vec<TaskStatus> {
    state.tasks.rows().into_iter().map(|row| row.status).collect()
}

/// What a task printed, in one text.
fn printed(state: &State, index: usize) -> String {
    let task = state.tasks.task(TaskId { batch: 0, index }).unwrap();
    task.output.lines().join("\n")
}

/// Whether some effects open a window.
fn opens_a_window(effects: &[Effect]) -> bool {
    effects.iter().any(|effect| matches!(effect, Effect::OpenWindow(_)))
}

#[test]
fn worktree_setup_command_steps__tasks_in_the_drawer_shows_both_commands_one_after_the_other_in_the_worktree() {
    let (_repository, mut state) = with_commands(&["echo one", "echo two"]);
    let tasks = TasksDriver::new();
    let _ended = create_fix(&mut state, &tasks);
    assert!(state.is_activity_open, "the drawer opens on the tasks");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let _second = find(&tree, Role::Button, "echo two, fix \u{b7} waits");
    let _done = tasks.until_ended(&mut state);
    let log = logged(&state);
    let ran: Vec<&String> = log.iter().filter(|line| line.starts_with("run ")).collect();
    assert_eq!(ran, ["run echo one", "run echo two"], "{log:?}");
    assert_eq!(statuses(&state), [TaskStatus::Succeeded, TaskStatus::Succeeded]);
    assert_eq!((printed(&state, 0).trim(), printed(&state, 1).trim()), ("one", "two"));
    let (_, tree) = window.draw(&state, &[]);
    let _first = find(&tree, Role::Button, "echo one, fix");
    let _second = find(&tree, Role::Button, "echo two, fix");
    assert!(says_in(&tree, "two"), "the drawer shows what the last one printed");
}

#[test]
fn worktree_setup_command_steps__when_the_first_exits_with_1_the_next_does_not_run_and_the_warning_offers_to_open() {
    let (_repository, mut state) = with_commands(&["exit 1", "echo two"]);
    let tasks = TasksDriver::new();
    let ended = create_fix(&mut state, &tasks);
    assert!(!opens_a_window(&ended));
    let _done = tasks.until_ended(&mut state);
    assert_eq!(
        statuses(&state),
        [TaskStatus::Failed(TaskEnd::Code(1)), TaskStatus::DidNotRun]
    );
    assert!(!logged(&state).contains(&String::from("run echo two")));
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
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::Button, "Open in new window"));
    let _row = find(&tree, Role::Button, "echo two, fix \u{b7} did not run");
}

/// A command that asks a name, reads a line, and says hello to it: a shell alias of git, which git runs with its own
/// shell on every system.
fn asking_command() -> String {
    let alias = "alias.ask=!printf 'Name? '; read name; echo Hello $name";
    if cfg!(windows) {
        format!("git -c \"{alias}\" ask")
    } else {
        // The alias holds single quotes, so `sh` is given it in double quotes, with its `$` kept from `sh`.
        format!("git -c \"{}\" ask", alias.replace('$', "\\$"))
    }
}

#[test]
fn worktree_setup_command_steps__a_command_that_waits_for_input_is_answered_in_the_drawer() {
    let command = asking_command();
    let (_repository, mut state) = with_commands(&[&command]);
    let tasks = TasksDriver::new();
    let _ended = create_fix(&mut state, &tasks);
    let _asked = tasks.until(&mut state, |state, _| printed(state, 0).contains("Name?"));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let placeholder = format!("Answer {command}, then press Enter");
    for intent in window.click(&state, Role::TextInput, &placeholder) {
        let _effects = run(&mut state, Message::Intent(intent));
    }
    let enter = InputEvent::KeyPressed(KeyEvent::press(Key::Named(NamedKey::Enter), Modifiers::NONE));
    for events in [vec![typed("Ada")], vec![enter]] {
        let (intents, _) = window.draw(&state, &events);
        for intent in intents {
            let effects = run(&mut state, Message::Intent(intent));
            tasks.take(&effects);
        }
    }
    let _done = tasks.until_ended(&mut state);
    assert_eq!(statuses(&state), [TaskStatus::Succeeded]);
    assert!(printed(&state, 0).contains("Hello Ada"), "{}", printed(&state, 0));
}

#[test]
fn worktree_setup_command_steps__open_in_worktree_opens_it_only_once_both_commands_exit_with_0() {
    let (_repository, mut state) = with_commands(&["echo one", "echo two"]);
    let tasks = TasksDriver::new();
    let target = Target::Branch(String::from("feature/login"));
    let ended = run_operation(
        &mut state,
        ("worktree.open-in-worktree", target, Answers::new()),
        &tasks,
    );
    assert!(
        !opens_a_window(&ended),
        "the worktree does not open before its commands ran"
    );
    let done = tasks.until_ended(&mut state);
    let opened: Vec<&PathBuf> = done
        .iter()
        .filter_map(|effect| match effect {
            Effect::OpenWindow(folder) => Some(folder),
            _ => None,
        })
        .collect();
    assert_eq!(opened.len(), 1, "{done:?}");
    assert!(opened.first().is_some_and(|folder| folder.ends_with("feature/login")));
    let (_repository, mut state) = with_commands(&["exit 1", "echo two"]);
    let target = Target::Branch(String::from("feature/login"));
    let _ended = run_operation(
        &mut state,
        ("worktree.open-in-worktree", target, Answers::new()),
        &tasks,
    );
    assert!(!opens_a_window(&tasks.until_ended(&mut state)));
}

#[test]
fn worktree_setup_command_steps__the_tasks_are_drawn_beside_what_runs() {
    let (repository, mut state) = with_commands(&[]);
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
    let _held = run(&mut state, Message::FeatureEnded { ended, log: Vec::new() });
    let first = TaskId { batch: 0, index: 0 };
    let printed = "\u{1b}[32madded 182 packages\u{1b}[0m in 3s\r\n\r\n24 packages are looking for funding\n";
    for told in [
        TaskMessage::Started(first),
        TaskMessage::Printed(first, printed.to_owned()),
    ] {
        let _ = run(&mut state, Message::Task(told));
    }
    let data = ViewData::new(&state, Vec::new(), NOW);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = window.frame(&[], |ui| views::view(&data.window_state(&state), ui));
    window.assert_snapshot("activity-tasks-dark");
}
