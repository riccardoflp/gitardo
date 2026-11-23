//! The questions to the AI provider of a window (`ai.resolve-conflicts`, `commit.draft-message`, `help.ask`): each on a
//! thread of its own, its key taken from the credential store, the text of its answer passed on as it comes, at most every
//! 120 ms, and cancelled when it is not wanted any more.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use engine::ai::{
    ConflictReply, ConflictRequest, CopilotCli, CopilotError, LanguageModel, Provider, ProviderModel, Sending, Waiting,
};
use engine::{Activities, SecretStore};
use git::read::commit::{DraftDiffRequest, read_draft_diff};
use http::{Cancel, Http, Proxies, ProxySettings, Trust};
use model::ai::{AiProvider, AiSettings, CopilotNeed, Key};
use model::commit::DraftInput;
use model::contract::Text;

use super::Deliver;
use crate::store::Message;

/// A question, among those of the window.
pub type AskId = u64;

/// What the Activity tab says while a message is drafted.
const DRAFTING: &str = "drafting commit message";
/// Why.
const DRAFTING_CAUSE: &str = "The sparkle of the commit box asked the AI provider for a message";
/// What the Activity tab and the status bar say while Ask Gitardo is answered (`help.ask`).
pub const ASKING_GITARDO: &str = "asking Gitardo";
/// Why.
const ASKING_GITARDO_CAUSE: &str = "A question was asked on the Ask Gitardo page";

/// How often, at most, the text of an answer that comes is passed on.
pub const PIECE_INTERVAL: Duration = Duration::from_millis(120);

/// What is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiWork {
    /// How to resolve one conflict (`ai.ask-conflict`).
    Conflict(ConflictRequest),
    /// The message of a commit (`commit.draft-message`).
    Draft(DraftJob),
    /// How to do something with Gitardo (`help.ask`).
    Help(engine::help::HelpRequest),
}

/// What the message of a commit is drafted from: the diff is read on the thread of the question.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftJob {
    /// The repository.
    pub folder: PathBuf,
    /// Whether something is staged: then only the staged changes are described.
    pub is_staged: bool,
    /// The untracked files, sent when nothing is staged.
    pub new_files: Vec<String>,
    /// The subjects of the last commits, newest first.
    pub subjects: Vec<String>,
}

/// A question to the AI provider.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiQuestion {
    /// Which question.
    pub id: AskId,
    /// The settings of the provider when it was asked.
    pub settings: AiSettings,
    /// What is asked.
    pub work: AiWork,
}

/// What the provider answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiReply {
    /// The resolution of a conflict, with the model that wrote it.
    Conflict(ConflictReply),
    /// The message of a commit, cleaned (`commit.draft-message`).
    Draft(String),
    /// The answer of Ask Gitardo (`help.ask`).
    Help {
        /// The answer read (`help.read-answer`).
        answer: model::help::HelpAnswer,
        /// The answer as the model wrote it, sent with the questions after it.
        text: String,
    },
}

/// Why a question has no answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiFailed {
    /// Why, in words.
    pub text: String,
    /// Whether no other question is to be asked: the model chosen is not there.
    pub stops_the_rest: bool,
    /// What GitHub Copilot needs first, which the window offers to do: install its CLI, or sign in (`ai.copilot`).
    pub need: Option<CopilotNeed>,
}

impl AiFailed {
    /// A failure in words, which asks the others all the same.
    #[must_use]
    pub fn saying(text: String) -> AiFailed {
        AiFailed {
            text,
            stops_the_rest: false,
            need: None,
        }
    }
}

impl From<engine::ai::AiError> for AiFailed {
    fn from(error: engine::ai::AiError) -> AiFailed {
        let need = match &error {
            engine::ai::AiError::Copilot(CopilotError::Needs(need)) => Some(*need),
            _ => None,
        };
        AiFailed {
            stops_the_rest: matches!(error, engine::ai::AiError::ModelMissing(_)),
            text: error.to_string(),
            need,
        }
    }
}

/// What a question came to: the answer, or why there is none.
pub type AiResult = Result<AiReply, AiFailed>;

/// The questions of a window under way.
pub struct Asking {
    /// Where their pieces and their answers go.
    deliver: Deliver,
    /// Where the key of the provider is kept.
    secrets: Arc<Mutex<Box<dyn SecretStore>>>,
    /// The client that sends the requests, once made.
    http: Arc<Mutex<Option<Http>>>,
    /// What cancels each question under way.
    running: Arc<Mutex<HashMap<AskId, Cancel>>>,
    /// Where a draft of a message is put while it is written.
    activities: Activities,
    /// The GitHub Copilot CLI of the window, asked when `ai.provider` is `copilot`.
    copilot: CopilotCli,
}

impl Asking {
    /// The questions of a window, which take the key of the provider from `secrets`, ask GitHub Copilot through
    /// `copilot`, and give what comes to `deliver`.
    #[must_use]
    pub fn new(
        deliver: &Deliver,
        activities: &Activities,
        secrets: Box<dyn SecretStore>,
        copilot: &CopilotCli,
    ) -> Asking {
        Asking {
            copilot: copilot.clone(),
            activities: activities.clone(),
            deliver: Arc::clone(deliver),
            secrets: Arc::new(Mutex::new(secrets)),
            http: Arc::new(Mutex::new(None)),
            running: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Asks a question on a thread of its own: [`Message::AiPiece`] says the text of its answer so far, and
    /// [`Message::AiAnswered`] what it came to.
    pub fn ask(&self, question: AiQuestion) {
        let cancel = Cancel::new();
        let asked = question.id;
        let _before = lock(&self.running).insert(asked, cancel.clone());
        let activities = self.activities.clone();
        let (deliver, secrets, http, running) = (
            Arc::clone(&self.deliver),
            Arc::clone(&self.secrets),
            Arc::clone(&self.http),
            Arc::clone(&self.running),
        );
        let copilot = self.copilot.clone();
        let spawned = thread::Builder::new()
            .name(format!("question {}", question.id))
            .spawn(move || {
                let id = question.id;
                let _activity = activity_words(&question.work).map(|(words, cause)| {
                    let started = model::Timestamp(seconds_now());
                    activities.track(Text::from(words), Text::from(cause), started)
                });
                let asked = Asked {
                    secrets: &secrets,
                    http: &http,
                    copilot: &copilot,
                    activities: &activities,
                };
                let result = answer(&question, asked, &cancel, &deliver);
                let _done = lock(&running).remove(&id);
                if !cancel.is_cancelled() {
                    deliver(Message::AiAnswered { ask: id, result });
                }
            });
        if let Err(error) = spawned {
            let _done = lock(&self.running).remove(&asked);
            (self.deliver)(Message::AiAnswered {
                ask: asked,
                result: Err(AiFailed::saying(error.to_string())),
            });
        }
    }

    /// Cancels questions under way: their requests are closed, and nothing more comes of them.
    pub fn stop(&self, ids: &[AskId]) {
        let running = lock(&self.running);
        for id in ids {
            if let Some(cancel) = running.get(id) {
                cancel.cancel();
            }
        }
    }
}

impl Drop for Asking {
    fn drop(&mut self) {
        for cancel in lock(&self.running).values() {
            cancel.cancel();
        }
    }
}

/// What the Activity tab and the status bar say of a question while it runs, and why: the draft of a message and Ask
/// Gitardo; the conflicts say it on their cards.
fn activity_words(work: &AiWork) -> Option<(&'static str, &'static str)> {
    match work {
        AiWork::Draft(_) => Some((DRAFTING, DRAFTING_CAUSE)),
        AiWork::Help(_) => Some((ASKING_GITARDO, ASKING_GITARDO_CAUSE)),
        AiWork::Conflict(_) => None,
    }
}

/// The seconds since 1970.
fn seconds_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(i64::MAX))
}

/// A lock, also when a thread holding it panicked.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Who is asked, and with what: the key of the provider, the client of its requests, the GitHub Copilot CLI, and where
/// the wait before a request is sent again is shown.
#[derive(Clone, Copy)]
struct Asked<'a> {
    /// Where the key of the provider is kept.
    secrets: &'a Mutex<Box<dyn SecretStore>>,
    /// The client that sends the requests, once made.
    http: &'a Mutex<Option<Http>>,
    /// The GitHub Copilot CLI of the window.
    copilot: &'a CopilotCli,
    /// The activities of the window, among which the status bar says *Waiting for Anthropic, then trying again…*.
    activities: &'a Activities,
}

/// Asks the provider, or GitHub Copilot through the CLI of the window, passing on the text of the answer so far at most
/// every [`PIECE_INTERVAL`].
fn answer(question: &AiQuestion, asked: Asked<'_>, cancel: &Cancel, deliver: &Deliver) -> AiResult {
    let Asked {
        secrets,
        http,
        copilot,
        activities,
    } = asked;
    let Some(kind) = question.settings.provider else {
        return Err(engine::ai::AiError::NotSetUp.into());
    };
    let mut so_far = String::new();
    let mut told: Option<Instant> = None;
    let id = question.id;
    let mut pieces = |piece: &str| {
        so_far.push_str(piece);
        if told.is_none_or(|at| at.elapsed() >= PIECE_INTERVAL) {
            told = Some(Instant::now());
            deliver(Message::AiPiece {
                ask: id,
                text: so_far.clone(),
            });
        }
    };
    if kind == AiProvider::Copilot {
        let mut model = engine::ai::copilot_model(copilot, &question.settings, cancel);
        return work(&question.work, &mut model, cancel, &mut pieces);
    }
    // A store that cannot be read keeps no key: the provider may need none, and says so when it does.
    let key = lock(secrets).read(&kind.key_name()).ok().flatten().map(Key);
    let http = client(http).map_err(|error| AiFailed::saying(error.to_string()))?;
    let provider = Provider {
        settings: question.settings.clone(),
        key,
    };
    let sending = Sending {
        http: &http,
        cancel,
        waiting: Some(Waiting {
            activities,
            clock: &super::operations::SystemClock,
        }),
    };
    let mut model = ProviderModel {
        sending,
        provider: &provider,
    };
    work(&question.work, &mut model, cancel, &mut pieces)
}

/// Asks a language model what a question is: how to resolve a conflict, or the message of a commit.
fn work(work: &AiWork, model: &mut dyn LanguageModel, cancel: &Cancel, pieces: &mut dyn FnMut(&str)) -> AiResult {
    let result = match work {
        AiWork::Conflict(request) => engine::ai::ask_conflict(model, request, pieces).map(AiReply::Conflict),
        AiWork::Draft(job) => {
            let diff = draft_diff(job, cancel)?;
            let input = DraftInput {
                diff: &diff,
                new_files: &job.new_files,
                subjects: &job.subjects,
            };
            engine::ai::draft(model, &input, pieces).map(AiReply::Draft)
        }
        AiWork::Help(request) => {
            let mut text = String::new();
            let result = engine::help::ask(model, request, &mut |piece| {
                text.push_str(piece);
                pieces(piece);
            });
            return result.map(|answer| AiReply::Help { answer, text }).map_err(|error| {
                let text = engine::help::said(&error);
                AiFailed {
                    text,
                    ..AiFailed::from(error)
                }
            });
        }
    };
    result.map_err(AiFailed::from)
}

/// The diff the message of a commit is drafted from (`commit.read-draft-diff`): of what is staged when something is,
/// of every change otherwise.
fn draft_diff(job: &DraftJob, cancel: &Cancel) -> Result<String, AiFailed> {
    let request = DraftDiffRequest {
        is_staged: job.is_staged,
    };
    let diff =
        read_draft_diff(&git::Git::new(&job.folder), &request).map_err(|error| AiFailed::saying(error.to_string()))?;
    if cancel.is_cancelled() {
        return Err(engine::ai::AiError::Cancelled.into());
    }
    Ok(diff)
}

/// The client that sends the requests: made once, trusting the certificates of the system and going through the proxy
/// of the environment.
fn client(made: &Mutex<Option<Http>>) -> Result<Http, http::Error> {
    let mut made = lock(made);
    if let Some(http) = made.as_ref() {
        return Ok(http.clone());
    }
    let proxies = Proxies::from_settings(&ProxySettings::from_environment(None))?;
    let http = Http::new(Trust::system().map_err(http::Error::Tls)?, proxies);
    *made = Some(http.clone());
    Ok(http)
}
