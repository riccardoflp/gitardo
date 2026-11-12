//! The input method of a window: the text it commits, and where its window goes.

use core::ffi::c_void;

use super::handle::WindowHandle;
use super::imm32::{self, HIMC};
use super::types::{POINT, RECT};
use crate::error::PlatformError;

/// The input context of a window, released when dropped.
struct InputContext {
    window: WindowHandle,
    context: HIMC,
}

impl InputContext {
    /// The input context of a window, when it has one.
    fn of(window: WindowHandle) -> Option<InputContext> {
        // SAFETY: the handle is checked by the system, which returns 0 for a window without a context.
        let context = unsafe { imm32::ImmGetContext(window.raw()) };
        (context != 0).then_some(InputContext { window, context })
    }
}

impl Drop for InputContext {
    fn drop(&mut self) {
        // SAFETY: the context came from ImmGetContext for this window and is released once. The result says whether
        // it was released, and nothing else could be done if it was not.
        let _ = unsafe { imm32::ImmReleaseContext(self.window.raw(), self.context) };
    }
}

/// The text the input method of a window just committed.
pub(crate) fn committed_text(window: WindowHandle) -> Option<String> {
    let input = InputContext::of(window)?;
    let nothing = std::ptr::null_mut();
    // SAFETY: a null buffer of 0 bytes asks only for the size of the text, in bytes.
    let bytes = unsafe { imm32::ImmGetCompositionStringW(input.context, imm32::GCS_RESULTSTR, nothing, 0) };
    let bytes = u32::try_from(bytes).ok().filter(|bytes| *bytes > 0)?;
    let mut units = vec![0_u16; usize::try_from(bytes / 2).ok()?];
    let buffer = units.as_mut_ptr().cast::<c_void>();
    // SAFETY: `buffer` has room for `bytes` bytes, as many as the call asked for just above, and is alive for the call.
    let written = unsafe { imm32::ImmGetCompositionStringW(input.context, imm32::GCS_RESULTSTR, buffer, bytes) };
    let written = usize::try_from(written).ok()?;
    units.truncate(written / 2);
    Some(String::from_utf16_lossy(&units))
}

/// Puts the input method's window at a point inside the window, in physical pixels: where the caret is.
pub(crate) fn set_composition_point(window: WindowHandle, x: i32, y: i32) -> Result<(), PlatformError> {
    let Some(input) = InputContext::of(window) else {
        return Ok(());
    };
    let form = imm32::COMPOSITIONFORM {
        dwStyle: imm32::CFS_POINT,
        ptCurrentPos: POINT { x, y },
        rcArea: RECT::default(),
    };
    // SAFETY: `form` is a COMPOSITIONFORM filled in as CFS_POINT asks, alive for the call.
    super::check("ImmSetCompositionWindow", unsafe {
        imm32::ImmSetCompositionWindow(input.context, &raw const form)
    })
}
