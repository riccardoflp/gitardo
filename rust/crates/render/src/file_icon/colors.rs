//! The colors an icon of files is drawn in: each color its SVG names belongs to a family of hues, and is drawn in the
//! color the theme gives that family, lighter or darker as it was lighter or darker than the family's own shade in the
//! set, so that the parts of an icon drawn in two shades stay apart (`file-view.file-icons`).

use std::fmt::Write as _;
use std::str::FromStr as _;

use crate::color::Color;
use crate::theme::Theme;

/// A family of hues of the colors of the icons of Symbols, which draws them in about twenty colors of Tailwind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ColorFamily {
    /// Grays, and colors too pale to have a hue, white and black among them: the outline of most folders.
    Gray,
    /// Reds.
    Red,
    /// Oranges.
    Orange,
    /// Ambers and yellows.
    Yellow,
    /// Greens.
    Green,
    /// Teals.
    Teal,
    /// Light blues, toward cyan.
    Sky,
    /// Blues.
    Blue,
    /// Violets and purples.
    Violet,
    /// Pinks.
    Pink,
}

impl ColorFamily {
    /// Every family.
    pub const ALL: [ColorFamily; 10] = [
        ColorFamily::Gray,
        ColorFamily::Red,
        ColorFamily::Orange,
        ColorFamily::Yellow,
        ColorFamily::Green,
        ColorFamily::Teal,
        ColorFamily::Sky,
        ColorFamily::Blue,
        ColorFamily::Violet,
        ColorFamily::Pink,
    ];

    /// The family of a color, by its hue; a color whose saturation is low is gray.
    #[must_use]
    pub fn of(color: Color) -> ColorFamily {
        let (hue, saturation, lightness) = hsl(color);
        if saturation < 0.22 || !(0.06..=0.97).contains(&lightness) {
            return ColorFamily::Gray;
        }
        match hue {
            hue if !(12.0..345.0).contains(&hue) => ColorFamily::Red,
            hue if hue < 36.0 => ColorFamily::Orange,
            hue if hue < 62.0 => ColorFamily::Yellow,
            hue if hue < 150.0 => ColorFamily::Green,
            hue if hue < 182.0 => ColorFamily::Teal,
            hue if hue < 205.0 => ColorFamily::Sky,
            hue if hue < 238.0 => ColorFamily::Blue,
            hue if hue < 290.0 => ColorFamily::Violet,
            _ => ColorFamily::Pink,
        }
    }

    /// The shade of the family the set draws with most, which is drawn in the color of the family itself: a lighter
    /// shade is drawn lighter, a darker one darker.
    fn reference(self) -> Color {
        match self {
            ColorFamily::Gray => Color::hex(0x64_74_8B),
            ColorFamily::Red => Color::hex(0xF8_71_71),
            ColorFamily::Orange => Color::hex(0xEA_58_0C),
            ColorFamily::Yellow => Color::hex(0xF5_9E_0B),
            ColorFamily::Green => Color::hex(0x22_C5_5E),
            ColorFamily::Teal => Color::hex(0x2D_D4_BF),
            ColorFamily::Sky => Color::hex(0x0E_A5_E9),
            ColorFamily::Blue => Color::hex(0x60_A5_FA),
            ColorFamily::Violet => Color::hex(0x8B_5C_F6),
            ColorFamily::Pink => Color::hex(0xF4_72_B6),
        }
    }
}

/// The color each family of hues is drawn in: those of a theme, or one color for all of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FileIconColors {
    /// The color of each family, in the order of [`ColorFamily::ALL`].
    colors: [Color; 10],
}

impl FileIconColors {
    /// The colors of a theme: the gray of the icons in the muted text, the hues in the theme's colors of the icons
    /// of files.
    #[must_use]
    pub fn of_theme(theme: &Theme) -> FileIconColors {
        let hues = theme.file_icons;
        FileIconColors {
            colors: [
                theme.colors.muted_foreground,
                hues.red,
                hues.orange,
                hues.yellow,
                hues.green,
                hues.teal,
                hues.sky,
                hues.blue,
                hues.violet,
                hues.pink,
            ],
        }
    }

    /// One color for every family: the icons drawn in the color of the text, as the other icons of the window.
    #[must_use]
    pub fn single(color: Color) -> FileIconColors {
        FileIconColors { colors: [color; 10] }
    }

    /// The color a family is drawn in.
    #[must_use]
    pub fn color(&self, family: ColorFamily) -> Color {
        let index = ColorFamily::ALL
            .iter()
            .position(|each| *each == family)
            .unwrap_or_default();
        self.colors.get(index).copied().unwrap_or(Color::BLACK)
    }

    /// The color a color of an icon is drawn in: that of its family, as much lighter or darker than it as the color
    /// was than the family's own shade in the set, with its opacity.
    #[must_use]
    pub fn recolor(&self, color: Color) -> Color {
        let family = ColorFamily::of(color);
        let (hue, saturation, lightness) = hsl(self.color(family));
        let shift = hsl(color).2 - hsl(family.reference()).2;
        from_hsl(hue, saturation, (lightness + shift).clamp(0.0, 1.0)).with_alpha(color.alpha)
    }
}

/// The names of the properties of SVG that hold a color, as attributes or in a `style`.
const COLOR_PROPERTIES: [&str; 6] = ["fill", "stroke", "stop-color", "flood-color", "lighting-color", "color"];

/// The text of an SVG with every color its properties name drawn in the colors given, so that it is read already in
/// them: `fill="#EA580C"`, `stroke:rgb(10%,20%,30%)`, `fill="white"`. What is not a color, `none`, `url(#a)` and
/// `currentColor`, is left as it is.
pub(super) fn recolored(source: &str, colors: &FileIconColors) -> String {
    let mut written = String::with_capacity(source.len());
    let mut rest = source;
    while let Some((before, value, after)) = next_color_value(rest) {
        written.push_str(before);
        match csscolor(value) {
            Some(color) => write_color(&mut written, colors.recolor(color)),
            None => written.push_str(value),
        }
        rest = after;
    }
    written.push_str(rest);
    written
}

/// The next value of a property of color in a text: the text before it, the value, and the text after it.
fn next_color_value(text: &str) -> Option<(&str, &str, &str)> {
    let mut searched = 0;
    loop {
        let (start, name) = COLOR_PROPERTIES
            .iter()
            .filter_map(|name| text.get(searched..)?.find(name).map(|at| (searched + at, *name)))
            .min_by_key(|(at, name)| (*at, std::cmp::Reverse(name.len())))?;
        let end = start + name.len();
        searched = end;
        let is_whole_name = text
            .get(..start)
            .and_then(|before| before.chars().next_back())
            .is_none_or(|letter| !(letter.is_ascii_alphanumeric() || letter == '-'));
        let after = text.get(end..).unwrap_or_default();
        let value_start = match after.chars().next() {
            Some('=') => after
                .get(1..)
                .and_then(|quoted| quoted.chars().next())
                .filter(|quote| *quote == '"' || *quote == '\'')
                .map(|_| end + 2),
            Some(':') => Some(end + 1),
            _ => None,
        };
        let (Some(value_start), true) = (value_start, is_whole_name) else {
            continue;
        };
        let value_text = text.get(value_start..).unwrap_or_default();
        let length = value_text.find(['"', '\'', ';', '}', '>']).unwrap_or(value_text.len());
        let value_end = value_start + length;
        return Some((
            text.get(..value_start).unwrap_or_default(),
            text.get(value_start..value_end).unwrap_or_default(),
            text.get(value_end..).unwrap_or_default(),
        ));
    }
}

/// The color a value of CSS names, when it names one.
fn csscolor(value: &str) -> Option<Color> {
    let color = svgtypes::Color::from_str(value.trim()).ok()?;
    Some(Color::rgba(color.red, color.green, color.blue, color.alpha))
}

/// Writes a color as SVG reads it.
fn write_color(written: &mut String, color: Color) {
    let Color {
        red,
        green,
        blue,
        alpha,
    } = color;
    let _ = if alpha == u8::MAX {
        write!(written, "#{red:02x}{green:02x}{blue:02x}")
    } else {
        write!(written, "rgba({red},{green},{blue},{:.3})", f32::from(alpha) / 255.0)
    };
}

/// The hue in degrees, the saturation and the lightness of a color, each but the hue from 0 to 1.
fn hsl(color: Color) -> (f32, f32, f32) {
    let [red, green, blue] = [color.red, color.green, color.blue].map(|channel| f32::from(channel) / 255.0);
    let most = red.max(green).max(blue);
    let least = red.min(green).min(blue);
    let lightness = f32::midpoint(most, least);
    let range = most - least;
    if range <= f32::EPSILON {
        return (0.0, 0.0, lightness);
    }
    let saturation = range / (1.0 - (2.0 * lightness - 1.0).abs());
    let hue = if (most - red).abs() <= f32::EPSILON {
        ((green - blue) / range).rem_euclid(6.0)
    } else if (most - green).abs() <= f32::EPSILON {
        (blue - red) / range + 2.0
    } else {
        (red - green) / range + 4.0
    };
    (hue * 60.0, saturation.clamp(0.0, 1.0), lightness)
}

/// The color of a hue in degrees, a saturation and a lightness, opaque.
fn from_hsl(hue: f32, saturation: f32, lightness: f32) -> Color {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let sector = hue.rem_euclid(360.0) / 60.0;
    let second = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue) = match sector {
        sector if sector < 1.0 => (chroma, second, 0.0),
        sector if sector < 2.0 => (second, chroma, 0.0),
        sector if sector < 3.0 => (0.0, chroma, second),
        sector if sector < 4.0 => (0.0, second, chroma),
        sector if sector < 5.0 => (second, 0.0, chroma),
        _ => (chroma, 0.0, second),
    };
    let lowest = lightness - chroma / 2.0;
    let channel = |value: f32| crate::number::channel((value + lowest) * 255.0);
    Color::rgb(channel(red), channel(green), channel(blue))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_color_comes_back_from_its_hue_saturation_and_lightness() {
        for color in [
            Color::hex(0xEA_58_0C),
            Color::hex(0x60_A5_FA),
            Color::hex(0x64_74_8B),
            Color::WHITE,
        ] {
            let (hue, saturation, lightness) = hsl(color);
            let back = from_hsl(hue, saturation, lightness);
            for (one, other) in [
                (back.red, color.red),
                (back.green, color.green),
                (back.blue, color.blue),
            ] {
                assert!(one.abs_diff(other) <= 1, "{color:?} came back as {back:?}");
            }
        }
    }

    #[test]
    fn every_way_of_writing_a_color_is_drawn_in_the_colors_given() {
        let colors = FileIconColors::single(Color::rgb(0x10, 0x20, 0x30));
        let source = r##"<path fill="#64748B" stroke='white' style="fill:rgb(39.215686%,45.490196%,54.509804%);stroke:none" fill-rule="evenodd"/><stop stop-color="#64748b"/><path fill="url(#a)"/>"##;
        let written = recolored(source, &colors);
        assert_eq!(written.matches("#102030").count(), 3, "{written}");
        assert!(written.contains("fill-rule=\"evenodd\""), "{written}");
        assert!(
            written.contains("stroke:none") && written.contains("url(#a)"),
            "{written}"
        );
        assert!(!written.contains("white"), "{written}");
    }
}
