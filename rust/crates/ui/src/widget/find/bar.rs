//! The find bar: an input box, the count of the matches, a switch for the case of letters, the buttons that go to the
//! previous and the next match, and the one that closes the bar, in a box at the top right of the page.

use render::{Icon, Rect};

use super::matches::PageMatches;
use super::{MatchCase, Step};
use crate::frame::{TextKind, Ui};
use crate::id::WidgetId;
use crate::interaction::Sense;
use crate::layout::{Align, Area, Insets, inset, place};
use crate::widget::Tone;
use crate::widget::button::Button;
use crate::widget::input_box::InputBox;
use crate::widget::label::Label;
use crate::widget::metrics;

impl Ui<'_> {
    /// Draws the bar over the top right of a page and applies what is done in it.
    pub(super) fn find_bar(&mut self, id: WidgetId, page: Rect, found: &PageMatches) {
        let theme = *self.theme();
        let spacing = theme.sizes.spacing;
        let control = metrics::control_height(&theme);
        let width = (theme.sizes.row_height * 22.0)
            .min(page.width - 2.0 * spacing.large)
            .max(0.0);
        let height = control + 2.0 * spacing.small;
        let rect = Rect::new(
            page.right() - width - spacing.large,
            page.y + spacing.small,
            width,
            height,
        );
        let bar = id.with("bar");
        let _ = self.interact(bar, rect, Sense::CLICK);
        self.floating_card(rect, theme.sizes.radius.large);
        self.access_node(
            bar,
            rect,
            &crate::access::Description::new(crate::access::AccessRole::Region, "Find in page"),
        );
        let mut area = Area::with_gap(inset(rect, Insets::all(spacing.small)), spacing.tiny);
        let close = area.right(control);
        let next = area.right(control);
        let previous = area.right(control);
        let case = area.right(control);
        let longest = format!("{total} of {total}", total = found.total);
        let needed = self.measure(&longest, self.text_style(TextKind::Small)) + 2.0 * spacing.small;
        let count = area.right(needed.max(theme.sizes.row_height * 4.0));
        let field = area.rest();
        self.within(id, |ui| {
            ui.find_field(field);
            ui.find_count(count, found);
            ui.find_buttons([case, previous, next, close], found.total);
        });
    }

    /// The input box of the bar: what is typed is looked for at once.
    fn find_field(&mut self, rect: Rect) {
        let query = self.memory_view().find.query.clone();
        let input = InputBox::new(&query, "Find");
        let typed = self.input_box("query", rect, &input);
        if let Some(text) = typed.changed_text {
            let now = self.now();
            let find = &mut self.memory().find;
            find.query = text;
            find.current = 0;
            find.must_reveal = true;
            self.request_frame_at(now);
        }
    }

    /// The count of the matches: "3 of 12", or that there is none.
    fn find_count(&mut self, rect: Rect, found: &PageMatches) {
        let find = &self.memory_view().find;
        let text = match (find.query.is_empty(), found.total) {
            (true, _) => String::new(),
            (false, 0) => String::from("No results"),
            (false, total) => format!("{} of {total}", find.current.min(total - 1) + 1),
        };
        let tone = if found.total == 0 { Tone::Muted } else { Tone::Normal };
        let label = Label::new(&text).kind(TextKind::Small).tone(tone).align(Align::Center);
        let _ = self.label(rect, &label);
    }

    /// The switch for the case of letters and the buttons of the bar.
    fn find_buttons(&mut self, [case, previous, next, close]: [Rect; 4], total: usize) {
        let is_exact = self.memory_view().find.case == MatchCase::Exact;
        let case_button = if is_exact {
            Button::primary("Aa")
        } else {
            Button::secondary("Aa")
        }
        .with_tooltip("Match case");
        let square = |rect: Rect| place(rect, (rect.width, rect.width), (Align::Start, Align::Center));
        if self.button("case", square(case), &case_button).clicked {
            let now = self.now();
            let find = &mut self.memory().find;
            find.case = if is_exact { MatchCase::Ignore } else { MatchCase::Exact };
            find.current = 0;
            find.must_reveal = true;
            self.request_frame_at(now);
        }
        let mut step = None;
        let mut previous_button = Button::icon(Icon::ArrowUp, "Previous match (Shift+Enter)");
        let mut next_button = Button::icon(Icon::ArrowDown, "Next match (Enter)");
        if total == 0 {
            previous_button = previous_button.disabled();
            next_button = next_button.disabled();
        }
        if self.button("previous", square(previous), &previous_button).clicked {
            step = Some(Step::Previous);
        }
        if self.button("next", square(next), &next_button).clicked {
            step = Some(Step::Next);
        }
        if step.is_some() {
            let _ = self.step_to(step, total);
            let now = self.now();
            self.request_frame_at(now);
        }
        let close_button = Button::icon(Icon::Close, "Close (Escape)");
        if self.button("close", square(close), &close_button).clicked {
            self.memory().find.is_open = false;
        }
    }
}
