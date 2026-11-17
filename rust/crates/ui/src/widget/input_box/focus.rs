//! The keyboard given to an input box from outside it, and its text read from outside it.

use crate::frame::Ui;
use crate::id::WidgetId;

impl Ui<'_> {
    /// The text of an input box as it stands, typed or given, when it was declared before.
    #[must_use]
    pub fn typed_text(&self, id: WidgetId) -> Option<String> {
        self.memory_view().input_text(id).map(str::to_owned)
    }

    /// Gives the keyboard to an input box with all its text selected, as `Ctrl+F` does to a search box. Called before
    /// the box is declared, the box is drawn so in the same frame.
    pub fn focus_input_box(&mut self, id: WidgetId) {
        let now = self.now();
        if let Some(state) = self.memory().input_boxes.get_mut(&id) {
            state.line.select_all();
            state.changed_at = now;
        }
        self.request_focus(id);
    }

    /// Gives the keyboard to an input box with all its text selected, once for each number: the box that finds in a
    /// page takes it each time `Ctrl+F` is pressed, and leaves it to a click elsewhere in between.
    pub fn focus_input_box_once(&mut self, id: WidgetId, serial: u64) {
        if self.memory().focus_serials.insert(id, serial) != Some(serial) {
            self.focus_input_box(id);
        }
    }
}
