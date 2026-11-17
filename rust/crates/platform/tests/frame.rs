//! A hidden window whose title bar the program draws, on a real event loop of its own (`winit` allows one for the life
//! of a program, so it is apart from the other windows of the tests).

#![cfg(test)]
#![cfg(any(windows, target_os = "linux"))]
#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use platform::{Decorations, Event, EventLoop, LogicalSize, Visibility, Window, WindowOptions};

#[test]
fn window_title_bar__the_window_opens_with_no_title_bar_of_the_system() {
    let mut event_loop = EventLoop::<u32>::new().unwrap();
    let options = WindowOptions {
        title: String::from("Gitardo Desktop test"),
        size: LogicalSize {
            width: 320.0,
            height: 200.0,
        },
        visibility: Visibility::Hidden,
        decorations: Decorations::Drawn,
        ..WindowOptions::default()
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
    let window = window.unwrap();
    let inside = window.size();
    let outside = window.outer_size();
    // The inside covers the whole window: no title bar above it, no border around it.
    assert_eq!(outside.width, inside.width);
    assert!(outside.height <= inside.height + 1, "{outside:?} around {inside:?}");
    window.set_frame_parts(|_, _| platform::FramePart::Caption);
    assert!(!window.is_maximized());
}
