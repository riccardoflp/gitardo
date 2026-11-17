//! The questions to the AI provider of a window (`ai.resolve-conflicts`, `commit.draft-message`): each on a thread of
//! its own, its key taken from the credential store, the text of its answer passed on as it comes, at most every
//! 120 ms, and cancelled when it is not wanted any more.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use engine::ai::{ConflictReply, ConflictRequest, Provider, ProviderModel, Sending};
use engine::{Activities, SecretStore};
use git::read::commit::{DraftDiffRequest, read_draft_diff};
use http::{Cancel, Http, Proxies, ProxySettings, Trust};
use model::ai::{AiSettings, Key};
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

/// How often, at most, the text of an answer that comes is passed on.
pub const PIECE_INTERVAL: Duration = Duration::from_millis(120);

/// What is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiWork {
    /// How to resolve one conflict (`ai.ask-conflict`).
    Conflict(ConflictRequest),
    /// The message of a commit (`commit.draft-message`).
    Draft(DraftJob),
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
}

/// Why a question has no answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiFailed {
    /// Why, in words.
    pub text: String,
    /// Whether no other question is to be asked: the model chosen is not there.
    pub stops_the_rest: bool,
}

impl AiFailed {
    /// A failure in words, which asks the others all the same.
    #[must_use]
    pub fn saying(text: String) -> AiFailed {
        AiFailed {
            text,
            stops_the_rest: false,
        }
    }
}

impl From<engine::ai::AiError> for AiFailed {
    fn from(error: engine::ai::AiError) -> AiFailed {
        AiFailed {
            stops_the_rest: matches!(error, engine::ai::AiError::ModelMissing(_)),
            text: error.to_string(),
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
}

impl Asking {
    /// The questions of a window, which take the key of the provider from `secrets` and give what comes to `deliver`.
    #[must_use]
    pub fn new(deliver: &Deliver, activities: &Activities, secrets: Box<dyn SecretStore>) -> Asking {
        Asking {
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
        let spawned = thread::Builder::new()
            .name(format!("question {}", question.id))
            .spawn(move || {
                let id = question.id;
                let _activity = matches!(question.work, AiWork::Draft(_)).then(|| {
                    let started = model::Timestamp(seconds_now());
                    activities.track(Text::from(DRAFTING), Text::from(DRAFTING_CAUSE), started)
                });
                let result = answer(question, (&secrets, &http), &cancel, &deliver);
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

/// Asks the provider, passing on the text of the answer so far at most every [`PIECE_INTERVAL`].
fn answer(
    question: AiQuestion,
    (secrets, http): (&Mutex<Box<dyn SecretStore>>, &Mutex<Option<Http>>),
    cancel: &Cancel,
    deliver: &Deliver,
) -> AiResult {
    let Some(kind) = question.settings.provider else {
        return Err(engine::ai::AiError::NotSetUp.into());
    };
    // A store that cannot be read keeps no key: the provider may need none, and says so when it does.
    let key = lock(secrets).read(&kind.key_name()).ok().flatten().map(Key);
    let http = client(http).map_err(|error| AiFailed::saying(error.to_string()))?;
    let provider = Provider {
        settings: question.settings,
        key,
    };
    let sending = Sending { http: &http, cancel };
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
    let mut model = ProviderModel {
        sending,
        provider: &provider,
    };
    let result = match &question.work {
        AiWork::Conflict(request) => engine::ai::ask_conflict(&mut model, request, &mut pieces).map(AiReply::Conflict),
        AiWork::Draft(job) => {
            let diff = draft_diff(job, cancel)?;
            let input = DraftInput {
                diff: &diff,
                new_files: &job.new_files,
                subjects: &job.subjects,
            };
            engine::ai::draft(&mut model, &input, &mut pieces).map(AiReply::Draft)
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
