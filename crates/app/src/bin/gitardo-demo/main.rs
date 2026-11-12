//! `gitardo-demo`: a window with every widget of the toolkit, to try them with the mouse and the keyboard, in each
//! theme and at the scale of each screen.

#![forbid(unsafe_code)]
#![windows_subsystem = "windows"]

#[cfg(windows)]
mod demo;

use std::process::ExitCode;

/// Opens the demo window and runs until it is closed. A failure ends the program with a code of 1: the window has no
/// console to say more in.
#[cfg(windows)]
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

/// There is a window on Windows only, for now.
#[cfg(not(windows))]
fn main() -> ExitCode {
    ExitCode::FAILURE
}

/// Runs the demo window until it is closed.
#[cfg(windows)]
fn run() -> Result<(), app::AppError> {
    let event_loop = platform::EventLoop::<()>::new()?;
    let mut demo = demo::Demo::open(&event_loop, platform::Visibility::Shown)?;
    let mut failure = None;
    event_loop.run(|event| match demo.handle(&event_loop, &event) {
        Ok(demo::Flow::Continue) => {}
        Ok(demo::Flow::Quit) => event_loop.quit(),
        Err(error) => {
            failure = Some(error);
            event_loop.quit();
        }
    })?;
    failure.map_or(Ok(()), Err)
}
