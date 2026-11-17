//! The blocks of a Markdown text, from the events of its parser: each block opened is kept on a stack until it ends,
//! then given to the block that holds it.

use pulldown_cmark::{Alignment as ParsedAlignment, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use super::{Alignment, Block, CodeBlock, Item, Look, Mark, Piece, Span, Table};

/// The blocks of a text, in order.
pub(super) fn blocks(text: &str) -> Vec<Block> {
    let options = Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH;
    let mut builder = Builder::default();
    for event in Parser::new_ext(text, options) {
        builder.take(event);
    }
    builder.root
}

/// A block opened and not ended yet.
#[derive(Debug)]
enum Open {
    /// A quote, with its blocks so far.
    Quote(Vec<Block>),
    /// A list, with the number of its next item when it is numbered.
    List {
        /// The number of the next item.
        next: Option<u64>,
        /// The items so far.
        items: Vec<Item>,
    },
    /// An item of a list, with its blocks so far and the text not in a paragraph of a list without blank lines.
    Item {
        /// What it starts with.
        mark: Mark,
        /// Its blocks so far.
        blocks: Vec<Block>,
        /// Its text not yet in a block.
        pieces: Vec<Piece>,
    },
    /// A paragraph.
    Paragraph(Vec<Piece>),
    /// A heading.
    Heading {
        /// Its level.
        level: u8,
        /// Its text so far.
        pieces: Vec<Piece>,
    },
    /// A block of code, or of HTML.
    Code(CodeBlock),
    /// A table, with the row being read and whether its head is.
    Table {
        /// The table so far.
        table: Table,
        /// The cells of the row being read.
        row: Vec<Vec<Piece>>,
        /// Whether the cells read are of the head.
        is_head: bool,
    },
    /// A cell of a table.
    Cell(Vec<Piece>),
    /// An image, whose text is being read.
    Image {
        /// Where it is.
        source: String,
        /// Its text so far.
        text: String,
        /// The link it is inside.
        link: Option<String>,
    },
}

/// The blocks read so far.
#[derive(Debug, Default)]
struct Builder {
    /// The blocks of the text that have ended.
    root: Vec<Block>,
    /// The blocks opened, the innermost last.
    open: Vec<Open>,
    /// How deep the strong, emphasized and struck text is, which tells the look of the next text.
    depths: [u32; 3],
    /// The links the text is inside, the innermost last.
    links: Vec<String>,
    /// The anchors of the headings so far, to tell apart two headings of the same text.
    anchors: Vec<String>,
}

impl Builder {
    /// Takes an event of the parser.
    fn take(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(end) => self.end(end),
            Event::Code(text) => self.text(&text, true),
            Event::SoftBreak => self.text(" ", false),
            Event::HardBreak => self.push_piece(Piece::Break),
            Event::Rule => {
                self.flush_item();
                self.push_block(Block::Rule);
            }
            Event::TaskListMarker(is_done) => self.mark_task(is_done),
            // HTML, and what the options leave out, is shown as the text it is.
            Event::Text(text)
            | Event::Html(text)
            | Event::InlineHtml(text)
            | Event::InlineMath(text)
            | Event::DisplayMath(text)
            | Event::FootnoteReference(text) => self.text(&text, false),
        }
    }

    /// Opens a block, or a look or a link of text.
    fn start(&mut self, tag: Tag<'_>) {
        if is_block(&tag) {
            self.flush_item();
        }
        let opened = match tag {
            Tag::Paragraph => Open::Paragraph(Vec::new()),
            Tag::Heading { level, .. } => Open::Heading {
                level: u8::try_from(level as usize).unwrap_or(6),
                pieces: Vec::new(),
            },
            Tag::BlockQuote(_) => Open::Quote(Vec::new()),
            Tag::CodeBlock(kind) => Open::Code(CodeBlock {
                language: match kind {
                    CodeBlockKind::Fenced(info) => info.split_whitespace().next().unwrap_or_default().to_owned(),
                    CodeBlockKind::Indented => String::new(),
                },
                text: String::new(),
            }),
            Tag::HtmlBlock => Open::Code(CodeBlock {
                language: String::from("html"),
                text: String::new(),
            }),
            Tag::List(start) => Open::List {
                next: start,
                items: Vec::new(),
            },
            Tag::Item => self.item(),
            Tag::Table(alignments) => Open::Table {
                table: Table {
                    alignments: alignments.iter().copied().map(alignment).collect(),
                    head: Vec::new(),
                    rows: Vec::new(),
                },
                row: Vec::new(),
                is_head: false,
            },
            Tag::TableCell => Open::Cell(Vec::new()),
            Tag::Image { dest_url, .. } => Open::Image {
                source: dest_url.to_string(),
                text: String::new(),
                link: self.links.last().cloned(),
            },
            tag => return self.start_inline(tag),
        };
        self.open.push(opened);
    }

    /// Opens a look or a link of text, or the head or a row of a table; what the options leave out opens nothing.
    fn start_inline(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Strong => self.depths[0] += 1,
            Tag::Emphasis => self.depths[1] += 1,
            Tag::Strikethrough => self.depths[2] += 1,
            Tag::Link { dest_url, .. } => self.links.push(dest_url.to_string()),
            Tag::TableHead => self.set_head(true),
            // Every other tag is a block handled above, or one the options leave out, whose text is read as it comes.
            _ => {}
        }
    }

    /// An item of the list opened: its number, and the next number of the list.
    fn item(&mut self) -> Open {
        let mark = match self.open.last_mut() {
            Some(Open::List { next: Some(next), .. }) => {
                let mark = Mark::Number(*next);
                *next += 1;
                mark
            }
            _ => Mark::Bullet,
        };
        Open::Item {
            mark,
            blocks: Vec::new(),
            pieces: Vec::new(),
        }
    }

    /// Ends what is open.
    fn end(&mut self, end: TagEnd) {
        match end {
            TagEnd::Strong => self.depths[0] = self.depths[0].saturating_sub(1),
            TagEnd::Emphasis => self.depths[1] = self.depths[1].saturating_sub(1),
            TagEnd::Strikethrough => self.depths[2] = self.depths[2].saturating_sub(1),
            TagEnd::Link => {
                let _ = self.links.pop();
            }
            TagEnd::TableHead => self.set_head(false),
            TagEnd::TableRow => self.end_row(),
            TagEnd::Item => self.end_item(),
            TagEnd::Paragraph
            | TagEnd::Heading(_)
            | TagEnd::BlockQuote(_)
            | TagEnd::CodeBlock
            | TagEnd::HtmlBlock
            | TagEnd::List(_)
            | TagEnd::Table
            | TagEnd::TableCell
            | TagEnd::Image => self.end_open(),
            // The ends of the tags the options leave out, which opened nothing.
            _ => {}
        }
    }

    /// Ends the block open last, and gives it to the block that holds it.
    fn end_open(&mut self) {
        let Some(open) = self.open.pop() else {
            return;
        };
        match open {
            Open::Paragraph(pieces) => self.push_block(Block::Paragraph(pieces)),
            Open::Heading { level, pieces } => {
                let anchor = self.anchor(&Block::plain_text(&pieces));
                self.push_block(Block::Heading {
                    level,
                    text: pieces,
                    anchor,
                });
            }
            Open::Quote(blocks) => self.push_block(Block::Quote(blocks)),
            Open::Code(mut code) => {
                if code.text.ends_with('\n') {
                    let _ = code.text.pop();
                }
                self.push_block(Block::Code(code));
            }
            Open::List { items, .. } => self.push_block(Block::List(items)),
            Open::Table { table, .. } => self.push_block(Block::Table(table)),
            Open::Cell(pieces) => self.end_cell(pieces),
            Open::Image { source, text, link } => self.push_piece(Piece::Image { source, text, link }),
            // An item ends with its own tag, which is read in `end_item`.
            Open::Item { .. } => self.open.push(open),
        }
    }

    /// Ends an item of a list, its text not in a paragraph made one.
    fn end_item(&mut self) {
        self.flush_item();
        if let Some(Open::Item { mark, blocks, .. }) = self.open.pop()
            && let Some(Open::List { items, .. }) = self.open.last_mut()
        {
            items.push(Item { mark, blocks });
        }
    }

    /// Ends a cell of a table, in its head or in the row being read.
    fn end_cell(&mut self, pieces: Vec<Piece>) {
        if let Some(Open::Table { table, row, is_head }) = self.open.last_mut() {
            if *is_head {
                table.head.push(pieces);
            } else {
                row.push(pieces);
            }
        }
    }

    /// Ends a row of a table.
    fn end_row(&mut self) {
        if let Some(Open::Table { table, row, .. }) = self.open.last_mut() {
            table.rows.push(std::mem::take(row));
        }
    }

    /// Starts or ends the head of the table open.
    fn set_head(&mut self, is_now: bool) {
        if let Some(Open::Table { is_head, .. }) = self.open.last_mut() {
            *is_head = is_now;
        }
    }

    /// Makes the text of the item open that is not in a block yet a paragraph of it, before a block of the item starts
    /// or the item ends.
    fn flush_item(&mut self) {
        if let Some(Open::Item { blocks, pieces, .. }) = self.open.last_mut()
            && !pieces.is_empty()
        {
            blocks.push(Block::Paragraph(std::mem::take(pieces)));
        }
    }

    /// Ticks or leaves the box of the item open.
    fn mark_task(&mut self, is_done: bool) {
        let item = self
            .open
            .iter_mut()
            .rev()
            .find(|open| matches!(open, Open::Item { .. }));
        if let Some(Open::Item { mark, .. }) = item {
            *mark = Mark::Task { is_done };
        }
    }

    /// Gives a block to the quote or the item open, or to the text itself.
    fn push_block(&mut self, block: Block) {
        let holder = self
            .open
            .iter_mut()
            .rev()
            .find(|open| matches!(open, Open::Quote(_) | Open::Item { .. }));
        match holder {
            Some(Open::Quote(blocks) | Open::Item { blocks, .. }) => blocks.push(block),
            _ => self.root.push(block),
        }
    }

    /// Text in the look and the link it is in: into the code or the image open, or the text of the block open.
    fn text(&mut self, text: &str, is_code: bool) {
        match self.open.last_mut() {
            Some(Open::Code(code)) => code.text.push_str(text),
            Some(Open::Image { text: said, .. }) => said.push_str(text),
            _ => {
                let look = Look {
                    is_strong: self.depths[0] > 0,
                    is_emphasis: self.depths[1] > 0,
                    is_struck: self.depths[2] > 0,
                    is_code,
                };
                let link = self.links.last().cloned();
                self.push_piece(Piece::Text(Span {
                    text: text.to_owned(),
                    look,
                    link,
                }));
            }
        }
    }

    /// Gives a part of text to the block open, joined to the text before it when they look the same.
    fn push_piece(&mut self, piece: Piece) {
        // Text outside any block of text is not made by the options taken; it is left out.
        let Some(
            Open::Paragraph(pieces) | Open::Heading { pieces, .. } | Open::Cell(pieces) | Open::Item { pieces, .. },
        ) = self.open.last_mut()
        else {
            return;
        };
        if let (Some(Piece::Text(last)), Piece::Text(span)) = (pieces.last_mut(), &piece)
            && last.look == span.look
            && last.link == span.link
        {
            last.text.push_str(&span.text);
            return;
        }
        pieces.push(piece);
    }

    /// The anchor of a heading, as GitHub makes it: in lower case, its spaces dashes, without the punctuation, and a
    /// number after it when a heading before had the same.
    fn anchor(&mut self, text: &str) -> String {
        let base: String = text
            .trim()
            .to_lowercase()
            .chars()
            .filter_map(|character| match character {
                ' ' => Some('-'),
                '-' | '_' => Some(character),
                _ if character.is_alphanumeric() => Some(character),
                _ => None,
            })
            .collect();
        let before = self.anchors.iter().filter(|anchor| **anchor == base).count();
        self.anchors.push(base.clone());
        if before == 0 { base } else { format!("{base}-{before}") }
    }
}

/// Whether a tag opens a block, before which the text of an item becomes a paragraph.
fn is_block(tag: &Tag<'_>) -> bool {
    matches!(
        tag,
        Tag::Paragraph
            | Tag::Heading { .. }
            | Tag::BlockQuote(_)
            | Tag::CodeBlock(_)
            | Tag::HtmlBlock
            | Tag::List(_)
            | Tag::Table(_)
    )
}

/// The alignment of a column as the parser reads it.
fn alignment(parsed: ParsedAlignment) -> Alignment {
    match parsed {
        ParsedAlignment::None | ParsedAlignment::Left => Alignment::Start,
        ParsedAlignment::Center => Alignment::Center,
        ParsedAlignment::Right => Alignment::End,
    }
}
