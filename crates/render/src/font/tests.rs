//! The font reader against the fonts kept for the tests.

use super::*;

fn font(name: &str) -> Font {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../testkit/fonts")
        .join(name);
    Font::from_bytes(std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn the_tables_of_a_font_are_read() {
    let sans = font("DejaVuSans.ttf");
    assert_eq!(sans.units_per_em(), 2048);
    let metrics = sans.vertical_metrics();
    assert!(metrics.ascender > 1500 && metrics.descender < -300, "{metrics:?}");
    assert!(sans.glyph_count() > 3000);
}

#[test]
fn characters_have_glyphs_with_advances_and_outlines() {
    let sans = font("DejaVuSans.ttf");
    let letter = sans.glyph_id('A').unwrap();
    assert!(sans.advance(letter) > 1000);
    let bounds = sans.outline(letter).unwrap().bounds().unwrap();
    assert!(bounds.y >= -1.0 && bounds.bottom() > 1400.0, "{bounds:?}");
    let space = sans.glyph_id(' ').unwrap();
    assert!(sans.outline(space).is_none());
    assert!(sans.glyph_id('\u{10FFFD}').is_none());
    assert!(sans.glyph_id('ж').is_some());
}

#[test]
fn composite_glyphs_are_drawn_from_their_components() {
    let sans = font("DejaVuSans.ttf");
    let plain = sans.outline(sans.glyph_id('e').unwrap()).unwrap().bounds().unwrap();
    let accented = sans.outline(sans.glyph_id('é').unwrap()).unwrap().bounds().unwrap();
    assert!(accented.bottom() > plain.bottom() + 200.0, "{plain:?} {accented:?}");
}

#[test]
fn kerning_brings_pairs_closer() {
    let sans = font("DejaVuSans.ttf");
    let (a, v) = (sans.glyph_id('A').unwrap(), sans.glyph_id('V').unwrap());
    assert!(sans.kerning(a, v) < 0, "{}", sans.kerning(a, v));
    let (o, x) = (sans.glyph_id('o').unwrap(), sans.glyph_id('x').unwrap());
    assert!(sans.kerning(o, o) == 0 || sans.kerning(x, x) == 0);
}

#[test]
fn every_test_font_reads_and_bad_bytes_do_not() {
    for name in ["DejaVuSans.ttf", "DejaVuSans-Bold.ttf", "DejaVuSansMono.ttf"] {
        let font = font(name);
        assert!(
            font.glyph_id('g').and_then(|glyph| font.outline(glyph)).is_some(),
            "{name}"
        );
    }
    assert_eq!(
        Font::from_bytes(b"not a font at all".to_vec()).unwrap_err(),
        Error::NotAFont
    );
    assert_eq!(
        Font::from_bytes(b"OTTO\0\0\0\0\0\0\0\0".to_vec()).unwrap_err(),
        Error::UnsupportedOutlines
    );
    let mut cut =
        std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../testkit/fonts/DejaVuSans.ttf"))
            .unwrap();
    cut.truncate(5000);
    assert!(Font::from_bytes(cut).is_err());
}

#[test]
fn kerning_is_read_from_gpos_and_from_kern_alike() {
    let data = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../testkit/fonts/DejaVuSans.ttf"))
        .unwrap();
    let file = Reader::new(&data);
    let directory = TableDirectory::read(file, 0).unwrap();
    let from_gpos = Kerning::read(directory.table(file, "GPOS"), None);
    let from_kern = Kerning::read(None, directory.table(file, "kern"));
    let sans = Font::from_bytes(data.clone()).unwrap();
    let pairs = [('A', 'V'), ('T', 'o'), ('L', 'T'), ('Y', 'a')];
    for (first, second) in pairs {
        let (first, second) = (sans.glyph_id(first).unwrap(), sans.glyph_id(second).unwrap());
        let gpos = from_gpos.between(first, second);
        assert!(gpos < 0, "{first:?} {second:?}: {gpos}");
        assert_eq!(gpos, from_kern.between(first, second), "{first:?} {second:?}");
    }
}
