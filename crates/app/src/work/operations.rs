//! The thread of the operations of a window: it runs one operation at a time through the engine of its repository,
//! which it keeps from one operation to the next for its undo journal, asks the window each question and each plan to
//! confirm, waits for the answer, and sends back how the operation ended with the lines it logged.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};

use engine::{Activities, Asker, Clock, CommandLog, Engine, EngineError, Finished, LogLevel};
use git::{CancelToken, Git};
use model::Timestamp;
use model::contract::{Answer, Answers, Context, FeatureId, FollowUp, Plan, Question, Text, Undo};

use super::Deliver;
use crate::store::Message;
use crate::store::feature::{Ended, Reply};

/// An operation to run.
struct Request {
    /// The repository.
    folder: PathBuf,
    /// The operation.
    feature: FeatureId,
    /// The answers it is given before it asks.
    preset: Answers,
    /// What it is given.
    context: Context,
    /// What the Command log records.
    level: LogLevel,
}

/// The thread of the operations, and the way to answer the one running.
pub struct Operations {
    /// Where the operations go; none once the window is closing.
    requests: Option<Sender<Request>>,
    /// Where the answers go.
    replies: Sender<Reply>,
    /// The token of the operation running, which cancels it when the window closes.
    cancel: CancelToken,
    /// The thread.
    thread: Option<JoinHandle<()>>,
}

impl Operations {
    /// The thread, which puts the plans it runs among `activities` and gives what the operations ask and how they ended
    /// to `deliver`.
    #[must_use]
    pub fn new(activities: &Activities, deliver: &Deliver) -> Operations {
        let (requests, requested) = mpsc::channel::<Request>();
        let (replies, answers) = mpsc::channel::<Reply>();
        let cancel = CancelToken::new();
        let activities = activities.clone();
        let deliver = std::sync::Arc::clone(deliver);
        let token = cancel.clone();
        let thread = thread::Builder::new()
            .name(String::from("operations"))
            .spawn(move || serve(&requested, &answers, (&activities, &deliver, &token)))
            .ok();
        Operations {
            requests: Some(requests),
            replies,
            cancel,
            thread,
        }
    }

    /// Runs an operation with the answers it is given before it asks, its context taken at this moment.
    pub fn run(&self, folder: PathBuf, (feature, preset): (FeatureId, Answers), mut context: Context, level: LogLevel) {
        context.now = SystemClock.now();
        if let Some(requests) = &self.requests {
            let _sent = requests.send(Request {
                folder,
                feature,
                preset,
                context,
                level,
            });
        }
    }

    /// Gives the operation running what the user answered.
    pub fn reply(&self, reply: Reply) {
        let _sent = self.replies.send(reply);
    }
}

impl Drop for Operations {
    fn drop(&mut self) {
        self.cancel.cancel();
        self.requests = None;
        // An operation waiting for an answer is told the user backed out.
        let _sent = self.replies.send(Reply::Answer(None));
        let _sent = self.replies.send(Reply::Confirm(false));
        if let Some(thread) = self.thread.take() {
            let _ended = thread.join();
        }
    }
}

/// The system's clock.
struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        Timestamp(i64::try_from(seconds).unwrap_or(i64::MAX))
    }
}

/// What the thread does until the window closes: runs each operation through the engine of its repository.
fn serve(
    requests: &Receiver<Request>,
    replies: &Receiver<Reply>,
    (activities, deliver, cancel): (&Activities, &Deliver, &CancelToken),
) {
    let mut engines: HashMap<PathBuf, Engine> = HashMap::new();
    while let Ok(request) = requests.recv() {
        if cancel.is_cancelled() {
            return;
        }
        // An answer left from an operation before is not one for this one.
        while replies.try_recv().is_ok() {}
        let engine = engines.entry(request.folder.clone()).or_insert_with(|| {
            let mut engine = Engine::new(
                Git::new(&request.folder),
                CommandLog::new(request.level),
                Box::new(SystemClock),
            );
            engine.share_activities(activities.clone());
            engine.find_operations_with(features::operation);
            engine
        });
        engine.cancel_with(cancel.clone());
        let operation = features::operation(request.feature.0).map(|(_, operation)| operation);
        let ended = match operation {
            Some(operation) => {
                let mut asker = WindowAsker {
                    deliver,
                    replies,
                    preset: &request.preset,
                };
                // The answers given before are known from the first call: those of questions are not asked, and the
                // others, such as the comparisons opened before, are what the window knows that git does not.
                let given = (&request.context, request.preset.clone());
                ended_of(
                    request.feature,
                    engine.run_feature_from(request.feature, operation, given, &mut asker),
                )
            }
            None => Ended::Refused(
                model::contract::Text::from(format!("{} is not built yet", request.feature.0)),
                None,
            ),
        };
        let ended = with_prepared_message(ended, &request.context);
        let log = engine.take_log(request.level);
        deliver(Message::FeatureEnded { ended, log });
    }
}

/// An operation that ran, with the message git prepared for the next commit when its plan asked for it: what
/// `SQUASH_MSG` in the git folder says.
fn with_prepared_message(mut ended: Ended, context: &Context) -> Ended {
    if let Ended::Ran {
        after,
        prepared_message,
        ..
    } = &mut ended
        && after.contains(&model::contract::FollowUp::PreparedMessage)
    {
        let file = context.repository.git_folder.join("SQUASH_MSG");
        *prepared_message = std::fs::read_to_string(file)
            .ok()
            .map(|message| message.trim_end().to_owned())
            .filter(|message| !message.is_empty());
    }
    ended
}

/// How an operation ended, in the words the store keeps.
fn ended_of(feature: FeatureId, result: Result<Finished, EngineError>) -> Ended {
    match result {
        Ok(Finished::Ran {
            plan, went_on_after, ..
        }) => Ended::Ran {
            can_undo: !matches!(plan.undo, Undo::Impossible(_)),
            title: plan.title,
            after: said_with_failures(plan.after, &went_on_after),
            prepared_message: None,
        },
        Ok(Finished::Refused(refusal)) => Ended::Refused(refusal.message, refusal.offer),
        Ok(Finished::BackedOut | Finished::NotConfirmed(_)) => Ended::BackedOut,
        Err(EngineError::Step(failure)) => Ended::Failed {
            title: format!("`{}`", failure.failed),
            said: failure.said,
            plan: failure.plan,
            on_failure: failure.on_failure.into_vec(),
        },
        Err(error) => Ended::Failed {
            title: feature.0.to_owned(),
            said: error.to_string(),
            plan: String::new(),
            on_failure: Vec::new(),
        },
    }
}

/// What a plan says once it ran, then what failed of the links and copies of a worktree while the others went on
/// (`worktree-setup.link-and-copy-steps`), in the one sentence the window shows.
fn said_with_failures(mut after: Vec<FollowUp>, went_on_after: &[engine::StepFailure]) -> Vec<FollowUp> {
    let failed: Vec<String> = went_on_after.iter().map(|failure| failure.failed.clone()).collect();
    let Some(warning) = model::worktree_setup::failed_setup_steps(&failed) else {
        return after;
    };
    let said = after.iter().rev().find_map(|follow_up| match follow_up {
        FollowUp::Say(words) => Some(words.as_str().to_owned()),
        _ => None,
    });
    after.push(FollowUp::Say(Text::from(match said {
        Some(said) => format!("{said} {warning}"),
        None => warning,
    })));
    after
}

/// Asks the window: sends the question or the plan, then waits on this thread for what the user answers.
struct WindowAsker<'a> {
    /// Where the question goes.
    deliver: &'a Deliver,
    /// Where the answer comes from.
    replies: &'a Receiver<Reply>,
    /// The answers given before it asks, which no dialog shows.
    preset: &'a Answers,
}

impl Asker for WindowAsker<'_> {
    fn ask(&mut self, question: &Question) -> Option<Answer> {
        if let Some(answer) = self.preset.get(question.key) {
            return Some(answer.clone());
        }
        (self.deliver)(Message::FeatureAsks(Box::new(question.clone())));
        loop {
            match self.replies.recv() {
                Ok(Reply::Answer(answer)) => return answer,
                Ok(Reply::Confirm(_)) => {}
                Err(_) => return None,
            }
        }
    }

    /// The clipboard is read on this thread: the system lets any thread of the program read it.
    fn clipboard(&mut self) -> Option<String> {
        // A clipboard that cannot be read holds no text the operation could use: it says there is none.
        platform::Clipboard::new().ok()?.read_text().ok().flatten()
    }

    fn confirm(&mut self, plan: &Plan) -> bool {
        (self.deliver)(Message::FeatureConfirms(Box::new(plan.clone())));
        loop {
            match self.replies.recv() {
                Ok(Reply::Confirm(is_confirmed)) => return is_confirmed,
                Ok(Reply::Answer(_)) => {}
                Err(_) => return false,
            }
        }
    }
}
