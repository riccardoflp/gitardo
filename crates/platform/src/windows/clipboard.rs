//! The clipboard of the system, for text.

use core::ffi::c_void;
use std::marker::PhantomData;
use std::time::Duration;

use super::handle::{OwnedWindow, WindowRequest, register_class};
use super::types::HGLOBAL;
use super::{check, kernel32, last_error, user32, wide};
use crate::error::PlatformError;

/// How many times the clipboard is tried when another program has it open, and how long to wait between tries.
const TRIES: u32 = 20;
const PAUSE: Duration = Duration::from_millis(5);

/// The clipboard, with a window of its own that owns what this program puts there.
pub struct Clipboard {
    owner: OwnedWindow,
}

impl Clipboard {
    /// The clipboard, for the calling thread.
    ///
    /// # Errors
    ///
    /// `PlatformError::System` when the system refused to make the window that owns what is copied.
    pub fn new() -> Result<Clipboard, PlatformError> {
        let class = wide("GitardoDesktopClipboard");
        register_class(&class, user32::DefWindowProcW)?;
        let title = wide("");
        let request = WindowRequest {
            class: &class,
            title: &title,
            style: 0,
            parent: user32::HWND_MESSAGE,
            state: std::ptr::null(),
        };
        Ok(Clipboard {
            owner: OwnedWindow::create(&request)?,
        })
    }

    /// The text on the clipboard, or `None` when what is there is not text.
    ///
    /// # Errors
    ///
    /// `PlatformError::ClipboardBusy` when another program kept it open; `PlatformError::System` when the system
    /// refused to give the text.
    pub fn read_text(&self) -> Result<Option<String>, PlatformError> {
        let _open = self.open()?;
        // SAFETY: only an integer is passed.
        if unsafe { user32::IsClipboardFormatAvailable(user32::CF_UNICODETEXT) } == 0 {
            return Ok(None);
        }
        // SAFETY: the clipboard is open for this thread. The handle stays the clipboard's: it is not freed here.
        let data = unsafe { user32::GetClipboardData(user32::CF_UNICODETEXT) };
        if data == 0 {
            return Err(last_error("GetClipboardData"));
        }
        let locked = Locked::new(data)?;
        Ok(Some(String::from_utf16_lossy(locked.text())))
    }

    /// Puts text on the clipboard, in place of what was there.
    ///
    /// # Errors
    ///
    /// `PlatformError::ClipboardBusy` when another program kept it open; `PlatformError::System` when the system
    /// refused the text.
    pub fn write_text(&self, text: &str) -> Result<(), PlatformError> {
        let memory = Global::with_text(text)?;
        let _open = self.open()?;
        // SAFETY: the clipboard is open for this thread, with this program's window as its owner.
        check("EmptyClipboard", unsafe { user32::EmptyClipboard() })?;
        // SAFETY: the memory is movable global memory holding UTF-16 text ended by 0, unlocked, as CF_UNICODETEXT
        // asks. When the call succeeds, the system owns it.
        if unsafe { user32::SetClipboardData(user32::CF_UNICODETEXT, memory.handle) } == 0 {
            return Err(last_error("SetClipboardData"));
        }
        memory.give_away();
        Ok(())
    }

    /// Whether the clipboard holds nothing at all, of any format.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        // SAFETY: the call takes nothing; it needs no open clipboard.
        unsafe { user32::CountClipboardFormats() == 0 }
    }

    /// Opens the clipboard for this thread, trying again while another program has it open.
    fn open(&self) -> Result<Opened, PlatformError> {
        for _ in 0..TRIES {
            // SAFETY: the owner is this program's window, made by this thread.
            if unsafe { user32::OpenClipboard(self.owner.handle().raw()) } != 0 {
                return Ok(Opened {
                    thread_bound: PhantomData,
                });
            }
            std::thread::sleep(PAUSE);
        }
        Err(PlatformError::ClipboardBusy)
    }
}

/// The clipboard opened by this thread, closed when dropped.
struct Opened {
    thread_bound: PhantomData<*const ()>,
}

impl Drop for Opened {
    fn drop(&mut self) {
        // SAFETY: the clipboard was opened by this thread. Closing it fails only when it is not open.
        let _ = unsafe { user32::CloseClipboard() };
    }
}

/// Global memory this program allocated, freed when dropped unless given to the system.
struct Global {
    handle: HGLOBAL,
}

impl Global {
    /// Movable global memory holding a text in UTF-16, ended by 0.
    fn with_text(text: &str) -> Result<Global, PlatformError> {
        let units = wide(text);
        let bytes = units.len() * size_of::<u16>();
        // SAFETY: only integers are passed.
        let handle = unsafe { kernel32::GlobalAlloc(kernel32::GMEM_MOVEABLE, bytes) };
        if handle == 0 {
            return Err(last_error("GlobalAlloc"));
        }
        let memory = Global { handle };
        let locked = Locked::new(handle)?;
        // SAFETY: the locked memory has room for at least `bytes` bytes, as allocated just above, and is aligned for
        // u16 as all global memory is; `units` is a different allocation.
        unsafe { std::ptr::copy_nonoverlapping(units.as_ptr(), locked.start.cast::<u16>(), units.len()) };
        drop(locked);
        Ok(memory)
    }

    /// Leaves the memory to the system, which took it.
    fn give_away(self) {
        std::mem::forget(self);
    }
}

impl Drop for Global {
    fn drop(&mut self) {
        // SAFETY: the memory was allocated by GlobalAlloc, is unlocked, and was not given to the system.
        let _ = unsafe { kernel32::GlobalFree(self.handle) };
    }
}

/// Global memory locked, so that its bytes can be read or written; unlocked when dropped.
struct Locked {
    handle: HGLOBAL,
    start: *mut c_void,
    bytes: usize,
}

impl Locked {
    /// Locks global memory.
    fn new(handle: HGLOBAL) -> Result<Locked, PlatformError> {
        // SAFETY: the handle is global memory, of this program or of the clipboard while it is open.
        let start = unsafe { kernel32::GlobalLock(handle) };
        if start.is_null() {
            return Err(last_error("GlobalLock"));
        }
        // SAFETY: the handle is the global memory just locked.
        let bytes = unsafe { kernel32::GlobalSize(handle) };
        Ok(Locked { handle, start, bytes })
    }

    /// The UTF-16 text in the memory, up to its first 0 or its end.
    fn text(&self) -> &[u16] {
        // SAFETY: the memory is locked for as long as `self` lives, holds `bytes` bytes, and is aligned for u16 as
        // all global memory is; nothing writes to it meanwhile, since the clipboard is open for this thread.
        let units = unsafe { std::slice::from_raw_parts(self.start.cast::<u16>(), self.bytes / size_of::<u16>()) };
        let end = units.iter().position(|unit| *unit == 0).unwrap_or(units.len());
        units.get(..end).unwrap_or_default()
    }
}

impl Drop for Locked {
    fn drop(&mut self) {
        // SAFETY: the memory was locked once by `new`. The result says whether it is still locked by others.
        let _ = unsafe { kernel32::GlobalUnlock(self.handle) };
    }
}
