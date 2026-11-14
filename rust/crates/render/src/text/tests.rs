//! Measuring, cutting and wrapping lines of text in the fonts that come with the program.

use super::*;

/// The fonts that come with the program, with a typeface of the interface font, its bold face, and the code font.
fn fonts() -> (Fonts, TextStyle, TextStyle, TextStyle) {
    let mut fonts = Fonts::bundled();
    let sans = fonts.add_typeface(&FontFamily::Interface, FontWeight::REGULAR).unwrap();
    let bold = fonts.add_typeface(&FontFamily::Interface, FontWeight::BOLD).unwrap();
    let mono = fonts.add_typeface(&FontFamily::Code, FontWeight::REGULAR).unwrap();
    let style = |typeface| TextStyle { typeface, size: 13.0 };
    (fonts, style(sans), style(bold), style(mono))
}

/// The family of the font each glyph of a line comes from, and the glyph's number in it.
fn glyph_sources(fonts: &Fonts, text: &str, style: TextStyle) -> Vec<(String, u16)> {
    let typeface = fonts.typeface(style.typeface).unwrap().face.clone();
    let mut engine = fonts.engine.borrow_mut();
    let glyphs = engine
        .shaped((style.typeface.0, 0), &typeface, text, style.size)
        .unwrap()
        .glyphs
        .clone();
    glyphs
        .iter()
        .map(|glyph| {
            let face = engine.system.db().face(glyph.font_id).unwrap();
            (face.families[0].0.clone(), glyph.glyph_id)
        })
        .collect()
}

#[test]
fn lines_are_measured_with_their_kerning() {
    let (fonts, sans, bold, mono) = fonts();
    let together = fonts.measure("AV", sans);
    let apart = fonts.measure("A", sans) + fonts.measure("V", sans);
    assert!(together < apart - 0.1, "{together} {apart}");
    assert!((fonts.measure("iiii", mono) - fonts.measure("MMMM", mono)).abs() < 1e-3);
    assert!(fonts.measure("Gitardo", bold) > fonts.measure("Gitardo", sans));
    assert!(fonts.measure("a ", sans) > fonts.measure("a", sans));
    assert!(fonts.measure("", sans).abs() < f32::EPSILON);
    let metrics = fonts.line_metrics(sans);
    assert!(
        metrics.ascent > 11.0 && metrics.ascent < 14.0 && metrics.height > 14.0 && metrics.height < 18.0,
        "{metrics:?}"
    );
}

#[test]
fn a_tab_is_as_wide_as_four_spaces() {
    let (fonts, _, _, mono) = fonts();
    let tab = fonts.measure("\t", mono);
    let spaces = fonts.measure("    ", mono);
    assert!((tab - spaces).abs() < 0.5, "{tab} {spaces}");
}

#[test]
fn a_character_the_font_lacks_comes_from_another_and_one_no_font_has_is_drawn_as_a_box() {
    let (fonts, sans, _, _) = fonts();
    // The APL symbol rho is in JetBrains Mono and not in Inter.
    let sources = glyph_sources(&fonts, "a\u{2374}", sans);
    assert_eq!(sources[0].0, "Inter");
    assert_eq!(sources[1].0, "JetBrains Mono");
    let missing = glyph_sources(&fonts, "\u{10FFFD}", sans);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].1, 0, "the glyph a font draws for a character it lacks");
    assert!(fonts.measure("\u{10FFFD}", sans) > 3.0);
}

#[test]
fn a_family_no_font_belongs_to_is_refused() {
    let mut fonts = Fonts::bundled();
    let error = fonts.add_typeface(&FontFamily::Named("No Such Font".to_owned()), FontWeight::REGULAR);
    assert_eq!(
        error,
        Err(Error::UnknownFamily {
            name: "No Such Font".to_owned()
        })
    );
    let nothing = Fonts::new().add_typeface(&FontFamily::Interface, FontWeight::REGULAR);
    assert!(nothing.is_err());
}

#[test]
fn a_line_too_wide_is_cut_before_an_ellipsis() {
    let (fonts, sans, _, _) = fonts();
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
    let (fonts, sans, _, _) = fonts();
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
    let word = "mmmmmmmmmm";
    let long: Vec<&str> = fonts
        .wrap(word, sans, fonts.measure("mmmm", sans) + 0.5)
        .into_iter()
        .map(|range| &word[range])
        .collect();
    assert_eq!(long, vec!["mmmm", "mmmm", "mm"]);
    assert_eq!(fonts.wrap("", sans, 50.0), vec![0..0]);
    assert_eq!(fonts.wrap("a\n\nb", sans, 50.0), vec![0..1, 2..2, 3..4]);
}
