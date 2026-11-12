//! The event loop of a UI thread: it waits for the messages of the system, turns them into events for the handler,
//! and wakes for the messages of other threads and for timers.

use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use super::handle::{OwnedWindow, WindowHandle, WindowRequest, register_class};
use super::mailbox::{Mailbox, Waker};
use super::types::{HWND, LPARAM, LRESULT, WPARAM};
use super::{check, user32, wide};
use crate::dispatch::Dispatcher;
use crate::error::PlatformError;
use crate::event::{Event, TimerId};

/// The message that tells the loop's own window that a waker handed it messages.
const WM_WAKE: u32 = user32::WM_APP + 1;

/// The event loop of the thread that makes it: windows opened on it get their input through it.
///
/// It belongs to that thread; other threads reach it through a [`Waker`].
pub struct EventLoop<M: 'static> {
    dispatcher: Rc<Dispatcher<M>>,
    wake: Arc<Mailbox<M>>,
    window: OwnedWindow,
    // Shared, never borrowed mutably: the loop's window reaches it through a pointer while the window lives.
    _target: Rc<LoopTarget>,
}

impl<M: 'static> EventLoop<M> {
    /// A loop for the calling thread. It also makes the program aware of the scale of each screen, before any
    /// window opens.
    ///
    /// # Errors
    ///
    /// `PlatformError::System` when the system refused to make the window the loop receives its messages with.
    pub fn new() -> Result<EventLoop<M>, PlatformError> {
        set_dpi_awareness();
        let dispatcher = Rc::new(Dispatcher::new());
        let wake = Arc::new(Mailbox::new(WM_WAKE));
        let inbox = LoopInbox {
            dispatcher: Rc::clone(&dispatcher),
            wake: Arc::clone(&wake),
        };
        let target = Rc::new(LoopTarget {
            receiver: Box::new(inbox),
        });
        let class = wide("GitardoDesktopLoop");
        register_class(&class, loop_procedure)?;
        let title = wide("");
        let window = OwnedWindow::create(&WindowRequest {
            class: &class,
            title: &title,
            style: 0,
            parent: user32::HWND_MESSAGE,
            state: Rc::as_ptr(&target).cast(),
        })?;
        wake.open(window.handle());
        Ok(EventLoop {
            dispatcher,
            wake,
            window,
            _target: target,
        })
    }

    /// A waker other threads use to hand messages to this loop.
    #[must_use]
    pub fn waker(&self) -> Waker<M> {
        Waker::new(Arc::clone(&self.wake))
    }

    /// Hands every event to `handler`, waiting for the system in between, until `quit` is called. The handler can
    /// borrow the loop and its windows: it is called on this thread only, never while it is already running.
    ///
    /// # Errors
    ///
    /// `PlatformError::System` when the system failed to wait for messages.
    pub fn run(&self, mut handler: impl FnMut(Event<M>)) -> Result<(), PlatformError> {
        self.dispatcher.with_handler(&mut handler, || self.pump())
    }

    /// Stops the loop once the handler returns; `run` then returns. Events that came meanwhile wait for the next run.
    pub fn quit(&self) {
        self.dispatcher.quit();
    }

    /// Starts a timer that goes off every `interval` as `Event::Timer`, or changes the interval of a timer already
    /// started with this id. The system does not go below 10 milliseconds.
    ///
    /// # Errors
    ///
    /// `PlatformError::System` when the system refused the timer.
    pub fn start_timer(&self, timer: TimerId, interval: Duration) -> Result<(), PlatformError> {
        let milliseconds = u32::try_from(interval.as_millis()).unwrap_or(u32::MAX);
        let id = usize::try_from(timer.0).unwrap_or_default();
        // SAFETY: the window is this loop's own; without a function, the timer goes off as a WM_TIMER message.
        let result = unsafe { user32::SetTimer(self.window.handle().raw(), id, milliseconds, std::ptr::null()) };
        if result == 0 {
            return Err(super::last_error("SetTimer"));
        }
        Ok(())
    }

    /// Stops a timer. Stopping one that is not running does nothing.
    pub fn stop_timer(&self, timer: TimerId) {
        let id = usize::try_from(timer.0).unwrap_or_default();
        // SAFETY: only integers are passed. A failure means the timer was not running, which is what is wanted.
        let _ = unsafe { user32::KillTimer(self.window.handle().raw(), id) };
    }

    /// What windows opened on this loop hand their events to.
    pub(crate) fn dispatcher(&self) -> &Rc<Dispatcher<M>> {
        &self.dispatcher
    }

    /// Waits for messages and hands them on, until the loop is asked to stop.
    fn pump(&self) -> Result<(), PlatformError> {
        self.dispatcher.deliver();
        while !self.dispatcher.is_quitting() {
            if let Some(message) = peek_message() {
                if message.message == user32::WM_QUIT {
                    self.dispatcher.quit();
                    break;
                }
                dispatch_message(&message);
                self.dispatcher.deliver();
            } else if self.dispatcher.flush_redraws() {
                self.dispatcher.deliver();
            } else {
                wait_message()?;
            }
        }
        Ok(())
    }
}

impl<M: 'static> Drop for EventLoop<M> {
    fn drop(&mut self) {
        self.wake.close();
    }
}

/// Makes the program aware of the scale of each screen, so that the system does not stretch its windows.
fn set_dpi_awareness() {
    // SAFETY: only an integer is passed. It fails when the awareness was already set, by an earlier loop or by the
    // program's manifest, which is the same awareness or one the program chose: either way there is nothing to do.
    let _ = unsafe { user32::SetProcessDpiAwarenessContext(user32::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
}

/// The next message of the thread's queue, if any, without waiting.
fn peek_message() -> Option<user32::MSG> {
    let mut message = user32::MSG::default();
    // SAFETY: `message` is a MSG the call writes into, alive for the call.
    let has_message = unsafe { user32::PeekMessageW(&raw mut message, 0, 0, 0, user32::PM_REMOVE) };
    (has_message != 0).then_some(message)
}

/// Makes the characters of a key message and hands the message to its window's procedure.
fn dispatch_message(message: &user32::MSG) {
    // SAFETY: `message` is a MSG that PeekMessageW filled in. The results say whether characters were made and what
    // the window procedure returned, neither of which the loop needs.
    let _ = unsafe { user32::TranslateMessage(message) };
    // SAFETY: as above.
    let _ = unsafe { user32::DispatchMessageW(message) };
}

/// Waits until a new message is in the thread's queue.
fn wait_message() -> Result<(), PlatformError> {
    // SAFETY: the call takes nothing.
    check("WaitMessage", unsafe { user32::WaitMessage() })
}

/// What the loop's own window does with the messages it receives.
trait LoopReceiver {
    /// A waker handed messages.
    fn woken(&self);
    /// A timer went off.
    fn timer(&self, timer: TimerId);
}

/// What the loop's own window points to.
struct LoopTarget {
    receiver: Box<dyn LoopReceiver>,
}

/// The loop's own window's view of the loop.
struct LoopInbox<M: 'static> {
    dispatcher: Rc<Dispatcher<M>>,
    wake: Arc<Mailbox<M>>,
}

impl<M: 'static> LoopReceiver for LoopInbox<M> {
    fn woken(&self) {
        for message in self.wake.take_all() {
            self.dispatcher.push(Event::Message(message));
        }
        self.dispatcher.deliver();
    }

    fn timer(&self, timer: TimerId) {
        self.dispatcher.push(Event::Timer(timer));
        self.dispatcher.deliver();
    }
}

/// The window procedure of the loop's own window, which only receives messages.
extern "system" fn loop_procedure(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let handle = WindowHandle::from_raw(window);
    super::procedure::keep_state_on_create(handle, message, lparam);
    let target = handle.user_data().cast::<LoopTarget>();
    // SAFETY: the pointer is null, or the `LoopTarget` that `EventLoop::new` passed to CreateWindowExW. The loop owns
    // it and destroys this window before dropping it, and the pointer is cleared when the window is destroyed.
    let Some(target) = (unsafe { target.as_ref() }) else {
        return handle.default_procedure(message, wparam, lparam);
    };
    match message {
        WM_WAKE => target.receiver.woken(),
        user32::WM_TIMER => target
            .receiver
            .timer(TimerId(u32::try_from(wparam).unwrap_or_default())),
        _ => return handle.default_procedure(message, wparam, lparam),
    }
    0
}
