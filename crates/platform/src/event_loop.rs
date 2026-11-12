//! The event loop of the program: it waits for the events of `winit`, turns them into events of Gitardo for the
//! handler, and wakes for the messages of other threads and for timers.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop as LibraryLoop, EventLoopProxy};
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use winit::window::WindowId as LibraryWindowId;

use crate::error::PlatformError;
use crate::event::{Event, TimerId, WindowId};
use crate::input::InputEvent;
use crate::mailbox::{Mailbox, Waker};
use crate::system::is_high_contrast;
use crate::theme::{Appearance, SystemTheme};
use crate::timers::Timers;
use crate::translate;
use crate::window_state::WindowShared;

/// The event loop of the program; a program has one, made on the thread that runs it.
///
/// Other threads reach it through a [`Waker`].
pub struct EventLoop<M: 'static> {
    library: LibraryLoop<()>,
    state: Rc<LoopState>,
    messages: Arc<Mailbox<M>>,
}

impl<M: 'static> EventLoop<M> {
    /// The loop of the program.
    ///
    /// # Errors
    ///
    /// `PlatformError::EventLoopExists` when the program already made one: `winit` allows one for the life of a
    /// program. `PlatformError::System` when the system has no display to open windows on.
    pub fn new() -> Result<EventLoop<M>, PlatformError> {
        let mut builder = LibraryLoop::builder();
        allow_any_thread(&mut builder);
        let library = builder.build().map_err(loop_error)?;
        let proxy = library.create_proxy();
        Ok(EventLoop {
            messages: Arc::new(Mailbox::new(proxy.clone())),
            state: Rc::new(LoopState::new(proxy)),
            library,
        })
    }

    /// A waker other threads use to hand messages to this loop.
    #[must_use]
    pub fn waker(&self) -> Waker<M> {
        Waker::new(&self.messages)
    }

    /// Hands every event to `handler`, waiting for the system in between, until the handler calls
    /// [`Context::quit`]. It can be run again afterwards: windows, timers and messages wait for the next run.
    ///
    /// # Errors
    ///
    /// `PlatformError::System` when the system failed to run the loop.
    pub fn run(&mut self, handler: impl FnMut(&Context<'_>, Event<M>)) -> Result<(), PlatformError> {
        self.state.is_quitting.set(false);
        let mut runner = Runner {
            state: &self.state,
            messages: &self.messages,
            handler,
        };
        self.library.run_app_on_demand(&mut runner).map_err(loop_error)
    }
}

impl<M: 'static> Drop for EventLoop<M> {
    fn drop(&mut self) {
        self.messages.close();
    }
}

/// Lets the loop run on a thread other than the first: the tests of the program run on threads of their own.
#[cfg(windows)]
fn allow_any_thread(builder: &mut winit::event_loop::EventLoopBuilder<()>) {
    use winit::platform::windows::EventLoopBuilderExtWindows;
    let _ = builder.with_any_thread(true);
}

/// Lets the loop run on a thread other than the first, on X11 and Wayland alike.
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn allow_any_thread(builder: &mut winit::event_loop::EventLoopBuilder<()>) {
    use winit::platform::x11::EventLoopBuilderExtX11;
    let _ = builder.with_any_thread(true);
}

/// macOS runs its loop on the first thread only.
#[cfg(not(any(
    windows,
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
fn allow_any_thread(_: &mut winit::event_loop::EventLoopBuilder<()>) {}

/// The error of making or running the loop.
fn loop_error(error: EventLoopError) -> PlatformError {
    match error {
        EventLoopError::RecreationAttempt => PlatformError::EventLoopExists,
        EventLoopError::NotSupported(_) => PlatformError::Unsupported {
            what: "run an event loop",
        },
        other => PlatformError::system("winit, to run the event loop", &other),
    }
}

/// What the handler can ask of the loop while it runs: open windows, start and stop timers, read the theme, stop.
pub struct Context<'a> {
    pub(crate) active: &'a ActiveEventLoop,
    pub(crate) state: &'a Rc<LoopState>,
}

impl Context<'_> {
    /// Stops the loop once the handler returns; `run` then returns. Events that come meanwhile are dropped, but
    /// messages and synthetic input wait for the next run.
    pub fn quit(&self) {
        self.state.is_quitting.set(true);
        self.active.exit();
    }

    /// Starts a timer that goes off every `interval` as `Event::Timer`, or restarts a timer already started with
    /// this id.
    pub fn start_timer(&self, timer: TimerId, interval: Duration) {
        self.state.timers.borrow_mut().start(timer, interval, Instant::now());
    }

    /// Stops a timer. Stopping one that is not running does nothing.
    pub fn stop_timer(&self, timer: TimerId) {
        self.state.timers.borrow_mut().stop(timer);
    }

    /// The theme the person set for the system: dark or light, and whether high contrast is on. A system that does
    /// not say is light.
    #[must_use]
    pub fn system_theme(&self) -> SystemTheme {
        SystemTheme {
            appearance: self
                .active
                .system_theme()
                .map_or(Appearance::Light, translate::appearance),
            is_high_contrast: is_high_contrast(),
        }
    }
}

/// What the loop keeps between runs, shared with its windows.
pub(crate) struct LoopState {
    windows: RefCell<BTreeMap<WindowId, Weak<WindowShared>>>,
    redraws: RefCell<BTreeSet<WindowId>>,
    timers: RefCell<Timers>,
    is_quitting: Cell<bool>,
    theme: Cell<Option<SystemTheme>>,
    /// What wakes the loop from other threads.
    pub(crate) proxy: EventLoopProxy<()>,
    /// When the loop was made.
    clock: Instant,
}

impl LoopState {
    /// The state of a new loop.
    fn new(proxy: EventLoopProxy<()>) -> LoopState {
        LoopState {
            windows: RefCell::default(),
            redraws: RefCell::default(),
            timers: RefCell::default(),
            is_quitting: Cell::new(false),
            theme: Cell::new(None),
            proxy,
            clock: Instant::now(),
        }
    }

    /// Takes in a window just opened.
    pub(crate) fn register(&self, window: &Rc<WindowShared>) {
        let _ = self.windows.borrow_mut().insert(window.id, Rc::downgrade(window));
    }

    /// Lets go of a window that closed.
    pub(crate) fn forget(&self, window: WindowId) {
        let _ = self.windows.borrow_mut().remove(&window);
        let _ = self.redraws.borrow_mut().remove(&window);
    }

    /// Asks for a frame of a hidden window, which the loop hands over once nothing else is waiting.
    pub(crate) fn ask_redraw(&self, window: WindowId) {
        let _ = self.redraws.borrow_mut().insert(window);
    }

    /// A window that is open.
    fn window(&self, window: WindowId) -> Option<Rc<WindowShared>> {
        self.windows.borrow().get(&window).and_then(Weak::upgrade)
    }

    /// Every window that is open.
    fn open_windows(&self) -> Vec<Rc<WindowShared>> {
        self.windows.borrow().values().filter_map(Weak::upgrade).collect()
    }
}

/// What `winit` calls while the loop runs: it hands the events of Gitardo to the handler.
struct Runner<'a, M, H> {
    state: &'a Rc<LoopState>,
    messages: &'a Mailbox<M>,
    handler: H,
}

impl<M, H: FnMut(&Context<'_>, Event<M>)> Runner<'_, M, H> {
    /// Hands an event to the handler, unless it asked the loop to stop.
    fn deliver(&mut self, active: &ActiveEventLoop, event: Event<M>) {
        if !self.state.is_quitting.get() {
            let context = Context {
                active,
                state: self.state,
            };
            (self.handler)(&context, event);
        }
    }

    /// Hands over the timers that are due.
    fn fire_timers(&mut self, active: &ActiveEventLoop) {
        let due = self.state.timers.borrow_mut().take_due(Instant::now());
        for timer in due {
            self.deliver(active, Event::Timer(timer));
        }
    }

    /// Hands over what other threads sent: messages, then the input of each window.
    fn take_mail(&mut self, active: &ActiveEventLoop) {
        if self.state.is_quitting.get() {
            return;
        }
        for message in self.messages.take_all() {
            self.deliver(active, Event::Message(message));
        }
        for window in self.state.open_windows() {
            for event in window.inbox.take_all() {
                self.deliver(
                    active,
                    Event::Input {
                        window: window.id,
                        event,
                    },
                );
            }
        }
    }

    /// Hands over the frames hidden windows asked for.
    fn deliver_redraws(&mut self, active: &ActiveEventLoop) {
        if self.state.is_quitting.get() {
            return;
        }
        let windows = std::mem::take(&mut *self.state.redraws.borrow_mut());
        for window in windows {
            if let Some(shared) = self.state.window(window) {
                shared.is_redraw_asked.set(false);
                let event = InputEvent::RedrawRequested;
                self.deliver(active, Event::Input { window, event });
            }
        }
    }

    /// Hands over the theme of the system when it changed since it was last read.
    fn check_theme(&mut self, active: &ActiveEventLoop) {
        let context = Context {
            active,
            state: self.state,
        };
        let theme = context.system_theme();
        if self.state.theme.replace(Some(theme)) != Some(theme) {
            self.deliver(active, Event::ThemeChanged(theme));
        }
    }
}

impl<M, H: FnMut(&Context<'_>, Event<M>)> ApplicationHandler for Runner<'_, M, H> {
    fn new_events(&mut self, active: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::Init) {
            let context = Context {
                active,
                state: self.state,
            };
            self.state.theme.set(Some(context.system_theme()));
            self.deliver(active, Event::Started);
            self.take_mail(active);
        }
        self.fire_timers(active);
    }

    fn resumed(&mut self, _: &ActiveEventLoop) {
        // Windows are opened on `Event::Started`, which every run begins with; the systems where windows must wait
        // for this call (Android, iOS) are not systems of Gitardo.
    }

    fn user_event(&mut self, active: &ActiveEventLoop, (): ()) {
        self.take_mail(active);
    }

    fn window_event(&mut self, active: &ActiveEventLoop, window: LibraryWindowId, event: WindowEvent) {
        let id = WindowId(u64::from(window));
        let Some(shared) = self.state.window(id) else {
            return;
        };
        shared.show_to_adapter(&event);
        if matches!(event, WindowEvent::RedrawRequested) {
            let _ = self.state.redraws.borrow_mut().remove(&id);
            if !shared.is_redraw_asked.replace(false) {
                // The system asks to paint again what the window showed. When it cannot be shown again, the next
                // frame is shown whole: nothing else can be done here.
                let _ = shared.presenter.borrow_mut().present_again();
            }
        }
        for input in shared.translate(&event, self.state.clock) {
            self.deliver(
                active,
                Event::Input {
                    window: id,
                    event: input,
                },
            );
        }
        if matches!(event, WindowEvent::ThemeChanged(_) | WindowEvent::Focused(true)) {
            // `winit` says when the system goes dark or light, not when high contrast changes: that is read again
            // whenever a window gets the keyboard back, as it does after the person changed it in the settings.
            self.check_theme(active);
        }
    }

    fn about_to_wait(&mut self, active: &ActiveEventLoop) {
        self.deliver_redraws(active);
        // A loop asked to stop, here or by a frame handed over just above, goes round once more without waiting, which
        // is when `winit` sees it should stop.
        let flow = if self.state.is_quitting.get() || !self.state.redraws.borrow().is_empty() {
            ControlFlow::Poll
        } else if let Some(due) = self.state.timers.borrow().next_due() {
            ControlFlow::WaitUntil(due)
        } else {
            ControlFlow::Wait
        };
        active.set_control_flow(flow);
    }
}
