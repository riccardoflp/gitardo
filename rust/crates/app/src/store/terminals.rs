//! The terminals of the bottom panel (`terminal.view`): each repository tab has its own, kept while another tab is
//! shown and ended when the tab closes; the store knows their names, how their shells stand and the screen each shows,
//! and asks the window's terminals to start, write, size, scroll, select and end.

use std::collections::BTreeMap;
use std::path::PathBuf;

use model::terminal::{Screen, Selection};
use views::panel::{PanelIntent, PanelTab};
use views::terminal::{TerminalIntent, TerminalState, TerminalStatus};
use views::text::panel as words;

use super::message::Effect;
use super::panel;
use super::state::State;

/// The size a terminal starts at before the panel said its own.
const START_SIZE: (u16, u16) = (80, 24);

/// How the shell of a terminal stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShellStatus {
    /// It starts.
    Starting,
    /// It runs.
    Running,
    /// It ended, with its exit code when the system gave one.
    Ended(Option<i32>),
    /// It could not start, with what the system said.
    NotStarted(String),
}

/// A terminal of a repository tab.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// Its number in the window, which the window's terminals know it by.
    pub id: u64,
    /// The name of its shell, `pwsh`, once it started.
    pub shell: String,
    /// How its shell stands.
    pub status: ShellStatus,
    /// What it shows, once it printed.
    pub screen: Option<Screen>,
}

/// The terminals of a repository tab, and which is shown.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Group {
    /// The terminals, in the order they were made.
    pub entries: Vec<Entry>,
    /// Which of them is shown.
    pub shown: usize,
    /// The cell the main button was pressed on in the terminal shown, while a selection is dragged from it.
    pub anchor: Option<(usize, usize)>,
}

/// The terminals of the window.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Terminals {
    /// The terminals of each repository tab, by the folder of the tab.
    pub groups: BTreeMap<PathBuf, Group>,
    /// The number of the next terminal.
    pub next_id: u64,
    /// The size the panel last gave the terminals, in columns and rows.
    pub size: Option<(u16, u16)>,
}

/// What the window's terminals are asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalEffect {
    /// Start a terminal of a number, its shell in a folder.
    Start {
        /// Its number.
        id: u64,
        /// The folder its shell starts in.
        folder: PathBuf,
        /// `terminal.shellPath`: empty for the system's own shell.
        shell_path: String,
        /// `terminal.scrollbackLines`.
        scrollback: usize,
        /// Its size, in columns and rows.
        size: (u16, u16),
    },
    /// Give the shell of a terminal what was typed or pasted.
    Write(u64, Vec<u8>),
    /// Give every terminal a size, in columns and rows.
    Resize((u16, u16)),
    /// Scroll a terminal by some lines, up into what it printed before for a positive number.
    Scroll(u64, i32),
    /// Select in a terminal, or take its selection away.
    Select(u64, Option<Selection>),
    /// End the shell of a terminal and forget it.
    Kill(u64),
}

/// What the window's terminals tell.
#[derive(Clone, Debug, PartialEq)]
pub enum TerminalMessage {
    /// A terminal started, with the name of its shell.
    Started(u64, String),
    /// A terminal could not start, with what the system said.
    NotStarted(u64, String),
    /// A terminal printed something: the window reads its screen, which it tells as [`TerminalMessage::Screen`].
    Printed(u64),
    /// What a terminal shows now.
    Screen(u64, Box<Screen>),
    /// The shell of a terminal ended, with its exit code when the system gave one.
    Ended(u64, Option<i32>),
}

/// The folder of the repository tab shown, which its terminals are kept under, and the folder its shells start in: the
/// repository or the worktree shown in it.
pub(super) fn shown_tab(state: &State) -> Option<(PathBuf, PathBuf)> {
    let folder = state.shown_folder()?.clone();
    let tab = state
        .tabs
        .shown
        .and_then(|shown| state.tabs.open.get(shown))
        .map_or_else(|| folder.clone(), |tab| tab.folder.clone());
    Some((tab, folder))
}

/// The terminals of the repository tab shown.
fn shown_group(state: &State) -> Option<&Group> {
    let (tab, _) = shown_tab(state)?;
    state.terminals.groups.get(&tab)
}

/// The terminal shown, of the repository tab shown.
fn shown_entry(state: &State) -> Option<&Entry> {
    let group = shown_group(state)?;
    group.entries.get(group.shown)
}

/// Starts a terminal in the repository tab shown, in place of the one at an index or after the others, and shows it.
fn start(state: &mut State, at: Option<usize>) -> Vec<Effect> {
    let Some((tab, folder)) = shown_tab(state) else {
        return Vec::new();
    };
    let id = state.terminals.next_id;
    state.terminals.next_id += 1;
    let entry = Entry {
        id,
        shell: String::new(),
        status: ShellStatus::Starting,
        screen: None,
    };
    let group = state.terminals.groups.entry(tab).or_default();
    let mut effects = Vec::new();
    if let Some(index) = at.filter(|index| *index < group.entries.len()) {
        if let Some(old) = group.entries.get_mut(index) {
            effects.push(Effect::Terminal(TerminalEffect::Kill(old.id)));
            *old = entry;
        }
        group.shown = index;
    } else {
        group.entries.push(entry);
        group.shown = group.entries.len() - 1;
    }
    group.anchor = None;
    effects.push(Effect::Terminal(TerminalEffect::Start {
        id,
        folder,
        shell_path: state.settings.terminal.shell_path.clone(),
        scrollback: state.settings.terminal.scrollback,
        size: state.terminals.size.unwrap_or(START_SIZE),
    }));
    effects
}

/// What was asked of the terminals.
pub fn reduce(state: &mut State, intent: TerminalIntent) -> Vec<Effect> {
    let shown = shown_entry(state).map(|entry| entry.id);
    let terminal = |effect| vec![Effect::Terminal(effect)];
    match (intent, shown) {
        (TerminalIntent::New, _) => {
            let mut effects = panel::reduce(state, &PanelIntent::Open);
            effects.extend(start(state, None));
            effects
        }
        (TerminalIntent::Restart, Some(_)) => {
            let at = shown_group(state).map(|group| group.shown);
            start(state, at)
        }
        (TerminalIntent::Kill, Some(id)) => kill_shown(state, id),
        (TerminalIntent::Show(index), _) => {
            if let Some(group) = shown_group_mut(state)
                && index < group.entries.len()
            {
                group.shown = index;
                group.anchor = None;
                state.panel.focus += 1;
            }
            Vec::new()
        }
        (TerminalIntent::Resize(columns, rows), _) => {
            state.terminals.size = Some((columns, rows));
            terminal(TerminalEffect::Resize((columns, rows)))
        }
        (TerminalIntent::Input(bytes), Some(id)) => terminal(TerminalEffect::Write(id, bytes)),
        (TerminalIntent::Scroll(lines), Some(id)) => terminal(TerminalEffect::Scroll(id, lines)),
        (TerminalIntent::Press(cell), Some(id)) => {
            if let Some(group) = shown_group_mut(state) {
                group.anchor = Some(cell);
            }
            terminal(TerminalEffect::Select(id, None))
        }
        (TerminalIntent::Drag(cell), Some(id)) => {
            let anchor = shown_group(state).and_then(|group| group.anchor);
            anchor.map_or_else(Vec::new, |from| {
                let selection = Selection {
                    from,
                    to: cell,
                    is_word: false,
                };
                terminal(TerminalEffect::Select(id, Some(selection)))
            })
        }
        (TerminalIntent::SelectWord(cell), Some(id)) => {
            let selection = Selection {
                from: cell,
                to: cell,
                is_word: true,
            };
            terminal(TerminalEffect::Select(id, Some(selection)))
        }
        (_, None) => Vec::new(),
    }
}

/// The terminals of the repository tab shown, to change.
fn shown_group_mut(state: &mut State) -> Option<&mut Group> {
    let (tab, _) = shown_tab(state)?;
    state.terminals.groups.get_mut(&tab)
}

/// **Kill the terminal**: its shell ends and it is taken away; the panel is hidden once the tab has none left, as it
/// would start another otherwise.
fn kill_shown(state: &mut State, id: u64) -> Vec<Effect> {
    let mut effects = vec![Effect::Terminal(TerminalEffect::Kill(id))];
    let Some((tab, _)) = shown_tab(state) else {
        return effects;
    };
    let Some(group) = state.terminals.groups.get_mut(&tab) else {
        return effects;
    };
    group.entries.retain(|entry| entry.id != id);
    group.shown = group.shown.min(group.entries.len().saturating_sub(1));
    group.anchor = None;
    if group.entries.is_empty() {
        let _ = state.terminals.groups.remove(&tab);
        effects.extend(panel::reduce(state, &PanelIntent::Hide));
    }
    effects
}

/// What the window's terminals told.
pub fn reduce_message(state: &mut State, message: TerminalMessage) -> Vec<Effect> {
    let id = match &message {
        TerminalMessage::Started(id, _)
        | TerminalMessage::NotStarted(id, _)
        | TerminalMessage::Printed(id)
        | TerminalMessage::Screen(id, _)
        | TerminalMessage::Ended(id, _) => *id,
    };
    let Some(entry) = state
        .terminals
        .groups
        .values_mut()
        .flat_map(|group| group.entries.iter_mut())
        .find(|entry| entry.id == id)
    else {
        return Vec::new();
    };
    match message {
        TerminalMessage::Started(_, shell) => {
            entry.shell = shell;
            if entry.status == ShellStatus::Starting {
                entry.status = ShellStatus::Running;
            }
        }
        TerminalMessage::NotStarted(_, reason) => entry.status = ShellStatus::NotStarted(reason),
        TerminalMessage::Screen(_, screen) => entry.screen = Some(*screen),
        TerminalMessage::Ended(_, code) => entry.status = ShellStatus::Ended(code),
        TerminalMessage::Printed(_) => {}
    }
    Vec::new()
}

/// After each message: the shells of the tabs that closed end, and the Terminal tab of the panel, shown for a
/// repository tab that has no terminal yet, starts one.
pub fn follow(state: &mut State) -> Vec<Effect> {
    let mut kept: Vec<PathBuf> = state.tabs.open.iter().map(|tab| tab.folder.clone()).collect();
    kept.extend(shown_tab(state).map(|(tab, _)| tab));
    let closed: Vec<PathBuf> = state
        .terminals
        .groups
        .keys()
        .filter(|folder| !kept.contains(folder))
        .cloned()
        .collect();
    let mut effects = Vec::new();
    for folder in closed {
        if let Some(group) = state.terminals.groups.remove(&folder) {
            let ids = group.entries.iter().map(|entry| entry.id);
            effects.extend(ids.map(|id| Effect::Terminal(TerminalEffect::Kill(id))));
        }
    }
    let look = panel::look(state);
    let wants_one = look.is_shown && look.tab == PanelTab::Terminal && shown_group(state).is_none();
    if wants_one && shown_tab(state).is_some() {
        effects.extend(start(state, None));
    }
    effects
}

/// What the Terminal tab of the bottom panel shows.
#[must_use]
pub fn view_state(state: &State) -> TerminalState<'_> {
    let group = shown_group(state);
    let entries = group.map(|group| group.entries.as_slice()).unwrap_or_default();
    let entry = group.and_then(|group| group.entries.get(group.shown));
    let status = match (shown_tab(state), entry.map(|entry| &entry.status)) {
        (None, _) => TerminalStatus::NoRepository,
        (Some(_), None | Some(ShellStatus::Starting)) => TerminalStatus::Starting,
        (Some(_), Some(ShellStatus::Running)) => TerminalStatus::Running,
        (Some(_), Some(ShellStatus::Ended(code))) => TerminalStatus::Ended(*code),
        (Some(_), Some(ShellStatus::NotStarted(reason))) => TerminalStatus::NotStarted(reason),
    };
    TerminalState {
        names: entries
            .iter()
            .enumerate()
            .map(|(index, entry)| words::terminal_name(index + 1, &entry.shell))
            .collect(),
        shown: group.map_or(0, |group| group.shown),
        screen: entry.and_then(|entry| entry.screen.as_ref()),
        status,
        size: state.terminals.size,
        focus: state.panel.focus,
    }
}
