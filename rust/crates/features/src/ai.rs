//! The language model of the AI provider (`docs/features/09-conflicts.md`, `ai.`): which model is asked, and how much
//! it thinks.

mod choose_model;

pub use choose_model::{COPILOT_INSTALL, COPILOT_SIGN_IN, choose_model, copilot_refusal};
