//! Handing events to the program's handler, the same on every system: in the order they came, never while the
//! handler is already running, and redraws only once nothing else is waiting.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, VecDeque};
use std::ptr::NonNull;

use crate::event::{Event, WindowId};
use crate::input::InputEvent;

/// What a window of the system tells the loop it belongs to.
pub(crate) trait WindowSink {
    /// Queues input for a window and hands it to the handler, unless the handler is running.
    fn push_input(&self, window: WindowId, event: InputEvent);
    /// Queues a change of the system's theme and hands it to the handler, unless the handler is running.
    fn push_theme_changed(&self);
    /// Asks for a redraw of a window, handed to the handler once nothing else is waiting.
    fn request_redraw(&self, window: WindowId);
    /// The system entered a loop of its own, such as the one that resizes a window while the person drags its edge.
    fn enter_modal(&self);
    /// The system left its own loop.
    fn leave_modal(&self);
}

/// The program's handler, as kept while the loop runs.
type Handler<M> = NonNull<dyn FnMut(Event<M>)>;

/// The events waiting for the handler, and the handler while the loop runs.
pub(crate) struct Dispatcher<M: 'static> {
    queue: RefCell<VecDeque<Event<M>>>,
    handler: Cell<Option<Handler<M>>>,
    redraws: RefCell<BTreeSet<WindowId>>,
    is_quitting: Cell<bool>,
    modal_depth: Cell<u32>,
    next_window: Cell<u64>,
}

impl<M: 'static> Dispatcher<M> {
    /// A dispatcher with nothing waiting and no handler.
    pub(crate) fn new() -> Dispatcher<M> {
        Dispatcher {
            queue: RefCell::new(VecDeque::new()),
            handler: Cell::new(None),
            redraws: RefCell::new(BTreeSet::new()),
            is_quitting: Cell::new(false),
            modal_depth: Cell::new(0),
            next_window: Cell::new(1),
        }
    }

    /// A name for a new window.
    pub(crate) fn new_window_id(&self) -> WindowId {
        let id = self.next_window.get();
        self.next_window.set(id.wrapping_add(1));
        WindowId(id)
    }

    /// Queues an event, to be handed to the handler by the next `deliver`.
    pub(crate) fn push(&self, event: Event<M>) {
        self.queue.borrow_mut().push_back(event);
    }

    /// Asks the loop to stop once the handler returns.
    pub(crate) fn quit(&self) {
        self.is_quitting.set(true);
    }

    /// Whether the loop was asked to stop.
    pub(crate) fn is_quitting(&self) -> bool {
        self.is_quitting.get()
    }

    /// Runs `body` with `handler` as the handler of every event, and with no handler once it returns, also when it
    /// unwinds.
    pub(crate) fn with_handler<R>(&self, handler: &mut dyn FnMut(Event<M>), body: impl FnOnce() -> R) -> R {
        /// Takes the handler out of the dispatcher when the run ends.
        struct Uninstall<'a, M: 'static>(&'a Dispatcher<M>);
        impl<M: 'static> Drop for Uninstall<'_, M> {
            fn drop(&mut self) {
                self.0.handler.set(None);
            }
        }
        let borrowed = NonNull::from(handler);
        // SAFETY: only the lifetime of the trait object is changed, not its layout. The pointer is kept only while
        // `body` runs: `Uninstall` takes it out when `body` returns or unwinds, before the borrow of `handler` ends,
        // and `deliver` calls it only while it is kept.
        let erased = unsafe {
            std::mem::transmute::<NonNull<dyn FnMut(Event<M>) + '_>, NonNull<dyn FnMut(Event<M>) + 'static>>(borrowed)
        };
        self.is_quitting.set(false);
        self.handler.set(Some(erased));
        let _uninstall = Uninstall(self);
        body()
    }

    /// Hands every waiting event to the handler, in order, until none is left or the loop is asked to stop. Does
    /// nothing while the handler is running, which then gets them when it returns, nor when no loop runs.
    pub(crate) fn deliver(&self) {
        let Some(mut handler) = self.handler.take() else {
            return;
        };
        let mut has_flushed_redraws = false;
        while !self.is_quitting() {
            let next = self.queue.borrow_mut().pop_front();
            let Some(event) = next else {
                // In a loop of the system's own, the main loop does not run: redraws are handed over here instead,
                // once per delivery so that a handler that asks again does not keep the system's loop waiting.
                if self.modal_depth.get() > 0 && !has_flushed_redraws && self.flush_redraws() {
                    has_flushed_redraws = true;
                    continue;
                }
                break;
            };
            // SAFETY: the pointer was installed by `with_handler` from a live `&mut` and is still installed, since it
            // was taken out of the cell just above, so no other call can reach it while this one runs.
            unsafe { handler.as_mut()(event) };
        }
        self.handler.set(Some(handler));
    }

    /// Queues a redraw for every window that asked for one. Whether there was any.
    pub(crate) fn flush_redraws(&self) -> bool {
        let windows = std::mem::take(&mut *self.redraws.borrow_mut());
        let has_any = !windows.is_empty();
        for window in windows {
            self.push(Event::Input {
                window,
                event: InputEvent::RedrawRequested,
            });
        }
        has_any
    }
}

impl<M: 'static> WindowSink for Dispatcher<M> {
    fn push_input(&self, window: WindowId, event: InputEvent) {
        self.push(Event::Input { window, event });
        self.deliver();
    }

    fn push_theme_changed(&self) {
        let is_queued = self
            .queue
            .borrow()
            .iter()
            .any(|event| matches!(event, Event::ThemeChanged));
        if !is_queued {
            self.push(Event::ThemeChanged);
        }
        self.deliver();
    }

    fn request_redraw(&self, window: WindowId) {
        let _ = self.redraws.borrow_mut().insert(window);
    }

    fn enter_modal(&self) {
        self.modal_depth.set(self.modal_depth.get().saturating_add(1));
    }

    fn leave_modal(&self) {
        self.modal_depth.set(self.modal_depth.get().saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::{Dispatcher, WindowSink};
    use crate::event::{Event, TimerId};
    use crate::input::InputEvent;

    #[test]
    fn hands_events_over_in_order_only_while_a_handler_is_installed() {
        let dispatcher = Dispatcher::<u32>::new();
        dispatcher.push(Event::Message(1));
        dispatcher.deliver();
        let mut seen = Vec::new();
        dispatcher.with_handler(&mut |event| seen.push(event), || {
            dispatcher.push(Event::Message(2));
            dispatcher.deliver();
        });
        dispatcher.push(Event::Message(3));
        dispatcher.deliver();
        assert_eq!(seen, vec![Event::Message(1), Event::Message(2)]);
    }

    #[test]
    fn queues_what_comes_while_the_handler_runs() {
        let dispatcher = Dispatcher::<u32>::new();
        let mut seen = Vec::new();
        dispatcher.with_handler(
            &mut |event| {
                if event == Event::Message(1) {
                    dispatcher.push(Event::Timer(TimerId(7)));
                    dispatcher.deliver();
                }
                seen.push(event);
            },
            || {
                dispatcher.push(Event::Message(1));
                dispatcher.deliver();
            },
        );
        assert_eq!(seen, vec![Event::Message(1), Event::Timer(TimerId(7))]);
    }

    #[test]
    fn stops_handing_over_once_asked_to_quit() {
        let dispatcher = Dispatcher::<u32>::new();
        let mut seen = Vec::new();
        dispatcher.with_handler(
            &mut |event| {
                dispatcher.quit();
                seen.push(event);
            },
            || {
                dispatcher.push(Event::Message(1));
                dispatcher.push(Event::Message(2));
                dispatcher.deliver();
            },
        );
        assert_eq!(seen, vec![Event::Message(1)]);
    }

    #[test]
    fn gathers_redraws_and_theme_changes() {
        let dispatcher = Dispatcher::<u32>::new();
        let window = dispatcher.new_window_id();
        dispatcher.request_redraw(window);
        dispatcher.request_redraw(window);
        dispatcher.push_theme_changed();
        dispatcher.push_theme_changed();
        assert!(dispatcher.flush_redraws());
        assert!(!dispatcher.flush_redraws());
        let mut seen = Vec::new();
        dispatcher.with_handler(&mut |event| seen.push(event), || dispatcher.deliver());
        let redraw = Event::Input {
            window,
            event: InputEvent::RedrawRequested,
        };
        assert_eq!(seen, vec![Event::ThemeChanged, redraw]);
    }

    #[test]
    fn redraws_inside_a_loop_of_the_system() {
        let dispatcher = Dispatcher::<u32>::new();
        let window = dispatcher.new_window_id();
        let mut seen = Vec::new();
        dispatcher.with_handler(&mut |event| seen.push(event), || {
            dispatcher.enter_modal();
            dispatcher.request_redraw(window);
            dispatcher.push_input(window, InputEvent::FocusGained);
            dispatcher.leave_modal();
        });
        let redraw = Event::Input {
            window,
            event: InputEvent::RedrawRequested,
        };
        assert_eq!(
            seen,
            vec![
                Event::Input {
                    window,
                    event: InputEvent::FocusGained
                },
                redraw
            ]
        );
    }
}
