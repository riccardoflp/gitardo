//! The terminals of the bottom panel (`terminal.view`), each driving the shell of the system (PowerShell on Windows,
//! the shell of `SHELL` elsewhere) in a throwaway repository; and the panel hidden while a command runs
//! (`panel.bottom`).

use std::cell::RefCell;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use app::store::terminals::{self, ShellStatus};
use app::store::{Effect, Message, State, panel};
use app::work::Deliver;
use model::terminal::{CellColor, Screen};
use platform::accesskit::{Node, Role, TreeUpdate};
use render::{Point, Rect, ThemeName};
use testkit::Repository;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::Intent;
use views::panel::PanelIntent;
use views::terminal::TerminalIntent;

use super::panel::without_sizes;
use super::support::{Window, find, run, says, shortcut};
use super::{repository_with_history, showing};

/// How long a shell has to do what a test waits for: a shell of the system starts slowly on a busy machine.
const PATIENCE: Duration = Duration::from_secs(60);

/// The size of the window.
const WIDE: (u32, u32) = (1000, 640);

/// The terminals of a window, run for a test: what the store asks of them is given to them, and what they tell comes
/// back to the test, which reduces it.
struct Shells {
    /// The terminals.
    host: RefCell<app::work::Terminals>,
    /// What they tell.
    told: Receiver<Message>,
}

impl Shells {
    /// No terminal yet.
    fn new() -> Shells {
        let (sender, told) = mpsc::channel();
        let sender = Mutex::new(sender);
        let deliver: Deliver = Arc::new(move |message| {
            let _sent = sender.lock().unwrap().send(message);
        });
        Shells {
            host: RefCell::new(app::work::Terminals::new(&deliver)),
            told,
        }
    }

    /// Gives the terminals what the store asked of them among some effects.
    fn take(&self, effects: Vec<Effect>) {
        for effect in effects {
            if let Effect::Terminal(asked) = effect {
                self.host.borrow_mut().take(asked);
            }
        }
    }

    /// Reduces intents in turn, giving the terminals what they ask.
    fn intents(&self, state: &mut State, intents: Vec<Intent>) {
        for intent in intents {
            let effects = run(state, Message::Intent(intent));
            self.take(effects);
        }
    }

    /// Types a line in the terminal shown, ended by `Enter`.
    fn type_line(&self, state: &mut State, line: &str) {
        let bytes = format!("{line}\r").into_bytes();
        self.intents(state, vec![Intent::Terminal(TerminalIntent::Input(bytes))]);
    }

    /// Reduces what the terminals tell until the store is as `wanted` says.
    fn until(&self, state: &mut State, what: &str, wanted: impl Fn(&State) -> bool) {
        let started = Instant::now();
        while !wanted(state) {
            assert!(
                started.elapsed() < PATIENCE,
                "{what} did not come within 60 s:\n{}",
                screen_text(state)
            );
            if let Ok(message) = self.told.recv_timeout(Duration::from_millis(100)) {
                let message = self.host.borrow_mut().read_screen(message);
                let effects = run(state, message);
                self.take(effects);
            }
        }
    }

    /// Draws the window with some input, and reduces what it asks.
    fn draw(&self, state: &mut State, window: &mut Window, events: &[InputEvent]) -> TreeUpdate {
        let (intents, tree) = window.draw(state, events);
        self.intents(state, intents);
        tree
    }

    /// The panel shown at a terminal of the repository shown, started at a size that holds any path of a temporary
    /// folder on one line, and its prompt there.
    fn open(&self, state: &mut State) {
        self.intents(
            state,
            vec![
                Intent::Terminal(TerminalIntent::Resize(160, 30)),
                Intent::Panel(PanelIntent::Open),
            ],
        );
        self.until(state, "the prompt", |state| !screen_text(state).trim().is_empty());
    }
}

/// What the terminal shown shows, as text.
fn screen_text(state: &State) -> String {
    terminals::view_state(state)
        .screen
        .map(Screen::text)
        .unwrap_or_default()
}

/// The terminal shown.
fn shown(state: &State) -> Option<&terminals::Entry> {
    let (_, group) = state.terminals.groups.iter().find(|(folder, _)| {
        state
            .tabs
            .shown
            .and_then(|index| state.tabs.open.get(index))
            .is_none_or(|tab| tab.folder == **folder)
    })?;
    group.entries.get(group.shown)
}

/// A command that prints a word followed by `ok`, which the command itself does not hold, so that a test sees what it
/// printed and not what was typed.
fn print_command(word: &str) -> String {
    if cfg!(windows) {
        format!("echo ('{word}' + 'ok')")
    } else {
        format!("echo {word}''ok")
    }
}

/// Whether the screen of the terminal shown has a line that is the word a [`print_command`] printed.
fn printed(state: &State, word: &str) -> bool {
    let line = format!("{word}ok");
    screen_text(state).lines().any(|shown| shown.trim() == line)
}

/// Prints a word in the terminal shown and waits until it is there.
fn print(shells: &Shells, state: &mut State, word: &str) {
    shells.type_line(state, &print_command(word));
    shells.until(state, word, |state| printed(state, word));
}

/// The rectangle of a node.
fn bounds(node: &Node) -> Rect {
    let bounds = node.bounds().unwrap();
    #[allow(clippy::cast_possible_truncation, reason = "the bounds of a window fit in an f32")]
    Rect::new(
        bounds.x0 as f32,
        bounds.y0 as f32,
        (bounds.x1 - bounds.x0) as f32,
        (bounds.y1 - bounds.y0) as f32,
    )
}

/// A key pressed alone.
fn key(named: NamedKey) -> InputEvent {
    InputEvent::KeyPressed(KeyEvent::press(Key::Named(named), Modifiers::NONE))
}

#[test]
fn terminal_view__the_first_terminal_of_a_repository_tab_starts_in_its_folder_and_pwd_prints_it() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let shells = Shells::new();
    shells.open(&mut state);
    shells.type_line(&mut state, "pwd");
    let folder = |path: &Path| path.file_name().unwrap().to_string_lossy().into_owned();
    let wanted = format!(
        "{}{}{}",
        folder(repository.path().parent().unwrap()),
        std::path::MAIN_SEPARATOR,
        folder(repository.path())
    );
    shells.until(&mut state, "the folder", |state| {
        screen_text(state)
            .lines()
            .any(|line| line.trim_end().ends_with(&wanted))
    });
}

#[test]
fn terminal_view__git_log_oneline_decorate_prints_its_labels_in_color() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let shells = Shells::new();
    shells.open(&mut state);
    shells.type_line(&mut state, "git log --oneline --decorate -1");
    let is_colored = |state: &State| {
        terminals::view_state(state).screen.is_some_and(|screen| {
            screen
                .rows
                .iter()
                .flatten()
                .any(|span| span.text.contains("HEAD") && !matches!(span.style.foreground, CellColor::Default))
        })
    };
    shells.until(&mut state, "the labels in color", is_colored);
}

#[test]
fn terminal_view__git_log_with_more_commits_than_rows_runs_its_pager_and_q_leaves_it() {
    let mut repository = Repository::new();
    for number in 1..=20 {
        let _ = repository.commit(&format!("Commit number {number}"), &[]);
    }
    let mut state = showing(repository.path().to_path_buf());
    let shells = Shells::new();
    shells.open(&mut state);
    shells.intents(&mut state, vec![Intent::Terminal(TerminalIntent::Resize(100, 10))]);
    shells.type_line(&mut state, "git log");
    shells.until(&mut state, "the pager", |state| {
        let text = screen_text(state);
        let last = text
            .lines()
            .map(str::trim)
            .rfind(|line| !line.is_empty())
            .unwrap_or_default();
        text.contains("Commit number 20") && (last == ":" || last.ends_with("(END)"))
    });
    shells.intents(&mut state, vec![Intent::Terminal(TerminalIntent::Input(b"q".to_vec()))]);
    shells.until(&mut state, "the prompt after the pager", |state| {
        let text = screen_text(state);
        let last = text
            .lines()
            .map(str::trim)
            .rfind(|line| !line.is_empty())
            .unwrap_or_default();
        last.ends_with(['>', '$', '#'])
    });
    print(&shells, &mut state, "left");
}

#[test]
fn terminal_view__git_commit_typed_in_the_terminal_shows_the_new_commit_in_the_timeline() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let shells = Shells::new();
    shells.open(&mut state);
    let (_watch, watcher) = super::watch::watched(repository.path());
    shells.type_line(&mut state, "git commit --allow-empty --quiet -m 'Made in the terminal'");
    print(&shells, &mut state, "committed");
    let changes = super::watch::changes_within(&watcher, Duration::from_secs(3));
    assert!(
        super::watch::reads_after(&mut state, changes) > 0,
        "the commit asks for a read"
    );
    let subjects: Vec<&str> = state
        .timeline
        .rows
        .iter()
        .filter_map(model::graph::Row::commit)
        .map(|commit| commit.subject.as_str())
        .collect();
    assert!(
        subjects.contains(&"Made in the terminal"),
        "the timeline shows the commit: {subjects:?}"
    );
}

#[test]
fn terminal_view__making_the_panel_narrower_changes_the_columns_the_shell_prints() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let shells = Shells::new();
    shells.open(&mut state);
    let ask = if cfg!(windows) {
        "'cols=' + [Console]::WindowWidth"
    } else {
        "echo cols=$(tput cols)"
    };
    let columns = |state: &State| state.terminals.size.unwrap().0;
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = shells.draw(&mut state, &mut window, &[]);
    let wide = columns(&state);
    shells.type_line(&mut state, ask);
    shells.until(&mut state, "the columns", |state| {
        screen_text(state).contains(&format!("cols={wide}"))
    });
    let mut narrow = Window::new(ThemeName::Dark, (700, 640));
    let _ = shells.draw(&mut state, &mut narrow, &[]);
    let fewer = columns(&state);
    assert!(fewer < wide, "{fewer} columns, from {wide}");
    shells.type_line(&mut state, ask);
    shells.until(&mut state, "fewer columns", |state| {
        screen_text(state).contains(&format!("cols={fewer}"))
    });
}

#[test]
fn terminal_view__text_selected_and_copied_with_ctrl_shift_c_is_on_the_clipboard() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let shells = Shells::new();
    shells.open(&mut state);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = shells.draw(&mut state, &mut window, &[]);
    shells.type_line(&mut state, if cfg!(windows) { "Clear-Host" } else { "clear" });
    let word = "copyme".repeat(10);
    print(&shells, &mut state, &word);
    let tree = shells.draw(&mut state, &mut window, &[]);
    let area = bounds(find(&tree, Role::TextInput, "Terminal"));
    let rows = usize::from(state.terminals.size.unwrap().1);
    let row = screen_text(&state)
        .lines()
        .position(|line| line.trim() == format!("{word}ok"))
        .unwrap();
    #[allow(clippy::cast_precision_loss, reason = "a few rows")]
    let y = area.y + (row as f32 + 0.3) * (area.height / rows as f32);
    let intents = window.click_at(&state, Point::new(area.x + 12.0, y), 2);
    shells.intents(&mut state, intents);
    let selected = format!("{word}ok");
    shells.until(&mut state, "the selection", |state| {
        terminals::view_state(state)
            .screen
            .and_then(|screen| screen.selected_text.as_deref())
            == Some(selected.as_str())
    });
    let _ = shells.draw(
        &mut state,
        &mut window,
        &[shortcut('c', Modifiers::CONTROL | Modifiers::SHIFT)],
    );
    assert_eq!(window.clipboard.as_deref(), Some(selected.as_str()));
}

#[test]
fn terminal_view__new_terminal_adds_a_second_one_and_each_keeps_what_it_printed_when_the_other_is_shown() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let shells = Shells::new();
    shells.open(&mut state);
    print(&shells, &mut state, "first");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, "New terminal");
    assert_eq!(without_sizes(&intents), [Intent::Terminal(TerminalIntent::New)]);
    shells.intents(&mut state, intents);
    assert_eq!(terminals::view_state(&state).names.len(), 2);
    shells.until(&mut state, "the second prompt", |state| {
        !screen_text(state).trim().is_empty()
    });
    assert!(!printed(&state, "first"), "the second terminal is a new one");
    print(&shells, &mut state, "second");
    shells.intents(&mut state, vec![Intent::Terminal(TerminalIntent::Show(0))]);
    assert!(printed(&state, "first") && !printed(&state, "second"));
    shells.intents(&mut state, vec![Intent::Terminal(TerminalIntent::Show(1))]);
    assert!(printed(&state, "second") && !printed(&state, "first"));
}

#[test]
fn terminal_view__showing_another_repository_tab_shows_its_terminals_and_coming_back_shows_the_first_as_they_were() {
    let (_kept, _clones, mut state) = super::tabs::three_clones();
    let shells = Shells::new();
    shells.open(&mut state);
    print(&shells, &mut state, "api");
    let api = shown(&state).unwrap().id;
    shells.intents(&mut state, vec![Intent::SwitchTab(1)]);
    shells.until(&mut state, "the prompt of web", |state| {
        shown(state).is_some_and(|entry| entry.id != api) && !screen_text(state).trim().is_empty()
    });
    assert!(!printed(&state, "api"));
    print(&shells, &mut state, "web");
    shells.intents(&mut state, vec![Intent::SwitchTab(0)]);
    assert_eq!(shown(&state).unwrap().id, api);
    assert!(printed(&state, "api") && !printed(&state, "web"));
}

#[test]
fn terminal_view__exit_says_that_the_shell_ended_with_exit_code_0_and_enter_starts_a_new_one() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let shells = Shells::new();
    shells.open(&mut state);
    let first = shown(&state).unwrap().id;
    shells.type_line(&mut state, "exit");
    shells.until(&mut state, "the end of the shell", |state| {
        shown(state).is_some_and(|entry| entry.status == ShellStatus::Ended(Some(0)))
    });
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let tree = shells.draw(&mut state, &mut window, &[]);
    assert!(says(&tree, "The shell ended with exit code 0"));
    let _ = shells.draw(&mut state, &mut window, &[key(NamedKey::Enter)]);
    shells.until(&mut state, "a new shell", |state| {
        shown(state).is_some_and(|entry| entry.id != first && entry.status == ShellStatus::Running)
    });
    assert_eq!(
        terminals::view_state(&state).names.len(),
        1,
        "the new shell is in place of the one that ended"
    );
}

#[test]
fn panel_bottom__hide_the_panel_while_a_command_runs_leaves_it_running_and_shown_again_its_output_has_grown() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let shells = Shells::new();
    shells.open(&mut state);
    let ticks = if cfg!(windows) {
        "1..8 | % { 'tick' + $_; Start-Sleep -Milliseconds 300 }"
    } else {
        "for i in 1 2 3 4 5 6 7 8; do echo tick$i; sleep 0.3; done"
    };
    shells.type_line(&mut state, ticks);
    shells.until(&mut state, "the first tick", |state| {
        screen_text(state).lines().any(|line| line.trim() == "tick1")
    });
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, "Hide the panel");
    assert_eq!(without_sizes(&intents), [Intent::Panel(PanelIntent::Hide)]);
    shells.intents(&mut state, intents);
    assert!(!panel::look(&state).is_shown);
    shells.until(&mut state, "the last tick", |state| {
        screen_text(state).lines().any(|line| line.trim() == "tick8")
    });
    shells.intents(&mut state, vec![Intent::Panel(PanelIntent::Open)]);
    let tree = shells.draw(&mut state, &mut window, &[]);
    let terminal = find(&tree, Role::TextInput, "Terminal");
    assert!(
        terminal.value().is_some_and(|value| value.contains("tick8")),
        "{:?}",
        terminal.value()
    );
}
