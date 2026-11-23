//! The GitHub Copilot CLI running, spoken to through GitHub's SDK for Rust, `copilot-sdk` (D87): the SDK keeps the
//! protocol, and this file keeps what Gitardo needs of it apart from the rest of the engine, which sees no future, no
//! `tokio` and no type of the SDK (R1.6).
//!
//! The SDK is asynchronous, on `tokio`. A runtime of one thread runs it for the window; the CLI's standard input and
//! output, or the pipes of a fake in a test, are carried to it and from it by two threads that read and write them as
//! they are, so that the SDK speaks to the same streams whatever started the CLI. A question is a task of the runtime,
//! whose pieces come back on a channel.

use std::future::Future;
use std::io::{Read, Write};
use std::process::Child;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use copilot_sdk::handler::DenyAllHandler;
use copilot_sdk::session_events::{AssistantMessageData, AssistantMessageDeltaData, SessionErrorData};
use copilot_sdk::{ClientMode, CreateSessionOptions, ErrorKind, ProtocolErrorKind, SessionEvent};
use http::Cancel;
use model::ai::{AiModel, CopilotNeed};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream, ReadHalf, WriteHalf};
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

use super::CopilotError;

/// How often a wait for the CLI looks whether the question was cancelled.
const CANCEL_CHECK: Duration = Duration::from_millis(50);

/// The model that lets Copilot choose the model of each question, offered when the subscription lists no other.
pub(super) const AUTO_MODEL: &str = "auto";

/// How much of what the CLI writes, or is written, is carried at once.
const CARRIED: usize = 64 * 1024;

/// What the CLI prints, and where what it reads is written: the pipes of the process, or those of a fake in a test.
pub struct CopilotStreams {
    /// What the CLI writes.
    pub output: Box<dyn Read + Send>,
    /// What the CLI reads.
    pub input: Box<dyn Write + Send>,
}

/// The process of the CLI and the processes it started, which end with it.
pub(super) struct Process {
    /// The CLI.
    pub(super) child: Child,
    /// The CLI and what it starts: on Windows `copilot.cmd` starts the program that answers.
    pub(super) tree: Option<system::ProcessTree>,
}

/// The settings of one question: the model, the effort, and the prompt.
pub(super) struct Question {
    /// The model asked.
    pub(super) model: String,
    /// The effort, as Copilot names it, when one is set.
    pub(super) effort: Option<String>,
    /// The text of the question.
    pub(super) prompt: String,
}

/// What a question sends back while it is answered.
enum Piece {
    /// A piece of the answer, as it comes.
    Text(String),
    /// The answer is over, its session ended: how it ended.
    End(Result<(), CopilotError>),
}

/// The CLI running and the SDK's client speaking to it.
pub(super) struct Client {
    /// The SDK's client; none once it is closing.
    sdk: Option<copilot_sdk::Client>,
    /// The runtime of one thread the SDK runs on; none once it is closing.
    runtime: Option<Runtime>,
    /// The thread that carries what the CLI writes to the SDK; it ends when the CLI's output ends.
    output: Option<JoinHandle<()>>,
    /// The thread that carries what the SDK writes to the CLI; it ends when the SDK is dropped.
    input: Option<JoinHandle<()>>,
    /// The process, when it is one.
    process: Option<Process>,
}

impl Client {
    /// Speaks to a CLI over its streams, with the process they are of when they are a process's: the runtime is
    /// started, the streams carried, and the SDK's handshake made, which refuses a CLI whose protocol it does not
    /// speak.
    pub(super) fn over(
        streams: CopilotStreams,
        process: Option<Process>,
        cancel: &Cancel,
    ) -> Result<Client, CopilotError> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("copilot")
            .enable_all()
            .build()
            .map_err(|error| CopilotError::Start(error.to_string()))?;
        let (sdk_side, carried) = tokio::io::duplex(CARRIED);
        let (to_sdk_reader, from_sdk_writer) = tokio::io::split(carried);
        let (sdk_reader, sdk_writer) = tokio::io::split(sdk_side);
        let handle = runtime.handle().clone();
        let output = carry_output(streams.output, from_sdk_writer, handle.clone())?;
        let input = carry_input(to_sdk_reader, streams.input, handle)?;
        let sdk = {
            let _inside = runtime.enter();
            // The SDK reads no file of the folder it is given: Gitardo's questions carry all they need, and Copilot
            // has no tool to read with.
            copilot_sdk::Client::from_streams(ClientMode::Empty, sdk_reader, sdk_writer, std::env::temp_dir())
                .map_err(|error| CopilotError::Start(error.to_string()))?
        };
        let client = Client {
            sdk: Some(sdk),
            runtime: Some(runtime),
            output: Some(output),
            input: Some(input),
            process,
        };
        let sdk = client.sdk()?;
        client.run(async move { sdk.verify_protocol_version().await }, cancel)?;
        Ok(client)
    }

    /// Whether the CLI still reads and writes: its output has not ended.
    pub(super) fn is_open(&self) -> bool {
        self.output.as_ref().is_some_and(|output| !output.is_finished())
    }

    /// Whether the user is signed in to the CLI.
    pub(super) fn is_signed_in(&self, cancel: &Cancel) -> Result<bool, CopilotError> {
        let sdk = self.sdk()?;
        let status = self.run(async move { sdk.get_auth_status().await }, cancel)?;
        Ok(status.is_authenticated)
    }

    /// The models of the user's subscription, in Copilot's order.
    pub(super) fn models(&self, cancel: &Cancel) -> Result<Vec<AiModel>, CopilotError> {
        let sdk = self.sdk()?;
        let listed = self.run(async move { sdk.list_models().await }, cancel)?;
        let models: Vec<AiModel> = listed
            .into_iter()
            .map(|model| AiModel {
                name: if model.name.is_empty() {
                    model.id.clone()
                } else {
                    model.name
                },
                id: model.id,
            })
            .collect();
        // A subscription that lets Copilot choose the model of each question lists none: its choice is the one model.
        if models.is_empty() {
            return Ok(vec![AiModel {
                id: String::from(AUTO_MODEL),
                name: String::from("Auto"),
            }]);
        }
        Ok(models)
    }

    /// Asks a question in a session of its own, each piece of its answer passed on as it comes, until the session is
    /// idle; a question cancelled is aborted. The session is ended either way, and the CLI is left running.
    pub(super) fn ask(
        &self,
        question: Question,
        cancel: &Cancel,
        pieces: &mut dyn FnMut(&str),
    ) -> Result<String, CopilotError> {
        let sdk = self.sdk()?;
        let (sender, received) = mpsc::channel();
        let (stop, stopped) = oneshot::channel();
        let task = self
            .runtime()?
            .spawn(async move { answer(&sdk, question, &sender, stopped).await });
        let mut text = String::new();
        let mut stop = Some(stop);
        let mut waiting = cancel;
        let never = Cancel::new();
        let ended = loop {
            match wait(&received, waiting) {
                Ok(Piece::Text(piece)) => {
                    text.push_str(&piece);
                    pieces(&piece);
                }
                Ok(Piece::End(ended)) => break ended,
                // The question is not wanted any more: Copilot is asked to stop writing, and its session is still
                // ended before the question returns, so that the CLI is ready for the next one.
                Err(CopilotError::Cancelled) => {
                    if let Some(stop) = stop.take() {
                        let _sent = stop.send(());
                    }
                    waiting = &never;
                }
                Err(error) => break Err(error),
            }
        };
        drop(task);
        if stop.is_none() {
            return Err(CopilotError::Cancelled);
        }
        ended.map(|()| text)
    }

    /// The SDK's client, which is cheap to clone.
    fn sdk(&self) -> Result<copilot_sdk::Client, CopilotError> {
        self.sdk.clone().ok_or(CopilotError::Ended)
    }

    /// The runtime.
    fn runtime(&self) -> Result<&Runtime, CopilotError> {
        self.runtime.as_ref().ok_or(CopilotError::Ended)
    }

    /// Runs a request of the SDK on the runtime and waits for its answer, or until it is cancelled.
    fn run<T: Send + 'static>(
        &self,
        request: impl Future<Output = Result<T, copilot_sdk::Error>> + Send + 'static,
        cancel: &Cancel,
    ) -> Result<T, CopilotError> {
        let (sender, answer) = mpsc::channel();
        let task = self.runtime()?.spawn(async move {
            // A question cancelled waits for no answer.
            let _sent = sender.send(request.await);
        });
        let answered = wait(&answer, cancel);
        if answered.is_err() {
            task.abort();
        }
        answered?.map_err(|error| said(&error))
    }
}

impl Drop for Client {
    /// Ends the SDK's client and its runtime, which closes the CLI's input; ends the CLI and what it started; and
    /// waits for the threads that carried its streams.
    fn drop(&mut self) {
        if let Some(sdk) = self.sdk.take() {
            sdk.force_stop();
        }
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_timeout(Duration::from_secs(1));
        }
        if let Some(mut process) = self.process.take() {
            // A CLI that ended already, or that the system will not end, has nothing more to end.
            if let Some(tree) = &process.tree {
                let _ended = tree.end();
            }
            let _killed = process.child.kill();
            let _waited = process.child.wait();
        }
        for carrying in [self.input.take(), self.output.take()].into_iter().flatten() {
            let _joined = carrying.join();
        }
    }
}

/// The question asked in a session of its own: created with the model and the effort, no tool and every permission
/// refused, its answer streamed; the session aborted when `stopped` comes first, and ended once over.
async fn answer(
    sdk: &copilot_sdk::Client,
    question: Question,
    sender: &Sender<Piece>,
    mut stopped: oneshot::Receiver<()>,
) {
    let session = match sdk.create_session(session_options(&question)).await {
        Ok(session) => session,
        Err(error) => {
            let _sent = sender.send(Piece::End(Err(said(&error))));
            return;
        }
    };
    let mut events = session.subscribe();
    let mut written = false;
    let ended = match session.send(question.prompt.as_str()).await {
        Err(error) => Err(said(&error)),
        Ok(_message) => loop {
            tokio::select! {
                event = events.recv() => match event {
                    Ok(event) => {
                        if let Some(ended) = take(&event, &mut written, sender) {
                            break ended;
                        }
                    }
                    Err(_closed) => break Err(CopilotError::Ended),
                },
                Ok(()) = &mut stopped => {
                    // A CLI that ended has no question to stop.
                    let _stopped = session.abort().await;
                    break Err(CopilotError::Cancelled);
                }
            }
        },
    };
    // A session that cannot be ended is one the CLI ended or forgot already.
    let _ended = session.disconnect().await;
    let _sent = sender.send(Piece::End(ended));
}

/// What an event of the session is to the answer: a piece passed on, or its end when the session is idle or failed;
/// its thinking, the tools it would use and the usage of the request are nothing of the answer.
fn take(event: &SessionEvent, written: &mut bool, sender: &Sender<Piece>) -> Option<Result<(), CopilotError>> {
    match event.event_type.as_str() {
        "assistant.message_delta" => {
            if let Some(delta) = event.typed_data::<AssistantMessageDeltaData>() {
                *written = true;
                let _sent = sender.send(Piece::Text(delta.delta_content));
            }
            None
        }
        // The whole message comes after its pieces; it is the answer when none came before it.
        "assistant.message" if !*written => {
            if let Some(message) = event.typed_data::<AssistantMessageData>() {
                *written = true;
                let _sent = sender.send(Piece::Text(message.content));
            }
            None
        }
        "session.idle" => Some(Ok(())),
        "session.error" => {
            let words = event
                .typed_data::<SessionErrorData>()
                .map_or_else(|| String::from("no reason given"), |error| error.message);
            Some(Err(CopilotError::Refused(words)))
        }
        _ => None,
    }
}

/// The options of a session: the model and the effort of the question, the answer streamed, no tool at all, and every
/// permission it would ask refused.
fn session_options(question: &Question) -> CreateSessionOptions {
    let effort = question.effort.clone();
    CreateSessionOptions::default()
        .with_model(question.model.clone())
        .with_inference(|inference| match effort {
            Some(effort) => inference.with_reasoning_effort(effort),
            None => inference,
        })
        .with_events(|events| events.with_streaming(true))
        .with_tools(|tools| tools.with_available_tools(Vec::<String>::new()))
        .with_permissions(|permissions| permissions.with_on_request(Arc::new(DenyAllHandler)))
}

/// What a failure of the SDK is to Gitardo: a CLI that ended, one whose protocol the SDK does not speak, or a refusal
/// in Copilot's own words.
fn said(error: &copilot_sdk::Error) -> CopilotError {
    match error.kind() {
        ErrorKind::Protocol(ProtocolErrorKind::VersionMismatch { server, min, .. }) if server < min => {
            CopilotError::Needs(CopilotNeed::Update)
        }
        ErrorKind::Protocol(ProtocolErrorKind::VersionMismatch { .. }) => CopilotError::TooNew,
        ErrorKind::Rpc { .. } => CopilotError::Refused(
            error
                .message()
                .map_or_else(|| String::from("no reason given"), str::to_owned),
        ),
        _ if error.is_transport_failure() => CopilotError::Ended,
        ErrorKind::Json | ErrorKind::Protocol(_) => CopilotError::Unreadable(error.to_string()),
        _ => CopilotError::Refused(error.to_string()),
    }
}

/// Waits for what comes on a channel, looking every [`CANCEL_CHECK`] whether it was cancelled.
fn wait<T>(answer: &Receiver<T>, cancel: &Cancel) -> Result<T, CopilotError> {
    loop {
        if cancel.is_cancelled() {
            return Err(CopilotError::Cancelled);
        }
        match answer.recv_timeout(CANCEL_CHECK) {
            Ok(value) => return Ok(value),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Err(CopilotError::Ended),
        }
    }
}

/// Carries what the CLI writes to the SDK, until the CLI's output ends or the SDK is gone; then the SDK reads the end.
fn carry_output(
    mut output: Box<dyn Read + Send>,
    mut to_sdk: WriteHalf<DuplexStream>,
    runtime: tokio::runtime::Handle,
) -> Result<JoinHandle<()>, CopilotError> {
    thread::Builder::new()
        .name(String::from("copilot output"))
        .spawn(move || {
            let mut carried = vec![0; CARRIED];
            // An output that cannot be read is a CLI that ended, as one whose output ended.
            while let Ok(read @ 1..) = output.read(&mut carried)
                && let Some(piece) = carried.get(..read)
                && runtime.block_on(to_sdk.write_all(piece)).is_ok()
            {}
            let _ended = runtime.block_on(to_sdk.shutdown());
        })
        .map_err(|error| CopilotError::Start(error.to_string()))
}

/// Carries what the SDK writes to the CLI, until the SDK is gone or the CLI's input is closed; then the CLI's input is
/// closed, which ends it.
fn carry_input(
    mut from_sdk: ReadHalf<DuplexStream>,
    mut input: Box<dyn Write + Send>,
    runtime: tokio::runtime::Handle,
) -> Result<JoinHandle<()>, CopilotError> {
    thread::Builder::new()
        .name(String::from("copilot input"))
        .spawn(move || {
            let mut carried = vec![0; CARRIED];
            while let Ok(read @ 1..) = runtime.block_on(from_sdk.read(&mut carried))
                && let Some(piece) = carried.get(..read)
                && input.write_all(piece).and_then(|()| input.flush()).is_ok()
            {}
        })
        .map_err(|error| CopilotError::Start(error.to_string()))
}
