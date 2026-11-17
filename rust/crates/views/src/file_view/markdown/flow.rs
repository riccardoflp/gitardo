//! The text of a block of the preview laid in lines as wide as its column: its words one after the other, a line
//! broken after a space when the next word does not fit, inside a word only when it is wider than the column; each
//! line aligned in the column, its parts on a common bottom.

use model::markdown::{Alignment, Look, Piece, Span};
use ui::{Color, Image, Point, Rect, TextStyle};

use super::layout::{Ink, Laid, Mark, TextLook};

/// Where a block's text is laid, and how it looks when nothing says otherwise.
#[derive(Clone, Copy, Debug)]
pub(super) struct Place {
    /// The left of the column.
    pub(super) left: f32,
    /// Its width.
    pub(super) width: f32,
    /// The top of the first line.
    pub(super) top: f32,
    /// The typeface and size of the text.
    pub(super) style: TextStyle,
    /// Its color.
    pub(super) color: Color,
    /// How its lines are aligned.
    pub(super) alignment: Alignment,
}

/// A part of a line not yet placed: text of one look, or an image.
#[derive(Debug)]
enum Atom {
    /// Text.
    Text {
        /// The text.
        text: String,
        /// How it looks.
        look: TextLook,
        /// Where it starts from the left of the column.
        x: f32,
        /// Its width.
        width: f32,
    },
    /// An image, drawn as wide as it is, or as the column.
    Image {
        /// The image.
        image: Image,
        /// The link it is inside.
        link: Option<usize>,
        /// Where it starts from the left of the column.
        x: f32,
        /// Its size.
        size: (f32, f32),
    },
}

/// The lines of a block being laid.
struct Flow<'f, 'a, 'u> {
    /// What the text is measured and drawn with.
    ink: &'f Ink<'a, 'u>,
    /// What is laid so far.
    laid: &'f mut Laid,
    /// Where.
    place: Place,
    /// The top of the line being laid.
    top: f32,
    /// Where the next part of the line starts, from the left of the column.
    x: f32,
    /// The parts of the line being laid.
    atoms: Vec<Atom>,
}

/// Lays the text of a block from the top of its place, and gives the bottom of its last line.
pub(super) fn flow(ink: &Ink<'_, '_>, laid: &mut Laid, pieces: &[Piece], place: Place) -> f32 {
    let mut flow = Flow {
        ink,
        laid,
        place,
        top: place.top,
        x: 0.0,
        atoms: Vec::new(),
    };
    for piece in pieces {
        match piece {
            Piece::Text(span) => flow.span(span),
            Piece::Image { source, text, link } => flow.image(source, text, link.as_deref()),
            Piece::Break => flow.end_line(true),
        }
    }
    flow.end_line(false);
    flow.top
}

impl Flow<'_, '_, '_> {
    /// Lays a span of text word by word.
    fn span(&mut self, span: &Span) {
        let link = span.link.as_ref().map(|href| self.laid.link(href));
        let look = self.look_of(span.look, link);
        for word in words(&span.text) {
            self.word(word, look);
        }
    }

    /// How text of a look is drawn in this block.
    fn look_of(&self, look: Look, link: Option<usize>) -> TextLook {
        let looks = &self.ink.looks;
        let style = if look.is_code {
            TextStyle {
                typeface: looks.code.typeface,
                size: self.place.style.size.min(looks.code.size),
            }
        } else if look.is_strong {
            TextStyle {
                typeface: looks.strong.typeface,
                size: self.place.style.size,
            }
        } else if look.is_emphasis {
            TextStyle {
                typeface: looks.emphasis.typeface,
                size: self.place.style.size,
            }
        } else {
            self.place.style
        };
        TextLook {
            style,
            color: if link.is_some() { looks.link } else { self.place.color },
            link,
            is_struck: look.is_struck,
            is_code: look.is_code,
        }
    }

    /// Lays a word and the spaces after it: on the line when it fits, else on the next, cut where it overflows when it
    /// is wider than the column.
    fn word(&mut self, word: &str, look: TextLook) {
        let shown = word.trim_end();
        if shown.is_empty() && self.x == 0.0 {
            return;
        }
        let fonts = self.ink.ui.fonts();
        let width = fonts.measure(shown, look.style);
        if self.x > 0.0 && self.x + width > self.place.width {
            self.end_line(false);
            if shown.is_empty() {
                return;
            }
        }
        if width <= self.place.width || shown.is_empty() {
            self.push_text(word, look);
            return;
        }
        let pieces = fonts.wrap(shown, look.style, self.place.width);
        let count = pieces.len();
        for (index, range) in pieces.into_iter().enumerate() {
            self.push_text(shown.get(range).unwrap_or_default(), look);
            if index + 1 < count {
                self.end_line(false);
            }
        }
        if word.len() > shown.len() {
            self.push_text(" ", look);
        }
    }

    /// Adds text to the line, joined to the text before it when they look the same.
    fn push_text(&mut self, text: &str, look: TextLook) {
        let width = self.ink.ui.fonts().measure(text, look.style);
        if let Some(Atom::Text {
            text: last,
            look: last_look,
            width: last_width,
            ..
        }) = self.atoms.last_mut()
            && *last_look == look
        {
            last.push_str(text);
            *last_width = self.ink.ui.fonts().measure(last, look.style);
            self.x = self.line_end();
            return;
        }
        self.atoms.push(Atom::Text {
            text: text.to_owned(),
            look,
            x: self.x,
            width,
        });
        self.x += width;
    }

    /// Where the parts of the line end.
    fn line_end(&self) -> f32 {
        match self.atoms.last() {
            Some(Atom::Text { x, width, .. }) => x + width,
            Some(Atom::Image { x, size, .. }) => x + size.0,
            None => 0.0,
        }
    }

    /// Lays an image: drawn when it was read, as wide as it is or as the column, else its text in its place.
    fn image(&mut self, source: &str, text: &str, link: Option<&str>) {
        let link = link.map(|href| self.laid.link(href));
        let found = self.ink.images.iter().find(|(shown, _)| shown == source);
        let Some((_, image)) = found else {
            let said = if text.is_empty() { source } else { text };
            let look = TextLook {
                color: if link.is_some() {
                    self.ink.looks.link
                } else {
                    self.ink.looks.muted
                },
                ..self.look_of(Look::default(), link)
            };
            for word in words(said) {
                self.word(word, look);
            }
            return;
        };
        let natural = (pixels(image.width()), pixels(image.height()));
        let width = natural.0.min(self.place.width).max(1.0);
        let size = (width, natural.1 * width / natural.0.max(1.0));
        if self.x > 0.0 && self.x + size.0 > self.place.width {
            self.end_line(false);
        }
        self.atoms.push(Atom::Image {
            image: image.clone(),
            link,
            x: self.x,
            size,
        });
        self.x += size.0;
    }

    /// Ends the line being laid: its parts placed on a common bottom and aligned in the column. An empty line takes
    /// room only when a line break asks for it.
    fn end_line(&mut self, is_asked: bool) {
        if self.atoms.is_empty() && !is_asked {
            return;
        }
        let line_height = self.ink.ui.line_height(self.place.style);
        let height = self
            .atoms
            .iter()
            .map(|atom| self.height_of(atom))
            .fold(line_height, f32::max);
        let room = (self.place.width - self.line_end()).max(0.0);
        let shift = match self.place.alignment {
            Alignment::Start => 0.0,
            Alignment::Center => room / 2.0,
            Alignment::End => room,
        };
        let left = self.place.left + shift;
        let bottom = self.top + height;
        for atom in std::mem::take(&mut self.atoms) {
            let mark = self.mark_of(atom, left, bottom);
            self.laid.marks.push(mark);
        }
        self.top = bottom + self.ink.looks.line_gap;
        self.x = 0.0;
    }

    /// The height of a part of a line.
    fn height_of(&self, atom: &Atom) -> f32 {
        match atom {
            Atom::Text { look, .. } => self.ink.ui.line_height(look.style),
            Atom::Image { size, .. } => size.1,
        }
    }

    /// A part of a line placed on its bottom.
    fn mark_of(&self, atom: Atom, left: f32, bottom: f32) -> Mark {
        let height = self.height_of(&atom);
        match atom {
            Atom::Text { text, look, x, width } => Mark::Text {
                text,
                look,
                origin: Point::new(left + x, bottom - height),
                width,
            },
            Atom::Image { image, link, x, size } => Mark::Image {
                rect: Rect::new(left + x, bottom - size.1, size.0, size.1),
                image,
                link,
            },
        }
    }
}

/// The words of a text, each with the spaces after it; a line break counts as a space.
fn words(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = text;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let word_end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let spaces = rest
            .get(word_end..)
            .unwrap_or_default()
            .find(|character: char| !character.is_whitespace())
            .map_or(rest.len(), |found| word_end + found);
        let (word, after) = rest.split_at(spaces);
        rest = after;
        Some(word)
    })
}

/// A size of an image, in pixels, as logical pixels: an image is drawn one of its pixels to one of the page.
fn pixels(size: u32) -> f32 {
    crate::number::float_of_count(usize::try_from(size).unwrap_or(usize::MAX))
}
