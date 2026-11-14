//! Overlays: menus, dialogs, pickers and the find bar, which take the keyboard while they are open and give it back
//! to the widget that had it when they close.

use super::Ui;
use crate::id::WidgetId;

/// What the toolkit remembers of an overlay while it is open.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct OverlayState {
    /// The widget that had the focus when it opened, which gets it back when it closes.
    pub(crate) focus_before: Option<WidgetId>,
    /// The widgets it is made of whose state is forgotten when it closes, such as its input box.
    pub(crate) owned: Vec<WidgetId>,
}

impl Ui<'_> {
    /// Notes that an overlay is declared in this frame and says whether it opened in this frame. The focus it found
    /// is given back when it is no longer declared, and the widgets it owns are forgotten then.
    pub(crate) fn declare_overlay(&mut self, id: WidgetId, owned: &[WidgetId]) -> bool {
        self.overlays.push(id);
        if self.memory.overlays.contains_key(&id) {
            return false;
        }
        let state = OverlayState {
            focus_before: self.memory.interaction.focus,
            owned: owned.to_vec(),
        };
        let _ = self.memory.overlays.insert(id, state);
        true
    }

    /// Closes the overlays that were not declared in this frame: their widgets are forgotten, and the focus goes back
    /// where it was when they opened when nothing in sight has it now.
    pub(super) fn close_overlays(&mut self) {
        let closed: Vec<WidgetId> = self
            .memory
            .overlays
            .keys()
            .filter(|id| !self.overlays.contains(id))
            .copied()
            .collect();
        for id in closed {
            let Some(state) = self.memory.overlays.remove(&id) else {
                continue;
            };
            for owned in &state.owned {
                self.memory.forget(*owned);
            }
            let interaction = &mut self.memory.interaction;
            let is_focus_lost = interaction
                .focus
                .is_none_or(|focus| !interaction.hits.is_focusable(focus) || state.owned.contains(&focus));
            if is_focus_lost {
                interaction.focus = state
                    .focus_before
                    .filter(|before| interaction.hits.is_focusable(*before));
            }
        }
    }
}
