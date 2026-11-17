//! The terminals of a window (`terminal.view`): each is started on a thread of its own, as starting a shell may take a
//! while, then kept here; what it prints is read on its own thread, which wakes the window, and the window reads its
//! screen when it draws.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use terminal::{GridSize, StartOptions, Terminal, TerminalEvent, shell_of};

use super::Deliver;
use crate::store::Message;
use crate::store::terminals::{TerminalEffect, TerminalMessage};

/// A terminal asked for: started, or still starting, with what was asked of it in the meantime.
enum Held {
    /// Its shell starts; what was typed waits for it.
    Starting(Vec<Vec<u8>>),
    /// Its shell runs, or ended.
    Started(Terminal),
}

/// The terminals of a window.
pub struct Terminals {
    /// Where they tell the window what happens.
    deliver: Deliver,
    /// Each by its number.
    held: BTreeMap<u64, Held>,
    /// The terminals the threads started, for the window to keep.
    started: Receiver<(u64, Terminal)>,
    /// Where the threads give them.
    sender: Sender<(u64, Terminal)>,
    /// The size they were last given.
    size: Option<GridSize>,
}

impl Terminals {
    /// No terminal yet; those to come tell `deliver` what they print and when they end.
    #[must_use]
    pub fn new(deliver: &Deliver) -> Terminals {
        let (sender, started) = mpsc::channel();
        Terminals {
            deliver: Arc::clone(deliver),
            held: BTreeMap::new(),
            started,
            sender,
            size: None,
        }
    }

    /// Does what the store asked.
    pub fn take(&mut self, effect: TerminalEffect) {
        self.keep_started();
        match effect {
            TerminalEffect::Start {
                id,
                folder,
                shell_path,
                scrollback,
                size,
            } => {
                let size = self.size.unwrap_or(GridSize {
                    columns: size.0,
                    rows: size.1,
                });
                let path = std::env::var_os("PATH");
                let options = StartOptions {
                    folder,
                    shell: shell_of(&shell_path, path.as_deref()),
                    size,
                    scrollback,
                };
                self.start(id, options);
            }
            TerminalEffect::Write(id, bytes) => self.write(id, bytes),
            TerminalEffect::Resize((columns, rows)) => self.resize(GridSize { columns, rows }),
            TerminalEffect::Scroll(id, lines) => self.with(id, |terminal| terminal.scroll(lines)),
            TerminalEffect::Select(id, selection) => self.with(id, |terminal| terminal.select(selection)),
            TerminalEffect::Kill(id) => {
                // Dropping a terminal ends its shell; one still starting is dropped when it arrives.
                let _ended = self.held.remove(&id);
            }
        }
    }

    /// A message of the threads, with the screen of a terminal that printed read now, for the window to draw it: the
    /// store keeps what each terminal shows. The other messages are given back as they are.
    #[must_use]
    pub fn read_screen(&mut self, message: Message) -> Message {
        self.keep_started();
        let Message::Terminal(TerminalMessage::Printed(id)) = message else {
            return message;
        };
        let Some(Held::Started(terminal)) = self.held.get(&id) else {
            return message;
        };
        terminal.take_wake();
        Message::Terminal(TerminalMessage::Screen(id, Box::new(terminal.screen())))
    }

    /// Starts a shell on a thread of its own, which gives the terminal back and tells the window.
    fn start(&mut self, id: u64, options: StartOptions) {
        let _previous = self.held.insert(id, Held::Starting(Vec::new()));
        let deliver = Arc::clone(&self.deliver);
        let sender = self.sender.clone();
        let tell_deliver = Arc::clone(&self.deliver);
        let tell: terminal::Tell = Arc::new(move |event| {
            let message = match event {
                TerminalEvent::Printed => TerminalMessage::Printed(id),
                TerminalEvent::Ended(code) => TerminalMessage::Ended(id, code),
            };
            tell_deliver(Message::Terminal(message));
        });
        let started = thread::Builder::new().name(String::from("terminal")).spawn(move || {
            let shell = options.shell.name();
            match Terminal::start(options, tell) {
                Ok(terminal) => {
                    // A window that closed takes no terminal: dropping it ends the shell.
                    let _given = sender.send((id, terminal));
                    deliver(Message::Terminal(TerminalMessage::Started(id, shell)));
                }
                Err(error) => deliver(Message::Terminal(TerminalMessage::NotStarted(id, error.to_string()))),
            }
        });
        if let Err(error) = started {
            (self.deliver)(Message::Terminal(TerminalMessage::NotStarted(id, error.to_string())));
        }
    }

    /// Keeps the terminals the threads started, giving each what was typed while it started; one killed in the
    /// meantime is dropped, which ends its shell.
    fn keep_started(&mut self) {
        while let Ok((id, mut terminal)) = self.started.try_recv() {
            let Some(Held::Starting(typed)) = self.held.remove(&id) else {
                continue;
            };
            if let Some(size) = self.size {
                terminal.resize(size);
            }
            for bytes in typed {
                terminal.write(bytes);
            }
            let _previous = self.held.insert(id, Held::Started(terminal));
        }
    }

    /// Gives the shell of a terminal what was typed, or keeps it until the shell started.
    fn write(&mut self, id: u64, bytes: Vec<u8>) {
        match self.held.get_mut(&id) {
            Some(Held::Starting(typed)) => typed.push(bytes),
            Some(Held::Started(terminal)) => terminal.write(bytes),
            None => {}
        }
    }

    /// Gives every terminal a size.
    fn resize(&mut self, size: GridSize) {
        self.size = Some(size);
        for held in self.held.values_mut() {
            if let Held::Started(terminal) = held {
                terminal.resize(size);
            }
        }
    }

    /// Does something to a terminal that started, and tells the window to read its screen again, as what it shows
    /// changed without it printing.
    fn with(&self, id: u64, act: impl FnOnce(&Terminal)) {
        if let Some(Held::Started(terminal)) = self.held.get(&id) {
            act(terminal);
            (self.deliver)(Message::Terminal(TerminalMessage::Printed(id)));
        }
    }
}
