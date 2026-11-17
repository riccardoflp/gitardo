//! A diagram of Mermaid of the preview of a Markdown file drawn on a worker (`file-view.mermaid`): laid out as an SVG
//! in the colors of a theme and with its labels measured in Inter (`text::mermaid`), then drawn into pixels with the
//! fonts of the program (`render::Image::from_svg`).

use std::sync::{Arc, Mutex, PoisonError};

use render::{Appearance, Color, FontFamily, FontWeight, Fonts, TextStyle, Theme, ThemeName};
use text::mermaid::{self, Font, Ground, Look, NotDrawn, Palette, Rgb, TextWidth, Weight};
use views::file_view::{DiagramFailure, DrawnDiagram};

/// How many pixels a diagram has for each logical pixel: two, so that it stays sharp on a screen that has as many, and
/// is drawn smoothly scaled down on one that has fewer.
const SCALE: f32 = 2.0;

/// A diagram to draw: its text and the theme it is drawn in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagramJob {
    /// The text of the block, without its fence.
    pub source: String,
    /// The theme of the window.
    pub theme: ThemeName,
}

/// Draws a diagram in its theme; why it is not drawn when its text has an error, is of a kind not drawn or is too large.
///
/// # Errors
///
/// Why the diagram is not drawn, which the preview says under its code.
pub fn draw_diagram(job: &DiagramJob) -> Result<DrawnDiagram, DiagramFailure> {
    let diagram = mermaid::diagram(&job.source, &look(job.theme)).map_err(|not_drawn| match not_drawn {
        NotDrawn::Invalid(reason) => DiagramFailure::Invalid(reason),
        NotDrawn::Unsupported => DiagramFailure::Unsupported,
        NotDrawn::TooLarge => DiagramFailure::TooLarge,
    })?;
    // An SVG the library wrote that cannot be drawn says nothing more useful than that it is too large to draw.
    let drawn = render::Image::from_svg(&diagram.svg, SCALE).ok_or(DiagramFailure::TooLarge)?;
    Ok(DrawnDiagram {
        image: drawn.image,
        width: drawn.width,
        height: drawn.height,
        title: diagram.title,
        description: diagram.description,
    })
}

/// How a diagram looks in a theme: on the ground of the page, its text in the color of text, its lines muted, the
/// borders of its shapes in the accent, inside them the ground of code; its labels in Inter at the size of text.
fn look(name: ThemeName) -> Look {
    let theme = Theme::new(name, Appearance::Dark);
    let colors = theme.colors;
    let ground = match name {
        ThemeName::Light | ThemeName::HighContrastLight => Ground::Light,
        ThemeName::Dark | ThemeName::HighContrastDark | ThemeName::System => Ground::Dark,
    };
    Look {
        ground,
        palette: Palette {
            background: rgb(colors.page_background),
            text: rgb(colors.foreground),
            line: rgb(colors.muted_foreground),
            border: rgb(colors.accent),
            fill: rgb(colors.input_background),
            note: rgb(colors.list_inactive_selection),
        },
        font: Font {
            family: String::from("Inter"),
            size: theme.sizes.font.body,
            width: text_width(),
        },
    }
}

/// A color of the theme as the diagram takes it.
fn rgb(color: Color) -> Rgb {
    Rgb {
        red: color.red,
        green: color.green,
        blue: color.blue,
    }
}

/// How Inter measures a line of text, regular or bold, as the window measures it; none when the fonts of the program
/// cannot make its typefaces, which measures as a browser measures Mermaid's own typeface.
fn text_width() -> Option<TextWidth> {
    let mut fonts = Fonts::bundled();
    let regular = fonts.add_typeface(&FontFamily::Interface, FontWeight::REGULAR).ok()?;
    let bold = fonts.add_typeface(&FontFamily::Interface, FontWeight::BOLD).ok()?;
    let fonts = Mutex::new(fonts);
    Some(Arc::new(move |text: &str, size: f64, weight: Weight| {
        let typeface = match weight {
            Weight::Regular => regular,
            Weight::Bold => bold,
        };
        // The fonts are only measured with: a measure that panicked left nothing half done in them.
        let fonts = fonts.lock().unwrap_or_else(PoisonError::into_inner);
        f64::from(fonts.measure(
            text,
            TextStyle {
                typeface,
                size: narrow(size),
            },
        ))
    }))
}

/// A size of text the library gives in `f64`, as the `f32` of the fonts.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the sizes of text of a diagram are a few dozen pixels, which an f32 holds"
)]
fn narrow(size: f64) -> f32 {
    size as f32
}
