//! The demo window opened hidden on a real event loop and driven with synthetic input, as a person would drive it:
//! the context menu of the timeline, the message of the draft, the wheel over the timeline of a million commits, the
//! theme chosen in the menu of the repository, and closing the window. Each run has a watchdog, so that a test that waits for what never
//! comes fails instead of hanging. `winit` allows one event loop for the life of a program: each test makes the loop
//! of its own program, and only one of them runs unless the measure is asked for.

#![cfg(any(windows, target_os = "linux"))]
#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

#[path = "../../src/bin/gitardo-demo/demo.rs"]
mod demo;

use std::time::{Duration, Instant};

use demo::{Demo, Flow};
use platform::{
    Context, Event, EventLoop, InputEvent, InputSender, Key, KeyEvent, Modifiers, NamedKey, PlatformError, Point,
    PointerButton, PointerButtonEvent, PointerMove, TimerId, Visibility, WheelDelta, WheelEvent,
};
use render::ThemeName;
use ui::gallery;

/// The timer that ends a loop that waited too long.
const WATCHDOG: TimerId = TimerId(9_999);

/// How long a loop waits for what a test expects.
const PATIENCE: Duration = Duration::from_secs(20);

/// A commit of the timeline of the gallery, with "Speed up the reading of large histories".
const ROW: (f32, f32) = (600.0, 394.0);
/// Where Cherry-pick is in the context menu opened at [`ROW`]: the third item.
const CHERRY_PICK: (f32, f32) = (ROW.0 + 60.0, ROW.1 + 80.0);
/// Where the pill of the repository is, in the header, whose menu has the themes.
const REPOSITORY: (f32, f32) = (66.0, 28.0);
/// Where the message of the draft is, at the top of the timeline.
const MESSAGE: (f32, f32) = (500.0, 96.0);

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

/// The demo window, once the loop opened it, and the sender of its input.
struct Opened {
    demo: Demo,
    sender: InputSender,
}

/// Takes an event of the loop: opens the demo hidden when the loop starts, then hands it every event. Whether the demo
/// asked to stop, or the watchdog went off.
fn take(opened: &mut Option<Opened>, context: &Context<'_>, event: &Event<()>) -> bool {
    match event {
        Event::Started => {
            context.start_timer(WATCHDOG, PATIENCE);
            let demo = Demo::open(context, Visibility::Hidden).unwrap();
            let sender = demo.surface().window().input_sender();
            *opened = Some(Opened { demo, sender });
            false
        }
        Event::Timer(WATCHDOG) => true,
        _ => opened
            .as_mut()
            .is_some_and(|open| open.demo.handle(context, event).unwrap() == Flow::Quit),
    }
}

/// Opens the demo hidden and runs the stages in turn, each once the one before is done and a frame was drawn; the last
/// stage closes the window. Checks that no window is left once the demo is gone.
fn run_stages(stages: &[Stage]) -> Ending {
    let mut event_loop = EventLoop::<()>::new().unwrap();
    let mut opened: Option<Opened> = None;
    let mut current: Option<usize> = None;
    let mut ending = None;
    event_loop
        .run(|context, event| {
            let is_watchdog = matches!(event, Event::Timer(WATCHDOG));
            if take(&mut opened, context, &event) {
                let stuck = current
                    .and_then(|index| stages.get(index))
                    .map_or("the first frame", |stage| stage.name);
                ending = Some(if is_watchdog {
                    Ending::StuckAt(stuck)
                } else {
                    Ending::Closed
                });
                context.quit();
                return;
            }
            let Some(open) = opened.as_ref() else {
                return;
            };
            let is_done = current
                .and_then(|index| stages.get(index))
                .is_none_or(|stage| (stage.is_done)(&open.demo));
            if open.demo.surface().frames_drawn() > 0 && is_done {
                let next = current.map_or(0, |index| index + 1);
                if let Some(stage) = stages.get(next) {
                    for input in &stage.input {
                        open.sender.send(input.clone()).unwrap();
                    }
                    current = Some(next);
                }
            }
        })
        .unwrap();
    let Opened { demo, sender } = opened.unwrap();
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

/// Three lines of the wheel down, over the timeline.
fn wheel() -> InputEvent {
    InputEvent::Wheel(WheelEvent {
        delta: WheelDelta::Lines { x: 0.0, y: 3.0 },
        position: Point { x: ROW.0, y: ROW.1 },
        modifiers: Modifiers::NONE,
    })
}

/// How far the timeline of the gallery is scrolled: not at all when the demo opens.
fn timeline_offset(demo: &Demo) -> f64 {
    demo.surface()
        .memory()
        .list_offset(gallery::widget_id("history"))
        .unwrap_or_default()
}

/// The stages that use the menus: a context menu on a commit, then the theme chosen in the menu of the repository.
fn menu_stages() -> Vec<Stage> {
    vec![
        Stage {
            name: "the context menu opens on a commit",
            input: click(PointerButton::Secondary, ROW),
            is_done: Box::new(|demo| demo.state().menu_row.is_some()),
        },
        Stage {
            name: "an item of the context menu is chosen",
            input: click(PointerButton::Primary, CHERRY_PICK),
            is_done: Box::new(|demo| {
                demo.state()
                    .status
                    .as_deref()
                    .is_some_and(|status| status.starts_with("Cherry-picked "))
            }),
        },
        Stage {
            name: "the menu of the repository opens",
            input: click(PointerButton::Primary, REPOSITORY),
            is_done: Box::new(|demo| demo.surface().memory().focus() == Some(gallery::widget_id("repository"))),
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

/// The stages that type into the message of the draft and turn the wheel over the timeline.
fn typing_and_wheel_stages() -> Vec<Stage> {
    vec![
        Stage {
            name: "the message of the draft takes the focus",
            input: click(PointerButton::Primary, MESSAGE),
            is_done: Box::new(|demo| demo.surface().memory().focus() == Some(gallery::widget_id("draft message"))),
        },
        Stage {
            name: "text is typed into it",
            input: vec![InputEvent::TextTyped(String::from(" again"))],
            is_done: Box::new(|demo| demo.state().message.contains(" again")),
        },
        Stage {
            name: "the wheel scrolls the timeline",
            input: vec![wheel()],
            is_done: Box::new(|demo| timeline_offset(demo) > 0.0),
        },
    ]
}

#[test]
fn the_demo_window_is_driven_by_input_and_closes() {
    if cfg!(target_os = "linux")
        && ["DISPLAY", "WAYLAND_DISPLAY"]
            .iter()
            .all(|name| std::env::var_os(name).is_none())
    {
        // A Linux without a display server, such as a server, has nowhere to open a window.
        return;
    }
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
fn the_timeline_of_a_million_commits_scrolls_in_the_real_loop() {
    let mut event_loop = EventLoop::<()>::new().unwrap();
    let mut opened: Option<Opened> = None;
    let mut started: Option<(Instant, u64)> = None;
    let mut measured = None;
    let mut drawn = 0;
    event_loop
        .run(|context, event| {
            if take(&mut opened, context, &event) {
                context.quit();
                return;
            }
            let Some(open) = opened.as_ref() else {
                return;
            };
            let frames = open.demo.surface().frames_drawn();
            if frames == drawn {
                return;
            }
            drawn = frames;
            let (start, first) = *started.get_or_insert((Instant::now(), frames));
            if start.elapsed() >= MEASURED {
                measured = Some((frames - first, start.elapsed()));
                context.quit();
            } else {
                open.sender.send(wheel()).unwrap();
            }
        })
        .unwrap();
    let demo = opened.unwrap().demo;
    let (frames, elapsed) = measured.unwrap();
    let rate = f64::from(u32::try_from(frames).unwrap()) / elapsed.as_secs_f64();
    let size = demo.surface().window().size();
    let scale = demo.surface().window().scale().factor();
    println!(
        "a million commits in the demo window ({} x {} pixels, scale {scale}), a turn of the wheel each frame: {frames} \
         frames in {:.2} s, {rate:.0} frames a second",
        size.width,
        size.height,
        elapsed.as_secs_f64()
    );
}
