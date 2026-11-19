//! The answer of Ask Gitardo, read (`help.read-answer`): `[[id]]` made the title of its card as a link that **Show me**
//! follows, or in bold in the extension, whose Chat follows no link of its own, and the features it names listed.

use std::fmt::Write as _;

use super::guide::{App, Guide};
use crate::commit::clean_draft;
use crate::contract::FeatureId;

/// Where a link to a feature goes, before its identifier: `show-me:squash.commits`.
pub const SHOW_ME_LINK: &str = "show-me:";

/// The answer as the page draws it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HelpAnswer {
    /// The Markdown of the answer, each feature it names a link to [`SHOW_ME_LINK`] and its identifier, or its title in
    /// bold in the extension.
    pub markdown: String,
    /// The features it names, each once, in the order first named.
    pub named: Vec<FeatureId>,
    /// The model that wrote it, as the provider names it; empty while it comes.
    pub model: String,
}

/// The feature a link of an answer goes to, when it is one.
#[must_use]
pub fn shown_feature(link: &str) -> Option<&str> {
    link.strip_prefix(SHOW_ME_LINK)
}

/// The answer read: a code fence around all of it taken off, each `[[id]]` of a card of the guide made the title of its
/// card as a link, or in bold with the guide of the extension, whose Chat writes the buttons of the features itself; any
/// other `[[id]]` taken out, and a `[[` that has no `]]` after it yet, while the answer comes, left out with what follows
/// it.
///
/// Feature: help.read-answer
#[must_use]
pub fn read_answer(text: &str, guide: &Guide) -> HelpAnswer {
    let text = without_fence(text);
    let mut answer = HelpAnswer::default();
    let mut rest = text.as_str();
    while let Some((before, after)) = rest.split_once("[[") {
        answer.markdown.push_str(before);
        let Some((identifier, left)) = after.split_once("]]") else {
            // Its end has not come yet.
            rest = "";
            break;
        };
        if let Some(card) = guide.card(identifier.trim()) {
            let _ = match guide.app {
                App::Desktop => write!(answer.markdown, "[{}]({SHOW_ME_LINK}{})", card.title, card.id.0),
                App::Extension => write!(answer.markdown, "**{}**", card.title),
            };
            if !answer.named.contains(&card.id) {
                answer.named.push(card.id);
            }
        } else {
            // A feature the model made up: its name goes, the words around it stay, with one space between.
            if answer.markdown.ends_with(' ') && left.starts_with([' ', '.', ',', ';', ':', ')']) {
                let _space = answer.markdown.pop();
            }
        }
        rest = left;
    }
    answer.markdown.push_str(rest);
    answer.markdown = answer.markdown.trim_end().to_owned();
    answer
}

/// The answer without the code fence the model put around all of it, as `commit.draft-message` takes it off; while it
/// comes, an opening fence with no closing one yet is taken off too.
fn without_fence(text: &str) -> String {
    let cleaned = clean_draft(text);
    match cleaned.strip_prefix("```") {
        Some(rest) if !rest.contains("```") => rest.split_once('\n').map_or("", |(_, after)| after).to_owned(),
        _ => cleaned,
    }
}

#[cfg(test)]
mod tests;
