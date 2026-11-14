//! Hidden windows on a real event loop: messages from other threads, timers, synthetic input, redraws, frames, the
//! theme and the screen reader. `winit` allows one event loop for the life of a program, so every check runs on the
//! same loop, in turn, in one test. Every run has a watchdog timer, so that a check that waits for an event that never
//! comes fails instead of hanging.

// The helpers of these tests fail as the tests do, loudly: clippy allows them what it allows tests.
#![cfg(test)]
#![cfg(any(windows, target_os = "linux"))]

use std::thread;
use std::time::Duration;

use platform::accesskit::{Node, NodeId, Role, TreeId, TreeInfo, TreeUpdate};
use platform::{
    AccessibilityRequest, Appearance, Context, Cursor, Event, EventLoop, Frame, InputEvent, Key, KeyEvent, LogicalSize,
    Modifiers, NamedKey, PhysicalSize, PixelRect, PlatformError, Point, PointerButton, PointerButtonEvent,
    ScreenPosition, TextInputArea, TimerId, Visibility, Window, WindowOptions,
};

/// The timer that ends a run that waited too long.
const WATCHDOG: TimerId = TimerId(9_999);

/// How long a run waits for what a check expects.
const PATIENCE: Duration = Duration::from_secs(10);

/// The loop the checks run on.
type Loop = EventLoop<u32>;

/// Runs the loop until `is_done` says so, or the watchdog gives up. Whether `is_done` said so.
fn run_until(event_loop: &mut Loop, mut is_done: impl FnMut(&Context<'_>, Event<u32>) -> bool) -> bool {
    let mut is_finished = false;
    event_loop
        .run(|context, event| match event {
            Event::Started => context.start_timer(WATCHDOG, PATIENCE),
            Event::Timer(WATCHDOG) => context.quit(),
            other => {
                if is_done(context, other) {
                    is_finished = true;
                    context.stop_timer(WATCHDOG);
                    context.quit();
                }
            }
        })
        .unwrap();
    is_finished
}

/// Opens a window of 320 by 200 logical pixels that never shows.
fn hidden_window(event_loop: &mut Loop) -> Window {
    let options = WindowOptions {
        title: String::from("Gitardo Desktop test"),
        size: LogicalSize {
            width: 320.0,
            height: 200.0,
        },
        minimum_size: LogicalSize {
            width: 100.0,
            height: 100.0,
        },
        visibility: Visibility::Hidden,
        title_bar: Some(Appearance::Dark),
    };
    let mut window = None;
    event_loop
        .run(|context, event| {
            if event == Event::Started {
                window = Some(Window::open(context, &options).unwrap());
                context.quit();
            }
        })
        .unwrap();
    window.unwrap()
}

/// Whether an event is a request for a frame of a window.
fn is_redraw(event: &Event<u32>, window: &Window) -> bool {
    *event
        == Event::Input {
            window: window.id(),
            event: InputEvent::RedrawRequested,
        }
}

#[test]
fn windows_and_their_input_on_the_event_loop_of_the_program() {
    if cfg!(target_os = "linux")
        && ["DISPLAY", "WAYLAND_DISPLAY"]
            .iter()
            .all(|name| std::env::var_os(name).is_none())
    {
        // A Linux without a display server, such as a server, has nowhere to open a window.
        return;
    }
    let mut event_loop = Loop::new().unwrap();
    assert_eq!(Loop::new().err(), Some(PlatformError::EventLoopExists));
    delivers_the_message_of_a_waker_from_another_thread(&mut event_loop);
    delivers_every_message_of_many_wakers_in_order(&mut event_loop);
    fires_a_timer_until_it_is_stopped(&mut event_loop);
    answers_what_the_theme_is(&mut event_loop);
    let window = hidden_window(&mut event_loop);
    asks_for_a_frame_when_a_window_opens_and_when_asked(&mut event_loop, &window);
    hands_synthetic_input_to_the_handler_in_order(&mut event_loop, &window);
    presents_frames(&mut event_loop, &window);
    knows_and_changes_its_scale_size_and_place(&mut event_loop, &window);
    hands_the_requests_of_a_screen_reader_over_and_takes_a_tree(&mut event_loop, &window);
    keeps_windows_of_one_loop_apart(&mut event_loop, &window);
    refuses_input_once_the_window_is_closed(window);
    let waker = event_loop.waker();
    drop(event_loop);
    assert_eq!(waker.wake(1), Err(PlatformError::Closed));
}

fn delivers_the_message_of_a_waker_from_another_thread(event_loop: &mut Loop) {
    let waker = event_loop.waker();
    let sender = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        waker.wake(42).unwrap();
    });
    let mut messages = Vec::new();
    let is_finished = run_until(event_loop, |_, event| {
        if let Event::Message(message) = event {
            messages.push(message);
        }
        !messages.is_empty()
    });
    sender.join().unwrap();
    assert!(is_finished, "the message never came");
    assert_eq!(messages, vec![42]);
}

fn delivers_every_message_of_many_wakers_in_order(event_loop: &mut Loop) {
    let senders: Vec<_> = (0..4)
        .map(|thread_number| {
            let waker = event_loop.waker();
            thread::spawn(move || (0..100).for_each(|count| waker.wake(thread_number * 1000 + count).unwrap()))
        })
        .collect();
    let mut messages = Vec::new();
    let is_finished = run_until(event_loop, |_, event| {
        if let Event::Message(message) = event {
            messages.push(message);
        }
        messages.len() == 400
    });
    for sender in senders {
        sender.join().unwrap();
    }
    assert!(is_finished, "{} messages of 400 came", messages.len());
    for thread_number in 0..4 {
        let of_thread: Vec<u32> = messages
            .iter()
            .copied()
            .filter(|message| message / 1000 == thread_number)
            .collect();
        assert_eq!(
            of_thread,
            (0..100).map(|count| thread_number * 1000 + count).collect::<Vec<_>>()
        );
    }
}

fn fires_a_timer_until_it_is_stopped(event_loop: &mut Loop) {
    let timer = TimerId(1);
    let mut fired = 0;
    let mut is_started = false;
    // A message, so that the handler is called once to start the timer.
    event_loop.waker().wake(0).unwrap();
    let is_finished = run_until(event_loop, |context, event| {
        if !is_started {
            context.start_timer(timer, Duration::from_millis(15));
            is_started = true;
        }
        if event == Event::Timer(timer) {
            fired += 1;
        }
        if fired == 3 {
            context.stop_timer(timer);
        }
        fired == 3
    });
    assert!(is_finished, "the timer fired {fired} times");
}

fn answers_what_the_theme_is(event_loop: &mut Loop) {
    let mut themes = Vec::new();
    let waker = event_loop.waker();
    waker.wake(0).unwrap();
    let is_finished = run_until(event_loop, |context, _| {
        themes.push(context.system_theme());
        themes.push(context.system_theme());
        true
    });
    assert!(is_finished);
    assert_eq!(themes.first(), themes.last());
}

fn asks_for_a_frame_when_a_window_opens_and_when_asked(event_loop: &mut Loop, window: &Window) {
    let mut redraws = 0;
    let is_finished = run_until(event_loop, |_, event| {
        if is_redraw(&event, window) {
            redraws += 1;
            if redraws == 1 {
                window.request_redraw();
                window.request_redraw();
            }
        }
        redraws == 2
    });
    assert!(is_finished, "{redraws} redraws came");
}

fn hands_synthetic_input_to_the_handler_in_order(event_loop: &mut Loop, window: &Window) {
    let key = KeyEvent {
        key: Key::Named(NamedKey::Enter),
        modifiers: Modifiers::CONTROL,
        is_repeat: false,
    };
    let click = PointerButtonEvent {
        button: PointerButton::Primary,
        position: Point { x: 10.0, y: 20.0 },
        modifiers: Modifiers::NONE,
        click_count: 2,
    };
    window.send_input(InputEvent::KeyPressed(key)).unwrap();
    window.send_input(InputEvent::PointerPressed(click)).unwrap();
    let sender = window.input_sender();
    let typist = thread::spawn(move || sender.send(InputEvent::TextTyped(String::from("é😀"))).unwrap());
    let mut seen = Vec::new();
    let is_finished = run_until(event_loop, |_, event| {
        if let Event::Input { window: id, event } = event {
            assert_eq!(id, window.id());
            if matches!(
                event,
                InputEvent::KeyPressed(_) | InputEvent::PointerPressed(_) | InputEvent::TextTyped(_)
            ) {
                seen.push(event);
            }
        }
        seen.len() == 3
    });
    typist.join().unwrap();
    assert!(is_finished, "only {seen:?} came");
    let expected = [
        InputEvent::KeyPressed(key),
        InputEvent::PointerPressed(click),
        InputEvent::TextTyped(String::from("é😀")),
    ];
    assert_eq!(seen, expected);
}

fn presents_frames(event_loop: &mut Loop, window: &Window) {
    let size = window.size();
    assert_eq!(
        size,
        window.scale().to_physical(LogicalSize {
            width: 320.0,
            height: 200.0
        })
    );
    let pixels: Vec<u32> = (0..size.width * size.height).map(|index| index % 0x00FF_FFFF).collect();
    let frame = Frame { pixels: &pixels, size };
    let changed = PixelRect {
        x: 5,
        y: 5,
        width: 50,
        height: 20,
    };
    let outside = PixelRect {
        x: 10_000,
        y: 0,
        width: 1,
        height: 1,
    };
    window.request_redraw();
    let mut results = Vec::new();
    let is_finished = run_until(event_loop, |_, event| {
        if is_redraw(&event, window) {
            results.push(window.present(frame, &[]));
            results.push(window.present(frame, &[changed]));
            results.push(window.present(frame, &[outside]));
            return true;
        }
        false
    });
    assert!(is_finished, "no redraw came");
    assert_eq!(results, vec![Ok(()), Ok(()), Ok(())]);
    let short = Frame {
        pixels: &pixels[1..],
        size,
    };
    assert!(matches!(
        window.present(short, &[]),
        Err(PlatformError::FrameSize { .. })
    ));
    let empty = Frame {
        pixels: &[],
        size: PhysicalSize::default(),
    };
    assert_eq!(window.present(empty, &[]), Ok(()));
}

fn knows_and_changes_its_scale_size_and_place(event_loop: &mut Loop, window: &Window) {
    assert!(window.scale().factor() >= 1.0, "{:?}", window.scale());
    window.set_size(LogicalSize {
        width: 400.0,
        height: 300.0,
    });
    window.set_position(ScreenPosition { x: 40, y: 60 });
    window.set_title("Another title");
    window.set_title_bar(Some(Appearance::Light));
    window.set_title_bar(None);
    window.set_cursor(Cursor::Text);
    window.set_cursor(Cursor::Arrow);
    let area = TextInputArea {
        position: Point { x: 10.0, y: 10.0 },
        size: LogicalSize {
            width: 1.0,
            height: 16.0,
        },
    };
    window.set_text_input_area(Some(area));
    window.set_text_input_area(None);
    let expected = window.scale().to_physical(LogicalSize {
        width: 400.0,
        height: 300.0,
    });
    let mut sizes = Vec::new();
    let is_finished = run_until(event_loop, |_, event| {
        if let Event::Input {
            event: InputEvent::Resized(size),
            ..
        } = event
        {
            sizes.push(size);
        }
        window.size() == expected
    });
    assert!(is_finished, "the sizes were {sizes:?}");
    if cfg!(windows) {
        assert_eq!(window.position(), Ok(ScreenPosition { x: 40, y: 60 }));
    } else {
        // X11 places a window that was never shown when it is shown, and Wayland never says where a window is.
        let position = window.position();
        assert!(
            matches!(position, Ok(_) | Err(PlatformError::Unsupported { .. })),
            "{position:?}"
        );
    }
}

fn hands_the_requests_of_a_screen_reader_over_and_takes_a_tree(event_loop: &mut Loop, window: &Window) {
    let root = NodeId(1);
    let mut node = Node::new(Role::Window);
    node.set_label("Gitardo Desktop test");
    // No screen reader listens while the tests run: the tree is taken and kept aside.
    window.update_accessibility(TreeUpdate {
        nodes: vec![(root, node)],
        tree: Some(TreeInfo::new(root)),
        tree_id: TreeId::ROOT,
        focus: root,
    });
    let request = InputEvent::Accessibility(AccessibilityRequest::InitialTree);
    window.send_input(request.clone()).unwrap();
    let is_finished = run_until(event_loop, |_, event| {
        event
            == Event::Input {
                window: window.id(),
                event: request.clone(),
            }
    });
    assert!(is_finished, "the request never came");
}

fn keeps_windows_of_one_loop_apart(event_loop: &mut Loop, first: &Window) {
    let second = hidden_window(event_loop);
    assert_ne!(first.id(), second.id());
    second.send_input(InputEvent::CloseRequested).unwrap();
    let mut closed = None;
    let is_finished = run_until(event_loop, |_, event| {
        if let Event::Input {
            window,
            event: InputEvent::CloseRequested,
        } = event
        {
            closed = Some(window);
        }
        closed.is_some()
    });
    assert!(is_finished);
    assert_eq!(closed, Some(second.id()));
}

fn refuses_input_once_the_window_is_closed(window: Window) {
    let sender = window.input_sender();
    drop(window);
    assert_eq!(sender.send(InputEvent::FocusGained), Err(PlatformError::Closed));
}
