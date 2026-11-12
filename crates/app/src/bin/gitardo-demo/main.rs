//! `gitardo-demo`: a window with every widget of the toolkit, to try them with the mouse and the keyboard, in each
//! theme and at the scale of each screen.

#![forbid(unsafe_code)]
#![windows_subsystem = "windows"]

mod demo;

use std::process::ExitCode;

/// Opens the demo window and runs until it is closed. A failure ends the program with a code of 1: the window has no
/// console to say more in.
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

/// Runs the demo window until it is closed.
fn run() -> Result<(), app::AppError> {
    let mut event_loop = platform::EventLoop::<()>::new()?;
    let mut demo: Option<demo::Demo> = None;
    let mut failure = None;
    event_loop.run(|context, event| {
        let flow = match demo.as_mut() {
            Some(open) => open.handle(context, &event),
            None if event == platform::Event::Started => {
                demo::Demo::open(context, platform::Visibility::Shown).map(|opened| {
                    demo = Some(opened);
                    demo::Flow::Continue
                })
            }
            None => Ok(demo::Flow::Continue),
        };
        match flow {
            Ok(demo::Flow::Continue) => {}
            Ok(demo::Flow::Quit) => context.quit(),
            Err(error) => {
                failure = Some(error);
                context.quit();
            }
        }
    })?;
    failure.map_or(Ok(()), Err)
}
