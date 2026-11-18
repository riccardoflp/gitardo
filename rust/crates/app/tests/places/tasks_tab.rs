//! The Tasks tab of the bottom panel (`tasks.view`), each test running the tasks of a throwaway repository with the
//! shell of the system (PowerShell on Windows, the shell of `SHELL` elsewhere); and what the tests of `tasks.run` and
//! `tasks.stop` drive the tab with.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use app::store::tasks_tab::{Entry, TabTasks, TaskRunEffect, TaskRunMessage, task_list};
use app::store::{Effect, Message, State};
use app::work::Deliver;
use model::tasks::TaskReference;
use model::terminal::Screen;
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use ui::InputEvent;
use views::Intent;
use views::feature::FeatureIntent;
use views::panel::{PanelIntent, PanelTab};
use views::tasks::{Row, RowStatus, TasksIntent};

use super::pointer::{middle, rect};
use super::showing;
use super::support::{Window, find, has, run, says, typed};

/// How long a task has to do what a test waits for: a shell of the system starts slowly on a busy machine.
pub(super) const PATIENCE: Duration = Duration::from_secs(60);

/// The size of the window.
pub(super) const WIDE: (u32, u32) = (1100, 700);

/// The tasks and the operations of a window, run for a test: what the store asks of them is given to them, and what
/// they tell comes back to the test, which reduces it.
pub(super) struct Runner {
    /// The thread of the operations.
    operations: app::work::Operations,
    /// The tasks.
    host: RefCell<app::work::TaskRuns>,
    /// What they tell.
    told: Receiver<Message>,
    /// What the store asked of the tasks and what they told, in order and in a few words: `start 0`, `ended 0`.
    pub log: RefCell<Vec<String>>,
    /// Whether no task is started: the test tells the store what a task would.
    pub pretend: Cell<bool>,
    /// The titles of the dialogs of the system the store asked for and no test answered yet.
    files_asked: RefCell<Vec<String>>,
}

impl Runner {
    /// No task yet.
    pub fn new() -> Runner {
        let (sender, told) = mpsc::channel();
        let sender = Mutex::new(sender);
        let deliver: Deliver = Arc::new(move |message| {
            let _sent = sender.lock().unwrap().send(message);
        });
        Runner {
            operations: app::work::Operations::new(&engine::Activities::new(), &deliver),
            host: RefCell::new(app::work::TaskRuns::new(&deliver)),
            told,
            log: RefCell::new(Vec::new()),
            pretend: Cell::new(false),
            files_asked: RefCell::new(Vec::new()),
        }
    }

    /// Gives the tasks and the operations what the store asked of them among some effects.
    pub fn take(&self, state: &State, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::RunFeature {
                    folder,
                    feature,
                    preset,
                    context,
                } => self
                    .operations
                    .run(folder, (feature, preset), *context, state.settings.log_level),
                Effect::ReplyToFeature(reply) => self.operations.reply(reply),
                Effect::TaskRuns(asked) => {
                    let said = match &asked {
                        TaskRunEffect::Start { id, .. } => Some(format!("start {id}")),
                        TaskRunEffect::Stop(id) => Some(format!("stop {id}")),
                        TaskRunEffect::Kill(id) => Some(format!("kill {id}")),
                        _ => None,
                    };
                    self.log.borrow_mut().extend(said);
                    if !self.pretend.get() || matches!(asked, TaskRunEffect::Read { .. }) {
                        self.host.borrow_mut().take(asked);
                    }
                }
                Effect::ChooseFile { title, .. } => self.files_asked.borrow_mut().push(title),
                Effect::Quit => self.log.borrow_mut().push(String::from("quit")),
                _ => {}
            }
        }
    }

    /// Reduces a message with the jobs it starts, giving the tasks and the operations what it asks.
    pub fn message(&self, state: &mut State, message: Message) {
        let effects = run(state, message);
        self.take(state, effects);
    }

    /// Reduces intents in turn.
    pub fn intents(&self, state: &mut State, intents: Vec<Intent>) {
        for intent in intents {
            self.message(state, Message::Intent(intent));
        }
    }

    /// Asks something of the Tasks tab.
    pub fn ask(&self, state: &mut State, intent: TasksIntent) {
        self.intents(state, vec![Intent::TasksTab(intent)]);
    }

    /// Reduces what the tasks and the operations tell until the store is as `wanted` says.
    pub fn until(&self, state: &mut State, what: &str, wanted: impl Fn(&State) -> bool) {
        let started = Instant::now();
        while !wanted(state) {
            assert!(
                started.elapsed() < PATIENCE,
                "{what} did not come within 60 s; {:?}\n{:#?}",
                self.log.borrow(),
                tab(state).map(|tab| &tab.entries)
            );
            self.next(state, Duration::from_millis(100));
        }
    }

    /// Reduces what the tasks and the operations tell for a while, whatever it is.
    pub fn for_a_while(&self, state: &mut State, long: Duration) {
        let started = Instant::now();
        while started.elapsed() < long {
            self.next(state, Duration::from_millis(50));
        }
    }

    /// Reduces the next thing told, if something is told in time.
    fn next(&self, state: &mut State, wait: Duration) {
        let Ok(message) = self.told.recv_timeout(wait) else {
            return;
        };
        let message = self.host.borrow_mut().read(message);
        if let Message::TaskRun(told) = &message {
            let said = match told {
                TaskRunMessage::Started(id) => Some(format!("started {id}")),
                TaskRunMessage::Ready(id) | TaskRunMessage::Screen(id, _, true) => Some(format!("ready {id}")),
                TaskRunMessage::Ended(id, ..) => Some(format!("ended {id}")),
                _ => None,
            };
            self.log.borrow_mut().extend(said);
        }
        self.message(state, message);
    }

    /// Draws the window with some input, and reduces what it asks.
    pub fn draw(&self, state: &mut State, window: &mut Window, events: &[InputEvent]) -> TreeUpdate {
        let (intents, tree) = window.draw(state, events);
        self.intents(state, intents);
        tree
    }

    /// Clicks the widget of a role and a name, and reduces what it asks.
    pub fn click(&self, state: &mut State, window: &mut Window, role: Role, name: &str) {
        let intents = window.click(state, role, name);
        self.intents(state, intents);
    }

    /// The bottom panel shown at the Tasks tab, the tasks of the repository shown read.
    pub fn open(&self, state: &mut State) {
        self.intents(
            state,
            vec![
                Intent::TasksTab(TasksIntent::Terminal(views::terminal::TerminalIntent::Resize(100, 20))),
                Intent::Panel(PanelIntent::Show(PanelTab::Tasks)),
            ],
        );
        self.until(state, "the tasks read", |state| {
            tab(state).is_some_and(|tab| tab.read.is_some() && tab.reading.is_none())
        });
    }

    /// Runs a task of `.vscode/tasks.json` by its label, and waits until its terminal started or it ended.
    pub fn run_task(&self, state: &mut State, label: &str) {
        self.ask(state, TasksIntent::Run(TaskReference::workspace(label)));
        self.until(state, &format!("the start of {label}"), |state| {
            entry(state, label).is_some()
        });
    }

    /// Waits until the operation asked for ended and said what it had to say.
    pub fn ended(&self, state: &mut State) {
        self.until(state, "the end of the operation", |state| {
            state.operations.running.is_none()
        });
    }

    /// Answers the question the operation running asks.
    pub fn answer(&self, state: &mut State, answer: model::contract::Answer) {
        self.until(state, "a question", |state| {
            let running = state.operations.running.as_ref();
            running.is_some_and(|running| running.asking.is_some())
        });
        self.intents(state, vec![Intent::Feature(FeatureIntent::Answer(Some(answer)))]);
    }

    /// Answers the dialog of the system of a title, which the operation running asked for, with a file.
    pub fn choose_file(&self, state: &mut State, title: &str, path: &std::path::Path) {
        self.until(state, "the dialog of the system", |_| {
            !self.files_asked.borrow().is_empty()
        });
        assert_eq!(self.files_asked.borrow_mut().pop().as_deref(), Some(title));
        let answer = model::contract::Answer::Path(path.to_owned());
        self.intents(state, vec![Intent::Feature(FeatureIntent::Answer(Some(answer)))]);
    }

    /// Answers the question the operation running asks with one of its choices.
    pub fn choose(&self, state: &mut State, choice: &str) {
        self.answer(state, model::contract::Answer::Choice(choice.to_owned()));
    }

    /// Drags from a point to another and lets go there, and reduces what the frames ask.
    pub fn drag(&self, state: &mut State, window: &mut Window, from: render::Point, to: render::Point) {
        let intents = super::pointer::drag(window, state, from, to);
        self.intents(state, intents);
    }
}

/// The tasks of the repository tab of a window with one.
pub(super) fn tab(state: &State) -> Option<&TabTasks> {
    state.tasks_tab.tabs.values().next()
}

/// The run of a task of `.vscode/tasks.json`, by its label.
pub(super) fn entry<'a>(state: &'a State, label: &str) -> Option<&'a Entry> {
    tab(state)?.entry(&TaskReference::workspace(label))
}

/// What the terminal of a task shows, as text.
pub(super) fn printed(state: &State, label: &str) -> String {
    let screen = entry(state, label).and_then(|entry| entry.screen.as_ref());
    screen.map(Screen::text).unwrap_or_default()
}

/// Whether the terminal of a task has a line that is some words.
pub(super) fn printed_line(state: &State, label: &str, line: &str) -> bool {
    printed(state, label).lines().any(|shown| shown.trim() == line)
}

/// How the row of a task of `.vscode/tasks.json` says it stands.
pub(super) fn standing(state: &State, label: &str) -> Option<RowStatus> {
    task_list(state).rows.into_iter().find_map(|row| match row {
        Row::Task(task) if task.reference == TaskReference::workspace(label) => Some(task.status),
        _ => None,
    })
}

/// The middle of the row of a task in the column, by the name of the task.
pub(super) fn row_of(tree: &TreeUpdate, name: &str) -> render::Point {
    let start = format!("{name}, ");
    let mut rows = tree.nodes.iter().filter(|(_, node)| node.role() == Role::ListBoxOption);
    let row = rows.find(|(_, node)| {
        node.label()
            .is_some_and(|label| label == name || label.starts_with(&start))
    });
    let (_, node) = row.unwrap_or_else(|| panic!("no row of the task {name}"));
    middle(rect(node.bounds().unwrap()))
}

/// The names of the tasks the column lists.
fn listed(state: &State) -> Vec<String> {
    let rows = task_list(state).rows.into_iter();
    rows.filter_map(|row| match row {
        Row::Task(task) => Some(task.name),
        Row::Heading(_) => None,
    })
    .collect()
}

/// A string as JSON writes it.
pub(super) fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// A shell task of `tasks.json`, with the command line PowerShell runs on Windows and the one a shell runs elsewhere,
/// and more members of the task, as JSON, when it has some.
pub(super) fn shell_task(label: &str, (windows, elsewhere): (&str, &str), more: &str) -> String {
    let command = if cfg!(windows) { windows } else { elsewhere };
    let more = if more.is_empty() {
        String::new()
    } else {
        format!(", {more}")
    };
    format!(
        "    {{ \"label\": {}, \"type\": \"shell\", \"command\": {}{more} }}",
        quoted(label),
        quoted(command)
    )
}

/// A task that prints a word, waits for a line typed in its terminal, then says the line it got.
pub(super) fn waiting_task(label: &str, word: &str) -> String {
    shell_task(
        label,
        (
            &format!("echo {word}; $line = Read-Host; echo \"got $line\""),
            &format!("echo {word}; read line; echo \"got $line\""),
        ),
        "",
    )
}

/// A repository with a commit and a `.vscode/tasks.json` of some tasks, shown in a window that trusts its folder.
pub(super) fn with_tasks(tasks: &[String]) -> (Repository, State) {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    write_tasks(&repository, tasks);
    let mut state = showing(repository.path().to_path_buf());
    let folder: PathBuf = state.shown_folder().cloned().unwrap();
    state.persisted.trusted_folders.push(folder);
    (repository, state)
}

/// Writes the `.vscode/tasks.json` of a repository, with a comment as VS Code allows them.
pub(super) fn write_tasks(repository: &Repository, tasks: &[String]) {
    let folder = repository.path().join(".vscode");
    std::fs::create_dir_all(&folder).unwrap();
    let text = format!(
        "{{\n  // The tasks of the tests.\n  \"version\": \"2.0.0\",\n  \"tasks\": [\n{}\n  ]\n}}\n",
        tasks.join(",\n")
    );
    std::fs::write(folder.join("tasks.json"), text).unwrap();
}

/// Two tasks that print a word, and the scripts of a `package.json` in a folder.
fn listed_tasks() -> (Repository, State) {
    let build = shell_task("build", ("echo building", "echo building"), "");
    let watch = shell_task("watch", ("echo watching", "echo watching"), "");
    let (repository, state) = with_tasks(&[build, watch]);
    let folder = repository.path().join("site");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join("package.json"),
        "{ \"name\": \"site\", \"scripts\": { \"watch:css\": \"sass --watch\", \"test\": \"vitest\" } }\n",
    )
    .unwrap();
    (repository, state)
}

#[test]
fn tasks_view__tasks_in_the_tabs_of_the_panel_shows_the_list_and_the_command_of_the_task_selected() {
    let (_repository, mut state) = listed_tasks();
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.intents(&mut state, vec![Intent::Panel(PanelIntent::Open)]);
    runner.click(&mut state, &mut window, Role::Button, "Tasks");
    assert_eq!(app::store::panel::look(&state).tab, PanelTab::Tasks);
    runner.until(&mut state, "the tasks read", |state| {
        tab(state).is_some_and(|tab| tab.read.is_some())
    });
    assert_eq!(listed(&state), ["build", "watch", "watch:css", "test"]);
    let tree = runner.draw(&mut state, &mut window, &[]);
    for heading in ["Groups", "Workspace", "npm", "site"] {
        assert!(has(&tree, Role::Button, heading), "the heading {heading}");
    }
    assert!(has(&tree, Role::ListBoxOption, "watch, echo watching"));
    assert!(says(&tree, "Select a task to see its terminal"));
    runner.click(&mut state, &mut window, Role::ListBoxOption, "build, echo building");
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(says(&tree, "echo building"), "the command of the task selected");
    assert!(says(&tree, "Folder: ."));
    assert!(!has(&tree, Role::TextInput, "Terminal of the task"), "nothing ran");
    assert!(runner.log.borrow().is_empty(), "{:?}", runner.log.borrow());
}

#[test]
fn tasks_view__watch_typed_in_the_search_leaves_only_the_tasks_whose_name_or_source_holds_it() {
    let (_repository, mut state) = listed_tasks();
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.open(&mut state);
    runner.click(&mut state, &mut window, Role::SearchInput, "Search tasks");
    let _ = runner.draw(&mut state, &mut window, &[typed("watch")]);
    assert_eq!(listed(&state), ["watch", "watch:css"]);
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(!has(&tree, Role::ListBoxOption, "build, echo building"));
    assert!(has(&tree, Role::Button, "site"), "the heading of what is left stays");
    // Every word must be there: a source counts as a name does.
    runner.ask(&mut state, TasksIntent::Search(String::from("npm test")));
    assert_eq!(listed(&state), ["test"]);
    runner.ask(&mut state, TasksIntent::Search(String::from("workspace")));
    assert_eq!(listed(&state), ["build", "watch"]);
}

#[test]
fn tasks_view__running_a_task_shows_its_terminal_on_the_right_and_a_count_of_1_beside_tasks() {
    let (_repository, mut state) = with_tasks(&[waiting_task("ask", "alpha")]);
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.open(&mut state);
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(has(&tree, Role::Button, "Tasks"), "no count while nothing runs");
    let intents = window.click_at(&state, row_of(&tree, "ask"), 2);
    assert_eq!(
        intents,
        [Intent::TasksTab(TasksIntent::Run(TaskReference::workspace("ask")))]
    );
    runner.intents(&mut state, intents);
    runner.until(&mut state, "its start", |state| entry(state, "ask").is_some());
    runner.until(&mut state, "what it prints", |state| {
        printed_line(state, "ask", "alpha")
    });
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(
        has(&tree, Role::Button, "Tasks (1)"),
        "the count beside the name of the tab"
    );
    let terminal = find(&tree, Role::TextInput, "Terminal of the task");
    assert!(terminal.value().is_some_and(|text| text.contains("alpha")));
    assert_eq!(standing(&state, "ask"), Some(RowStatus::Running { is_ready: false }));
    runner.ask(&mut state, TasksIntent::Stop(TaskReference::workspace("ask")));
    runner.until(&mut state, "its end", |state| {
        !entry(state, "ask").unwrap().is_running()
    });
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(has(&tree, Role::Button, "Tasks"), "no count once it ended");
}
