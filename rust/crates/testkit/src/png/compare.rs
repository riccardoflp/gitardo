//! Comparing an image with its reference, and drawing where they differ, for the view snapshots.

use super::image::Image;

/// The color of a pixel that differs, strong enough to be seen at once on the dimmed image around it.
const DIFFERING_COLOR: [u8; 4] = [255, 0, 255, 255];

/// How an image differs from its reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Difference {
    /// How many pixels differ, counting those that only one of the two images has when their sizes differ.
    pub differing_pixels: u64,
    /// The differences drawn as an image as large as both: the pixels that differ in magenta, the others as a dimmed
    /// gray of the reference, so that a snapshot test can write it beside the reference.
    pub image: Image,
}

impl Image {
    /// How this image, the actual one, differs from a reference, or nothing when they are the same size with the same
    /// pixels.
    #[must_use]
    pub fn difference_from(&self, reference: &Image) -> Option<Difference> {
        if self == reference {
            return None;
        }
        let width = self.width.max(reference.width);
        let height = self.height.max(reference.height);
        let mut image = Image::filled(width, height, DIFFERING_COLOR);
        let mut differing_pixels = 0;
        for row in 0..height {
            for column in 0..width {
                match (reference.pixel(column, row), self.pixel(column, row)) {
                    (Some(expected), Some(actual)) if expected == actual => {
                        image.set_pixel(column, row, dimmed(expected));
                    }
                    _ => differing_pixels += 1,
                }
            }
        }
        Some(Difference {
            differing_pixels,
            image,
        })
    }
}

/// A pixel as a faint, opaque gray of its brightness, its transparency counted as darkness.
fn dimmed(color: [u8; 4]) -> [u8; 4] {
    let [red, green, blue, alpha] = color.map(u32::from);
    let brightness = (red + green + blue) / 3 * alpha / 255;
    let gray = u8::try_from(96 + brightness / 4).unwrap_or(u8::MAX);
    [gray, gray, gray, 255]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_images_have_no_difference() {
        let image = Image::filled(3, 2, [10, 20, 30, 255]);
        assert_eq!(image.difference_from(&image.clone()), None);
    }

    #[test]
    fn differing_pixels_are_counted_and_drawn_in_magenta_the_others_dimmed() {
        let reference = Image::filled(4, 3, [255, 255, 255, 255]);
        let mut actual = reference.clone();
        actual.set_pixel(1, 2, [0, 0, 0, 255]);
        actual.set_pixel(3, 0, [255, 255, 255, 254]);
        let difference = actual.difference_from(&reference).expect("two pixels differ");
        assert_eq!(difference.differing_pixels, 2);
        assert_eq!(difference.image.pixel(1, 2), Some(DIFFERING_COLOR));
        assert_eq!(difference.image.pixel(3, 0), Some(DIFFERING_COLOR));
        assert_eq!(difference.image.pixel(0, 0), Some([159, 159, 159, 255]));
        assert_eq!((difference.image.width, difference.image.height), (4, 3));
    }

    #[test]
    fn pixels_that_only_one_image_has_differ() {
        let reference = Image::filled(2, 2, [0, 0, 0, 255]);
        let actual = Image::filled(3, 1, [0, 0, 0, 255]);
        let difference = actual.difference_from(&reference).expect("the sizes differ");
        assert_eq!((difference.image.width, difference.image.height), (3, 2));
        assert_eq!(difference.differing_pixels, 4);
        assert_eq!(difference.image.pixel(0, 0), Some([96, 96, 96, 255]));
        assert_eq!(difference.image.pixel(2, 0), Some(DIFFERING_COLOR));
        assert_eq!(difference.image.pixel(0, 1), Some(DIFFERING_COLOR));
    }
}
