#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::*;
use crate::color::Color;
use crate::frame::FrameBuffer;
use crate::theme::{Appearance, Theme, ThemeName};
use crate::{Assets, Canvas};

/// The name of the icon a file or a folder gets.
fn icon_name(asked: FileOrFolder<'_>) -> &'static str {
    FileIcons::default().icon_of(asked).name()
}

/// The four themes.
fn themes() -> [Theme; 4] {
    [
        ThemeName::Dark,
        ThemeName::Light,
        ThemeName::HighContrastDark,
        ThemeName::HighContrastLight,
    ]
    .map(|name| Theme::new(name, Appearance::Dark))
}

#[test]
fn file_view_file_icons__a_file_is_named_by_its_name_then_its_longest_extension_then_its_language() {
    let cases = [
        ("main.rs", "rust"),
        ("package.json", "node"),
        ("Dockerfile", "docker"),
        ("README.md", "readme"),
        ("notes.md", "markdown"),
        ("index.d.ts", "ts-types"),
        ("index.ts", "ts"),
        ("INDEX.PHP", "php"),
        ("Makefile", "gear"),
        (".gitignore", "git"),
        ("notes.xyz", "document"),
        ("LICENSE", "license"),
        ("notes", "document"),
    ];
    for (name, expected) in cases {
        assert_eq!(icon_name(FileOrFolder::File(name)), expected, "{name}");
    }
}

#[test]
fn file_view_file_icons__a_folder_is_named_by_its_name_whatever_its_case_and_the_top_folder_by_the_theme() {
    let cases = [
        ("src", "folder-orange-code"),
        ("SRC", "folder-orange-code"),
        ("node_modules", "folder-node-modules"),
        (".github", "folder-github"),
        ("docs", "folder-documents"),
        ("notes", "folder"),
    ];
    for (name, expected) in cases {
        assert_eq!(icon_name(FileOrFolder::Folder(name)), expected, "{name}");
    }
    assert_eq!(icon_name(FileOrFolder::Top), "folder-gray");
    assert!(FileIcons::default().icon_of(FileOrFolder::Folder("src")).is_folder());
    assert!(!FileIcons::default().icon_of(FileOrFolder::File("src")).is_folder());
}

#[test]
fn file_view_file_icons__every_entry_of_the_theme_names_an_icon_and_every_icon_draws_inside_its_square() {
    let mut entries: Vec<(&str, String)> = THEME_ENTRIES
        .lines()
        .filter(|line| !line.starts_with('#') && !line.starts_with("default"))
        .filter_map(|line| line.split('\t').next().zip(line.split('\t').nth(1)))
        .map(|(what, key)| {
            (
                what,
                if what == "language" {
                    key.to_owned()
                } else {
                    key.to_lowercase()
                },
            )
        })
        .collect();
    entries.sort();
    entries.dedup();
    assert_eq!(
        lookup::Lookup::bundled().count(),
        entries.len(),
        "every entry names an icon the program has"
    );
    let colors = FileIconColors::of_theme(&themes()[0]);
    for (set, sources) in [(Set::Files, &sources::FILES[..]), (Set::Folders, &sources::FOLDERS[..])] {
        for (index, (name, source)) in sources.iter().enumerate() {
            assert!(
                usvg::Tree::from_str(source, &usvg::Options::default()).is_ok(),
                "{name} is read"
            );
            let icon = FileIcon {
                set,
                index: u16::try_from(index).unwrap(),
            };
            assert_eq!(icon.name(), *name);
            for side in [16, 32] {
                let pixels = rasterize(source, &colors, side);
                let covered = pixels.as_chunks::<4>().0.iter().filter(|pixel| pixel[3] > 32).count();
                assert!(covered > 4, "{name} at {side} covers {covered} pixels");
            }
        }
    }
}

/// The text of the theme, to count its entries.
const THEME_ENTRIES: &str = include_str!("../../icons/symbols/theme.txt");

/// The contrast of two colors, as WCAG 2 measures it, from 1 to 21.
fn contrast(one: Color, other: Color) -> f32 {
    let luminance = |color: Color| {
        let linear = |channel: u8| {
            let value = f32::from(channel) / 255.0;
            if value <= 0.040_45 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(color.red) + 0.7152 * linear(color.green) + 0.0722 * linear(color.blue)
    };
    let (one, other) = (luminance(one), luminance(other));
    (one.max(other) + 0.05) / (one.min(other) + 0.05)
}

#[test]
fn file_view_file_icons__each_family_of_hues_reads_on_the_ground_of_each_theme_and_gray_is_the_muted_text() {
    for theme in themes() {
        let colors = FileIconColors::of_theme(&theme);
        let least = match theme.name {
            ThemeName::HighContrastDark | ThemeName::HighContrastLight => 4.5,
            _ => 3.0,
        };
        for family in ColorFamily::ALL {
            let ratio = contrast(colors.color(family), theme.colors.page_background);
            assert!(ratio >= least, "{family:?} in {:?}: {ratio}", theme.name);
        }
        assert_eq!(colors.color(ColorFamily::Gray), theme.colors.muted_foreground);
        assert_eq!(colors.color(ColorFamily::Blue), theme.file_icons.blue);
    }
    let families = [
        (0x64_74_8B, ColorFamily::Gray),
        (0xFF_FF_FF, ColorFamily::Gray),
        (0xF8_71_71, ColorFamily::Red),
        (0xEA_58_0C, ColorFamily::Orange),
        (0xF5_9E_0B, ColorFamily::Yellow),
        (0x22_C5_5E, ColorFamily::Green),
        (0x2D_D4_BF, ColorFamily::Teal),
        (0x0E_A5_E9, ColorFamily::Sky),
        (0x25_63_EB, ColorFamily::Blue),
        (0xC0_84_FC, ColorFamily::Violet),
        (0xF4_72_B6, ColorFamily::Pink),
    ];
    for (color, family) in families {
        assert_eq!(ColorFamily::of(Color::hex(color)), family, "{color:06X}");
    }
}

/// The color of the middle of the gear of the icon of Rust drawn at 32 pixels in some colors.
fn rust_pixel(colors: FileIconColors) -> Color {
    let rust = FileIcons::default().icon_of(FileOrFolder::File("main.rs"));
    let mut frame = FrameBuffer::new(32, 32);
    let mut assets = Assets::new();
    let mut canvas = Canvas::new(&mut frame, &mut assets, 1.0);
    let look = FileIconLook { colors, opacity: 255 };
    canvas.draw_file_icon(rust, Point::new(0.0, 0.0), 32.0, &look);
    // A point on the ring of the gear, which the icon fills in its one color.
    frame.pixel(4, 16).unwrap()
}

#[test]
fn file_view_file_icons__the_icon_of_rust_is_drawn_in_the_orange_of_each_theme() {
    for theme in themes() {
        let drawn = rust_pixel(FileIconColors::of_theme(&theme));
        let orange = theme.file_icons.orange;
        for (one, other) in [
            (drawn.red, orange.red),
            (drawn.green, orange.green),
            (drawn.blue, orange.blue),
        ] {
            assert!(one.abs_diff(other) <= 2, "{:?}: {drawn:?} for {orange:?}", theme.name);
        }
    }
}

#[test]
fn file_view_file_icons__in_one_color_every_icon_is_drawn_in_that_color() {
    let muted = themes()[0].colors.muted_foreground;
    let drawn = rust_pixel(FileIconColors::single(muted));
    for (one, other) in [
        (drawn.red, muted.red),
        (drawn.green, muted.green),
        (drawn.blue, muted.blue),
    ] {
        assert!(one.abs_diff(other) <= 2, "{drawn:?} for {muted:?}");
    }
    let shades = FileIconColors::single(muted);
    let lighter = shades.recolor(Color::hex(0x93_C5_FD));
    let darker = shades.recolor(Color::hex(0x25_63_EB));
    assert!(
        lighter.red > darker.red,
        "two shades of a family stay apart: {lighter:?} {darker:?}"
    );
}

#[test]
fn file_view_file_icons__an_icon_is_drawn_into_pixels_once_for_each_size_scale_and_theme() {
    let [dark, light, ..] = themes();
    let mut frame = FrameBuffer::new(64, 64);
    let mut assets = Assets::new();
    let rust = assets.file_icons.icon_of(FileOrFolder::File("main.rs"));
    let look = |theme: &Theme| FileIconLook {
        colors: FileIconColors::of_theme(theme),
        opacity: 255,
    };
    {
        let mut canvas = Canvas::new(&mut frame, &mut assets, 1.0);
        canvas.draw_file_icon(rust, Point::new(0.0, 0.0), 16.0, &look(&dark));
        canvas.draw_file_icon(rust, Point::new(0.0, 20.0), 16.0, &look(&dark));
    }
    assert_eq!(assets.file_icons.drawn.len(), 1, "a second row draws nothing new");
    {
        let mut canvas = Canvas::new(&mut frame, &mut assets, 2.0);
        canvas.draw_file_icon(rust, Point::new(0.0, 0.0), 16.0, &look(&dark));
    }
    assert_eq!(assets.file_icons.drawn.len(), 2, "another scale draws it again");
    {
        let mut canvas = Canvas::new(&mut frame, &mut assets, 1.0);
        canvas.draw_file_icon(rust, Point::new(0.0, 0.0), 16.0, &look(&light));
        canvas.draw_file_icon(rust, Point::new(0.0, 0.0), 16.0, &look(&light));
    }
    assert_eq!(assets.file_icons.drawn.len(), 3, "another theme draws it again, once");
}

#[test]
fn an_icon_drawn_faded_takes_part_of_its_opacity() {
    let theme = themes()[0];
    let mut frame = FrameBuffer::new(32, 32);
    let mut assets = Assets::new();
    let rust = assets.file_icons.icon_of(FileOrFolder::File("main.rs"));
    let mut canvas = Canvas::new(&mut frame, &mut assets, 1.0);
    let look = FileIconLook {
        colors: FileIconColors::of_theme(&theme),
        opacity: 0,
    };
    canvas.draw_file_icon(rust, Point::new(0.0, 0.0), 32.0, &look);
    assert_eq!(frame.pixel(4, 16), Some(Color::BLACK));
}
