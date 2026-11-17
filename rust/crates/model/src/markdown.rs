//! A Markdown file as it reads (`file-view.markdown-preview`): its text cut into the blocks the preview lays out, and
//! where its links go. `CommonMark` with the tables, the task lists and the strikethrough of GitHub; HTML in the file is
//! kept as text, never as markup, so that nothing in it runs.

mod links;
mod parse;

#[cfg(test)]
mod tests;

pub use links::{LinkTarget, code_file_name, image_path, link_target};

/// A block of a Markdown file, as it is laid out one under the other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// A heading, from 1, the largest, to 6.
    Heading {
        /// Its level, from 1 to 6.
        level: u8,
        /// What it says.
        text: Vec<Piece>,
        /// The name a link to it gives after its `#`, as GitHub names it: `getting-started`.
        anchor: String,
    },
    /// A paragraph.
    Paragraph(Vec<Piece>),
    /// A list, of bullets, of numbers or of tasks.
    List(Vec<Item>),
    /// A quote, of blocks.
    Quote(Vec<Block>),
    /// A block of code, or of HTML, shown as its text.
    Code(CodeBlock),
    /// A line across the page.
    Rule,
    /// A table.
    Table(Table),
}

/// An item of a list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// What it starts with.
    pub mark: Mark,
    /// What it holds.
    pub blocks: Vec<Block>,
}

/// What an item of a list starts with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mark {
    /// A bullet.
    Bullet,
    /// Its number.
    Number(u64),
    /// A box, ticked when the task is done.
    Task {
        /// Whether the box is ticked.
        is_done: bool,
    },
}

/// A block of code, with the language its fence names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    /// The first word after the fence, such as `rust`; empty when it names none.
    pub language: String,
    /// The text, without the line break after its last line.
    pub text: String,
}

/// A table, with the alignment of each column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// How each column is aligned.
    pub alignments: Vec<Alignment>,
    /// The cells of the head.
    pub head: Vec<Vec<Piece>>,
    /// The rows of the body, each of its cells.
    pub rows: Vec<Vec<Vec<Piece>>>,
}

/// How the cells of a column are aligned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Alignment {
    /// At the start, as text is when the table says nothing.
    Start,
    /// In the middle.
    Center,
    /// At the end.
    End,
}

/// A part of the text of a block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    /// Text in one look.
    Text(Span),
    /// An image: where it is, as the file writes it, and its text, shown when it is not drawn.
    Image {
        /// Where the image is, as the file writes it.
        source: String,
        /// Its text.
        text: String,
        /// Where a click on it goes, as the file writes it, when it is inside a link.
        link: Option<String>,
    },
    /// A line break inside the block.
    Break,
}

/// Text in one look, with the link it is part of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// The text.
    pub text: String,
    /// How it looks.
    pub look: Look,
    /// Where a click on it goes, as the file writes it, when it is part of a link.
    pub link: Option<String>,
}

/// How a span of text looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each is a look of its own, which a span has or not"
)]
pub struct Look {
    /// Strong: `**word**`.
    pub is_strong: bool,
    /// Emphasized: `*word*`.
    pub is_emphasis: bool,
    /// Struck through: `~~word~~`.
    pub is_struck: bool,
    /// Code: `` `word` ``.
    pub is_code: bool,
}

impl CodeBlock {
    /// Whether the block is a diagram of Mermaid: its fence names `mermaid`, in any case (`file-view.mermaid`).
    #[must_use]
    pub fn is_mermaid(&self) -> bool {
        self.language.eq_ignore_ascii_case("mermaid")
    }
}

impl Block {
    /// The text a block shows, its parts one after the other, for the screen reader and for find.
    #[must_use]
    pub fn plain_text(pieces: &[Piece]) -> String {
        let mut text = String::new();
        for piece in pieces {
            match piece {
                Piece::Text(span) => text.push_str(&span.text),
                Piece::Image { text: said, .. } => text.push_str(said),
                Piece::Break => text.push(' '),
            }
        }
        text
    }
}

/// The blocks of a Markdown text, in order, as its preview lays them out.
#[must_use]
pub fn blocks(text: &str) -> Vec<Block> {
    parse::blocks(text)
}

/// Whether a file is Markdown, by the end of its name: `.md` or `.markdown`, in any case.
#[must_use]
pub fn is_markdown(path: &str) -> bool {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.rsplit_once('.').is_some_and(|(stem, extension)| {
        !stem.is_empty() && (extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown"))
    })
}

/// The blocks of code of some blocks, in the order they come, those inside lists and quotes included: the order the
/// preview takes their colors in.
#[must_use]
pub fn code_blocks(blocks: &[Block]) -> Vec<&CodeBlock> {
    let mut found = Vec::new();
    for block in blocks {
        match block {
            Block::Code(code) => found.push(code),
            Block::Quote(inner) => found.extend(code_blocks(inner)),
            Block::List(items) => found.extend(items.iter().flat_map(|item| code_blocks(&item.blocks))),
            Block::Heading { .. } | Block::Paragraph(_) | Block::Rule | Block::Table(_) => {}
        }
    }
    found
}

/// The text of the diagrams of Mermaid of some blocks, each once, in the order they come (`file-view.mermaid`).
#[must_use]
pub fn mermaid_sources(blocks: &[Block]) -> Vec<&str> {
    let mut found: Vec<&str> = Vec::new();
    for code in code_blocks(blocks) {
        if code.is_mermaid() && !found.contains(&code.text.as_str()) {
            found.push(&code.text);
        }
    }
    found
}

/// Where the images of some blocks are, as the file writes it, each once, in the order they come.
#[must_use]
pub fn image_sources(blocks: &[Block]) -> Vec<&str> {
    let mut found: Vec<&str> = Vec::new();
    for piece in texts(blocks).into_iter().flatten() {
        if let Piece::Image { source, .. } = piece
            && !found.contains(&source.as_str())
        {
            found.push(source);
        }
    }
    found
}

/// The texts of some blocks, those inside lists, quotes and tables included, in the order they come.
fn texts(blocks: &[Block]) -> Vec<&[Piece]> {
    let mut found: Vec<&[Piece]> = Vec::new();
    for block in blocks {
        match block {
            Block::Heading { text, .. } | Block::Paragraph(text) => found.push(text),
            Block::Quote(inner) => found.extend(texts(inner)),
            Block::List(items) => found.extend(items.iter().flat_map(|item| texts(&item.blocks))),
            Block::Table(table) => {
                found.extend(table.head.iter().chain(table.rows.iter().flatten()).map(Vec::as_slice));
            }
            Block::Code(_) | Block::Rule => {}
        }
    }
    found
}
