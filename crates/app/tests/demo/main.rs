//! The demo window opened hidden on a real event loop and driven with synthetic input, as a person would drive it:
//! the context menu of the list, an input box, the wheel over the list of a million rows, the theme chosen in the
//! menu of the toolbar, and closing the window. Each loop has a watchdog, so that a test that waits for what never
//! comes fails instead of hanging.

#![cfg(windows)]
#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

#[path = "../../src/bin/gitardo-demo/demo.rs"]
mod demo;

use std::time::{Duration, Instant};

use demo::{Demo, Flow};
use platform::{
    Event, EventLoop, InputEvent, Key, KeyEvent, Modifiers, NamedKey, PlatformError, Point, PointerButton,
    PointerButtonEvent, PointerMove, TimerId, Visibility, WheelDelta, WheelEvent,
};
use render::ThemeName;
use ui::gallery::{self, ROW_COUNT};

/// The timer that ends a loop that waited too long.
const WATCHDOG: TimerId = TimerId(9_999);

/// How long a loop waits for what a test expects.
const PATIENCE: Duration = Duration::from_secs(20);

/// A row of the list of the gallery, with "Add the login page".
const ROW: (f32, f32) = (700.0, 242.0);

/// A stage of a test: the input sent to the window, then what the demo must show before the next stage starts.
struct Stage {
    /// What the stage does, to say where a test got stuck.
    name: &'static str,
    /// The input.
    input: Vec<InputEvent>,
    /// Whether the demo shows what the input should have done.
    is_done: Box<dyn Fn(&Demo) -> bool>,
}

/// How a run of stages ended.
#[derive(Debug, PartialEq, Eq)]
enum Ending {
    /// The window was closed once every stage was done.
    Closed,
    /// The watchdog went off during a stage.
    StuckAt(&'static str),
}

/// Opens the demo hidden and runs the stages in turn, each once the one before is done and a frame was drawn; the last
/// stage closes the window. Checks that no window is left once the demo is gone.
fn run_stages(stages: &[Stage]) -> Ending {
    let event_loop = EventLoop::<()>::new().unwrap();
    let mut demo = Demo::open(&event_loop, Visibility::Hidden).unwrap();
    let sender = demo.surface().window().input_sender();
    let mut current: Option<usize> = None;
    let mut ending = None;
    event_loop.start_timer(WATCHDOG, PATIENCE).unwrap();
    event_loop
        .run(|event| {
            if matches!(event, Event::Timer(WATCHDOG)) {
                let stuck = current
                    .and_then(|index| stages.get(index))
                    .map_or("the first frame", |stage| stage.name);
                ending = Some(Ending::StuckAt(stuck));
                event_loop.quit();
                return;
            }
            if demo.handle(&event_loop, &event).unwrap() == Flow::Quit {
                ending = Some(Ending::Closed);
                event_loop.quit();
                return;
            }
            let is_done = current
                .and_then(|index| stages.get(index))
                .is_none_or(|stage| (stage.is_done)(&demo));
            if demo.surface().frames_drawn() > 0 && is_done {
                let next = current.map_or(0, |index| index + 1);
                if let Some(stage) = stages.get(next) {
                    for input in &stage.input {
                        sender.send(input.clone()).unwrap();
                    }
                    current = Some(next);
                }
            }
        })
        .unwrap();
    event_loop.stop_timer(WATCHDOG);
    drop(demo);
    assert_eq!(
        sender.send(InputEvent::PointerLeft),
        Err(PlatformError::Closed),
        "the window is gone"
    );
    ending.unwrap()
}

/// A click of a button at a point.
fn click(button: PointerButton, (x, y): (f32, f32)) -> Vec<InputEvent> {
    let position = Point { x, y };
    let event = PointerButtonEvent {
        button,
        position,
        modifiers: Modifiers::NONE,
        click_count: 1,
    };
    vec![
        InputEvent::PointerMoved(PointerMove {
            position,
            modifiers: Modifiers::NONE,
        }),
        InputEvent::PointerPressed(event),
        InputEvent::PointerReleased(event),
    ]
}

/// A key pressed.
fn press(key: Key) -> InputEvent {
    InputEvent::KeyPressed(KeyEvent {
        key,
        modifiers: Modifiers::NONE,
        is_repeat: false,
    })
}

/// Three lines of the wheel down, over the list.
fn wheel() -> InputEvent {
    InputEvent::Wheel(WheelEvent {
        delta: WheelDelta::Lines { x: 0.0, y: 3.0 },
        position: Point { x: ROW.0, y: ROW.1 },
        modifiers: Modifiers::NONE,
    })
}

/// How far the list of the gallery is scrolled.
fn list_offset(demo: &Demo) -> f64 {
    demo.surface()
        .memory()
        .list_offset(gallery::widget_id("commits"))
        .unwrap_or_default()
}

/// How far the list of the gallery is scrolled when the demo opens.
fn starting_offset(demo: &Demo) -> f64 {
    let rows = u32::try_from(ROW_COUNT / 2 - 4).unwrap();
    f64::from(rows) * f64::from(demo.surface().theme().sizes.row_height)
}

/// The stages that use the menus: a context menu on a row, then the theme chosen in the menu of the toolbar.
fn menu_stages() -> Vec<Stage> {
    vec![
        Stage {
            name: "the context menu opens on a row",
            input: click(PointerButton::Secondary, ROW),
            is_done: Box::new(|demo| demo.state().menu_row.is_some()),
        },
        Stage {
            name: "an item of the context menu is chosen",
            input: click(PointerButton::Primary, (ROW.0 + 60.0, ROW.1 + 59.0)),
            is_done: Box::new(|demo| {
                demo.state()
                    .status
                    .as_deref()
                    .is_some_and(|status| status.starts_with("Cherry-picked "))
            }),
        },
        Stage {
            name: "the menu of the toolbar opens",
            input: click(PointerButton::Primary, (1259.0, 16.0)),
            is_done: Box::new(|demo| demo.surface().memory().focus() == Some(gallery::widget_id("more"))),
        },
        Stage {
            name: "the light theme is chosen with the keys",
            input: [Key::Character('t'), Key::Named(NamedKey::ArrowRight)]
                .into_iter()
                .chain([NamedKey::ArrowDown, NamedKey::ArrowDown, NamedKey::Enter].map(Key::Named))
                .map(press)
                .collect(),
            is_done: Box::new(|demo| demo.surface().theme().name == ThemeName::Light),
        },
    ]
}

/// The stages that type into an input box and turn the wheel over the list.
fn typing_and_wheel_stages() -> Vec<Stage> {
    vec![
        Stage {
            name: "an input box takes the focus",
            input: click(PointerButton::Primary, (220.0, 458.0)),
            is_done: Box::new(|demo| demo.surface().memory().focus() == Some(gallery::widget_id("search"))),
        },
        Stage {
            name: "text is typed into it",
            input: vec![InputEvent::TextTyped(String::from("login"))],
            is_done: Box::new(|demo| demo.state().search == "login"),
        },
        Stage {
            name: "the wheel scrolls the list",
            input: vec![wheel()],
            is_done: Box::new(|demo| list_offset(demo) > starting_offset(demo)),
        },
    ]
}

#[test]
fn the_demo_window_is_driven_by_input_and_closes() {
    let mut stages = menu_stages();
    stages.extend(typing_and_wheel_stages());
    stages.push(Stage {
        name: "the window closes",
        input: vec![InputEvent::CloseRequested],
        is_done: Box::new(|_| false),
    });
    assert_eq!(run_stages(&stages), Ending::Closed);
}

/// How long the list is scrolled for the measure.
const MEASURED: Duration = Duration::from_secs(2);

#[test]
#[ignore = "a measure: run it in release with --ignored --nocapture"]
#[allow(clippy::print_stdout, reason = "the measure is printed")]
fn the_list_of_a_million_rows_scrolls_in_the_real_loop() {
    let event_loop = EventLoop::<()>::new().unwrap();
    let mut demo = Demo::open(&event_loop, Visibility::Hidden).unwrap();
    let sender = demo.surface().window().input_sender();
    let mut started: Option<(Instant, u64)> = None;
    let mut measured = None;
    let mut drawn = 0;
    event_loop.start_timer(WATCHDOG, PATIENCE).unwrap();
    event_loop
        .run(|event| {
            if matches!(event, Event::Timer(WATCHDOG)) || demo.handle(&event_loop, &event).unwrap() == Flow::Quit {
                event_loop.quit();
                return;
            }
            let frames = demo.surface().frames_drawn();
            if frames == drawn {
                return;
            }
            drawn = frames;
            let (start, first) = *started.get_or_insert((Instant::now(), frames));
            if start.elapsed() >= MEASURED {
                measured = Some((frames - first, start.elapsed()));
                event_loop.quit();
            } else {
                sender.send(wheel()).unwrap();
            }
        })
        .unwrap();
    event_loop.stop_timer(WATCHDOG);
    let (frames, elapsed) = measured.unwrap();
    let rate = f64::from(u32::try_from(frames).unwrap()) / elapsed.as_secs_f64();
    let size = demo.surface().window().size().unwrap();
    let scale = demo.surface().window().scale().factor();
    println!(
        "a million rows in the demo window ({} x {} pixels, scale {scale}), a turn of the wheel each frame: {frames} \
         frames in {:.2} s, {rate:.0} frames a second",
        size.width,
        size.height,
        elapsed.as_secs_f64()
    );
}
