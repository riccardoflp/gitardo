//! The window of Gitardo Desktop: the store, the views drawn from it, the workers that run its jobs, the watcher of
//! its settings files and the writer of what is kept between runs, around one event loop.
//!
//! Each event goes to the store as a [`Message`]; what `reduce` asks is carried out at once, in order, and a frame is
//! asked for. The intents of each frame are reduced after it.

mod reads;
pub mod running;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use engine::Activities;
use model::contract::{Answers, FeatureId};
use platform::{Context, Event, EventLoop, FileDialog, LogicalSize, TimerId, Visibility};

use crate::error::AppError;
use crate::persisted::{FILE_NAME, Persisted};
use crate::settings::{self, SettingsFiles, keys, read_files, user_settings_file};
use crate::store::{Effect, Message, State, ViewData, reduce};
use crate::window::frame::system_title_bar;
use crate::window::{Step, Surface, SurfaceOptions};
use crate::work::github::{GitHubAccess, GitHubReader};
use crate::work::{CommandsWork, Deliver, Operations, PersistedWriter, SettingWriter, SettingsWatcher, Workers};
use views::window::{CaptionState, WindowIntent};

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
    /// The thread of the operations.
    operations: Operations,
    /// The questions to the AI provider (`ai.resolve-conflicts`, `commit.draft-message`).
    asking: crate::work::ai::Asking,
    /// What reads the pull requests of GitHub.
    github: GitHubReader,
    /// The Command log of the commands of new worktrees, and the steps their plans have after them.
    commands: CommandsWork,
    /// The terminals of the bottom panel (`terminal.view`).
    terminals: crate::work::Terminals,
    /// The tasks of the Tasks tab, each in its terminal (`tasks.run`).
    task_runs: crate::work::TaskRuns,
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
    /// The watcher of the repository shown, the timer of the next read, and how long each read took.
    reads: reads::ReadClock,
    /// Where the messages of the threads of the window go.
    deliver: Deliver,
}

impl DesktopWindow {
    /// Opens the window with what was kept from the runs before and the user's settings.
    fn open(context: &Context<'_>, options: &StartOptions, deliver: &Deliver) -> Result<DesktopWindow, AppError> {
        let persisted = options
            .configuration
            .as_ref()
            .map_or_default(|folder| Persisted::read(&folder.join(FILE_NAME)));
        let user_file = options.configuration.as_deref().map(user_settings_file);
        let settings = read_files(&SettingsFiles {
            user: user_file.clone(),
            repository: None,
        });
        let (width, height) = persisted.window.size.unwrap_or(FIRST_SIZE);
        let mut surface = Surface::open(
            context,
            SurfaceOptions {
                title: views::text::window_title(None),
                size: LogicalSize { width, height },
                visibility: options.visibility,
                theme: settings.theme,
                frame_timer: FRAME_TIMER,
                title_bar: system_title_bar(),
                position: persisted
                    .window
                    .position
                    .map(|(x, y)| platform::ScreenPosition { x, y }),
                is_maximized: persisted.window.is_maximized,
            },
        )?;
        apply_text(&mut surface, &settings)?;
        let activities = Activities::new();
        let mut state = State::new(persisted, settings, user_file);
        state.caption = caption_of(&surface);
        Ok(DesktopWindow {
            surface,
            state,
            workers: Workers::new(WORKERS, &activities, deliver),
            operations: Operations::new(&activities, deliver),
            asking: crate::work::ai::Asking::new(deliver, &activities, Box::new(engine::SystemSecrets)),
            github: GitHubReader::new(GitHubAccess::system(), &activities, deliver),
            commands: CommandsWork::new(deliver),
            terminals: crate::work::Terminals::new(deliver),
            task_runs: crate::work::TaskRuns::new(deliver),
            watcher: SettingsWatcher::new(Arc::clone(deliver)),
            writer: options
                .configuration
                .as_ref()
                .map(|folder| PersistedWriter::new(folder.join(FILE_NAME))),
            setting_writer: SettingWriter::new(),
            activities,
            running: running::RunningClock::default(),
            reads: reads::ReadClock::new(),
            deliver: Arc::clone(deliver),
        })
    }

    /// Takes an event of the loop.
    fn handle(&mut self, context: &Context<'_>, event: Event<Message>) -> Result<Flow, AppError> {
        let flow = self.handle_event(context, event);
        self.reads.arm(context);
        flow
    }

    /// Takes an event of the loop, before the timer of the next read is set.
    fn handle_event(&mut self, context: &Context<'_>, event: Event<Message>) -> Result<Flow, AppError> {
        let event = match event {
            Event::Message(message) => return self.dispatch(message),
            Event::Timer(timer) if timer == reads::READ_TIMER => {
                let woken = self.reads.woken(context);
                return self.dispatch(woken);
            }
            other => other,
        };
        if let Event::Input { event: input, .. } = &event
            && let Some(message) = self.reads.input(input)
        {
            let _flow = self.dispatch(message)?;
        }
        if let Event::Input {
            event: platform::InputEvent::Resized(_),
            ..
        } = &event
        {
            let caption = caption_of(&self.surface);
            if caption != self.state.caption {
                let _flow = self.dispatch(Message::Caption(caption))?;
            }
        }
        match self.surface.handle(&event) {
            Step::Draw => self.draw(context),
            Step::Close => self.dispatch(self.close_requested()),
            Step::Ignored | Step::Handled => Ok(Flow::Continue),
        }
    }

    /// The person asked to close the window, by the system's close request or by **Close** of its title bar: where it
    /// is and how large as it is restored, and whether it is maximized, to open so at the next start.
    fn close_requested(&self) -> Message {
        let placement = self.surface.window().placement();
        Message::CloseRequested(crate::persisted::WindowPlace {
            size: Some((placement.size.width, placement.size.height)),
            position: placement.position.map(|position| (position.x, position.y)),
            is_maximized: placement.is_maximized,
        })
    }

    /// Reduces a message and the ones that follow from it, carries out what the store asks, and asks for a frame.
    fn dispatch(&mut self, message: Message) -> Result<Flow, AppError> {
        let mut waiting = VecDeque::from([message]);
        let mut flow = Flow::Continue;
        while let Some(message) = waiting.pop_front() {
            if let Message::RepositoriesFound { job, .. }
            | Message::RepositoryRead { job, .. }
            | Message::DetailsRead { job, .. }
            | Message::CommitPageRead { job, .. } = &message
            {
                waiting.extend(self.reads.finished(*job));
            }
            if let Some(job) = crate::store::job_done(&message) {
                self.workers.finished(job);
            }
            let message = self.terminals.read_screen(message);
            let message = self.task_runs.read(message);
            for effect in reduce(&mut self.state, message) {
                match self.carry_out(effect)? {
                    Some(Followed::Message(next)) => waiting.push_back(*next),
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
            Effect::Start(job) => {
                let words = self.reads.started(&self.state, &job, model::Timestamp(seconds_now()));
                self.workers.start_described(job, words);
            }
            Effect::WatchRepository(folder) => self.reads.watch(folder.as_deref(), &self.deliver),
            Effect::WatchTabs(folders) => self.reads.watch_tabs(&folders, &self.deliver),
            Effect::WakeAt(at) => self.reads.wake_at(at),
            Effect::ReadSummaries(asks) => crate::work::summaries::read_in_background(asks, &self.deliver),
            Effect::Cancel(job) => self.workers.cancel(job),
            Effect::Ask(question) => self.asking.ask(*question),
            Effect::StopAsking(asks) => self.asking.stop(&asks),
            Effect::ChooseFolder => return Ok(Some(self.choose_folder())),
            Effect::ChooseWorkspaceFile => return Ok(Some(self.choose_workspace_file())),
            Effect::WritePersisted(persisted) => {
                if let Some(writer) = &self.writer {
                    writer.write(persisted);
                }
            }
            Effect::WriteSetting { file, key, value } => self.setting_writer.write(file, key, value),
            Effect::WatchSettings(files) => self.watcher.watch(files),
            Effect::ApplySettings => {
                self.surface.set_theme(self.state.settings.theme);
                self.surface.set_file_icons(self.state.settings.file_icons);
                apply_text(&mut self.surface, &self.state.settings)?;
            }
            Effect::RunFeature {
                folder,
                feature,
                preset,
                context,
            } => self.run_feature(folder, (feature, preset), *context),
            Effect::ReplyToFeature(reply) => self.operations.reply(reply),
            Effect::CopyText(text) => self.surface.copy(&text),
            Effect::OpenWindow(folder) => open_window(&folder),
            Effect::ChooseFile {
                title,
                question,
                choosing,
            } => return Ok(Some(self.choose_file(title, question, choosing))),
            Effect::GitHub(job) => self.github.start(job),
            Effect::Commands(asked) => self.commands.take(asked, self.state.settings.log_level),
            Effect::Terminal(asked) => self.terminals.take(asked),
            Effect::TaskRuns(asked) => self.task_runs.take(asked),
            Effect::OpenUrl(url) => {
                if let Err(error) = platform::open_url(&url) {
                    let said = views::text::github::could_not_open(&error.to_string());
                    return Ok(Some(Followed::Message(Box::new(Message::UrlNotOpened(said)))));
                }
            }
            Effect::Window(asked) => return Ok(self.window_asked(asked)),
            Effect::Quit => return Ok(Some(Followed::Quit)),
        }
        Ok(None)
    }

    /// Minimizes, maximizes, restores or closes the window, as a button of its title bar asked
    /// (`window.caption-buttons`); closing goes as the system's close request.
    fn window_asked(&self, asked: WindowIntent) -> Option<Followed> {
        let window = self.surface.window();
        match asked {
            WindowIntent::Minimize => window.minimize(),
            WindowIntent::Maximize => window.maximize(),
            WindowIntent::Restore => window.restore(),
            WindowIntent::Close => return Some(Followed::Message(Box::new(self.close_requested()))),
        }
        None
    }

    /// The system's folder dialog, for **Open folder…** (`repo.open-folder`), and the folder chosen.
    fn choose_folder(&self) -> Followed {
        let dialog = FileDialog {
            title: String::from(views::text::repo::CHOOSE_FOLDER),
            ..FileDialog::default()
        };
        let folder = dialog.choose_folder(self.surface.window());
        Followed::Message(Box::new(Message::FolderChosen(folder)))
    }

    /// The system's dialog for opening a file, showing the workspace files of VS Code (`repo.open-code-workspace`),
    /// and the file chosen with its text, or why it could not be read.
    fn choose_workspace_file(&self) -> Followed {
        let dialog = FileDialog {
            title: String::from(views::text::repo::CHOOSE_WORKSPACE_FILE),
            filters: vec![platform::FileFilter {
                name: String::from(views::text::repo::WORKSPACE_FILES),
                extensions: vec![String::from("code-workspace")],
            }],
            ..FileDialog::default()
        };
        let chosen = dialog.choose_file(self.surface.window()).map(|file| {
            let text = std::fs::read_to_string(&file).map_err(|error| error.to_string());
            (file, text)
        });
        Followed::Message(Box::new(Message::WorkspaceFileChosen(chosen)))
    }

    /// Runs an operation on the thread of the operations; one that makes a repository with no workspace open runs from
    /// the user's home folder.
    fn run_feature(&mut self, folder: PathBuf, started: (FeatureId, Answers), mut context: model::contract::Context) {
        let home = settings::home_folder().unwrap_or_default();
        let folder = if folder.as_os_str().is_empty() {
            home.clone()
        } else {
            folder
        };
        context.settings.worktrees.home = home;
        self.operations
            .run(folder, started, context, self.state.settings.log_level);
    }

    /// Opens the system's dialog to choose a file to open or a path to save to, for the question of an operation, and
    /// answers it with what was chosen, or with nothing when it was cancelled.
    fn choose_file(
        &self,
        title: String,
        question: model::contract::FileQuestion,
        choosing: crate::store::Choosing,
    ) -> Followed {
        let dialog = FileDialog {
            title,
            folder: Some(question.folder),
            file_name: question.file_name,
            filters: question
                .filters
                .into_iter()
                .map(|filter| platform::FileFilter {
                    name: filter.name.as_str().to_owned(),
                    extensions: filter.extensions,
                })
                .collect(),
        };
        let window = self.surface.window();
        let chosen = match choosing {
            crate::store::Choosing::FileToOpen => dialog.choose_file(window),
            crate::store::Choosing::WhereToSave => dialog.choose_save_path(window),
            crate::store::Choosing::Folder => dialog.choose_folder(window),
        };
        let answer = chosen.map(model::contract::Answer::Path);
        Followed::Message(Box::new(Message::Intent(views::Intent::Feature(
            views::feature::FeatureIntent::Answer(answer),
        ))))
    }

    /// Draws a frame of the views from the store, then reduces the intents they returned.
    fn draw(&mut self, context: &Context<'_>) -> Result<Flow, AppError> {
        let report = self.activities.report();
        let running = self.running.tasks(&report, std::time::Instant::now());
        let data = ViewData::new(&self.state, running, seconds_now()).with_ran(self.running.ran());
        self.surface.set_title(&views::text::window_title(data.name()));
        let state = &self.state;
        let intents = self
            .surface
            .draw(context, |ui| views::view(&data.window_state(state), ui))?;
        self.surface.keep_frame_parts(self.state.caption);
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
    Message(Box<Message>),
    /// The window is to close.
    Quit,
}

/// Who draws the title bar of a window, and whether it is maximized.
fn caption_of(surface: &Surface) -> CaptionState {
    CaptionState {
        title_bar: system_title_bar(),
        is_maximized: surface.window().is_maximized(),
    }
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

/// Starts Gitardo Desktop again on a folder, in a window of its own. When it cannot be started there is no one to tell
/// but the window, which shows nothing new.
fn open_window(folder: &std::path::Path) {
    if let Ok(program) = std::env::current_exe() {
        let _started = std::process::Command::new(program).arg(folder).spawn();
    }
}
