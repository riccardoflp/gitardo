//! The remotes: adding, removing, renaming and repointing them, fetching one, and deleting a branch from one
//! (`docs/features/06-branches-tags-remotes.md`).

mod add;
mod delete_branch;
mod fetch;
mod manage;
mod remove;
mod rename;
mod set_url;

pub use add::{NAME, URL, add};
pub use delete_branch::delete_branch;
pub use fetch::fetch;
pub use manage::{ACTION, REMOTE, manage};
pub use remove::remove;
pub use rename::{NEW_NAME, rename};
pub use set_url::set_url;

use model::contract::{AnswerKey, Context, Danger, Outcome, Plan, Query, Refusal, Text, Undo};

/// The remotes with their URLs, read once and shared by the questions of `remote.manage` and the feature it runs.
pub const REMOTES: AnswerKey = AnswerKey("remotes");

/// The remote a feature was invoked on, or the refusal when it was invoked on something else.
fn target_remote(context: &Context) -> Result<&str, Box<Outcome>> {
    match &context.target {
        model::contract::Target::Remote(name) => Ok(name),
        _ => Err(Box::new(Outcome::Refuse(Refusal::saying("Pick a remote first.")))),
    }
}

/// The URL of a remote among those read, or the query that reads them.
fn url_of<'a>(answers: &'a model::contract::Answers, name: &str) -> Result<Option<&'a str>, Box<Outcome>> {
    let Some(remotes) = answers.remotes(REMOTES) else {
        return Err(Box::new(Outcome::Read(REMOTES, Query::Remotes)));
    };
    Ok(remotes
        .iter()
        .find(|remote| remote.name == name)
        .map(|remote| remote.url.as_str()))
}

/// The plan of a change that turned out to change nothing: no step, and a sentence that says so.
fn unchanged(said: String) -> Plan {
    Plan {
        title: Text::from(said.trim_end_matches('.')),
        steps: Vec::new(),
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Nothing changed.")),
        after: vec![model::contract::FollowUp::Say(Text::from(said))],
        on_failure: Vec::new(),
    }
}
