//! The tasks a window runs (`tasks.run`, `tasks.stop`): each in a pseudo-terminal of its own, started on a thread as
//! starting a process may take a while, then kept here with the processes it started, which end with it. What it
//! prints is read on its own thread, which wakes the window; the window reads its screen when it draws, and looks at
//! its last lines to say when a background task is ready. The tasks of a worktree are read on a thread too.

mod read;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use model::tasks::{Background, Readiness, Task, task_start};
use system::ProcessTree;
use terminal::{GridSize, Shell, StartOptions, Terminal, TerminalEvent, shell_of};

use super::Deliver;
use crate::store::Message;
use crate::store::tasks_tab::{TaskRunEffect, TaskRunMessage};

/// How long a task asked to stop has to end by itself before it is ended.
pub const STOP_AFTER: Duration = Duration::from_secs(3);
/// How often a background task that is not ready yet is looked at.
const TICK: Duration = Duration::from_millis(250);
/// How many of its last lines say whether a background task is ready.
const READ_LINES: usize = 40;

/// A task that waits for its terminal: what was typed for it, and what happened to it in the meantime.
struct Starting {
    /// What was typed, which it gets once it started.
    typed: Vec<Vec<u8>>,
    /// How a background task says it is ready.
    background: Option<Background>,
    /// Whether it was asked to stop.
    is_stopping: bool,
}

/// A task whose terminal started.
struct Started {
    /// Its terminal.
    terminal: Terminal,
    /// The processes it started, when the system let them be held together.
    tree: Option<ProcessTree>,
    /// When a background task is ready, until it is.
    readiness: Option<Readiness>,
    /// What stops the thread that asks to look at a background task again.
    ticking: Option<Ticking>,
    /// Whether it was asked to stop.
    is_stopping: bool,
    /// Whether its process ended.
    is_ended: bool,
}

/// The thread that asks the window, now and then, to look whether a background task is ready; it ends once this is
/// dropped.
struct Ticking(Arc<AtomicBool>);

impl Drop for Ticking {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

/// A task asked for.
enum Held {
    /// Its terminal starts.
    Starting(Starting),
    /// Its terminal started.
    Started(Box<Started>),
}

/// The tasks of a window.
pub struct TaskRuns {
    /// Where they tell the window what happens.
    deliver: Deliver,
    /// Each by the number of its terminal.
    held: BTreeMap<u64, Held>,
    /// The terminals the threads started, each with the processes of its task, for the window to keep.
    started: Receiver<(u64, Terminal, Option<ProcessTree>)>,
    /// Where the threads give them.
    sender: Sender<(u64, Terminal, Option<ProcessTree>)>,
    /// The size the terminals were last given.
    size: Option<GridSize>,
    /// When the window opened, which the moments of a background task are counted from.
    opened: Instant,
}

impl TaskRuns {
    /// No task yet; those to come tell `deliver` what they print and when they end.
    #[must_use]
    pub fn new(deliver: &Deliver) -> TaskRuns {
        let (sender, started) = mpsc::channel();
        TaskRuns {
            deliver: Arc::clone(deliver),
            held: BTreeMap::new(),
            started,
            sender,
            size: None,
            opened: Instant::now(),
        }
    }

    /// Does what the store asked.
    pub fn take(&mut self, effect: TaskRunEffect) {
        self.keep_started();
        match effect {
            TaskRunEffect::Read { tab, folder, serial } => read::start(&self.deliver, tab, folder, serial),
            TaskRunEffect::Start {
                id,
                task,
                shell_path,
                scrollback,
                size,
            } => {
                let size = self.size.unwrap_or(GridSize {
                    columns: size.0,
                    rows: size.1,
                });
                self.start(id, &task, (&shell_path, scrollback, size));
            }
            TaskRunEffect::Write(id, bytes) => self.write(id, bytes),
            TaskRunEffect::Resize((columns, rows)) => self.resize(GridSize { columns, rows }),
            TaskRunEffect::Scroll(id, lines) => self.with(id, |terminal| terminal.scroll(lines)),
            TaskRunEffect::Select(id, selection) => self.with(id, |terminal| terminal.select(selection)),
            TaskRunEffect::Stop(id) => self.stop(id),
            TaskRunEffect::Kill(id) => {
                self.end(id);
                // Dropping a terminal ends what is left of its task; one still starting is dropped when it arrives.
                let _ended = self.held.remove(&id);
            }
        }
    }

    /// A message of the threads, answered here when the answer is in a terminal: the screen of a task that printed is
    /// read now, for the window to draw it, with whether a background task is ready; a moment that passed says a
    /// background task ready once it printed nothing for long enough; a task asked to stop that still runs three
    /// seconds later is ended, with every process it started. The other messages are given back as they are.
    #[must_use]
    pub fn read(&mut self, message: Message) -> Message {
        self.keep_started();
        let Message::TaskRun(told) = message else {
            return message;
        };
        let now = self.opened.elapsed();
        let told = match told {
            TaskRunMessage::Printed(id) => match self.held.get_mut(&id) {
                Some(Held::Started(started)) => {
                    started.terminal.take_wake();
                    let is_ready = started.look(now);
                    TaskRunMessage::Screen(id, Box::new(started.terminal.screen()), is_ready)
                }
                _ => told,
            },
            TaskRunMessage::Tick(id) => {
                let is_ready = match self.held.get_mut(&id) {
                    Some(Held::Started(started)) => started.look(now),
                    _ => false,
                };
                if is_ready { TaskRunMessage::Ready(id) } else { told }
            }
            TaskRunMessage::StopDue(id) => {
                if let Some(Held::Started(started)) = self.held.get(&id)
                    && !started.is_ended
                {
                    started.end_processes();
                }
                told
            }
            TaskRunMessage::Ended(id, ..) => {
                if let Some(Held::Started(started)) = self.held.get_mut(&id) {
                    started.is_ended = true;
                    started.ticking = None;
                    // What a task that was stopped started ends with it, even when the task itself took the interrupt.
                    if started.is_stopping {
                        started.end_processes();
                    }
                }
                told
            }
            other => other,
        };
        Message::TaskRun(told)
    }

    /// Starts a task in a terminal on a thread of its own, which gives the terminal back and tells the window.
    fn start(&mut self, id: u64, task: &Task, (shell_path, scrollback, size): (&str, usize, GridSize)) {
        let path = std::env::var_os("PATH");
        let shell = shell_of(shell_path, path.as_deref());
        let tell = |told| Message::TaskRun(told);
        let Some(start) = task_start(task, &shell.program) else {
            let why = String::from("it has no command to run");
            (self.deliver)(tell(TaskRunMessage::NotStarted(id, why)));
            return;
        };
        let options = StartOptions {
            folder: start.folder,
            shell: Shell {
                program: start.program,
                args: start.arguments,
            },
            size,
            scrollback,
            environment: start.environment,
        };
        let _previous = self.held.insert(
            id,
            Held::Starting(Starting {
                typed: Vec::new(),
                background: task.background.clone(),
                is_stopping: false,
            }),
        );
        let deliver = Arc::clone(&self.deliver);
        let sender = self.sender.clone();
        let tell_deliver = Arc::clone(&self.deliver);
        let began = Instant::now();
        let told: terminal::Tell = Arc::new(move |event| {
            let message = match event {
                TerminalEvent::Printed => TaskRunMessage::Printed(id),
                TerminalEvent::Ended(code) => TaskRunMessage::Ended(id, code, began.elapsed()),
            };
            tell_deliver(Message::TaskRun(message));
        });
        let started = thread::Builder::new().name(String::from("task")).spawn(move || {
            match Terminal::start(options, told) {
                Ok(terminal) => {
                    let tree = terminal.process().and_then(|process| ProcessTree::of(process).ok());
                    // A window that closed takes no terminal: dropping it ends the task.
                    let _given = sender.send((id, terminal, tree));
                    deliver(tell(TaskRunMessage::Started(id)));
                }
                Err(error) => deliver(tell(TaskRunMessage::NotStarted(id, error.to_string()))),
            }
        });
        if let Err(error) = started {
            (self.deliver)(tell(TaskRunMessage::NotStarted(id, error.to_string())));
        }
    }

    /// Keeps the terminals the threads started, giving each what was typed while it started, and asks the window to
    /// read what each printed before it was kept; one killed in the meantime is ended and dropped.
    fn keep_started(&mut self) {
        while let Ok((id, mut terminal, tree)) = self.started.try_recv() {
            let Some(Held::Starting(starting)) = self.held.remove(&id) else {
                if let Some(tree) = tree {
                    // A task nothing waits for any more: what the system says of its end is of no use.
                    let _ended = tree.end();
                }
                continue;
            };
            if let Some(size) = self.size {
                terminal.resize(size);
            }
            for bytes in starting.typed {
                terminal.write(bytes);
            }
            let now = self.opened.elapsed();
            let readiness = starting.background.map(|background| Readiness::new(&background, now));
            let ticking = readiness.as_ref().map(|_| self.tick(id));
            let started = Started {
                terminal,
                tree,
                readiness,
                ticking,
                is_stopping: false,
                is_ended: false,
            };
            let _previous = self.held.insert(id, Held::Started(Box::new(started)));
            (self.deliver)(Message::TaskRun(TaskRunMessage::Printed(id)));
            if starting.is_stopping {
                self.stop(id);
            }
        }
    }

    /// Starts the thread that asks the window to look at a background task now and then, until it is told to end.
    fn tick(&self, id: u64) -> Ticking {
        let is_over = Arc::new(AtomicBool::new(false));
        let ticking = Ticking(Arc::clone(&is_over));
        let deliver = Arc::clone(&self.deliver);
        // A thread that could not start leaves the task to say it is ready by what it prints.
        let _thread = thread::Builder::new().name(String::from("task ready")).spawn(move || {
            while !is_over.load(Ordering::Acquire) {
                thread::sleep(TICK);
                deliver(Message::TaskRun(TaskRunMessage::Tick(id)));
            }
        });
        ticking
    }

    /// Stops a task: an interrupt now, as `Ctrl+C` in its terminal, and the window is told three seconds later, to end
    /// it if it still runs.
    fn stop(&mut self, id: u64) {
        match self.held.get_mut(&id) {
            Some(Held::Starting(starting)) => starting.is_stopping = true,
            Some(Held::Started(started)) => {
                started.is_stopping = true;
                started.terminal.write(vec![0x03]);
                let deliver = Arc::clone(&self.deliver);
                let waiting = thread::Builder::new().name(String::from("task stop")).spawn(move || {
                    thread::sleep(STOP_AFTER);
                    deliver(Message::TaskRun(TaskRunMessage::StopDue(id)));
                });
                if waiting.is_err() {
                    started.end_processes();
                }
            }
            None => {}
        }
    }

    /// Ends a task and every process it started at once; its terminal stays.
    fn end(&mut self, id: u64) {
        match self.held.get_mut(&id) {
            Some(Held::Starting(starting)) => starting.is_stopping = true,
            Some(Held::Started(started)) => {
                started.is_stopping = true;
                if !started.is_ended {
                    started.end_processes();
                }
            }
            None => {}
        }
    }

    /// Gives a task what was typed, or keeps it until its terminal started.
    fn write(&mut self, id: u64, bytes: Vec<u8>) {
        match self.held.get_mut(&id) {
            Some(Held::Starting(starting)) => starting.typed.push(bytes),
            Some(Held::Started(started)) => started.terminal.write(bytes),
            None => {}
        }
    }

    /// Gives the terminal of every task a size.
    fn resize(&mut self, size: GridSize) {
        self.size = Some(size);
        for held in self.held.values_mut() {
            if let Held::Started(started) = held {
                started.terminal.resize(size);
            }
        }
    }

    /// Does something to the terminal of a task, and tells the window to read its screen again, as what it shows
    /// changed without it printing.
    fn with(&self, id: u64, act: impl FnOnce(&Terminal)) {
        if let Some(Held::Started(started)) = self.held.get(&id) {
            act(&started.terminal);
            (self.deliver)(Message::TaskRun(TaskRunMessage::Printed(id)));
        }
    }
}

impl Started {
    /// Looks at the last lines a background task printed, at a moment counted from when the window opened: whether
    /// it is ready now. Told once: a task that was said ready is not looked at again.
    fn look(&mut self, now: Duration) -> bool {
        let Some(readiness) = self.readiness.as_mut() else {
            return false;
        };
        readiness.shows(&self.terminal.last_lines(READ_LINES), now);
        if !readiness.is_ready(now) {
            return false;
        }
        self.readiness = None;
        self.ticking = None;
        true
    }

    /// Ends every process the task started. Without a hold on them, its terminal is asked to end what it runs.
    fn end_processes(&self) {
        match &self.tree {
            // A process that ended in the meantime leaves nothing to end: what the system says then is of no use.
            Some(tree) => drop(tree.end()),
            None => self.terminal.write(vec![0x03]),
        }
    }
}

impl Drop for TaskRuns {
    /// The window closes: no task outlives it, nor what a task started.
    fn drop(&mut self) {
        self.keep_started();
        for held in self.held.values() {
            if let Held::Started(started) = held
                && !started.is_ended
            {
                started.end_processes();
            }
        }
    }
}
