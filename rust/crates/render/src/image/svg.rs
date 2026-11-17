//! An SVG drawn into pixels, by `resvg`, with the fonts the program comes with: the diagrams of Mermaid of the preview
//! of Markdown (`file-view.mermaid`). Nothing outside the SVG is read: an image it names is not drawn, and no font of
//! the system is looked for.

use std::sync::Arc;

use resvg::usvg;

use super::{Image, MOST_PIXELS};
use crate::number;

/// An SVG drawn: its pixels, and its size in logical pixels, which the pixels are that size times the scale asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SvgImage {
    /// The pixels.
    pub image: Image,
    /// Its width, in logical pixels.
    pub width: u32,
    /// Its height, in logical pixels.
    pub height: u32,
}

impl Image {
    /// An SVG drawn on a clear ground with the fonts of the program, at a scale: two pixels for each logical pixel at a
    /// scale of 2, so that it stays sharp on a screen that has that many. A drawing that would have more pixels than an
    /// image may have is drawn at the scale that fits. None when the text is not an SVG, or it has no size.
    #[must_use]
    pub fn from_svg(svg: &str, scale: f32) -> Option<SvgImage> {
        let options = usvg::Options {
            fontdb: Arc::new(crate::text::bundled_fonts()),
            font_family: crate::text::INTERFACE_FAMILY.to_owned(),
            // An image the SVG names, in a file, on the web or inside it, is never read (D60).
            image_href_resolver: usvg::ImageHrefResolver {
                resolve_data: Box::new(|_, _, _| None),
                resolve_string: Box::new(|_, _| None),
            },
            resources_dir: None,
            ..usvg::Options::default()
        };
        let tree = usvg::Tree::from_str(svg, &options).ok()?;
        let size = tree.size();
        let (width, height) = (size.width(), size.height());
        let pixels_at_one = f64::from(width) * f64::from(height);
        if pixels_at_one < 1.0 {
            return None;
        }
        let fitting = number::narrowed((number::float_of_large(MOST_PIXELS) / pixels_at_one).sqrt());
        let scale = scale.min(fitting).max(f32::MIN_POSITIVE);
        let mut pixmap = tiny_skia::Pixmap::new(
            number::size(number::ceil(width * scale)).max(1),
            number::size(number::ceil(height * scale)).max(1),
        )?;
        resvg::render(
            &tree,
            tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        Some(SvgImage {
            image: Image::of_pixmap(&pixmap),
            width: number::size(number::ceil(width)),
            height: number::size(number::ceil(height)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::png;

    #[test]
    fn an_svg_is_drawn_at_its_scale_and_measured_in_logical_pixels() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20" viewBox="0 0 40 20">
            <rect x="0" y="0" width="20" height="20" fill="#ff0000"/></svg>"##;
        let drawn = Image::from_svg(svg, 2.0).unwrap();
        assert_eq!((drawn.width, drawn.height), (40, 20));
        assert_eq!((drawn.image.width(), drawn.image.height()), (80, 40));
        assert_eq!(drawn.image.pixel(10, 10), [255, 0, 0, 255]);
        assert_eq!(drawn.image.pixel(70, 10)[3], 0);
    }

    #[test]
    fn text_is_drawn_with_the_fonts_of_the_program() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="80" height="30">
            <text x="4" y="22" font-family="Inter" font-size="20" fill="#000000">Git</text></svg>"##;
        let drawn = Image::from_svg(svg, 1.0).unwrap();
        let inked = (0..80)
            .flat_map(|x| (0..30).map(move |y| (x, y)))
            .filter(|&(x, y)| drawn.image.pixel(x, y)[3] > 0);
        assert!(inked.count() > 40);
    }

    #[test]
    fn an_image_the_svg_names_is_never_read() {
        let folder = std::env::temp_dir().join(format!("gitardo-svg-image-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let file = folder.join("red.png");
        std::fs::write(&file, png::encode(&png::Image::filled(4, 4, [255, 0, 0, 255])).unwrap()).unwrap();
        let inline = png::encode(&png::Image::filled(4, 4, [0, 255, 0, 255])).unwrap();
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="20" height="10">
            <image x="0" y="0" width="10" height="10" xlink:href="{}"/>
            <image x="10" y="0" width="10" height="10" xlink:href="data:image/png;base64,{}"/></svg>"#,
            file.display(),
            base64(&inline),
        );
        let drawn = Image::from_svg(&svg, 1.0).unwrap();
        assert!((0..20).all(|x| (0..10).all(|y| drawn.image.pixel(x, y)[3] == 0)));
        let _removed = std::fs::remove_dir_all(&folder);
    }

    /// Bytes in base64, as a data URL writes them.
    fn base64(bytes: &[u8]) -> String {
        const LETTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut written = String::new();
        for chunk in bytes.chunks(3) {
            let number = chunk
                .iter()
                .enumerate()
                .fold(0u32, |sum, (index, byte)| sum | u32::from(*byte) << (16 - 8 * index));
            for index in 0..4 {
                let letter = if index <= chunk.len() {
                    LETTERS[((number >> (18 - 6 * index)) & 63) as usize]
                } else {
                    b'='
                };
                written.push(char::from(letter));
            }
        }
        written
    }

    #[test]
    fn text_that_is_not_an_svg_is_not_drawn() {
        assert_eq!(Image::from_svg("flowchart LR", 1.0), None);
    }
}
