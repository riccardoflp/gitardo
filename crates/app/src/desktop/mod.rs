//! The window of Gitardo Desktop: the store, the views drawn from it, the workers that run its jobs, the watcher of
//! its settings files and the writer of what is kept between runs, around one event loop.
//!
//! Each event goes to the store as a [`Message`]; what `reduce` asks is carried out at once, in order, and a frame is
//! asked for. The intents of each frame are reduced after it.

mod running;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use engine::Activities;
use platform::{Context, Event, EventLoop, FileDialog, LogicalSize, TimerId, Visibility};

use crate::error::AppError;
use crate::persisted::{FILE_NAME, Persisted};
use crate::settings::{self, SettingsFiles, keys, read_files, user_settings_file};
use crate::store::{Effect, Message, State, ViewData, reduce};
use crate::window::{Step, Surface, SurfaceOptions};
use crate::work::{Deliver, PersistedWriter, SettingWriter, SettingsWatcher, Workers};

/// The timer of the frames of the window.
const FRAME_TIMER: TimerId = TimerId(1);
/// How many workers a window has.
const WORKERS: usize = 4;
/// The size of the window the first time, in logical pixels.
const FIRST_SIZE: (f32, f32) = (1280.0, 800.0);

/// How the program starts.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StartOptions {
    /// The folders of the command line, the workspace opened at once.
    pub folders: Vec<PathBuf>,
    /// Gitardo's configuration folder, with the settings and what is kept between runs; none when the system does
    /// not say where the user's folders are, and then nothing is read or kept.
    pub configuration: Option<PathBuf>,
    /// Whether the window shows at once: tests keep it hidden.
    pub visibility: Visibility,
}

/// Opens the window and runs until it is closed.
///
/// # Errors
///
/// `AppError::Platform` when the event loop or the window could not be made, and the errors of the fonts.
pub fn run(options: &StartOptions) -> Result<(), AppError> {
    let mut event_loop = EventLoop::<Message>::new()?;
    let waker = event_loop.waker();
    let deliver: Deliver = Arc::new(move |message| {
        // A message for a loop that is gone is not wanted.
        let _delivered = waker.wake(message);
    });
    let mut window: Option<DesktopWindow> = None;
    let mut failure = None;
    event_loop.run(|context, event| {
        let flow = match (&mut window, &event) {
            (Some(open), _) => open.handle(context, event),
            (None, Event::Started) => DesktopWindow::open(context, options, &deliver).and_then(|opened| {
                let open = window.insert(opened);
                open.dispatch(Message::Started {
                    folders: options.folders.clone(),
                })
            }),
            (None, _) => Ok(Flow::Continue),
        };
        match flow {
            Ok(Flow::Continue) => {}
            Ok(Flow::Quit) => context.quit(),
            Err(error) => {
                failure = Some(error);
                context.quit();
            }
        }
    })?;
    drop(window);
    failure.map_or(Ok(()), Err)
}

/// Whether the loop goes on after an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flow {
    /// It goes on.
    Continue,
    /// The window closed.
    Quit,
}

/// The window, with its store and the threads that work for it.
struct DesktopWindow {
    /// The window of the platform and the toolkit.
    surface: Surface,
    /// The store.
    state: State,
    /// The workers.
    workers: Workers,
    /// The watcher of the settings files.
    watcher: SettingsWatcher,
    /// The writer of what is kept between runs; none when there is no configuration folder.
    writer: Option<PersistedWriter>,
    /// The writer of the settings the window changes.
    setting_writer: SettingWriter,
    /// What runs, as the workers tell it.
    activities: Activities,
    /// When each activity was first seen, for how long it has run.
    running: running::RunningClock,
}

impl DesktopWindow {
    /// Opens the window with what was kept from the runs before and the user's settings.
    fn open(context: &Context<'_>, options: &StartOptions, deliver: &Deliver) -> Result<DesktopWindow, AppError> {
        let persisted = options
            .configuration
            .as_ref()
            .map(|folder| Persisted::read(&folder.join(FILE_NAME)))
            .unwrap_or_default();
        let user_file = options.configuration.as_deref().map(user_settings_file);
        let settings = read_files(&SettingsFiles {
            user: user_file.clone(),
            repository: None,
        });
        let (width, height) = persisted.window_size.unwrap_or(FIRST_SIZE);
        let mut surface = Surface::open(
            context,
            SurfaceOptions {
                title: views::text::window_title(None),
                size: LogicalSize { width, height },
                visibility: options.visibility,
                theme: settings.theme,
                frame_timer: FRAME_TIMER,
            },
        )?;
        apply_text(&mut surface, &settings)?;
        let activities = Activities::new();
        Ok(DesktopWindow {
            surface,
            state: State::new(persisted, settings, user_file),
            workers: Workers::new(WORKERS, &activities, deliver),
            watcher: SettingsWatcher::new(Arc::clone(deliver)),
            writer: options
                .configuration
                .as_ref()
                .map(|folder| PersistedWriter::new(folder.join(FILE_NAME))),
            setting_writer: SettingWriter::new(),
            activities,
            running: running::RunningClock::default(),
        })
    }

    /// Takes an event of the loop.
    fn handle(&mut self, context: &Context<'_>, event: Event<Message>) -> Result<Flow, AppError> {
        if let Event::Message(message) = event {
            return self.dispatch(message);
        }
        match self.surface.handle(&event) {
            Step::Draw => self.draw(context),
            Step::Close => {
                let size = self.surface.window().size();
                let scale = self.surface.window().scale().factor();
                #[allow(
                    clippy::cast_precision_loss,
                    reason = "a window is far smaller than an f32 counts exactly"
                )]
                let window_size = Some((size.width as f32 / scale, size.height as f32 / scale));
                self.dispatch(Message::CloseRequested { window_size })
            }
            Step::Ignored | Step::Handled => Ok(Flow::Continue),
        }
    }

    /// Reduces a message and the ones that follow from it, carries out what the store asks, and asks for a frame.
    fn dispatch(&mut self, message: Message) -> Result<Flow, AppError> {
        let mut waiting = VecDeque::from([message]);
        let mut flow = Flow::Continue;
        while let Some(message) = waiting.pop_front() {
            if let Message::RepositoriesFound { job, .. } | Message::RepositoryRead { job, .. } = &message {
                self.workers.finished(*job);
            }
            for effect in reduce(&mut self.state, message) {
                match self.carry_out(effect)? {
                    Some(Followed::Message(next)) => waiting.push_back(next),
                    Some(Followed::Quit) => flow = Flow::Quit,
                    None => {}
                }
            }
        }
        self.surface.window().request_redraw();
        Ok(flow)
    }

    /// Carries out what the store asked.
    fn carry_out(&mut self, effect: Effect) -> Result<Option<Followed>, AppError> {
        match effect {
            Effect::Start(job) => self.workers.start(job),
            Effect::Cancel(job) => self.workers.cancel(job),
            Effect::ChooseFolder => {
                let dialog = FileDialog {
                    title: String::from(views::text::repo::CHOOSE_FOLDER),
                    ..FileDialog::default()
                };
                let folder = dialog.choose_folder(self.surface.window());
                return Ok(Some(Followed::Message(Message::FolderChosen(folder))));
            }
            Effect::WritePersisted(persisted) => {
                if let Some(writer) = &self.writer {
                    writer.write(persisted);
                }
            }
            Effect::WriteSetting { file, key, value } => self.setting_writer.write(file, key, value),
            Effect::WatchSettings(files) => self.watcher.watch(files),
            Effect::ApplySettings => {
                self.surface.set_theme(self.state.settings.theme);
                apply_text(&mut self.surface, &self.state.settings)?;
            }
            Effect::Quit => return Ok(Some(Followed::Quit)),
        }
        Ok(None)
    }

    /// Draws a frame of the views from the store, then reduces the intents they returned.
    fn draw(&mut self, context: &Context<'_>) -> Result<Flow, AppError> {
        let running = self.running.tasks(&self.activities.running());
        let data = ViewData::new(&self.state, running, seconds_now());
        self.surface.set_title(&views::text::window_title(data.name()));
        let state = &self.state;
        let intents = self
            .surface
            .draw(context, |ui| views::view(&data.window_state(state), ui))?;
        let mut flow = Flow::Continue;
        for intent in intents {
            if self.dispatch(Message::Intent(intent))? == Flow::Quit {
                flow = Flow::Quit;
            }
        }
        Ok(flow)
    }
}

/// What follows from an effect carried out.
enum Followed {
    /// A message to reduce next, as the answer of the folder dialog.
    Message(Message),
    /// The window is to close.
    Quit,
}

/// Draws the window with the fonts and the size of the text of the settings.
fn apply_text(surface: &mut Surface, settings: &settings::Settings) -> Result<(), AppError> {
    let fonts = &settings.fonts;
    surface.set_text(&fonts.interface, &fonts.code, fonts.size / keys::FONT_SIZE)
}

/// The moment, in seconds since 1970: what the dates of the timeline are said from.
fn seconds_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(i64::MAX))
}
