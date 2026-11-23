//! GitHub Copilot as the AI provider (`ai.copilot`): the GitHub Copilot CLI, found on the `PATH`, started once for the
//! window in its server mode and spoken to through GitHub's SDK, `copilot-sdk` (D87), with the user's own sign-in.
//! Nothing is sent to it until a person asks, and every tool it would use is refused, so that it answers with text
//! alone.
//!
//! The protocol is the SDK's to keep: the card names the version of the SDK and the version of the CLI it was tried
//! with.

mod client;

use std::ffi::OsStr;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, PoisonError};

use http::Cancel;
use model::ai::{AiModel, AiRequest, AiSettings, CopilotNeed};

pub use client::CopilotStreams;
use client::{Client, Process, Question};

use super::{AiError, Asked, LanguageModel, Purpose};

/// The arguments the CLI is started with: its server mode over its standard input and output, no update of itself
/// while it runs, and only its errors written to its log.
pub const ARGUMENTS: [&str; 5] = ["--server", "--stdio", "--no-auto-update", "--log-level", "error"];

/// What went wrong asking GitHub Copilot.
#[derive(Debug)]
pub enum CopilotError {
    /// The user has something to do first: install the CLI, update it, or sign in to it.
    Needs(CopilotNeed),
    /// The CLI could not be started: what the system said.
    Start(String),
    /// The CLI ended, or closed its output, before it answered.
    Ended,
    /// Copilot refused the question, in its own words: no subscription, no requests left, a model not offered.
    Refused(String),
    /// What the CLI answered is not what its protocol says.
    Unreadable(String),
    /// The CLI speaks a protocol newer than the SDK of this Gitardo.
    TooNew,
    /// The question was cancelled.
    Cancelled,
    /// GitHub Copilot is asked through its CLI, never over HTTP.
    NotOverHttp,
}

impl fmt::Display for CopilotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CopilotError::Needs(need) => formatter.write_str(need.sentence()),
            CopilotError::Start(reason) => write!(formatter, "The GitHub Copilot CLI could not be started: {reason}"),
            CopilotError::Ended => formatter.write_str("The GitHub Copilot CLI ended before it answered. Try again."),
            CopilotError::Refused(words) => write!(formatter, "GitHub Copilot refused: {words}"),
            CopilotError::Unreadable(reason) => {
                write!(
                    formatter,
                    "The answer of the GitHub Copilot CLI could not be read: {reason}"
                )
            }
            CopilotError::TooNew => {
                formatter.write_str("The GitHub Copilot CLI is too new for Gitardo. Update Gitardo, then try again.")
            }
            CopilotError::Cancelled => formatter.write_str("The question was cancelled."),
            CopilotError::NotOverHttp => formatter.write_str("GitHub Copilot is asked through its CLI."),
        }
    }
}

impl From<CopilotError> for AiError {
    fn from(error: CopilotError) -> AiError {
        match error {
            CopilotError::Cancelled => AiError::Cancelled,
            error => AiError::Copilot(error),
        }
    }
}

/// How the CLI is started: the program on the `PATH`, or the streams of a fake in a test.
type Start = dyn Fn() -> Result<(CopilotStreams, Option<Process>), CopilotError> + Send + Sync;

/// The GitHub Copilot CLI of a window: started the first time it is asked, kept for the questions after, started again
/// when it ended, and ended when the last clone of it is dropped, as the window closes.
#[derive(Clone)]
pub struct CopilotCli {
    /// The CLI running, once started.
    running: Arc<Mutex<Option<Arc<Client>>>>,
    /// How it is started.
    start: Arc<Start>,
}

impl fmt::Debug for CopilotCli {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CopilotCli")
    }
}

impl CopilotCli {
    /// The `copilot` program found on the `PATH` when it is first asked, started in its server mode.
    #[must_use]
    pub fn system() -> CopilotCli {
        CopilotCli::starting(|| {
            let path = std::env::var_os("PATH").unwrap_or_default();
            let extensions = program_extensions(std::env::var_os("PATHEXT").as_deref());
            let program = find_copilot(&path, &extensions).ok_or(CopilotError::Needs(CopilotNeed::Install))?;
            start_program(&program)
        })
    }

    /// A CLI spoken to over the streams `start` gives each time it is started: a fake in a test.
    #[must_use]
    pub fn over(start: impl Fn() -> Result<CopilotStreams, CopilotError> + Send + Sync + 'static) -> CopilotCli {
        CopilotCli::starting(move || start().map(|streams| (streams, None)))
    }

    /// A CLI started by `start`.
    fn starting(
        start: impl Fn() -> Result<(CopilotStreams, Option<Process>), CopilotError> + Send + Sync + 'static,
    ) -> CopilotCli {
        CopilotCli {
            running: Arc::new(Mutex::new(None)),
            start: Arc::new(start),
        }
    }

    /// Whether the CLI is running: started, and its output not ended.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .is_some_and(|client| client.is_open())
    }

    /// The CLI running, started now when it is not; one whose protocol the SDK does not speak is not kept.
    fn client(&self, cancel: &Cancel) -> Result<Arc<Client>, CopilotError> {
        let mut running = self.running.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(client) = running.as_ref().filter(|client| client.is_open()) {
            return Ok(Arc::clone(client));
        }
        let (streams, process) = (self.start)()?;
        let client = Arc::new(Client::over(streams, process, cancel)?);
        *running = Some(Arc::clone(&client));
        Ok(client)
    }

    /// The models the user's subscription offers, in Copilot's order.
    ///
    /// # Errors
    ///
    /// When the CLI is not installed or not signed in, it cannot be started, or it refuses.
    pub fn models(&self, cancel: &Cancel) -> Result<Vec<AiModel>, CopilotError> {
        self.signed_in(cancel)?.models(cancel)
    }

    /// The CLI running and signed in.
    fn signed_in(&self, cancel: &Cancel) -> Result<Arc<Client>, CopilotError> {
        let client = self.client(cancel)?;
        if client.is_signed_in(cancel)? {
            Ok(client)
        } else {
            Err(CopilotError::Needs(CopilotNeed::SignIn))
        }
    }

    /// Asks a question in a session of its own, with the model and the effort of the settings, its answer streamed;
    /// the session is ended once answered, failed or cancelled, and the CLI is left running.
    ///
    /// # Errors
    ///
    /// As [`CopilotCli::models`], and when Copilot refuses the question, the CLI ends, or it is cancelled.
    pub fn ask(
        &self,
        settings: &AiSettings,
        prompt: &str,
        cancel: &Cancel,
        pieces: &mut dyn FnMut(&str),
    ) -> Result<String, CopilotError> {
        let question = Question {
            model: settings.model.clone(),
            effort: settings
                .effort
                .map(|effort| super::wire::chat_effort(effort).to_owned()),
            prompt: prompt.to_owned(),
        };
        self.signed_in(cancel)?.ask(question, cancel, pieces)
    }
}

/// GitHub Copilot as the language model that `ai.ask-conflict`, `commit.draft-message` and `ai.choose-model` are
/// given when `ai.provider` is `copilot`, in place of [`super::ProviderModel`].
#[derive(Clone, Copy, Debug)]
pub struct CopilotModel<'a> {
    /// The CLI of the window.
    pub cli: &'a CopilotCli,
    /// The settings: the model and the effort.
    pub settings: &'a AiSettings,
    /// What cancels the question.
    pub cancel: &'a Cancel,
}

/// GitHub Copilot asked through the CLI of the window, with the model and the effort of the settings, as the language
/// model of a question when `ai.provider` is `copilot`.
///
/// Feature: ai.copilot
#[must_use]
pub fn copilot_model<'a>(cli: &'a CopilotCli, settings: &'a AiSettings, cancel: &'a Cancel) -> CopilotModel<'a> {
    CopilotModel { cli, settings, cancel }
}

impl LanguageModel for CopilotModel<'_> {
    fn models(&mut self) -> Result<Vec<AiModel>, AiError> {
        Ok(self.cli.models(self.cancel)?)
    }

    fn ask(&mut self, _purpose: Purpose, request: &AiRequest, pieces: &mut dyn FnMut(&str)) -> Result<Asked, AiError> {
        let model = if self.settings.model.is_empty() {
            self.models()?.into_iter().next().ok_or(AiError::NoModels)?.id
        } else {
            self.settings.model.clone()
        };
        let settings = AiSettings {
            model: model.clone(),
            ..self.settings.clone()
        };
        let text = self.cli.ask(&settings, &request.prompt, self.cancel, pieces)?;
        Ok(Asked { model, text })
    }
}

/// The extensions a program may have on this system, from `PATHEXT` on Windows (`.COM;.EXE;.BAT;.CMD` when it is not
/// set, where `copilot.cmd` is what npm installs); none elsewhere.
#[must_use]
pub fn program_extensions(pathext: Option<&OsStr>) -> Vec<String> {
    if cfg!(windows) {
        let listed = pathext.map_or_else(
            || String::from(".COM;.EXE;.BAT;.CMD"),
            |text| text.to_string_lossy().into(),
        );
        listed
            .split(';')
            .filter(|extension| !extension.is_empty())
            .map(str::to_owned)
            .collect()
    } else {
        vec![String::new()]
    }
}

/// The `copilot` program in the first folder of `path` that has it, with the first of `extensions` it has.
#[must_use]
pub fn find_copilot(path: &OsStr, extensions: &[String]) -> Option<PathBuf> {
    std::env::split_paths(path).find_map(|folder| {
        extensions
            .iter()
            .map(|extension| folder.join(format!("copilot{extension}")))
            .find(|candidate| candidate.is_file())
    })
}

/// Starts the program in its server mode, its standard input and output piped, and what it writes to its standard
/// error left aside, with the processes it starts held to end with it.
fn start_program(program: &Path) -> Result<(CopilotStreams, Option<Process>), CopilotError> {
    let mut command = Command::new(program);
    let _ = command
        .args(ARGUMENTS)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    own_group(&mut command);
    let mut child = command
        .spawn()
        .map_err(|error| CopilotError::Start(error.to_string()))?;
    let (Some(input), Some(output)) = (child.stdin.take(), child.stdout.take()) else {
        let _killed = child.kill();
        return Err(CopilotError::Start(String::from("its input and output are not piped")));
    };
    let tree = system::ProcessTree::of(child.id()).ok();
    let streams = CopilotStreams {
        output: Box::new(output),
        input: Box::new(input),
    };
    Ok((streams, Some(Process { child, tree })))
}

/// On Windows the CLI opens no console window of its own; on Linux and macOS it leads a process group, so that it
/// ends with what it started.
fn own_group(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        /// `CREATE_NO_WINDOW`, from the documentation of the process creation flags of Windows.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let _ = command.process_group(0);
    }
}

#[cfg(test)]
mod tests;
