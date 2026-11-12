//! Things handed to a window from any thread: the messages of a waker, and synthetic input. Each comes with a
//! message posted to the window, which wakes its loop.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::handle::WindowHandle;
use super::types::HWND;
use crate::error::PlatformError;
use crate::input::InputEvent;

/// The things waiting for a window, and the window, while it is open.
pub(crate) struct Mailbox<T> {
    inside: Mutex<Inside<T>>,
    message: u32,
}

/// What the lock of a mailbox keeps.
struct Inside<T> {
    items: VecDeque<T>,
    window: Option<HWND>,
}

impl<T> Mailbox<T> {
    /// A mailbox that posts `message` to its window when something arrives; closed until `open`.
    pub(crate) fn new(message: u32) -> Mailbox<T> {
        Mailbox {
            inside: Mutex::new(Inside {
                items: VecDeque::new(),
                window: None,
            }),
            message,
        }
    }

    /// The window that receives what arrives.
    pub(crate) fn open(&self, window: WindowHandle) {
        self.lock().window = Some(window.raw());
    }

    /// Refuses everything from now on, before the window is destroyed, and drops what was waiting.
    pub(crate) fn close(&self) {
        let mut inside = self.lock();
        inside.window = None;
        inside.items.clear();
    }

    /// Hands something to the window and wakes its loop.
    pub(crate) fn post(&self, item: T) -> Result<(), PlatformError> {
        let mut inside = self.lock();
        let Some(window) = inside.window else {
            return Err(PlatformError::Closed);
        };
        let was_empty = inside.items.is_empty();
        inside.items.push_back(item);
        // One message is enough for everything that arrives before the window takes it; posting under the lock keeps
        // `close` from passing in between and the handle from naming another window by then.
        if was_empty && let Err(error) = WindowHandle::from_raw(window).post(self.message) {
            let _ = inside.items.pop_back();
            return Err(error);
        }
        Ok(())
    }

    /// Everything waiting, in the order it arrived.
    pub(crate) fn take_all(&self) -> VecDeque<T> {
        std::mem::take(&mut self.lock().items)
    }

    /// The lock, also after a thread panicked while holding it: what it keeps is still whole.
    fn lock(&self) -> MutexGuard<'_, Inside<T>> {
        self.inside.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Wakes an event loop from another thread, handing it a message. Cloned freely; fails once the loop is dropped.
pub struct Waker<M> {
    mailbox: Arc<Mailbox<M>>,
}

impl<M> Waker<M> {
    /// A waker for the mailbox of a loop.
    pub(crate) fn new(mailbox: Arc<Mailbox<M>>) -> Waker<M> {
        Waker { mailbox }
    }

    /// Hands a message to the loop, which gives it to its handler as `Event::Message`.
    ///
    /// # Errors
    ///
    /// `PlatformError::Closed` when the loop is gone; `PlatformError::System` when the system refused to wake it.
    pub fn wake(&self, message: M) -> Result<(), PlatformError> {
        self.mailbox.post(message)
    }
}

impl<M> Clone for Waker<M> {
    fn clone(&self) -> Waker<M> {
        Waker {
            mailbox: Arc::clone(&self.mailbox),
        }
    }
}

/// Feeds input into a window as if it came from the system, from any thread: for tests that drive a window.
#[derive(Clone)]
pub struct InputSender {
    mailbox: Arc<Mailbox<InputEvent>>,
}

impl InputSender {
    /// A sender for the mailbox of a window.
    pub(crate) fn new(mailbox: Arc<Mailbox<InputEvent>>) -> InputSender {
        InputSender { mailbox }
    }

    /// Queues an event behind what the system already queued for the window; its loop hands it to the handler in
    /// that order.
    ///
    /// # Errors
    ///
    /// `PlatformError::Closed` when the window is gone; `PlatformError::System` when the system refused to wake it.
    pub fn send(&self, event: InputEvent) -> Result<(), PlatformError> {
        self.mailbox.post(event)
    }
}
