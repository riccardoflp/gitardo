//! The words of Ask Gitardo (`help.page`, `help.show-me`).

/// The name of the page in its trail.
pub const PAGE: &str = "Ask Gitardo";
/// The entry of Quick search and of the **…** menu that opens the page.
pub const COMMAND: &str = "Ask Gitardo\u{2026}";
/// What the last entry of Quick search is, on the right of its row.
pub const QUESTION: &str = "Question";
/// What the empty page says.
pub const EMPTY: &str = "Ask how to do something with Gitardo, in your own words.";
/// The examples of the empty page, each a button that asks it.
pub const EXAMPLES: [&str; 3] = [
    "How do I squash my last three commits?",
    "How do I see who changed a line?",
    "How do I move a commit to another branch?",
];
/// What the box says while it is empty, which is also its name for the screen reader.
pub const PLACEHOLDER: &str = "Ask how to do something\u{2026}";
/// The button that sends the question.
pub const ASK: &str = "Ask";
/// The button that stops the answer that comes.
pub const STOP: &str = "Stop";
/// What shows under the question until the first words of its answer.
pub const LOOKING: &str = "Looking for the features\u{2026}";
/// The button of the header of the page that forgets the questions and the answers.
pub const NEW_CONVERSATION: &str = "New conversation";
/// The button under a failure that asks its question again.
pub const TRY_AGAIN: &str = "Try again";
/// The button of the page with no AI provider, which shows `ai.provider` on the settings page.
pub const OPEN_SETTINGS: &str = "Open the settings";
/// The name of the conversation for the screen reader.
pub const CONVERSATION: &str = "Conversation";

/// The last entry of Quick search when what is typed finds no command: **Ask Gitardo: "how do I undo a push"**.
#[must_use]
pub fn ask_about(typed: &str) -> String {
    format!("{PAGE}: \"{}\"", typed.trim())
}
