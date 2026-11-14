//! Showing a frame of pixels in a window with `softbuffer`: the rectangles that changed are copied into its buffer
//! and shown, or the whole frame when the buffer does not hold the last one.

use std::num::NonZeroU32;
use std::rc::Rc;

use softbuffer::{Context, Rect, Surface};
use winit::window::Window as LibraryWindow;

use crate::convert::to_index;
use crate::error::PlatformError;
use crate::geometry::{PhysicalSize, PixelRect};

/// A frame of pixels to show: rows from the top, each pixel `0x00RRGGBB`, `width * height` of them.
#[derive(Clone, Copy, Debug)]
pub struct Frame<'a> {
    /// The pixels, row after row from the top.
    pub pixels: &'a [u32],
    /// Its size in physical pixels.
    pub size: PhysicalSize,
}

/// What shows the frames of a window.
pub(crate) struct Presenter {
    /// The buffer of the window.
    surface: Surface<Rc<LibraryWindow>, Rc<LibraryWindow>>,
    /// The size of the last frame shown.
    shown: Option<PhysicalSize>,
    /// Whether the window lost what it showed, so that the next frame is shown whole.
    is_stale: bool,
}

impl Presenter {
    /// What shows the frames of a window, with nothing shown yet.
    pub(crate) fn new(window: &Rc<LibraryWindow>) -> Result<Presenter, PlatformError> {
        let error =
            |error: softbuffer::SoftBufferError| PlatformError::system("softbuffer, to draw in a window", &error);
        let context = Context::new(Rc::clone(window)).map_err(error)?;
        let surface = Surface::new(&context, Rc::clone(window)).map_err(error)?;
        Ok(Presenter {
            surface,
            shown: None,
            is_stale: false,
        })
    }

    /// Shows the rectangles of a frame that changed; the whole frame when its size changed or the window lost what
    /// it showed. A frame with no pixel, as of a window made as small as it goes, shows nothing.
    pub(crate) fn present(&mut self, frame: Frame<'_>, dirty: &[PixelRect]) -> Result<(), PlatformError> {
        let expected = to_index(frame.size.width).checked_mul(to_index(frame.size.height));
        if expected != Some(frame.pixels.len()) {
            return Err(PlatformError::FrameSize {
                pixels: frame.pixels.len(),
                size: frame.size,
            });
        }
        let (Some(width), Some(height)) = (NonZeroU32::new(frame.size.width), NonZeroU32::new(frame.size.height))
        else {
            return Ok(());
        };
        let is_whole = self.is_stale || self.shown != Some(frame.size);
        let changed: Vec<PixelRect> = if is_whole {
            vec![PixelRect::covering(frame.size)]
        } else {
            dirty.iter().filter_map(|rect| rect.clipped(frame.size)).collect()
        };
        if changed.is_empty() {
            return Ok(());
        }
        if self.shown != Some(frame.size) {
            self.surface
                .resize(width, height)
                .map_err(|error| present_error(&error))?;
        }
        let mut buffer = self.surface.buffer_mut().map_err(|error| present_error(&error))?;
        // A buffer of age 1 holds the last frame shown; any other holds what the system left, and is filled whole.
        let copied = if buffer.age() == 1 {
            changed.clone()
        } else {
            vec![PixelRect::covering(frame.size)]
        };
        for rect in &copied {
            copy(&mut buffer, frame, *rect);
        }
        let damage: Vec<Rect> = changed.iter().filter_map(|rect| library_rect(*rect)).collect();
        buffer
            .present_with_damage(&damage)
            .map_err(|error| present_error(&error))?;
        self.shown = Some(frame.size);
        self.is_stale = false;
        Ok(())
    }

    /// Shows the last frame again, when the system lost what the window showed; when the buffer no longer holds it,
    /// the next frame is shown whole instead.
    pub(crate) fn present_again(&mut self) -> Result<(), PlatformError> {
        if self.shown.is_none() {
            return Ok(());
        }
        let buffer = self.surface.buffer_mut().map_err(|error| present_error(&error))?;
        if buffer.age() == 1 {
            buffer.present().map_err(|error| present_error(&error))
        } else {
            self.is_stale = true;
            Ok(())
        }
    }
}

/// The error of showing a frame.
fn present_error(error: &softbuffer::SoftBufferError) -> PlatformError {
    PlatformError::system("softbuffer, to show a frame", error)
}

/// Copies a rectangle of a frame, inside it, into a buffer of the same size.
fn copy(buffer: &mut [u32], frame: Frame<'_>, rect: PixelRect) {
    let width = to_index(frame.size.width);
    let (left, columns) = (to_index(rect.x), to_index(rect.width));
    for row in rect.y..rect.bottom() {
        let start = to_index(row).saturating_mul(width).saturating_add(left);
        let range = start..start.saturating_add(columns);
        if let (Some(source), Some(target)) = (frame.pixels.get(range.clone()), buffer.get_mut(range)) {
            target.copy_from_slice(source);
        }
    }
}

/// A rectangle as `softbuffer` takes it, when it covers any pixel.
fn library_rect(rect: PixelRect) -> Option<Rect> {
    Some(Rect {
        x: rect.x,
        y: rect.y,
        width: NonZeroU32::new(rect.width)?,
        height: NonZeroU32::new(rect.height)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{Frame, copy, library_rect};
    use crate::geometry::{PhysicalSize, PixelRect};

    #[test]
    fn copies_only_the_rectangle() {
        let size = PhysicalSize { width: 4, height: 3 };
        let pixels: Vec<u32> = (1..=12).collect();
        let mut buffer = vec![0; 12];
        let rect = PixelRect {
            x: 1,
            y: 1,
            width: 2,
            height: 2,
        };
        copy(&mut buffer, Frame { pixels: &pixels, size }, rect);
        assert_eq!(buffer, vec![0, 0, 0, 0, 0, 6, 7, 0, 0, 10, 11, 0]);
        assert!(library_rect(rect).is_some());
        assert!(library_rect(PixelRect { width: 0, ..rect }).is_none());
    }
}
