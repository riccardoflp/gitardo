//! The diagrams of Mermaid (`file-view.mermaid`, D60): the text of a block fenced as `mermaid` read and laid out as
//! Mermaid 12 does, by the library `merman`, and written as an SVG in the colors and the typeface it is given. Nothing
//! of the SVG runs or reaches out: labels written in HTML are kept as their text, and no `foreignObject`, script or
//! image is written. None of the library's types leaves this module.

mod accessible;

#[cfg(test)]
mod tests;

use std::fmt;
use std::sync::Arc;

use merman::svg::{
    DeterministicTextMeasurer, DiagramThemeCompiler, DiagramThemeSpec, MeasurementProfileId, MermaidThemeCompatibility,
    SvgPipeline, TextMeasurementPolicy, TextMeasurementProfile, TextMeasurementProfileIdentity, TextStyle,
};
use merman::{
    MermaidThemeId, OperationControl, RenderError, RenderOutput, RenderRequest, Renderer, SvgEnvironment, SvgRequest,
};

/// A color, as red, green and blue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb {
    /// Red, from 0 to 255.
    pub red: u8,
    /// Green, from 0 to 255.
    pub green: u8,
    /// Blue, from 0 to 255.
    pub blue: u8,
}

impl Rgb {
    /// The color as CSS writes it: `#1e1e1e`.
    fn css(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
    }
}

/// Whether a diagram is drawn on a dark ground or a light one, which the colors Mermaid derives follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ground {
    /// Light text on a dark ground.
    Dark,
    /// Dark text on a light ground.
    Light,
}

/// The colors a diagram is drawn in, named for what they are in the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Palette {
    /// The ground of the page under the diagram.
    pub background: Rgb,
    /// Text.
    pub text: Rgb,
    /// The lines between shapes.
    pub line: Rgb,
    /// The borders of shapes.
    pub border: Rgb,
    /// Inside shapes.
    pub fill: Rgb,
    /// Inside notes.
    pub note: Rgb,
}

/// How heavy the text measured is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Weight {
    /// Running text.
    Regular,
    /// Bold text, such as a title.
    Bold,
}

/// The width of a line of text in pixels, from its text, its size in pixels and its weight, as the typeface the diagram
/// is drawn in measures it.
pub type TextWidth = Arc<dyn Fn(&str, f64, Weight) -> f64 + Send + Sync>;

/// The typeface of the labels of a diagram.
#[derive(Clone)]
pub struct Font {
    /// The name of its family, as the SVG names it: `Inter`.
    pub family: String,
    /// Its size, in pixels.
    pub size: f32,
    /// How it measures text, so that labels fit their shapes; none to measure as a browser measures Mermaid's own
    /// typeface, for an SVG a browser draws.
    pub width: Option<TextWidth>,
}

impl fmt::Debug for Font {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Font")
            .field("family", &self.family)
            .field("size", &self.size)
            .field("is_measured", &self.width.is_some())
            .finish()
    }
}

/// How a diagram looks: its ground, its colors and its typeface.
#[derive(Clone, Debug)]
pub struct Look {
    /// Dark or light.
    pub ground: Ground,
    /// The colors.
    pub palette: Palette,
    /// The typeface of its labels.
    pub font: Font,
}

/// A diagram drawn: its SVG, and what the screen reader says of it.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagram {
    /// The SVG, as text.
    pub svg: String,
    /// Its title, from `accTitle`.
    pub title: Option<String>,
    /// Its description, from `accDescr`.
    pub description: Option<String>,
    /// Its width, in pixels, from its `viewBox`; 0 when it has none.
    pub width: f64,
    /// Its height, in pixels, from its `viewBox`.
    pub height: f64,
}

/// Why a block of Mermaid is not drawn, which the preview says under its code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotDrawn {
    /// The text has an error: what the library says of it.
    Invalid(String),
    /// The text is of no kind of diagram the library draws.
    Unsupported,
    /// The diagram would take more than the library allows one diagram.
    TooLarge,
}

/// A block of Mermaid drawn as an SVG in a look: its shapes laid out as Mermaid 12 lays them out, in the colors and
/// the typeface of the look, with no link, script, HTML or image that could run or be fetched.
///
/// # Errors
///
/// [`NotDrawn`] when the text has an error, is of no kind of diagram the library draws, or is too large.
///
/// Feature: file-view.mermaid
pub fn diagram(source: &str, look: &Look) -> Result<Diagram, NotDrawn> {
    let request = SvgRequest {
        environment: environment(&look.font),
        // The SVG of a browser draws its labels as HTML inside `foreignObject`; this one keeps their text, which a
        // library that draws SVG can draw and which runs nothing.
        pipeline: Some(SvgPipeline::resvg_safe()),
        ..SvgRequest::default()
    };
    let mut request = RenderRequest::svg(source, OperationControl::new(), request);
    if let Some(theme) = theme(look) {
        request = request.with_theme(theme);
    }
    match Renderer::new().render(request) {
        Ok(RenderOutput::Svg(Some(output))) => {
            let svg = output.svg().to_owned();
            let (width, height) = size_of(&svg);
            Ok(Diagram {
                width,
                height,
                title: accessible::text_of(&svg, "title"),
                description: accessible::text_of(&svg, "desc"),
                svg,
            })
        }
        Ok(_) => Err(NotDrawn::Unsupported),
        Err(error) => Err(not_drawn(&error)),
    }
}

/// The width and the height of the `viewBox` of the root of an SVG, which the library writes as wide as its page
/// (`width="100%"`) and as large as the diagram; nothing when the root has none.
fn size_of(svg: &str) -> (f64, f64) {
    let root = svg.split_once('>').map_or(svg, |(root, _)| root);
    let Some((_, after)) = root.split_once(" viewBox=\"") else {
        return (0.0, 0.0);
    };
    let numbers: Vec<f64> = after
        .split('"')
        .next()
        .unwrap_or_default()
        .split([' ', ','])
        .filter_map(|number| number.parse().ok())
        .collect();
    match numbers.as_slice() {
        [_, _, width, height] if *width > 0.0 && *height > 0.0 => (*width, *height),
        _ => (0.0, 0.0),
    }
}

/// Why the library did not draw a diagram, in the words the preview says.
fn not_drawn(error: &RenderError) -> NotDrawn {
    match error {
        RenderError::NoDiagram => NotDrawn::Unsupported,
        RenderError::ResourceLimitExceeded(_) => NotDrawn::TooLarge,
        RenderError::Parse(diagnostic) if diagnostic.is_resource_limit() => NotDrawn::TooLarge,
        RenderError::Parse(diagnostic) => {
            let details = diagnostic.terminal_diagnostic_details();
            if details.code == "merman.parse.no_diagram_detected" || details.code.contains("unknown_diagram") {
                NotDrawn::Unsupported
            } else {
                NotDrawn::Invalid(reason(&diagnostic.terminal_safe_message()))
            }
        }
        // Another failure of the library, such as a layout it could not finish, is said in its own words, as an error
        // of the text is, so that the code shown under it says why.
        other => NotDrawn::Invalid(reason(&other.to_string())),
    }
}

/// What the library says of an error, without the kind of diagram it starts with and on one line:
/// `Diagram parse error (flowchart-v2): Unterminated node label` is `Unterminated node label`.
fn reason(message: &str) -> String {
    let line = message.lines().next().unwrap_or_default();
    let said = match line.split_once("): ") {
        Some((start, rest)) if start.starts_with("Diagram parse error") => rest,
        _ => line,
    };
    said.trim().trim_end_matches('.').to_owned()
}

/// The measures of the text of a diagram: those of its typeface when the look gives them, else those a browser takes
/// for Mermaid's own.
fn environment(font: &Font) -> SvgEnvironment {
    let deterministic = SvgEnvironment::deterministic();
    let Some(width) = font.width.clone() else {
        return deterministic;
    };
    let measurer = DeterministicTextMeasurer::default()
        .with_width_callback(move |text: &str, style: &TextStyle| width(text, style.font_size.max(1.0), weight(style)));
    // The identity names the typeface, so that the library keeps no measure of another typeface for it.
    let identity = MeasurementProfileId::new("gitardo.font")
        .ok()
        .and_then(|id| TextMeasurementProfileIdentity::new(id, format!("1:{}", font.family)).ok());
    match identity {
        Some(identity) => deterministic.with_text_measurement_policy(TextMeasurementPolicy::uniform(
            TextMeasurementProfile::new(identity, measurer),
        )),
        None => deterministic,
    }
}

/// How heavy the text of a style is: bold from 600 up, or when it says `bold`.
fn weight(style: &TextStyle) -> Weight {
    let is_bold = style
        .font_weight
        .as_deref()
        .is_some_and(|said| said.eq_ignore_ascii_case("bold") || said.parse::<u16>().is_ok_and(|number| number >= 600));
    if is_bold { Weight::Bold } else { Weight::Regular }
}

/// The theme of Mermaid a look asks for: its theme `base`, whose colors follow the variables it is given, on a dark
/// ground or a light one; none when the library refuses the variables, which draws Mermaid's own colors.
fn theme(look: &Look) -> Option<merman::svg::DiagramTheme> {
    let palette = look.palette;
    let variables = [
        ("background", palette.background),
        ("primaryColor", palette.fill),
        ("secondaryColor", palette.fill),
        ("tertiaryColor", palette.background),
        ("mainBkg", palette.fill),
        ("primaryTextColor", palette.text),
        ("secondaryTextColor", palette.text),
        ("tertiaryTextColor", palette.text),
        ("textColor", palette.text),
        ("titleColor", palette.text),
        ("primaryBorderColor", palette.border),
        ("secondaryBorderColor", palette.border),
        ("tertiaryBorderColor", palette.border),
        ("nodeBorder", palette.border),
        ("lineColor", palette.line),
        ("clusterBkg", palette.background),
        ("clusterBorder", palette.line),
        ("edgeLabelBackground", palette.background),
        ("noteBkgColor", palette.note),
        ("noteTextColor", palette.text),
        ("noteBorderColor", palette.line),
        ("actorBkg", palette.fill),
        ("actorBorder", palette.border),
        ("actorTextColor", palette.text),
        ("actorLineColor", palette.line),
        ("signalColor", palette.line),
        ("signalTextColor", palette.text),
        ("labelBoxBkgColor", palette.fill),
        ("labelBoxBorderColor", palette.border),
        ("labelTextColor", palette.text),
        ("loopTextColor", palette.text),
        ("sequenceNumberColor", palette.background),
    ];
    let mut compatibility = MermaidThemeCompatibility::default()
        .with_theme_id(MermaidThemeId::parse("base").ok()?)
        .with_dark_mode(look.ground == Ground::Dark)
        .ok()?;
    for (name, color) in variables {
        compatibility = compatibility.with_variable(name, color.css()).ok()?;
    }
    compatibility = compatibility
        .with_variable("fontFamily", look.font.family.clone())
        .ok()?
        .with_variable("fontSize", format!("{}px", look.font.size))
        .ok()?;
    DiagramThemeCompiler::new()
        .compile(DiagramThemeSpec::new().with_mermaid_compatibility(compatibility))
        .ok()
}
