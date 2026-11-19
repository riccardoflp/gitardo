//! The blocks of a Markdown file laid out one under the other, as wide as the page: what is drawn of each, where, and
//! where its headings and its links are.

use model::markdown::{Alignment, Block, CodeBlock, Item, Mark as ItemMark, Piece};
use ui::{AccessRole, CodeSpan, Color, Image, Point, Rect, TextKind, TextStyle, Ui};

use super::MarkdownContent;
use super::diagram::{self, DiagramShown};
use super::flow::{Place, flow};

/// What the blocks are measured and drawn with.
pub(super) struct Ink<'a, 'u> {
    /// The frame, to measure text.
    pub(super) ui: &'a Ui<'u>,
    /// The images of the repository read, by their source as the file writes it.
    pub(super) images: &'a [(String, Image)],
    /// The colors of the code of each block of code, in order, by line.
    pub(super) code_colors: &'a [Vec<Vec<CodeSpan>>],
    /// The diagrams of Mermaid drawn, or not, by their text.
    pub(super) diagrams: &'a [(String, DiagramShown)],
    /// The typefaces, colors and spaces.
    pub(super) looks: Looks,
}

/// The typefaces, colors and spaces of the preview, from the theme.
#[derive(Clone, Copy, Debug)]
pub(super) struct Looks {
    /// The text.
    pub(super) body: TextStyle,
    /// Strong text.
    pub(super) strong: TextStyle,
    /// Emphasized text: in the medium weight, the window having no italic.
    pub(super) emphasis: TextStyle,
    /// Code.
    pub(super) code: TextStyle,
    /// The color of text.
    pub(super) text: Color,
    /// The color of quotes and of the text of an image not drawn.
    pub(super) muted: Color,
    /// The color of links.
    pub(super) link: Color,
    /// The color of lines and borders.
    pub(super) border: Color,
    /// Behind code.
    pub(super) fill: Color,
    /// Between two lines of a block.
    pub(super) line_gap: f32,
    /// Between two blocks.
    pub(super) block_gap: f32,
    /// Around code, and inside the cells of tables.
    pub(super) padding: f32,
    /// The room of the marks of a list.
    pub(super) indent: f32,
    /// The width of lines.
    pub(super) line: f32,
    /// The radius of the corners of code.
    pub(super) radius: f32,
}

impl Looks {
    /// The looks of a frame's theme.
    pub(super) fn of(ui: &Ui<'_>) -> Looks {
        let theme = ui.theme();
        let spacing = theme.sizes.spacing;
        Looks {
            body: ui.text_style(TextKind::Body),
            strong: ui.text_style(TextKind::Strong),
            emphasis: ui.text_style(TextKind::Medium),
            code: ui.text_style(TextKind::Code),
            text: theme.colors.foreground,
            muted: theme.colors.muted_foreground,
            link: theme.colors.accent,
            border: theme.colors.border,
            fill: theme.colors.input_background,
            line_gap: spacing.small,
            block_gap: spacing.large,
            padding: spacing.medium,
            indent: spacing.huge + spacing.small,
            line: theme.sizes.border,
            radius: theme.sizes.radius.small,
        }
    }
}

/// How a run of text of the preview looks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TextLook {
    /// Its typeface and size.
    pub(super) style: TextStyle,
    /// Its color.
    pub(super) color: Color,
    /// The link it is part of, by its place among the links of the preview.
    pub(super) link: Option<usize>,
    /// Whether a line strikes it through.
    pub(super) is_struck: bool,
    /// Whether it is code, drawn on a ground of its own.
    pub(super) is_code: bool,
}

/// Something the preview draws, in logical pixels from the top left of its text.
#[derive(Clone, Debug)]
pub(super) enum Mark {
    /// A run of text on one line.
    Text {
        /// The text.
        text: String,
        /// How it looks.
        look: TextLook,
        /// Its top left corner.
        origin: Point,
        /// Its width.
        width: f32,
    },
    /// A run of code of a block, cut with an ellipsis at the edge of its block.
    Code {
        /// The text.
        text: String,
        /// Its typeface and size.
        style: TextStyle,
        /// Its color.
        color: Color,
        /// Its top left corner.
        origin: Point,
        /// The widest it may be.
        max_width: f32,
    },
    /// A rectangle filled.
    Fill {
        /// Where.
        rect: Rect,
        /// The color.
        color: Color,
        /// The radius of its corners.
        radius: f32,
    },
    /// The border of a rectangle, which tells a ground apart in the themes of high contrast.
    Frame {
        /// Where.
        rect: Rect,
        /// The color.
        color: Color,
        /// The radius of its corners.
        radius: f32,
    },
    /// An image.
    Image {
        /// Where.
        rect: Rect,
        /// The image.
        image: Image,
        /// The link it is inside.
        link: Option<usize>,
    },
    /// The box of a task, ticked when it is done.
    Check {
        /// Where.
        rect: Rect,
        /// Whether it is ticked.
        is_done: bool,
    },
}

/// What is laid out: the marks drawn, the links, the headings, what the screen reader is told, and the height.
#[derive(Debug, Default)]
pub(super) struct Laid {
    /// What is drawn, in order.
    pub(super) marks: Vec<Mark>,
    /// Where each link goes, as the file writes it.
    pub(super) links: Vec<String>,
    /// The anchor and the top of each heading.
    pub(super) anchors: Vec<(String, f32)>,
    /// The text of each block, where it is and what it is, for the screen reader.
    pub(super) nodes: Vec<(Rect, AccessRole, String)>,
    /// How many blocks of code are laid, to take the colors of the next.
    pub(super) code_count: usize,
    /// Whether a block is a diagram of Mermaid.
    pub(super) has_diagrams: bool,
    /// The bottom of the last block.
    pub(super) height: f32,
}

impl Laid {
    /// The number of a new link to a place.
    pub(super) fn link(&mut self, href: &str) -> usize {
        self.links.push(href.to_owned());
        self.links.len() - 1
    }

    /// The top of the heading of an anchor.
    pub(super) fn anchor(&self, anchor: &str) -> Option<f32> {
        self.anchors
            .iter()
            .find(|(name, _)| name == anchor)
            .map(|(_, top)| *top)
    }
}

/// A column the blocks are laid in, and the color of its text.
#[derive(Clone, Copy, Debug)]
pub(super) struct Column {
    /// Its left.
    pub(super) left: f32,
    /// Its width.
    pub(super) width: f32,
    /// The color of its text.
    pub(super) color: Color,
}

/// Lays out the blocks of a preview in a width, from the top.
pub(super) fn lay_out(content: &MarkdownContent, ui: &Ui<'_>, width: f32) -> Laid {
    let ink = Ink {
        ui,
        images: &content.images,
        code_colors: &content.code_colors,
        diagrams: &content.diagrams,
        looks: Looks::of(ui),
    };
    let mut laid = Laid::default();
    let column = Column {
        left: 0.0,
        width,
        color: ink.looks.text,
    };
    laid.height = blocks(&ink, &mut laid, &content.blocks, (column, 0.0));
    laid
}

/// Lays blocks one under the other from a top, and gives the bottom of the last.
pub(super) fn blocks(ink: &Ink<'_, '_>, laid: &mut Laid, blocks: &[Block], (column, top): (Column, f32)) -> f32 {
    let mut bottom = top;
    for (index, block) in blocks.iter().enumerate() {
        let start = if index == 0 { top } else { bottom + ink.looks.block_gap };
        bottom = self::block(ink, laid, block, (column, start));
    }
    bottom
}

/// Lays one block from a top, and gives its bottom.
fn block(ink: &Ink<'_, '_>, laid: &mut Laid, block: &Block, (column, top): (Column, f32)) -> f32 {
    match block {
        Block::Heading { level, text, anchor } => {
            laid.anchors.push((anchor.clone(), top));
            heading(ink, laid, (*level, text), (column, top))
        }
        Block::Paragraph(pieces) => text_block(ink, laid, (pieces, ink.looks.body, AccessRole::Label), (column, top)),
        Block::List(items) => list(ink, laid, items, (column, top)),
        Block::Quote(inner) => quote(ink, laid, inner, (column, top)),
        Block::Code(code) => code_block(ink, laid, code, (column, top)),
        Block::Rule => {
            let rect = Rect::new(column.left, top + ink.looks.padding, column.width, ink.looks.line);
            laid.marks.push(fill(rect, ink.looks.border));
            rect.bottom() + ink.looks.padding
        }
        Block::Table(table) => super::table::table(ink, laid, table, (column, top)),
    }
}

/// Lays the text of a paragraph or a heading in a style, said to the screen reader in a role.
fn text_block(
    ink: &Ink<'_, '_>,
    laid: &mut Laid,
    (pieces, style, role): (&[Piece], TextStyle, AccessRole),
    (column, top): (Column, f32),
) -> f32 {
    let place = Place {
        left: column.left,
        width: column.width,
        top,
        style,
        color: column.color,
        alignment: Alignment::Start,
    };
    let bottom = flow(ink, laid, pieces, place);
    let rect = Rect::new(column.left, top, column.width, (bottom - top).max(0.0));
    laid.nodes.push((rect, role, Block::plain_text(pieces)));
    (bottom - ink.looks.line_gap).max(top)
}

/// Lays a heading in the strong typeface at the size of its level; the first two levels have a line under them.
fn heading(ink: &Ink<'_, '_>, laid: &mut Laid, (level, pieces): (u8, &[Piece]), (column, top): (Column, f32)) -> f32 {
    let looks = ink.looks;
    let heading = ink.ui.text_style(TextKind::Heading).size;
    let size = match level {
        1 => heading + (heading - looks.body.size),
        2 => heading,
        3 => f32::midpoint(heading, looks.body.size),
        _ => looks.body.size,
    };
    let style = TextStyle {
        typeface: looks.strong.typeface,
        size,
    };
    let top = if top > 0.0 { top + looks.padding } else { top };
    let bottom = text_block(ink, laid, (pieces, style, AccessRole::Heading), (column, top));
    if level > 2 {
        return bottom;
    }
    let rule = Rect::new(column.left, bottom + looks.padding, column.width, looks.line);
    laid.marks.push(fill(rule, looks.border));
    rule.bottom()
}

/// Lays a list: each item's mark in the room at its left, a bullet, its number or the box of its task, and its
/// blocks beside it.
fn list(ink: &Ink<'_, '_>, laid: &mut Laid, items: &[Item], (column, top): (Column, f32)) -> f32 {
    let looks = ink.looks;
    let inner = Column {
        left: column.left + looks.indent,
        width: (column.width - looks.indent).max(1.0),
        ..column
    };
    let mut bottom = top;
    for (index, item) in items.iter().enumerate() {
        let start = if index == 0 { top } else { bottom + looks.line_gap };
        item_mark(ink, laid, item.mark, (column, start));
        bottom = blocks(ink, laid, &item.blocks, (inner, start)).max(start + ink.ui.line_height(looks.body));
    }
    bottom
}

/// The mark of an item of a list, at the left of its first line.
fn item_mark(ink: &Ink<'_, '_>, laid: &mut Laid, mark: ItemMark, (column, top): (Column, f32)) {
    let looks = ink.looks;
    let line_height = ink.ui.line_height(looks.body);
    let right = column.left + looks.indent - looks.padding;
    let said = match mark {
        ItemMark::Bullet => String::from("\u{2022}"),
        ItemMark::Number(number) => format!("{number}."),
        ItemMark::Task { is_done } => {
            let side = looks.body.size;
            let rect = Rect::new(right - side, top + (line_height - side) / 2.0, side, side);
            laid.marks.push(Mark::Check { rect, is_done });
            return;
        }
    };
    let width = ink.ui.fonts().measure(&said, looks.body);
    laid.marks.push(Mark::Text {
        text: said,
        look: TextLook {
            style: looks.body,
            color: looks.muted,
            link: None,
            is_struck: false,
            is_code: false,
        },
        origin: Point::new(right - width, top),
        width,
    });
}

/// Lays a quote: a bar at its left, and its blocks beside it in the muted color.
fn quote(ink: &Ink<'_, '_>, laid: &mut Laid, inner: &[Block], (column, top): (Column, f32)) -> f32 {
    let looks = ink.looks;
    let bar = looks.padding / 2.0;
    let inside = Column {
        left: column.left + bar + looks.padding + looks.padding / 2.0,
        width: (column.width - bar - looks.padding - looks.padding / 2.0).max(1.0),
        color: looks.muted,
    };
    let start = laid.marks.len();
    let bottom = blocks(ink, laid, inner, (inside, top));
    let rect = Rect::new(column.left, top, bar, (bottom - top).max(0.0));
    laid.marks.insert(start, fill(rect, looks.border));
    bottom
}

/// Lays a block of code: a diagram of Mermaid as its diagram once it is drawn (`file-view.mermaid`), its code with why
/// under it when it cannot be, else the code.
fn code_block(ink: &Ink<'_, '_>, laid: &mut Laid, code: &CodeBlock, (column, top): (Column, f32)) -> f32 {
    let colors = ink.code_colors.get(laid.code_count);
    laid.code_count += 1;
    if !code.is_mermaid() {
        return code_lines(ink, laid, (code, colors), (column, top));
    }
    laid.has_diagrams = true;
    match diagram::shown(ink, &code.text) {
        Some(DiagramShown::Drawn(drawn)) => diagram::lay_drawn(laid, drawn, (column, top)),
        Some(DiagramShown::NotDrawn(failure)) => {
            let bottom = code_lines(ink, laid, (code, colors), (column, top));
            diagram::lay_failure(ink, laid, failure, (column, bottom))
        }
        None => code_lines(ink, laid, (code, colors), (column, top)),
    }
}

/// Lays the lines of a block of code on its ground, each in the colors of its language, cut at the edge of the block.
fn code_lines(
    ink: &Ink<'_, '_>,
    laid: &mut Laid,
    (code, colors): (&CodeBlock, Option<&Vec<Vec<CodeSpan>>>),
    (column, top): (Column, f32),
) -> f32 {
    let looks = ink.looks;
    let line_height = ink.ui.line_height(looks.code);
    let start = laid.marks.len();
    let left = column.left + looks.padding;
    let right = column.left + column.width - looks.padding;
    let mut y = top + looks.padding;
    for (index, line) in code.text.split('\n').enumerate() {
        let spans = colors.and_then(|lines| lines.get(index)).map_or(&[][..], Vec::as_slice);
        code_line(ink, laid, (line, spans), (Point::new(left, y), right));
        y += line_height;
    }
    let bottom = y + looks.padding;
    let rect = Rect::new(column.left, top, column.width, bottom - top);
    let ground = Mark::Fill {
        rect,
        color: looks.fill,
        radius: looks.radius,
    };
    let frame = Mark::Frame {
        rect,
        color: looks.border,
        radius: looks.radius,
    };
    let _none_replaced: Vec<Mark> = laid.marks.splice(start..start, [ground, frame]).collect();
    laid.nodes.push((rect, AccessRole::Label, code.text.clone()));
    bottom
}

/// Lays a line of code: its tokens in their colors, the text between them in the color of text.
fn code_line(ink: &Ink<'_, '_>, laid: &mut Laid, (line, spans): (&str, &[CodeSpan]), (origin, right): (Point, f32)) {
    let theme = ink.ui.theme();
    let plain = ink.looks.text;
    let mut parts: Vec<(&str, Color)> = Vec::new();
    let mut at = 0;
    for span in spans {
        if span.range.start > at {
            parts.push((line.get(at..span.range.start).unwrap_or_default(), plain));
        }
        let color = theme.code.color(span.kind, plain);
        parts.push((line.get(span.range.clone()).unwrap_or_default(), color));
        at = span.range.end.max(at);
    }
    parts.push((line.get(at..).unwrap_or_default(), plain));
    let mut x = origin.x;
    for (text, color) in parts {
        let text = text.replace('\t', "    ");
        if text.is_empty() || x >= right {
            continue;
        }
        let width = ink.ui.fonts().measure(&text, ink.looks.code);
        laid.marks.push(Mark::Code {
            text,
            style: ink.looks.code,
            color,
            origin: Point::new(x, origin.y),
            max_width: right - x,
        });
        x += width;
    }
}

/// A rectangle filled with a color, square.
pub(super) fn fill(rect: Rect, color: Color) -> Mark {
    Mark::Fill {
        rect,
        color,
        radius: 0.0,
    }
}
