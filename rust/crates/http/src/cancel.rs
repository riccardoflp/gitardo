//! Cancelling a request nobody wants any more, and giving up on a server that says nothing: the connection under a
//! request waits for each read a short while at a time, and looks between two waits whether it should go on.

use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// How long a read waits at a time before it looks whether its request was cancelled.
pub(crate) const LOOK_EVERY: Duration = Duration::from_millis(50);

/// A way to stop requests that are no longer wanted. Cancelling it ends every request sent with it, now or later, with
/// [`crate::Error::Cancelled`]; one that waits for an answer stops waiting within a twentieth of a second.
#[derive(Clone, Default)]
pub struct Cancel {
    flag: Arc<AtomicBool>,
    also: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
}

impl std::fmt::Debug for Cancel {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Cancel")
            .field("is_cancelled", &self.is_cancelled())
            .finish()
    }
}

impl Cancel {
    /// A way to cancel, not cancelled yet.
    #[must_use]
    pub fn new() -> Cancel {
        Cancel::default()
    }

    /// A way to cancel that is cancelled also when a question says so: whether the job that sends the request was
    /// cancelled, as its own token says.
    #[must_use]
    pub fn following(is_cancelled: impl Fn() -> bool + Send + Sync + 'static) -> Cancel {
        Cancel {
            flag: Arc::default(),
            also: Some(Arc::new(is_cancelled)),
        }
    }

    /// Cancels every request sent with it, and those sent with it later.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    /// Whether it was cancelled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst) || self.also.as_ref().is_some_and(|also| also())
    }
}

/// A TCP connection whose reads wait [`LOOK_EVERY`] at a time: between two waits, a read ends when the request was
/// cancelled, or when nothing came for as long as the request waits between two reads. What is above it, TLS
/// included, never sees a read that timed out and goes on.
pub(crate) struct Patient {
    tcp: TcpStream,
    cancel: Cancel,
    idle: Duration,
}

impl Patient {
    /// Watches a connection whose read timeout is [`LOOK_EVERY`].
    pub(crate) fn new(tcp: TcpStream, cancel: &Cancel, idle: Duration) -> std::io::Result<Patient> {
        tcp.set_read_timeout(Some(LOOK_EVERY))?;
        tcp.set_write_timeout(Some(idle))?;
        Ok(Patient {
            tcp,
            cancel: cancel.clone(),
            idle,
        })
    }
}

impl Read for Patient {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let started = Instant::now();
        loop {
            match self.tcp.read(buffer) {
                Err(error) if matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) => {
                    if self.cancel.is_cancelled() {
                        return Err(ErrorKind::ConnectionAborted.into());
                    }
                    if started.elapsed() >= self.idle {
                        return Err(ErrorKind::TimedOut.into());
                    }
                }
                other => return other,
            }
        }
    }
}

impl Write for Patient {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.cancel.is_cancelled() {
            return Err(ErrorKind::ConnectionAborted.into());
        }
        self.tcp.write(bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.tcp.flush()
    }
}
