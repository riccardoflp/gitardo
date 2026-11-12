//! Hidden windows on a real event loop: messages from other threads, timers, synthetic input, redraws and frames.
//! Every loop has a watchdog timer, so that a test that waits for an event that never comes fails instead of hanging.

// The helpers of these tests fail as the tests do, loudly: clippy allows them what it allows tests.
#![cfg(test)]
#![cfg(windows)]

use std::thread;
use std::time::Duration;

use platform::{
    Appearance, Cursor, Event, EventLoop, Frame, InputEvent, Key, KeyEvent, LogicalSize, Modifiers, NamedKey,
    PhysicalSize, PixelRect, PlatformError, Point, PointerButton, PointerButtonEvent, ScreenPosition, TimerId,
    Visibility, Window, WindowOptions,
};

/// The timer that ends a loop that waited too long.
const WATCHDOG: TimerId = TimerId(9_999);

/// How long a loop waits for what a test expects.
const PATIENCE: Duration = Duration::from_secs(10);

/// A window of 320 by 200 logical pixels that never shows.
fn hidden_window<M: 'static>(event_loop: &EventLoop<M>) -> Window {
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
        title_bar: Appearance::Dark,
    };
    Window::open(event_loop, &options).unwrap()
}

/// Runs the loop until `is_done` says so, or the watchdog gives up. Whether `is_done` said so.
fn run_until<M: 'static>(event_loop: &EventLoop<M>, mut is_done: impl FnMut(Event<M>) -> bool) -> bool {
    event_loop.start_timer(WATCHDOG, PATIENCE).unwrap();
    let mut is_finished = false;
    event_loop
        .run(|event| {
            if matches!(event, Event::Timer(WATCHDOG)) {
                event_loop.quit();
            } else if is_done(event) {
                is_finished = true;
                event_loop.quit();
            }
        })
        .unwrap();
    event_loop.stop_timer(WATCHDOG);
    is_finished
}

#[test]
fn delivers_the_message_of_a_waker_from_another_thread() {
    let event_loop = EventLoop::<u32>::new().unwrap();
    let waker = event_loop.waker();
    let sender = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        waker.wake(42).unwrap();
    });
    let mut messages = Vec::new();
    let is_finished = run_until(&event_loop, |event| {
        if let Event::Message(message) = event {
            messages.push(message);
        }
        !messages.is_empty()
    });
    sender.join().unwrap();
    assert!(is_finished, "the message never came");
    assert_eq!(messages, vec![42]);
}

#[test]
fn delivers_every_message_of_many_wakers_in_order() {
    let event_loop = EventLoop::<u32>::new().unwrap();
    let senders: Vec<_> = (0..4)
        .map(|thread_number| {
            let waker = event_loop.waker();
            thread::spawn(move || (0..100).for_each(|count| waker.wake(thread_number * 1000 + count).unwrap()))
        })
        .collect();
    let mut messages = Vec::new();
    let is_finished = run_until(&event_loop, |event| {
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

#[test]
fn refuses_a_message_once_the_loop_is_gone() {
    let event_loop = EventLoop::<u32>::new().unwrap();
    let waker = event_loop.waker();
    drop(event_loop);
    assert_eq!(waker.wake(1), Err(PlatformError::Closed));
}

#[test]
fn fires_a_timer_until_it_is_stopped() {
    let event_loop = EventLoop::<()>::new().unwrap();
    let timer = TimerId(1);
    event_loop.start_timer(timer, Duration::from_millis(15)).unwrap();
    let mut fired = 0;
    let is_finished = run_until(&event_loop, |event| {
        if event == Event::Timer(timer) {
            fired += 1;
        }
        fired == 3
    });
    event_loop.stop_timer(timer);
    assert!(is_finished, "the timer fired {fired} times");
}

#[test]
fn hands_synthetic_input_to_the_handler_in_order() {
    let event_loop = EventLoop::<()>::new().unwrap();
    let window = hidden_window(&event_loop);
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
    let is_finished = run_until(&event_loop, |event| {
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

#[test]
fn refuses_input_once_the_window_is_closed() {
    let event_loop = EventLoop::<()>::new().unwrap();
    let window = hidden_window(&event_loop);
    let sender = window.input_sender();
    drop(window);
    assert_eq!(sender.send(InputEvent::FocusGained), Err(PlatformError::Closed));
}

#[test]
fn asks_for_a_frame_when_a_window_opens_and_when_asked() {
    let event_loop = EventLoop::<()>::new().unwrap();
    let window = hidden_window(&event_loop);
    let mut redraws = 0;
    let is_finished = run_until(&event_loop, |event| {
        if event
            == (Event::Input {
                window: window.id(),
                event: InputEvent::RedrawRequested,
            })
        {
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

#[test]
fn presents_frames() {
    let event_loop = EventLoop::<()>::new().unwrap();
    let window = hidden_window(&event_loop);
    let size = window.size().unwrap();
    assert_eq!(
        size,
        window.scale().to_physical(LogicalSize {
            width: 320.0,
            height: 200.0
        })
    );
    let pixels: Vec<u32> = (0..size.width * size.height).map(|index| index % 0x00FF_FFFF).collect();
    let frame = Frame { pixels: &pixels, size };
    let mut results = Vec::new();
    let is_finished = run_until(&event_loop, |event| {
        if event
            == (Event::Input {
                window: window.id(),
                event: InputEvent::RedrawRequested,
            })
        {
            results.push(window.present(frame, &[]));
            results.push(window.present(
                frame,
                &[PixelRect {
                    x: 5,
                    y: 5,
                    width: 50,
                    height: 20,
                }],
            ));
            results.push(window.present(
                frame,
                &[PixelRect {
                    x: 10_000,
                    y: 0,
                    width: 1,
                    height: 1,
                }],
            ));
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

#[test]
fn knows_and_changes_its_scale_size_and_place() {
    let event_loop = EventLoop::<()>::new().unwrap();
    let window = hidden_window(&event_loop);
    assert!(window.scale().factor() >= 1.0, "{:?}", window.scale());
    window
        .set_size(LogicalSize {
            width: 400.0,
            height: 300.0,
        })
        .unwrap();
    assert_eq!(
        window.size().unwrap(),
        window.scale().to_physical(LogicalSize {
            width: 400.0,
            height: 300.0
        })
    );
    window.set_position(ScreenPosition { x: 40, y: 60 }).unwrap();
    assert_eq!(window.position().unwrap(), ScreenPosition { x: 40, y: 60 });
    window.set_title("Another title").unwrap();
    window.set_title_bar(Appearance::Light).unwrap();
    window.set_cursor(Cursor::Text);
    window.set_cursor(Cursor::Arrow);
    window.set_text_input_area(Point { x: 10.0, y: 10.0 }).unwrap();
    let mut sizes = Vec::new();
    let is_finished = run_until(&event_loop, |event| {
        if let Event::Input {
            event: InputEvent::Resized(size),
            ..
        } = event
        {
            sizes.push(size);
        }
        sizes.last() == Some(&window.size().unwrap())
    });
    assert!(is_finished, "the sizes were {sizes:?}");
}

#[test]
fn keeps_windows_of_one_loop_apart() {
    let event_loop = EventLoop::<()>::new().unwrap();
    let first = hidden_window(&event_loop);
    let second = hidden_window(&event_loop);
    assert_ne!(first.id(), second.id());
    second.send_input(InputEvent::CloseRequested).unwrap();
    let mut closed = None;
    let is_finished = run_until(&event_loop, |event| {
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
