//! The folders the user trusts: the question asked before running what a folder asks to run (`trust.ask`), and the
//! list of the folders trusted, where some stop being trusted (`trust.manage`).

mod ask;
mod manage;

pub use ask::{TRUST, TrustAsked, ask, trusted_by_answer};
pub use manage::{TRUSTED, manage};
