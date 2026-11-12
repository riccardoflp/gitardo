//! Text and icons that only show something: labels cut with an ellipsis when they do not fit, headings, paragraphs
//! wrapped by words, and icons.

use render::{Icon, Point, Rect, TextRun};

use super::Tone;
use crate::frame::{TextKind, Ui};
use crate::interaction::Sense;
use crate::layout::{Align, place};

/// A line of text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Label<'text> {
    /// The text.
    pub text: &'text str,
    /// Its kind, which gives its typeface and size.
    pub kind: TextKind,
    /// Its color.
    pub tone: Tone,
    /// Where it is across its rectangle; it is always in the middle down.
    pub align: Align,
    /// Whether the whole text shows in a tooltip when it was cut.
    pub has_tooltip_when_cut: bool,
}

impl<'text> Label<'text> {
    /// Body text in the normal color, at the start of its rectangle, with a tooltip when it is cut.
    #[must_use]
    pub fn new(text: &'text str) -> Label<'text> {
        Label {
            text,
            kind: TextKind::Body,
            tone: Tone::Normal,
            align: Align::Start,
            has_tooltip_when_cut: true,
        }
    }

    /// The same label of another kind.
    #[must_use]
    pub fn kind(self, kind: TextKind) -> Label<'text> {
        Label { kind, ..self }
    }

    /// The same label in another color.
    #[must_use]
    pub fn tone(self, tone: Tone) -> Label<'text> {
        Label { tone, ..self }
    }

    /// The same label placed otherwise across its rectangle.
    #[must_use]
    pub fn align(self, align: Align) -> Label<'text> {
        Label { align, ..self }
    }
}

impl Ui<'_> {
    /// Draws a line of text in the middle of a rectangle, down, cut with an ellipsis when it does not fit across, and
    /// returns the width it takes.
    pub fn label(&mut self, rect: Rect, label: &Label<'_>) -> f32 {
        let style = self.text_style(label.kind);
        let width = self.measure(label.text, style);
        let height = self.line_height(style);
        let placed = place(rect, (width, height), (label.align, Align::Center));
        let color = label.tone.color(self.theme());
        let run = TextRun {
            text: label.text,
            style,
            color,
            origin: Point::new(placed.x, placed.y),
            max_width: Some(rect.width),
        };
        let drawn = self.text(&run);
        if label.has_tooltip_when_cut && width > rect.width {
            let id = self.id(("label", label.text, rect.x.to_bits(), rect.y.to_bits()));
            let response = self.interact(id, rect, Sense::HOVER);
            self.tooltip(&response, label.text);
        }
        drawn
    }

    /// Draws the title of a page, a box or a dialog, and returns the width it takes.
    pub fn heading(&mut self, rect: Rect, text: &str) -> f32 {
        self.label(rect, &Label::new(text).kind(TextKind::Heading))
    }

    /// Draws text wrapped by words to the width of a rectangle, from its top, and returns the height it takes; the
    /// lines past the bottom of the rectangle are not drawn.
    pub fn paragraph(&mut self, rect: Rect, text: &str, (kind, tone): (TextKind, Tone)) -> f32 {
        let style = self.text_style(kind);
        let height = self.line_height(style);
        let color = tone.color(self.theme());
        let lines = self.fonts().wrap(text, style, rect.width);
        let mut y = rect.y;
        for line in &lines {
            if y + height > rect.bottom() + 0.5 {
                break;
            }
            let run = TextRun {
                text: text.get(line.clone()).unwrap_or_default(),
                style,
                color,
                origin: Point::new(rect.x, y),
                max_width: Some(rect.width),
            };
            let _ = self.text(&run);
            y += height;
        }
        crate::number::float_of_count(lines.len()) * height
    }

    /// Draws an icon of the theme's size in the middle of a rectangle.
    pub fn icon_in(&mut self, rect: Rect, icon: Icon, tone: Tone) {
        let side = self.theme().sizes.icon;
        let placed = place(rect, (side, side), (Align::Center, Align::Center));
        let color = tone.color(self.theme());
        self.icon(icon, Point::new(placed.x, placed.y), side, color);
    }
}
