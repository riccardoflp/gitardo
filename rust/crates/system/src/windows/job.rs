//! Job objects: what Windows holds the processes of a task in, so that ending the job ends the task and every process
//! it started, those that no longer have a parent too (`tasks.stop`).

use super::kernel32;
use crate::error::SystemError;

/// The exit code of the processes of a job ended: the one Windows gives a process ended from outside.
const ENDED: u32 = 1;

/// A job object, closed when dropped: closing it ends nothing, its processes go on.
pub(crate) struct Job(kernel32::HANDLE);

// SAFETY: a handle of a job object is a number the system takes from any thread; the job is only ever read through it.
unsafe impl Send for Job {}
// SAFETY: as above: the calls that take the handle may run from several threads at once.
unsafe impl Sync for Job {}

impl Drop for Job {
    fn drop(&mut self) {
        // SAFETY: the handle was opened by the system for this value, which owns it and closes it once.
        let _closed = unsafe { kernel32::CloseHandle(self.0) };
    }
}

impl Job {
    /// A new job that holds a process, by its number, and every process it starts from now on.
    pub(crate) fn holding(process: u32) -> Result<Job, SystemError> {
        // SAFETY: both arguments may be null: a job with the default security and no name.
        let job = unsafe { kernel32::CreateJobObjectW(std::ptr::null_mut(), std::ptr::null()) };
        if job.is_null() {
            return Err(last_error("CreateJobObjectW"));
        }
        let job = Job(job);
        let access = kernel32::PROCESS_SET_QUOTA | kernel32::PROCESS_TERMINATE;
        // SAFETY: the arguments are flags and a number; a process that is gone gives a null handle, checked below.
        let handle = unsafe { kernel32::OpenProcess(access, 0, process) };
        if handle.is_null() {
            return Err(last_error("OpenProcess"));
        }
        // SAFETY: both handles are open, the one of the process with the rights the call asks for.
        let assigned = unsafe { kernel32::AssignProcessToJobObject(job.0, handle) };
        let refused = (assigned == 0).then(|| last_error("AssignProcessToJobObject"));
        // SAFETY: the handle of the process was opened above and is closed once; the job keeps the process.
        let _closed = unsafe { kernel32::CloseHandle(handle) };
        refused.map_or(Ok(job), Err)
    }

    /// Ends every process of the job; a job with none left ends nothing.
    pub(crate) fn end(&self) -> Result<(), SystemError> {
        // SAFETY: the handle is open, with every right over the job it made.
        let ended = unsafe { kernel32::TerminateJobObject(self.0, ENDED) };
        if ended == 0 {
            return Err(last_error("TerminateJobObject"));
        }
        Ok(())
    }
}

/// The error the system says it last had on this thread.
fn last_error(call: &'static str) -> SystemError {
    // SAFETY: the call has no arguments and only reads the error of this thread.
    let code = unsafe { kernel32::GetLastError() };
    SystemError::system(call, &std::io::Error::from_raw_os_error(code.cast_signed()))
}
