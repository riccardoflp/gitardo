//! What the tests of the places share: the window drawn in memory from the store, with the fonts that come with the
//! program; the jobs of the store run at once on the thread of the test; finding a widget the screen reader is told of
//! and clicking it.

#![allow(dead_code, reason = "each test file uses its own part of the support")]

use std::path::Path;
use std::time::Duration;

use app::store::{Effect, Message, State, ViewData, reduce};
use app::work::jobs;
use git::CancelToken;
use platform::accesskit::{Node, Role, TreeUpdate};
use render::{Appearance, Assets, FontFamily, FontWeight, FrameBuffer, Point, Theme, ThemeName};
use testkit::png::{self, Image};
use ui::{
    FrameInput, FrameSettings, InputEvent, PointerButton, PointerButtonEvent, PointerMove, Typefaces, Ui, UiMemory,
};
use views::Intent;

/// The moment the dates of the timeline are said from in the tests: three hours after the first commit of the test
/// repositories, so that the pictures do not change from one day to the next.
pub const NOW: i64 = 1_735_689_600 + 3 * 3_600;

/// A window drawn in memory at a scale of 1, in a theme.
pub struct Window {
    /// The pixels.
    pub frame: FrameBuffer,
    /// The fonts.
    assets: Assets,
    /// What the toolkit keeps between frames.
    memory: UiMemory,
    /// The typefaces.
    typefaces: Typefaces,
    /// The theme.
    theme: Theme,
    /// The time of the frames, which a test moves on to let a scroll glide to its end.
    pub now: Duration,
    /// What the last frame put on the clipboard.
    pub clipboard: Option<String>,
}

impl Window {
    /// A window of a size in logical pixels.
    pub fn new(theme: ThemeName, (width, height): (u32, u32)) -> Window {
        let mut assets = Assets::bundled();
        let mut add = |family, weight| assets.fonts.add_typeface(&family, weight).unwrap();
        let typefaces = Typefaces {
            sans: add(FontFamily::Interface, FontWeight::REGULAR),
            medium: add(FontFamily::Interface, FontWeight::MEDIUM),
            bold: add(FontFamily::Interface, FontWeight::SEMIBOLD),
            mono: add(FontFamily::Code, FontWeight::REGULAR),
        };
        Window {
            frame: FrameBuffer::new(width, height),
            assets,
            memory: UiMemory::new(),
            typefaces,
            theme: Theme::new(theme, Appearance::Dark),
            now: Duration::from_secs(1),
            clipboard: None,
        }
    }

    /// Draws a frame of a view with some input, and gives what it returned and the tree told to the screen reader.
    pub fn frame<R>(&mut self, events: &[InputEvent], view: impl FnOnce(&mut Ui<'_>) -> R) -> (R, TreeUpdate) {
        let (returned, tree, _output) = self.frame_output(events, view);
        (returned, tree)
    }

    /// Draws a frame of a view with some input, and gives what it returned, the tree told to the screen reader and what
    /// the frame asks of the window, such as text to copy.
    pub fn frame_output<R>(
        &mut self,
        events: &[InputEvent],
        view: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> (R, TreeUpdate, ui::FrameOutput) {
        let mut ui = Ui::new(FrameInput {
            frame: &mut self.frame,
            assets: &mut self.assets,
            memory: &mut self.memory,
            events,
            settings: FrameSettings {
                theme: self.theme,
                typefaces: self.typefaces,
                scale: 1.0,
                now: self.now,
            },
        });
        let returned = view(&mut ui);
        let tree = ui.accessibility_update();
        let output = ui.finish();
        self.clipboard.clone_from(&output.clipboard);
        (returned, tree, output)
    }

    /// Draws the window of a store with some input, and gives the intents of the frame and its tree.
    pub fn draw(&mut self, state: &State, events: &[InputEvent]) -> (Vec<Intent>, TreeUpdate) {
        let data = ViewData::new(state, Vec::new(), NOW);
        self.frame(events, |ui| views::view(&data.window_state(state), ui))
    }

    /// Draws the window of a store twice, the second time with a click on the widget of a role and a name, and gives
    /// the intents of that click.
    pub fn click(&mut self, state: &State, role: Role, name: &str) -> Vec<Intent> {
        self.click_times(state, (role, name), 1)
    }

    /// Draws the window of a store twice, the second time with a double click on the widget of a role and a name, and
    /// gives the intents of that double click.
    pub fn double_click(&mut self, state: &State, role: Role, name: &str) -> Vec<Intent> {
        self.click_times(state, (role, name), 2)
    }

    /// Draws the window of a store, then clicks the widget of a role and a name once or twice.
    fn click_times(&mut self, state: &State, (role, name): (Role, &str), count: u8) -> Vec<Intent> {
        let (_, tree) = self.draw(state, &[]);
        let point = center(&tree, role, name);
        self.click_at(state, point, count)
    }

    /// Draws the window of a store with the pointer moved to a point, then with a click there, once or twice, and gives
    /// the intents of the click.
    pub fn click_at(&mut self, state: &State, point: Point, count: u8) -> Vec<Intent> {
        let button = PointerButtonEvent {
            button: PointerButton::Primary,
            position: point,
            modifiers: ui::Modifiers::NONE,
            click_count: 1,
        };
        let second = PointerButtonEvent {
            click_count: 2,
            ..button
        };
        let moved = InputEvent::PointerMoved(PointerMove {
            position: point,
            modifiers: ui::Modifiers::NONE,
        });
        let (_, _) = self.draw(state, &[moved]);
        let mut events = vec![InputEvent::PointerPressed(button), InputEvent::PointerReleased(button)];
        if count > 1 {
            events.extend([InputEvent::PointerPressed(second), InputEvent::PointerReleased(second)]);
        }
        self.draw(state, &events).0
    }

    /// Checks the pixels against `tests/snapshots/<name>.png`.
    pub fn assert_snapshot(&self, name: &str) {
        let reference = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/snapshots/{name}.png"));
        let actual = Image {
            width: self.frame.width(),
            height: self.frame.height(),
            pixels: self.frame.to_rgba(),
        };
        png::assert_snapshot(reference, &actual);
    }
}

/// The node of a role and a name in the tree, which the test fails without.
pub fn find<'tree>(tree: &'tree TreeUpdate, role: Role, name: &str) -> &'tree Node {
    let found = tree
        .nodes
        .iter()
        .find(|(_, node)| node.role() == role && node.label() == Some(name))
        .map(|(_, node)| node);
    let Some(node) = found else {
        let named: Vec<String> = tree
            .nodes
            .iter()
            .filter_map(|(_, node)| node.label().map(|label| format!("{:?} {label}", node.role())))
            .collect();
        panic!("no {role:?} named {name} among {named:?}");
    };
    node
}

/// Whether the tree has a node of a role and a name.
pub fn has(tree: &TreeUpdate, role: Role, name: &str) -> bool {
    tree.nodes
        .iter()
        .any(|(_, node)| node.role() == role && node.label() == Some(name))
}

/// Whether the tree has a node whose name or value is a text.
pub fn says(tree: &TreeUpdate, text: &str) -> bool {
    tree.nodes
        .iter()
        .any(|(_, node)| node.label() == Some(text) || node.value() == Some(text))
}

/// Whether the tree has a node whose name or value holds a text.
pub fn says_in(tree: &TreeUpdate, text: &str) -> bool {
    tree.nodes.iter().any(|(_, node)| {
        node.label().is_some_and(|label| label.contains(text)) || node.value().is_some_and(|value| value.contains(text))
    })
}

/// A key that types a character, pressed with modifiers, such as Ctrl+F.
pub fn shortcut(character: char, modifiers: ui::Modifiers) -> InputEvent {
    InputEvent::KeyPressed(ui::KeyEvent::press(ui::Key::Character(character), modifiers))
}

/// Text typed.
pub fn typed(text: &str) -> InputEvent {
    InputEvent::TextTyped(text.to_owned())
}

/// The middle of the node of a role and a name.
fn center(tree: &TreeUpdate, role: Role, name: &str) -> Point {
    let bounds = find(tree, role, name).bounds().unwrap();
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a window is far smaller than an f32 counts"
    )]
    Point::new(
        f64::midpoint(bounds.x0, bounds.x1) as f32,
        f64::midpoint(bounds.y0, bounds.y1) as f32,
    )
}

/// Reduces a message, then runs at once every job it starts, as a worker would, until none is left.
pub fn run(state: &mut State, message: Message) -> Vec<Effect> {
    let mut all = Vec::new();
    let mut waiting = vec![message];
    while let Some(message) = waiting.pop() {
        for effect in reduce(state, message) {
            if let Effect::Start(job) = &effect {
                waiting.push(jobs::run(job, &CancelToken::new()));
            }
            all.push(effect);
        }
    }
    all
}

/// The thread of the operations of a window, run for a test: what the store asks of it is given to it, and what it
/// says comes back to the test, which reduces it.
pub struct OperationsDriver {
    /// The thread.
    operations: app::work::Operations,
    /// What it says.
    said: std::sync::mpsc::Receiver<Message>,
}

impl OperationsDriver {
    /// A thread of the operations of its own.
    pub fn new() -> OperationsDriver {
        OperationsDriver::with_access(app::work::github::GitHubAccess::system())
    }

    /// A thread of the operations of its own, keeping the GitHub token where the access says and running git with its
    /// variables.
    pub fn with_access(access: app::work::github::GitHubAccess) -> OperationsDriver {
        let (sender, said) = std::sync::mpsc::channel();
        let sender = std::sync::Mutex::new(sender);
        let deliver: app::work::Deliver = std::sync::Arc::new(move |message| {
            let _sent = sender.lock().unwrap().send(message);
        });
        OperationsDriver {
            operations: app::work::Operations::with_access(&engine::Activities::new(), &deliver, access),
            said,
        }
    }

    /// Gives the thread what the store asked of it among some effects.
    pub fn take(&self, state: &State, effects: &[Effect]) {
        for effect in effects {
            match effect {
                Effect::RunFeature {
                    folder,
                    feature,
                    preset,
                    context,
                } => {
                    let started = (*feature, preset.clone());
                    self.operations
                        .run(folder.clone(), started, (**context).clone(), state.settings.log_level);
                }
                Effect::ReplyToFeature(reply) => self.operations.reply(reply.clone()),
                _ => {}
            }
        }
    }

    /// Waits for what the thread says next, reduces it with the jobs it starts, and gives it.
    pub fn next(&self, state: &mut State) -> Message {
        let message = self
            .said
            .recv_timeout(Duration::from_secs(30))
            .unwrap_or_else(|_| panic!("the operation says nothing within 30 s"));
        let effects = run(state, message.clone());
        self.take(state, &effects);
        message
    }

    /// Waits for what the thread says next, and gives it as it is, for the test to reduce.
    pub fn raw_next(&self) -> Message {
        self.said
            .recv_timeout(Duration::from_secs(30))
            .unwrap_or_else(|_| panic!("the operation says nothing within 30 s"))
    }

    /// Reduces an intent with the jobs it starts, and gives the thread what it asks of it.
    pub fn intent(&self, state: &mut State, intent: Intent) {
        let effects = run(state, Message::Intent(intent));
        self.take(state, &effects);
    }
}

/// The questions to the AI provider of a window, asked for a test: the key is taken from secrets kept in memory, never
/// from the user's store, and what comes back goes to the test, which reduces it.
pub struct AiDriver {
    /// The questions.
    asking: app::work::ai::Asking,
    /// What they say.
    said: std::sync::mpsc::Receiver<Message>,
}

impl AiDriver {
    /// Questions of their own, with the secrets given.
    pub fn new(secrets: engine::MemorySecrets) -> AiDriver {
        let (sender, said) = std::sync::mpsc::channel();
        let sender = std::sync::Mutex::new(sender);
        let deliver: app::work::Deliver = std::sync::Arc::new(move |message| {
            let _sent = sender.lock().unwrap().send(message);
        });
        AiDriver {
            asking: app::work::ai::Asking::new(&deliver, &engine::Activities::new(), Box::new(secrets)),
            said,
        }
    }

    /// Asks the questions and stops those the store asked among some effects.
    pub fn take(&self, effects: &[Effect]) {
        for effect in effects {
            match effect {
                Effect::Ask(question) => self.asking.ask((**question).clone()),
                Effect::StopAsking(asks) => self.asking.stop(asks),
                _ => {}
            }
        }
    }

    /// Reduces an intent with the jobs it starts, and asks what it asks.
    pub fn intent(&self, state: &mut State, intent: Intent) -> Vec<Effect> {
        let effects = run(state, Message::Intent(intent));
        self.take(&effects);
        effects
    }

    /// Reduces what the questions say until one is answered, and gives what it came to.
    pub fn until_answered(&self, state: &mut State) -> Message {
        loop {
            let message = self
                .said
                .recv_timeout(Duration::from_secs(30))
                .unwrap_or_else(|_| panic!("no question is answered within 30 s"));
            let effects = run(state, message.clone());
            self.take(&effects);
            if matches!(message, Message::AiAnswered { .. }) {
                return message;
            }
        }
    }
}
