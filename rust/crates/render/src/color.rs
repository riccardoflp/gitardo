//! Colors, and how a color with some transparency is laid over a pixel.

/// A color of 8-bit red, green and blue, with an alpha from 0 (transparent) to 255 (opaque). The channels are not
/// multiplied by the alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    /// The red channel.
    pub red: u8,
    /// The green channel.
    pub green: u8,
    /// The blue channel.
    pub blue: u8,
    /// How opaque the color is: 0 is not seen at all, 255 hides what is under it.
    pub alpha: u8,
}

impl Color {
    /// Nothing: drawing with it changes no pixel.
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);
    /// Opaque black.
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    /// Opaque white.
    pub const WHITE: Color = Color::rgb(255, 255, 255);

    /// An opaque color from its three channels.
    #[must_use]
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Color {
        Color::rgba(red, green, blue, 255)
    }

    /// A color from its three channels and its alpha.
    #[must_use]
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Color {
        Color {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// An opaque color written as `0xRRGGBB`, the way colors are written in the themes.
    #[must_use]
    pub const fn hex(value: u32) -> Color {
        let [_, red, green, blue] = value.to_be_bytes();
        Color::rgb(red, green, blue)
    }

    /// The same color with another alpha.
    #[must_use]
    pub const fn with_alpha(self, alpha: u8) -> Color {
        Color { alpha, ..self }
    }

    /// The same color at a part of its opacity, out of 255: 255 leaves it as it is, 0 makes it transparent.
    #[must_use]
    pub fn faded(self, opacity: u8) -> Color {
        let alpha = coverage_alpha(self.alpha, opacity);
        self.with_alpha(u8::try_from(alpha).unwrap_or(u8::MAX))
    }

    /// The color as a pixel of the frame buffer, `0x00RRGGBB`, its alpha left out.
    #[must_use]
    pub const fn pixel(self) -> u32 {
        u32::from_be_bytes([0, self.red, self.green, self.blue])
    }

    /// The color of a pixel of the frame buffer, opaque.
    #[must_use]
    pub const fn from_pixel(pixel: u32) -> Color {
        let [_, red, green, blue] = pixel.to_be_bytes();
        Color::rgb(red, green, blue)
    }

    /// Whether drawing with this color changes nothing.
    #[must_use]
    pub const fn is_transparent(self) -> bool {
        self.alpha == 0
    }

    /// Whether drawing with this color hides what is under it.
    #[must_use]
    pub const fn is_opaque(self) -> bool {
        self.alpha == 255
    }

    /// This color laid over another, as an opaque color: what a pixel of `below` becomes when this color is drawn on
    /// it.
    #[must_use]
    pub fn over(self, below: Color) -> Color {
        Color::from_pixel(blend(below.pixel(), self.pixel(), u32::from(self.alpha)))
    }
}

/// The opacity of a color drawn with a coverage, both from 0 to 255: the alpha of the color times the part of the
/// pixel the shape covers.
pub(crate) fn coverage_alpha(alpha: u8, coverage: u8) -> u32 {
    divide_by_255(u32::from(alpha) * u32::from(coverage))
}

/// A product of two bytes divided by 255, rounded to the nearest.
fn divide_by_255(value: u32) -> u32 {
    let rounded = value + 128;
    (rounded + (rounded >> 8)) >> 8
}

/// A pixel with a color laid over it with an opacity from 0 to 255 (source-over): each channel becomes
/// `source × opacity + destination × (255 − opacity)`, divided by 255 and rounded.
///
/// Red and blue are worked out together, 16 bits apart in one integer, and green on its own; each product fits in its
/// 16 bits, so the lanes never spill into each other.
pub(crate) fn blend(destination: u32, source: u32, opacity: u32) -> u32 {
    let opacity = opacity.min(255);
    let remaining = 255 - opacity;
    let red_blue = (source & 0x00FF_00FF) * opacity + (destination & 0x00FF_00FF) * remaining + 0x0080_0080;
    let red_blue = ((red_blue + ((red_blue >> 8) & 0x00FF_00FF)) >> 8) & 0x00FF_00FF;
    let green = (source & 0x0000_FF00) * opacity + (destination & 0x0000_FF00) * remaining + 0x0000_8000;
    let green = ((green + (green >> 8)) >> 8) & 0x0000_FF00;
    red_blue | green
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Blending one channel the slow way, to check the fast one against.
    fn blend_channel(destination: u8, source: u8, opacity: u32) -> u8 {
        let sum = u32::from(source) * opacity + u32::from(destination) * (255 - opacity);
        u8::try_from((2 * sum + 255) / 510).unwrap()
    }

    #[test]
    fn blending_matches_the_rounded_formula_for_every_opacity() {
        let pairs = [(0u8, 255u8), (255, 0), (12, 200), (128, 127), (255, 255), (1, 254)];
        for opacity in 0..=255 {
            for (destination, source) in pairs {
                let pixel = blend(
                    Color::rgb(destination, source, destination).pixel(),
                    Color::rgb(source, destination, source).pixel(),
                    opacity,
                );
                let color = Color::from_pixel(pixel);
                assert_eq!(
                    color.red,
                    blend_channel(destination, source, opacity),
                    "red at {opacity}"
                );
                assert_eq!(
                    color.green,
                    blend_channel(source, destination, opacity),
                    "green at {opacity}"
                );
                assert_eq!(
                    color.blue,
                    blend_channel(destination, source, opacity),
                    "blue at {opacity}"
                );
            }
        }
    }

    #[test]
    fn colors_become_pixels_and_back() {
        let color = Color::hex(0x12_34_56);
        assert_eq!(color.pixel(), 0x0012_3456);
        assert_eq!(Color::from_pixel(0x00AB_CDEF), Color::rgb(0xAB, 0xCD, 0xEF));
        assert_eq!(Color::WHITE.with_alpha(0).over(Color::BLACK), Color::BLACK);
        assert_eq!(
            Color::WHITE.with_alpha(128).over(Color::BLACK),
            Color::rgb(128, 128, 128)
        );
        assert_eq!(coverage_alpha(255, 255), 255);
        assert_eq!(coverage_alpha(128, 255), 128);
        assert_eq!(coverage_alpha(0, 255), 0);
    }

    #[test]
    fn a_faded_color_keeps_its_channels_and_multiplies_its_alpha() {
        let color = Color::rgba(10, 20, 30, 200);
        assert_eq!(color.faded(255), color);
        assert_eq!(color.faded(0), Color::rgba(10, 20, 30, 0));
        assert_eq!(Color::WHITE.faded(153), Color::rgba(255, 255, 255, 153));
        assert_eq!(color.faded(128).alpha, 100);
    }
}
