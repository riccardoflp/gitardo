//! Things handed to the event loop from any thread: the messages of a waker, synthetic input and the requests of a
//! screen reader. Each mailbox wakes the loop when something arrives in it empty.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use winit::event_loop::EventLoopProxy;

use crate::error::PlatformError;
use crate::input::InputEvent;

/// The things waiting for the loop, while the loop or the window they are for is open.
pub(crate) struct Mailbox<T> {
    inside: Mutex<Inside<T>>,
    proxy: EventLoopProxy<()>,
}

/// What the lock of a mailbox keeps.
struct Inside<T> {
    items: VecDeque<T>,
    is_open: bool,
}

impl<T> Mailbox<T> {
    /// An open mailbox that wakes the loop of this proxy.
    pub(crate) fn new(proxy: EventLoopProxy<()>) -> Mailbox<T> {
        Mailbox {
            inside: Mutex::new(Inside {
                items: VecDeque::new(),
                is_open: true,
            }),
            proxy,
        }
    }

    /// Refuses everything from now on, and drops what was waiting.
    pub(crate) fn close(&self) {
        let mut inside = self.lock();
        inside.is_open = false;
        inside.items.clear();
    }

    /// Hands something to the loop and wakes it.
    pub(crate) fn post(&self, item: T) -> Result<(), PlatformError> {
        let mut inside = self.lock();
        if !inside.is_open {
            return Err(PlatformError::Closed);
        }
        let was_empty = inside.items.is_empty();
        inside.items.push_back(item);
        // One wake is enough for everything that arrives before the loop takes it.
        if was_empty && self.proxy.send_event(()).is_err() {
            let _ = inside.items.pop_back();
            return Err(PlatformError::Closed);
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
    pub(crate) fn new(mailbox: &Arc<Mailbox<M>>) -> Waker<M> {
        Waker {
            mailbox: Arc::clone(mailbox),
        }
    }

    /// Hands a message to the loop, which gives it to its handler as `Event::Message`.
    ///
    /// # Errors
    ///
    /// `PlatformError::Closed` when the loop is gone.
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
    pub(crate) fn new(mailbox: &Arc<Mailbox<InputEvent>>) -> InputSender {
        InputSender {
            mailbox: Arc::clone(mailbox),
        }
    }

    /// Queues an event for the window; its loop hands it to the handler after what came before it.
    ///
    /// # Errors
    ///
    /// `PlatformError::Closed` when the window is gone.
    pub fn send(&self, event: InputEvent) -> Result<(), PlatformError> {
        self.mailbox.post(event)
    }
}
