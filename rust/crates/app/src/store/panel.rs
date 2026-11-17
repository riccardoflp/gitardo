//! The bottom panel of the window (`panel.bottom`): whether it is shown, its height and the tab it shows are the
//! window's, kept for the workspace with the sizes of its panes (Q418); before a workspace is open they are the window's
//! alone.

use views::panel::{PanelIntent, PanelTab};

use super::message::Effect;
use super::state::State;
use super::timeline::workspace_panes;
use crate::persisted::same_folders;
pub use crate::persisted::timeline::PanelLook;
use crate::persisted::timeline::remember;

/// The bottom panel of a window.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Panel {
    /// How it is while no workspace is open.
    pub look: PanelLook,
    /// A number that changes each time the keyboard is to go to the terminal.
    pub focus: u64,
    /// The line the Command log was opened at, counted from the first line the log recorded, with a number that changes
    /// each time it is opened so; none to follow the last line (`activity.said`).
    pub log_at: Option<(usize, u64)>,
}

/// How the bottom panel is: as kept for the workspace open, or as the window has it before one is.
#[must_use]
pub fn look(state: &State) -> PanelLook {
    if state.workspace.is_some() {
        workspace_panes(state).panel
    } else {
        state.panel.look
    }
}

/// The bottom panel changed: kept for the workspace open, when one is.
fn set(state: &mut State, look: PanelLook) -> Vec<Effect> {
    state.panel.look = look;
    if state.workspace.is_none() {
        return Vec::new();
    }
    let mut panes = workspace_panes(state);
    if panes.panel == look {
        return Vec::new();
    }
    panes.panel = look;
    let folders = panes.folders.clone();
    remember(&mut state.persisted.panes, panes, |kept| {
        same_folders(&kept.folders, &folders)
    });
    vec![Effect::WritePersisted(state.persisted.clone())]
}

/// What was asked of the bottom panel: shown at a tab, hidden, its height dragged. `` Ctrl+` `` shows it at the
/// terminal with the keyboard in it.
pub fn reduce(state: &mut State, intent: &PanelIntent) -> Vec<Effect> {
    let mut look = look(state);
    match intent {
        PanelIntent::Open => {
            look.is_shown = true;
            look.tab = PanelTab::Terminal;
            state.panel.focus += 1;
        }
        PanelIntent::Show(tab) => {
            look.is_shown = true;
            look.tab = *tab;
            if *tab == PanelTab::CommandLog {
                state.panel.log_at = None;
            }
        }
        PanelIntent::ShowLogAt(line) => {
            look.is_shown = true;
            look.tab = PanelTab::CommandLog;
            let serial = state.panel.log_at.map_or(1, |(_, serial)| serial + 1);
            state.panel.log_at = Some((*line, serial));
        }
        PanelIntent::Hide => look.is_shown = false,
        PanelIntent::Toggle => {
            // The button of the terminal never leaves the panel at what runs, which has a button of its own.
            let was_at_activity = look.tab == PanelTab::Activity;
            look.is_shown = !look.is_shown || was_at_activity;
            if was_at_activity {
                look.tab = PanelTab::Terminal;
            }
        }
        PanelIntent::ToggleActivity => {
            look.is_shown = !(look.is_shown && look.tab == PanelTab::Activity);
            look.tab = PanelTab::Activity;
        }
        PanelIntent::SetHeight(height) => look.height = *height,
    }
    set(state, look)
}

/// Whether the bottom panel shows the Command log, whose lines are then made.
#[must_use]
pub fn is_log_shown(state: &State) -> bool {
    let look = look(state);
    look.is_shown && look.tab == PanelTab::CommandLog
}

/// Whether the bottom panel shows its Activity tab, whose Tasks are then made.
#[must_use]
pub fn is_activity_shown(state: &State) -> bool {
    let look = look(state);
    look.is_shown && look.tab == PanelTab::Activity
}

/// **Show Command Log**, or something asked of the bottom panel.
pub fn reduce_intent(state: &mut State, intent: views::Intent) -> Vec<Effect> {
    match intent {
        views::Intent::ShowCommandLog => reduce(state, &PanelIntent::Show(PanelTab::CommandLog)),
        views::Intent::Panel(intent) => reduce(state, &intent),
        _ => Vec::new(),
    }
}
