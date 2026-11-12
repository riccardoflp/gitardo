//! Interaction: which widget the pointer is over, which one holds a press, which one has the keyboard, and what each
//! one was told in a frame.

mod hits;
mod response;
mod route;

pub use hits::Sense;
pub use response::Response;
pub use route::WidgetEvent;

pub(crate) use hits::{Hit, Hits};
pub(crate) use route::{Active, Interaction, RouteSettings, Routed, route, update_hover};

#[cfg(test)]
mod tests;
