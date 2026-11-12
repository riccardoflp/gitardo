//! Showing a frame of pixels in a window: the window keeps the last frame, copies into it the rectangles that
//! changed, and paints them with `StretchDIBits` when the system asks it to paint.

use core::ffi::c_void;

use super::gdi32::{self, BITMAPINFO, BITMAPINFOHEADER, RGBQUAD};
use super::handle::WindowHandle;
use super::procedure::WindowState;
use super::types::{HDC, RECT};
use super::user32::{self, PAINTSTRUCT};
use crate::convert::{to_index, to_signed, to_unsigned};
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

/// The last frame shown in a window.
#[derive(Debug, Default)]
pub(crate) struct RetainedFrame {
    pixels: Vec<u32>,
    size: PhysicalSize,
}

impl RetainedFrame {
    /// Its size.
    pub(crate) fn size(&self) -> PhysicalSize {
        self.size
    }

    /// Takes the rectangles of a frame that changed; all of it when its size changed. The rectangles to paint again.
    pub(crate) fn update(&mut self, frame: Frame<'_>, dirty: &[PixelRect]) -> Result<Vec<PixelRect>, PlatformError> {
        let expected = to_index(frame.size.width).checked_mul(to_index(frame.size.height));
        if expected != Some(frame.pixels.len()) {
            return Err(PlatformError::FrameSize {
                pixels: frame.pixels.len(),
                size: frame.size,
            });
        }
        if frame.size != self.size {
            frame.pixels.clone_into(&mut self.pixels);
            self.size = frame.size;
            return Ok(PixelRect::covering(frame.size)
                .clipped(frame.size)
                .into_iter()
                .collect());
        }
        let changed: Vec<PixelRect> = dirty.iter().filter_map(|rect| rect.clipped(frame.size)).collect();
        for rect in &changed {
            self.copy(frame.pixels, *rect);
        }
        Ok(changed)
    }

    /// Copies one rectangle of a frame of the same size.
    fn copy(&mut self, source: &[u32], rect: PixelRect) {
        let width = to_index(self.size.width);
        for row in to_index(rect.y)..to_index(rect.bottom()) {
            let start = row * width + to_index(rect.x);
            let range = start..start + to_index(rect.width);
            if let (Some(target), Some(source)) = (self.pixels.get_mut(range.clone()), source.get(range)) {
                target.copy_from_slice(source);
            }
        }
    }

    /// The whole rows a rectangle covers, from its top to its bottom.
    fn rows(&self, rect: PixelRect) -> Option<&[u32]> {
        let width = to_index(self.size.width);
        self.pixels
            .get(to_index(rect.y) * width..to_index(rect.bottom()) * width)
    }
}

/// Shows the rectangles of a frame that changed: they are kept, and painted at once.
pub(crate) fn present(
    state: &WindowState,
    window: WindowHandle,
    frame: Frame<'_>,
    dirty: &[PixelRect],
) -> Result<(), PlatformError> {
    let changed = state.frame.borrow_mut().update(frame, dirty)?;
    for rect in changed {
        let area = RECT {
            left: to_signed(rect.x),
            top: to_signed(rect.y),
            right: to_signed(rect.right()),
            bottom: to_signed(rect.bottom()),
        };
        // SAFETY: `area` is a RECT alive for the call.
        super::check("InvalidateRect", unsafe {
            user32::InvalidateRect(window.raw(), &raw const area, 0)
        })?;
    }
    // SAFETY: the window belongs to this thread; the call sends it WM_PAINT at once when something is to be painted.
    super::check("UpdateWindow", unsafe { user32::UpdateWindow(window.raw()) })
}

/// `WM_PAINT`: paints what the system asks from the last frame, and asks for a new frame when the last one does not
/// fit the window.
pub(crate) fn paint(state: &WindowState, window: WindowHandle) {
    let needs_frame = {
        let painting = Painting::begin(window);
        let frame = state.frame.borrow();
        if let Some(rect) = painting.area().and_then(|area| area.clipped(frame.size())) {
            // A failure leaves this part of the window as it was until the next frame; nothing better can be done.
            let _ = draw(painting.context(), &frame, rect);
        }
        frame.size() != state.size.get()
    };
    if needs_frame {
        state.sink.request_redraw(state.id);
    }
}

/// Copies a rectangle of the frame into the same place of the window.
fn draw(context: HDC, frame: &RetainedFrame, rect: PixelRect) -> Result<(), PlatformError> {
    let rows = frame.rows(rect).ok_or(PlatformError::FrameSize {
        pixels: 0,
        size: frame.size(),
    })?;
    // The bitmap given is the whole rows of the rectangle, top-down (a negative height), so that the source is all of
    // its height and the meaning of the source's vertical origin does not matter.
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: u32::try_from(size_of::<BITMAPINFOHEADER>()).unwrap_or_default(),
            biWidth: to_signed(frame.size().width),
            biHeight: -to_signed(rect.height),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: gdi32::BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [RGBQUAD {
            rgbBlue: 0,
            rgbGreen: 0,
            rgbRed: 0,
            rgbReserved: 0,
        }],
    };
    let (x, y, width, height) = (
        to_signed(rect.x),
        to_signed(rect.y),
        to_signed(rect.width),
        to_signed(rect.height),
    );
    let bits = rows.as_ptr().cast::<c_void>();
    let usage = gdi32::DIB_RGB_COLORS;
    // SAFETY: `rows` holds `rect.height` whole rows of the frame's width, 32 bits a pixel, as `info` says, and is
    // alive for the call; the source rectangle (x, 0, width, height) lies inside them since `rect` is inside the frame.
    let lines = unsafe {
        gdi32::StretchDIBits(
            context,
            x,
            y,
            width,
            height,
            x,
            0,
            width,
            height,
            bits,
            &raw const info,
            usage,
            gdi32::SRCCOPY,
        )
    };
    if lines == 0 {
        return Err(super::last_error("StretchDIBits"));
    }
    Ok(())
}

/// A painting started by `BeginPaint`, ended by `EndPaint` when dropped.
struct Painting {
    window: WindowHandle,
    paint: PAINTSTRUCT,
}

impl Painting {
    /// Starts painting the part of the window the system asks for.
    fn begin(window: WindowHandle) -> Painting {
        let mut paint = PAINTSTRUCT::default();
        // SAFETY: `paint` is a PAINTSTRUCT the call writes into, alive for the call; called while handling WM_PAINT.
        // On failure the context is 0, which `area` and `draw` then do nothing with.
        let _ = unsafe { user32::BeginPaint(window.raw(), &raw mut paint) };
        Painting { window, paint }
    }

    /// The device context to paint into.
    fn context(&self) -> HDC {
        self.paint.hdc
    }

    /// The rectangle to paint, when there is one.
    fn area(&self) -> Option<PixelRect> {
        if self.paint.hdc == 0 {
            return None;
        }
        let area = self.paint.rcPaint;
        let (left, top) = (to_unsigned(area.left), to_unsigned(area.top));
        let rect = PixelRect {
            x: left,
            y: top,
            width: to_unsigned(area.right).saturating_sub(left),
            height: to_unsigned(area.bottom).saturating_sub(top),
        };
        (!rect.is_empty()).then_some(rect)
    }
}

impl Drop for Painting {
    fn drop(&mut self) {
        // SAFETY: `paint` is the PAINTSTRUCT BeginPaint filled in for this window. EndPaint always succeeds.
        let _ = unsafe { user32::EndPaint(self.window.raw(), &raw const self.paint) };
    }
}

#[cfg(test)]
mod tests {
    use super::{Frame, RetainedFrame};
    use crate::geometry::{PhysicalSize, PixelRect};

    const SIZE: PhysicalSize = PhysicalSize { width: 4, height: 3 };

    #[test]
    fn takes_a_new_size_whole() {
        let mut retained = RetainedFrame::default();
        let pixels: Vec<u32> = (0..12).collect();
        let changed = retained
            .update(
                Frame {
                    pixels: &pixels,
                    size: SIZE,
                },
                &[],
            )
            .unwrap();
        assert_eq!(
            changed,
            vec![PixelRect {
                x: 0,
                y: 0,
                width: 4,
                height: 3
            }]
        );
        assert_eq!(retained.pixels, pixels);
        assert_eq!(
            retained.rows(PixelRect {
                x: 1,
                y: 1,
                width: 2,
                height: 2
            }),
            Some(&pixels[4..12])
        );
    }

    #[test]
    fn copies_only_the_rectangles_that_changed() {
        let mut retained = RetainedFrame::default();
        let _ = retained
            .update(
                Frame {
                    pixels: &[0; 12],
                    size: SIZE,
                },
                &[],
            )
            .unwrap();
        let changed = retained
            .update(
                Frame {
                    pixels: &[1; 12],
                    size: SIZE,
                },
                &[PixelRect {
                    x: 2,
                    y: 1,
                    width: 5,
                    height: 1,
                }],
            )
            .unwrap();
        assert_eq!(
            changed,
            vec![PixelRect {
                x: 2,
                y: 1,
                width: 2,
                height: 1
            }]
        );
        assert_eq!(retained.pixels, vec![0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0]);
    }

    #[test]
    fn refuses_a_frame_of_the_wrong_length() {
        let mut retained = RetainedFrame::default();
        assert!(
            retained
                .update(
                    Frame {
                        pixels: &[0; 11],
                        size: SIZE
                    },
                    &[]
                )
                .is_err()
        );
    }
}
