//! The widgets that stay above the others of their layer, whatever the order they are declared in: the grip of a
//! splitter, declared before the panes it sits between, and the bar of a fold that a widget declared after it lies over.

use render::Rect;

use super::Ui;
use crate::id::WidgetId;
use crate::interaction::{Response, Sense, WidgetEvent};

impl Ui<'_> {
    /// Declares a widget that takes input and stays above the other widgets of its layer, whatever the order they are
    /// declared in: the bar of a fold drawn in a row of a list that a widget declared after the list lies over.
    pub fn interact_above(&mut self, id: WidgetId, rect: Rect, sense: Sense) -> Response {
        let events = self.take_events(id);
        let response = self.interact_raised(id, rect, (sense, &events));
        self.claim_response_keys(id, sense);
        response
    }

    /// Takes the events a widget was given, for a widget that must apply them before it knows where it is, such as
    /// the grip of a splitter that moves with them; it then declares itself with [`Ui::interact_raised`].
    pub(crate) fn take_events(&mut self, id: WidgetId) -> Vec<WidgetEvent> {
        self.routed.events.remove(&id).unwrap_or_default()
    }

    /// Declares a widget that stays above the other widgets of its layer, whatever the order they are declared in,
    /// with the events taken for it: the grip of a splitter, declared before the panes it sits between.
    pub(crate) fn interact_raised(
        &mut self,
        id: WidgetId,
        rect: Rect,
        (sense, events): (Sense, &[WidgetEvent]),
    ) -> Response {
        self.register(id, rect, (sense, true));
        Response::new(id, rect, sense, events, &self.memory.interaction)
    }
}
