//! Measuring, cutting and wrapping lines of text in the fonts kept for the tests.

use super::layout::{self, Shape};
use super::*;

fn font(name: &str) -> Font {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../testkit/fonts")
        .join(name);
    Font::from_bytes(std::fs::read(path).unwrap()).unwrap()
}

/// Fonts with a typeface of the sans-serif font alone, and one of the monospaced font falling back to it.
fn fonts() -> (Fonts, TextStyle, TextStyle) {
    let mut fonts = Fonts::new();
    let sans = fonts.add_font(font("DejaVuSans.ttf")).unwrap();
    let mono = fonts.add_font(font("DejaVuSansMono.ttf")).unwrap();
    let sans_only = fonts.add_typeface(&[sans]).unwrap();
    let mono_then_sans = fonts.add_typeface(&[mono, sans]).unwrap();
    let style = |typeface| TextStyle { typeface, size: 13.0 };
    (fonts, style(sans_only), style(mono_then_sans))
}

fn shapes(fonts: &Fonts, text: &str, style: TextStyle) -> Vec<Shape> {
    let mut shapes = Vec::new();
    let _ = layout::place(fonts, text, style, |placed| shapes.push(placed.shape));
    shapes
}

#[test]
fn lines_are_measured_with_their_kerning() {
    let (fonts, sans, mono) = fonts();
    let together = fonts.measure("AV", sans);
    let apart = fonts.measure("A", sans) + fonts.measure("V", sans);
    assert!(together < apart - 0.1, "{together} {apart}");
    assert!((fonts.measure("iiii", mono) - fonts.measure("MMMM", mono)).abs() < 1e-3);
    assert!(fonts.measure("", sans).abs() < f32::EPSILON);
    let metrics = fonts.line_metrics(sans);
    assert!(
        metrics.ascent > 11.0 && metrics.ascent < 13.0 && metrics.height > 14.0,
        "{metrics:?}"
    );
}

#[test]
fn a_character_the_first_font_lacks_comes_from_the_next_and_one_none_has_is_a_box() {
    let (fonts, sans, mono) = fonts();
    let mono_font = FontId(1);
    let sans_font = FontId(0);
    let character = (0x2000..0x3000).filter_map(char::from_u32).find(|character| {
        fonts.font(mono_font).unwrap().glyph_id(*character).is_none()
            && fonts.font(sans_font).unwrap().glyph_id(*character).is_some()
    });
    let character = character.expect("a character DejaVu Sans has and DejaVu Sans Mono lacks");
    let text = format!("a{character}");
    let placed = shapes(&fonts, &text, mono);
    assert!(matches!(placed[0], Shape::Glyph { font, .. } if font == mono_font));
    assert!(matches!(placed[1], Shape::Glyph { font, .. } if font == sans_font));
    assert_eq!(shapes(&fonts, "\u{E000}", sans), vec![Shape::Missing]);
    assert_eq!(shapes(&fonts, "\t\n", sans), vec![Shape::Blank, Shape::Blank]);
}

#[test]
fn a_line_too_wide_is_cut_before_an_ellipsis() {
    let (fonts, sans, _) = fonts();
    let text = "Fix the parser of the configuration file";
    let whole = fonts.truncate(text, sans, 1000.0);
    assert_eq!((whole.shown, whole.is_cut), (text.len(), false));
    let cut = fonts.truncate(text, sans, 100.0);
    assert!(cut.is_cut && cut.width <= 100.0, "{cut:?}");
    let shown = &text[..cut.shown];
    assert!(
        !shown.ends_with(' ') && text.starts_with(shown) && !shown.is_empty(),
        "{shown:?}"
    );
    let nothing = fonts.truncate(text, sans, 2.0);
    assert_eq!(nothing.shown, 0);
}

#[test]
fn text_wraps_by_words_and_at_line_breaks() {
    let (fonts, sans, _) = fonts();
    let text = "Show the branches checked out last above the others\nand then the rest";
    let width = fonts.measure("Show the branches", sans) + 1.0;
    let lines: Vec<&str> = fonts
        .wrap(text, sans, width)
        .into_iter()
        .map(|range| &text[range])
        .collect();
    assert_eq!(
        lines,
        vec![
            "Show the branches",
            "checked out last",
            "above the others",
            "and then the rest"
        ]
    );
    for line in &lines {
        assert!(fonts.measure(line, sans) <= width, "{line}");
    }
    let long: Vec<&str> = fonts
        .wrap("abcdefghij", sans, fonts.measure("abcd", sans) + 0.5)
        .into_iter()
        .map(|range| &"abcdefghij"[range])
        .collect();
    assert_eq!(long, vec!["abcd", "efgh", "ij"]);
    assert_eq!(fonts.wrap("", sans, 50.0), vec![0..0]);
}
