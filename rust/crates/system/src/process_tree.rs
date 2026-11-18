//! The processes a task started (`tasks.stop`): the task and whatever it started, held together so that they end
//! together, in a job object on Windows and as a process group on Linux and macOS.

use crate::error::SystemError;

/// A process and every process it starts, to end together.
pub struct ProcessTree {
    /// The job object that holds them.
    #[cfg(windows)]
    job: crate::windows::Job,
    /// The number of the process that leads their group: the one given, which started a group of its own.
    #[cfg(not(windows))]
    leader: u32,
}

impl ProcessTree {
    /// The processes of a process that just started, by its number: on Windows it is put in a job object of its own,
    /// which holds what it starts from then on; elsewhere it leads a process group already, as a process started in a
    /// pseudo-terminal does.
    ///
    /// # Errors
    ///
    /// When the process is gone already, or the system refuses the job object.
    pub fn of(process: u32) -> Result<ProcessTree, SystemError> {
        #[cfg(windows)]
        return crate::windows::Job::holding(process).map(|job| ProcessTree { job });
        #[cfg(not(windows))]
        return Ok(ProcessTree { leader: process });
    }

    /// Ends every process of them that still runs, at once and without asking them.
    ///
    /// # Errors
    ///
    /// When the system refuses.
    pub fn end(&self) -> Result<(), SystemError> {
        #[cfg(windows)]
        return self.job.end();
        #[cfg(unix)]
        return crate::unix::end_group(self.leader);
        #[cfg(not(any(windows, unix)))]
        return Err(SystemError::Unsupported {
            what: "end the processes a task started",
        });
    }
}

#[cfg(test)]
mod tests {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    use super::ProcessTree;

    /// A process that would run for a minute.
    fn sleeper() -> Command {
        let mut command = if cfg!(windows) {
            let mut command = Command::new("ping");
            let _ = command.args(["-n", "60", "127.0.0.1"]);
            command
        } else {
            let mut command = Command::new("sleep");
            let _ = command.arg("60");
            command
        };
        let _ = command.stdout(Stdio::null()).stderr(Stdio::null()).stdin(Stdio::null());
        command
    }

    #[cfg(windows)]
    #[test]
    fn ending_the_processes_of_a_process_ends_it_long_before_it_would_end() {
        let mut child = sleeper().spawn().unwrap();
        let tree = ProcessTree::of(child.id()).unwrap();
        let started = Instant::now();
        tree.end().unwrap();
        let status = child.wait().unwrap();
        assert!(started.elapsed() < Duration::from_secs(20));
        assert!(!status.success(), "a process ended from outside did not succeed");
        tree.end().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn ending_the_processes_of_a_process_ends_it_long_before_it_would_end() {
        use std::os::unix::process::CommandExt;
        let mut command = sleeper();
        // The process leads a group of its own, as one started in a pseudo-terminal does.
        let _ = command.process_group(0);
        let mut child = command.spawn().unwrap();
        let tree = ProcessTree::of(child.id()).unwrap();
        let started = Instant::now();
        tree.end().unwrap();
        let status = child.wait().unwrap();
        assert!(started.elapsed() < Duration::from_secs(20));
        assert!(!status.success(), "a process ended from outside did not succeed");
    }
}
