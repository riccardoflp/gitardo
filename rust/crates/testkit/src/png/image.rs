//! An image in memory: rows of pixels of red, green, blue and alpha.

/// An image of 8-bit red, green, blue and alpha, row after row from the top, left to right.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    /// The width in pixels.
    pub width: u32,
    /// The height in pixels.
    pub height: u32,
    /// Four bytes for each pixel, red, green, blue and alpha, `width × height × 4` in all.
    pub pixels: Vec<u8>,
}

impl Image {
    /// An image of one color.
    ///
    /// # Panics
    ///
    /// When the image is too large to hold in memory.
    #[must_use]
    pub fn filled(width: u32, height: u32, color: [u8; 4]) -> Image {
        let count = pixel_count(width, height).expect("the image fits in memory");
        Image {
            width,
            height,
            pixels: color.repeat(count),
        }
    }

    /// The color of the pixel at a column and a row, from 0 at the top left, or nothing outside the image.
    #[must_use]
    pub fn pixel(&self, column: u32, row: u32) -> Option<[u8; 4]> {
        let start = self.offset(column, row)?;
        let bytes = self.pixels.get(start..start + 4)?;
        bytes.try_into().ok()
    }

    /// Paints the pixel at a column and a row with a color.
    ///
    /// # Panics
    ///
    /// When the pixel is outside the image.
    pub fn set_pixel(&mut self, column: u32, row: u32, color: [u8; 4]) {
        let (width, height) = (self.width, self.height);
        let pixel = self
            .offset(column, row)
            .and_then(|start| self.pixels.get_mut(start..start + 4))
            .unwrap_or_else(|| panic!("the pixel at ({column}, {row}) is outside the image of {width}×{height}"));
        pixel.copy_from_slice(&color);
    }

    /// Where a pixel's bytes start, when it is inside the image.
    fn offset(&self, column: u32, row: u32) -> Option<usize> {
        if column >= self.width || row >= self.height {
            return None;
        }
        let index = usize::try_from(u64::from(row) * u64::from(self.width) + u64::from(column)).ok()?;
        index.checked_mul(4)
    }
}

/// The number of pixels of an image of a width and a height, or nothing when it is more than memory can index.
pub(crate) fn pixel_count(width: u32, height: u32) -> Option<usize> {
    usize::try_from(u64::from(width) * u64::from(height)).ok()
}
