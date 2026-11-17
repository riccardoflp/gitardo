//! The JSON of what version 1.9 adds for milestone M17 (`file-view.mermaid`, D60): `read/file-view.mermaid`, a diagram
//! of Mermaid written as an SVG in the colors of a kind of theme of VS Code, for its preview of Markdown. It reads no
//! repository, so it takes none.

use serde::{Deserialize, Serialize};
use text::mermaid::{Diagram, NotDrawn};

/// The names of the methods of the diagrams of Mermaid.
pub mod methods {
    /// `file-view.mermaid`: a diagram of Mermaid as an SVG (version 1.9).
    pub const FILE_VIEW_MERMAID: &str = "read/file-view.mermaid";
}

/// The kind of theme of VS Code a diagram is drawn for, as `ColorThemeKind` has it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemeKindJson {
    /// A dark theme.
    Dark,
    /// A light theme.
    Light,
    /// A dark theme of high contrast.
    HighContrastDark,
    /// A light theme of high contrast.
    HighContrastLight,
}

/// The parameters of `read/file-view.mermaid`: the text of the block, without its fence, and the kind of theme.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MermaidParams {
    /// The text of the block.
    pub source: String,
    /// The kind of theme of VS Code.
    pub theme: ThemeKindJson,
}

/// The answer of `read/file-view.mermaid`: the diagram, or why it is not drawn.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum MermaidJson {
    /// The diagram, its SVG with nothing that runs or is fetched, its size and what the screen reader says of it.
    Drawn {
        /// The SVG, as text.
        svg: String,
        /// Its width, in pixels.
        width: f64,
        /// Its height, in pixels.
        height: f64,
        /// Its title, from `accTitle`.
        title: Option<String>,
        /// Its description, from `accDescr`.
        description: Option<String>,
    },
    /// The text has an error: what the library says of it.
    Invalid {
        /// What the library says, on one line.
        reason: String,
    },
    /// The text is of no kind of diagram the library draws.
    Unsupported,
    /// The diagram is too large to draw.
    TooLarge,
}

impl From<Result<Diagram, NotDrawn>> for MermaidJson {
    fn from(drawn: Result<Diagram, NotDrawn>) -> MermaidJson {
        match drawn {
            Ok(diagram) => MermaidJson::Drawn {
                svg: diagram.svg,
                width: diagram.width,
                height: diagram.height,
                title: diagram.title,
                description: diagram.description,
            },
            Err(NotDrawn::Invalid(reason)) => MermaidJson::Invalid { reason },
            Err(NotDrawn::Unsupported) => MermaidJson::Unsupported,
            Err(NotDrawn::TooLarge) => MermaidJson::TooLarge,
        }
    }
}
