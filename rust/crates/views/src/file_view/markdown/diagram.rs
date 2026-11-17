//! The diagrams of Mermaid of the preview (`file-view.mermaid`): a block fenced as `mermaid` laid out as its diagram,
//! drawn on a worker, as wide as it is or as the page; or, until it is drawn and when it cannot be, as its code, with
//! a line under it that says why.

use model::markdown::{Alignment, Piece, Span};
use ui::{AccessRole, Image, Rect};

use super::flow::{Place, flow};
use super::layout::{Column, Ink, Laid, Mark};
use crate::text::file_view as words;

/// A diagram of Mermaid drawn into pixels, with its size and what the screen reader says of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawnDiagram {
    /// Its pixels, at twice its size or so, to stay sharp on a screen of two pixels for each logical one.
    pub image: Image,
    /// Its width, in logical pixels.
    pub width: u32,
    /// Its height, in logical pixels.
    pub height: u32,
    /// Its title, from `accTitle`.
    pub title: Option<String>,
    /// Its description, from `accDescr`.
    pub description: Option<String>,
}

/// Why a diagram of Mermaid is not drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagramFailure {
    /// Its text has an error: what the library says of it.
    Invalid(String),
    /// It is of no kind of diagram the library draws.
    Unsupported,
    /// It is too large to draw.
    TooLarge,
}

/// What the preview shows of a diagram of Mermaid it asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagramShown {
    /// The diagram.
    Drawn(DrawnDiagram),
    /// Its code, and why it is not drawn.
    NotDrawn(DiagramFailure),
}

/// What is shown of the diagram of a text, in the theme the preview drew it for; none until it is drawn.
pub(super) fn shown<'a>(ink: &Ink<'a, '_>, source: &str) -> Option<&'a DiagramShown> {
    ink.diagrams
        .iter()
        .find(|(drawn, _)| drawn == source)
        .map(|(_, shown)| shown)
}

/// Lays a diagram drawn from a top, as wide as it is or as the column when it is wider, its height scaled with it, and
/// gives its bottom.
pub(super) fn lay_drawn(laid: &mut Laid, diagram: &DrawnDiagram, (column, top): (Column, f32)) -> f32 {
    let natural = (number(diagram.width), number(diagram.height));
    let width = natural.0.min(column.width).max(1.0);
    let height = natural.1 * width / natural.0.max(1.0);
    let rect = Rect::new(column.left, top, width, height);
    laid.marks.push(Mark::Image {
        rect,
        image: diagram.image.clone(),
        link: None,
    });
    laid.nodes.push((rect, AccessRole::Label, said(diagram)));
    rect.bottom()
}

/// Lays the line that says why a diagram is not drawn, under its code, from a top, and gives its bottom.
pub(super) fn lay_failure(
    ink: &Ink<'_, '_>,
    laid: &mut Laid,
    failure: &DiagramFailure,
    (column, top): (Column, f32),
) -> f32 {
    let text = match failure {
        DiagramFailure::Invalid(reason) => words::diagram_error(reason),
        DiagramFailure::Unsupported => words::DIAGRAM_UNSUPPORTED.to_owned(),
        DiagramFailure::TooLarge => words::DIAGRAM_TOO_LARGE.to_owned(),
    };
    let top = top + ink.looks.line_gap;
    let pieces = [Piece::Text(Span {
        text: text.clone(),
        look: model::markdown::Look::default(),
        link: None,
    })];
    let place = Place {
        left: column.left,
        width: column.width,
        top,
        style: ink.looks.body,
        color: ink.looks.muted,
        alignment: Alignment::Start,
    };
    let bottom = flow(ink, laid, &pieces, place);
    laid.nodes.push((
        Rect::new(column.left, top, column.width, (bottom - top).max(0.0)),
        AccessRole::Label,
        text,
    ));
    (bottom - ink.looks.line_gap).max(top)
}

/// What the screen reader says of a diagram: its title and its description, or that it is a diagram of Mermaid.
fn said(diagram: &DrawnDiagram) -> String {
    match (&diagram.title, &diagram.description) {
        (Some(title), Some(description)) => format!("{title}. {description}"),
        (Some(title), None) => title.clone(),
        (None, Some(description)) => format!("{}. {description}", words::DIAGRAM),
        (None, None) => words::DIAGRAM.to_owned(),
    }
}

/// A size in logical pixels as a length of layout.
fn number(size: u32) -> f32 {
    crate::number::float_of_count(usize::try_from(size).unwrap_or(usize::MAX))
}
