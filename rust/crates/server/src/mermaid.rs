//! `read/file-view.mermaid` (1.9, D60): a diagram of Mermaid written as an SVG for VS Code's preview of Markdown, which
//! shows it as an image. The extension knows only the kind of the theme of VS Code, not its colors, so each kind is
//! drawn in the colors of VS Code's default theme of that kind; the labels are in Mermaid's own typeface, measured as a
//! browser measures it, since the preview draws the SVG with the fonts of the system. No repository is read.

use protocol::mermaid::{MermaidJson, MermaidParams, ThemeKindJson};
use protocol::rpc::ResponseError;
use serde_json::Value;
use text::mermaid::{Font, Ground, Look, Palette, Rgb};

use crate::session::json;

/// Mermaid's own typeface, which the library measures labels in when it is given no measure of its own.
const MERMAID_FONT: &str = "\"trebuchet ms\", verdana, arial, sans-serif";

/// The size of the text of a diagram, Mermaid's own.
const FONT_SIZE: f32 = 16.0;

/// The diagram of a block of Mermaid in a kind of theme, or why it is not drawn.
///
/// # Errors
///
/// When the answer cannot be written as JSON.
pub fn draw(params: &MermaidParams) -> Result<Value, ResponseError> {
    json(&MermaidJson::from(text::mermaid::diagram(
        &params.source,
        &look(params.theme),
    )))
}

/// How a diagram looks in a kind of theme: the colors of VS Code's default theme of that kind, the ground of the editor,
/// its text, its muted text for the lines, its focus border for the borders of shapes, the ground of its inputs inside
/// them.
fn look(theme: ThemeKindJson) -> Look {
    let (ground, palette) = match theme {
        ThemeKindJson::Dark => (
            Ground::Dark,
            palette([
                [0x1f, 0x1f, 0x1f],
                [0xcc, 0xcc, 0xcc],
                [0x9d, 0x9d, 0x9d],
                [0x00, 0x78, 0xd4],
                [0x31, 0x31, 0x31],
                [0x3c, 0x3c, 0x3c],
            ]),
        ),
        ThemeKindJson::Light => (
            Ground::Light,
            palette([
                [0xff, 0xff, 0xff],
                [0x3b, 0x3b, 0x3b],
                [0x61, 0x61, 0x61],
                [0x00, 0x5f, 0xb8],
                [0xf8, 0xf8, 0xf8],
                [0xe8, 0xe8, 0xe8],
            ]),
        ),
        ThemeKindJson::HighContrastDark => (
            Ground::Dark,
            palette([
                [0x00, 0x00, 0x00],
                [0xff, 0xff, 0xff],
                [0xff, 0xff, 0xff],
                [0x6f, 0xc3, 0xdf],
                [0x00, 0x00, 0x00],
                [0x00, 0x00, 0x00],
            ]),
        ),
        ThemeKindJson::HighContrastLight => (
            Ground::Light,
            palette([
                [0xff, 0xff, 0xff],
                [0x29, 0x29, 0x29],
                [0x29, 0x29, 0x29],
                [0x0f, 0x4a, 0x85],
                [0xff, 0xff, 0xff],
                [0xff, 0xff, 0xff],
            ]),
        ),
    };
    Look {
        ground,
        palette,
        font: Font {
            family: String::from(MERMAID_FONT),
            size: FONT_SIZE,
            width: None,
        },
    }
}

/// A palette from its colors as red, green and blue: the ground, text, lines, borders, inside shapes and notes.
fn palette([background, text, line, border, fill, note]: [[u8; 3]; 6]) -> Palette {
    Palette {
        background: rgb(background),
        text: rgb(text),
        line: rgb(line),
        border: rgb(border),
        fill: rgb(fill),
        note: rgb(note),
    }
}

/// A color from its red, green and blue.
fn rgb([red, green, blue]: [u8; 3]) -> Rgb {
    Rgb { red, green, blue }
}
