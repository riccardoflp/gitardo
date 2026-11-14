//! Gitardo Desktop: the windows, the event loop, the store, the settings and what is kept between runs.
//!
//! The library holds what the binaries share: the window that shows a view of the toolkit ([`window::Surface`]); the
//! window of `gitardo` around it ([`desktop`]), with its store ([`store`]), the workers that run its reads ([`work`]),
//! its settings ([`settings`]) and what it keeps between runs ([`persisted`]).

#![forbid(unsafe_code)]

pub mod desktop;
pub mod error;
pub mod persisted;
pub mod settings;
pub mod store;
pub mod window;
pub mod work;

pub use error::AppError;
