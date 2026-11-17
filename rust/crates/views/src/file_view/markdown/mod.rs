//! A Markdown file as it reads (`file-view.markdown-preview`): its blocks laid out as wide as the page, which scrolls;
//! a click on a link asks for where it goes, and find in page finds in the text shown.

mod diagram;
mod flow;
mod layout;
mod table;

use model::markdown::Block;
use ui::layout::{Insets, inset};
use ui::{AccessRole, Border, CodeSpan, Cursor, Description, Image, Point, Rect, Sense, TextRun, ThemeName, Ui};

use super::{FilePageId, FilePageIntent, FileViewIntent};
use crate::icons;
use crate::intent::Intent;
use crate::text::file_view as words;
pub use diagram::{DiagramFailure, DiagramShown, DrawnDiagram};
use layout::{Laid, Mark, TextLook, lay_out};

/// What the preview of a Markdown file shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkdownState<'a> {
    /// The page it is the preview of.
    pub id: FilePageId,
    /// What it lays out.
    pub content: &'a MarkdownContent,
}

/// What the preview of a Markdown file lays out, which the page keeps while it shows it.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct MarkdownContent {
    /// The blocks of the file.
    pub blocks: Vec<Block>,
    /// The colors of the code of each block of code, in the order they come in the file, by line.
    pub code_colors: Vec<Vec<Vec<CodeSpan>>>,
    /// The images of the repository read and drawn, by their source as the file writes it; the others show their
    /// text.
    pub images: Vec<(String, Image)>,
    /// The diagrams of Mermaid drawn, or not, for the theme the preview asked them in, by their text
    /// (`file-view.mermaid`); those not there yet show their code.
    pub diagrams: Vec<(String, DiagramShown)>,
    /// The theme the preview last asked its diagrams in.
    pub diagram_theme: Option<ThemeName>,
    /// A heading to scroll to, by its anchor, once for each number.
    pub reveal: Option<(u64, String)>,
}

/// Something asked of the preview of a Markdown file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarkdownIntent {
    /// **Preview**: the file laid out as it reads.
    ShowPreview,
    /// **Source**: the text of the file, to edit.
    ShowSource,
    /// A click on a link, which goes where it says as the file writes it.
    OpenLink(String),
    /// The diagrams of Mermaid of the preview, drawn in a theme: the one the window is drawn in, when the preview has
    /// diagrams and has not asked for them in it (`file-view.mermaid`).
    DrawDiagrams(ThemeName),
}

/// The preview of a Markdown file, over the rectangle its caller clipped it to: its blocks laid out one under the
/// other, scrolled, each link a place to click; find in page finds in its text.
///
/// Feature: file-view.markdown-preview
pub fn markdown_view(state: &MarkdownState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let padding = ui.theme().sizes.spacing.huge;
    let width = (rect.width - 2.0 * padding).max(1.0);
    let laid = lay_out(state, ui, width);
    let key = ("markdown preview", state.id);
    if let Some((serial, anchor)) = &state.content.reveal
        && let Some(top) = laid.anchor(anchor)
    {
        let _ = ui.scroll_to_once(key, *serial, top);
    }
    let size = (rect.width, laid.height + 2.0 * padding);
    let mut intents = Vec::new();
    let theme = ui.theme().name;
    if laid.has_diagrams && state.content.diagram_theme != Some(theme) {
        intents.push(Intent::FileView(FileViewIntent::Page(
            state.id,
            FilePageIntent::Markdown(MarkdownIntent::DrawDiagrams(theme)),
        )));
    }
    ui.region(key, (rect, AccessRole::Region, words::PREVIEW_REGION), |ui| {
        let (_, asked) = ui.scroll_area(key, rect, size, |ui, content| {
            let origin = Point::new(content.x + padding, content.y + padding);
            draw(state, &laid, ui, origin)
        });
        intents.extend(asked);
    });
    let _ = ui.find_in_page(("find in markdown preview", state.id), rect);
    intents
}

/// Draws what is laid out from an origin, in sight only, and reports the clicks on its links.
fn draw(state: &MarkdownState<'_>, laid: &Laid, ui: &mut Ui<'_>, origin: Point) -> Vec<Intent> {
    let clip = ui.current_clip();
    let mut intents = Vec::new();
    for (index, mark) in laid.marks.iter().enumerate() {
        let bounds = moved(bounds_of(ui, mark), origin);
        if bounds.intersection(clip).is_empty() {
            continue;
        }
        draw_mark(ui, mark, origin);
        if let Some(link) = link_of(mark) {
            let said = said_of(mark);
            if is_clicked(ui, (state.id, index), bounds, &said) {
                let href = laid.links.get(link).cloned().unwrap_or_default();
                intents.push(Intent::FileView(FileViewIntent::Page(
                    state.id,
                    FilePageIntent::Markdown(MarkdownIntent::OpenLink(href)),
                )));
            }
        }
    }
    for (index, (rect, role, text)) in laid.nodes.iter().enumerate() {
        let rect = moved(*rect, origin);
        if !rect.intersection(clip).is_empty() {
            let id = ui.id(("markdown block", state.id, index));
            ui.describe(id, rect, &Description::new(*role, text));
        }
    }
    intents
}

/// Whether a link was clicked: a place to click, with the hand over it, named for the screen reader.
fn is_clicked(ui: &mut Ui<'_>, key: (FilePageId, usize), rect: Rect, said: &str) -> bool {
    let id = ui.id(("markdown link", key));
    let response = ui.interact(id, rect, Sense::CLICK);
    ui.describe(id, rect, &Description::new(AccessRole::Button, said));
    if response.hovered {
        ui.set_cursor(Cursor::Hand);
    }
    response.clicked
}

/// Draws one mark from an origin.
fn draw_mark(ui: &mut Ui<'_>, mark: &Mark, origin: Point) {
    match mark {
        Mark::Text {
            text,
            look,
            origin: at,
            width,
        } => draw_text(ui, (text, *look), moved_point(*at, origin), *width),
        Mark::Code {
            text,
            style,
            color,
            origin: at,
            max_width,
        } => {
            let _ = ui.text(&TextRun {
                text,
                style: *style,
                color: *color,
                origin: moved_point(*at, origin),
                max_width: Some(*max_width),
            });
        }
        Mark::Fill { rect, color, radius } => ui.fill_rounded_rect(moved(*rect, origin), *radius, *color),
        Mark::Frame { rect, color, radius } => ui.border(
            moved(*rect, origin),
            Border {
                width: ui.theme().sizes.border,
                color: *color,
                radius: *radius,
            },
        ),
        Mark::Image { rect, image, .. } => ui.image(moved(*rect, origin), image),
        Mark::Check { rect, is_done } => draw_check(ui, moved(*rect, origin), *is_done),
    }
}

/// Draws a run of text: on its ground when it is code, struck through when it is.
fn draw_text(ui: &mut Ui<'_>, (text, look): (&str, TextLook), at: Point, width: f32) {
    let theme = *ui.theme();
    let height = ui.line_height(look.style);
    if look.is_code {
        let spacing = theme.sizes.spacing;
        let ground = Rect::new(at.x - spacing.tiny, at.y, width + 2.0 * spacing.tiny, height);
        ui.fill_rounded_rect(ground, theme.sizes.radius.small, theme.colors.input_background);
    }
    let _ = ui.text(&TextRun {
        text,
        style: look.style,
        color: look.color,
        origin: at,
        max_width: None,
    });
    if look.is_struck {
        let line = Rect::new(at.x, at.y + height / 2.0, width, theme.sizes.border);
        ui.fill_rect(line, look.color);
    }
}

/// Draws the box of a task, with a check in it when it is done.
fn draw_check(ui: &mut Ui<'_>, rect: Rect, is_done: bool) {
    let theme = *ui.theme();
    let radius = theme.sizes.radius.small;
    if is_done {
        ui.fill_rounded_rect(rect, radius, theme.colors.accent);
        let inset_by = rect.width / 8.0;
        let mark = inset(rect, Insets::all(inset_by));
        ui.icon(
            icons::CHOSEN,
            Point::new(mark.x, mark.y),
            mark.width,
            theme.colors.button_foreground,
        );
    } else {
        ui.border(
            rect,
            Border {
                width: theme.sizes.border,
                color: theme.colors.muted_foreground,
                radius,
            },
        );
    }
}

/// The rectangle a mark covers, before it is moved to its origin.
fn bounds_of(ui: &Ui<'_>, mark: &Mark) -> Rect {
    match mark {
        Mark::Text {
            look, origin, width, ..
        } => Rect::new(origin.x, origin.y, *width, ui.line_height(look.style)),
        Mark::Code {
            style,
            origin,
            max_width,
            ..
        } => Rect::new(origin.x, origin.y, *max_width, ui.line_height(*style)),
        Mark::Fill { rect, .. } | Mark::Frame { rect, .. } | Mark::Image { rect, .. } | Mark::Check { rect, .. } => {
            *rect
        }
    }
}

/// The link a mark is part of.
fn link_of(mark: &Mark) -> Option<usize> {
    match mark {
        Mark::Text { look, .. } => look.link,
        Mark::Image { link, .. } => *link,
        Mark::Code { .. } | Mark::Fill { .. } | Mark::Frame { .. } | Mark::Check { .. } => None,
    }
}

/// What a link says to the screen reader: its text, or that it is an image.
fn said_of(mark: &Mark) -> String {
    match mark {
        Mark::Text { text, .. } => text.trim().to_owned(),
        _ => words::IMAGE.to_owned(),
    }
}

/// A rectangle moved by an origin.
fn moved(rect: Rect, origin: Point) -> Rect {
    Rect::new(rect.x + origin.x, rect.y + origin.y, rect.width, rect.height)
}

/// A point moved by an origin.
fn moved_point(point: Point, origin: Point) -> Point {
    Point::new(point.x + origin.x, point.y + origin.y)
}
