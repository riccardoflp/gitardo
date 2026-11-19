//! The guide of the features (`help.guide`): the file `guide.txt`, which `cargo xtask guide` makes from the cards of the
//! catalog, held in the program and read into the cards of the app that asks.

use crate::contract::FeatureId;

/// The guide as `cargo xtask guide` wrote it.
const GUIDE: &str = include_str!("guide.txt");

/// The app a question is asked in, whose features and places the guide gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum App {
    /// Gitardo Desktop.
    Desktop,
    /// The extension for VS Code.
    Extension,
}

impl App {
    /// The word of the app in the file of the guide.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Extension => "extension",
        }
    }
}

/// What **Show me** does with a feature, from its place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Show {
    /// An operation Quick search runs: the button runs it as Quick search does.
    QuickSearch,
    /// A page: the button opens it.
    Page,
    /// Something done on a commit, a branch, a file or a stash: the button shows its place.
    ShowWhere,
    /// Anything else: no button.
    Nothing,
}

impl Show {
    /// The word of the guide for it; anything the guide does not know is nothing.
    fn from_word(word: &str) -> Self {
        match word {
            "quick search" => Self::QuickSearch,
            "page" => Self::Page,
            "show where" => Self::ShowWhere,
            _ => Self::Nothing,
        }
    }
}

/// A feature of the guide of an app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuideCard {
    /// The identifier of its card.
    pub id: FeatureId,
    /// The title of its card.
    pub title: &'static str,
    /// The place it is in: the timeline, the branch rail, the Files page.
    pub place: &'static str,
    /// What **Show me** does with it.
    pub show: Show,
    /// What its card says of it, one line for its first paragraph and one for each of its entry points, what it is
    /// offered when and what it refuses, or its line for VS Code.
    pub text: &'static str,
}

impl GuideCard {
    /// The card as the model reads it among the features chosen: its identifier and title, its place, its text.
    #[must_use]
    pub fn described(&self) -> String {
        format!(
            "Feature {} ({})\nPlace: {}\n{}",
            self.id.0, self.title, self.place, self.text
        )
    }
}

/// The guide of an app: the names of its places and its features, in the order of the catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Guide {
    /// The app it is the guide of.
    pub app: App,
    /// The names of the places of the app and what each is, one a line.
    pub places: &'static str,
    /// Its features.
    pub cards: Vec<GuideCard>,
}

impl Guide {
    /// The index of the guide: the identifier and the title of each feature, one a line.
    #[must_use]
    pub fn index(&self) -> String {
        let mut index = String::new();
        for card in &self.cards {
            index.push_str(card.id.0);
            index.push(' ');
            index.push_str(card.title);
            index.push('\n');
        }
        index
    }

    /// The feature of an identifier, when the guide has it.
    #[must_use]
    pub fn card(&self, identifier: &str) -> Option<&GuideCard> {
        self.cards.iter().find(|card| card.id.0 == identifier)
    }
}

/// The guide of an app, read from the file held in the program. Reading it costs a scan of its lines and copies none
/// of them: each part of a card is a slice of the file.
///
/// Feature: help.guide
#[must_use]
pub fn guide(app: App) -> Guide {
    read_guide(GUIDE, app)
}

/// The guide of an app read from a text written as `cargo xtask guide` writes it.
fn read_guide(text: &'static str, app: App) -> Guide {
    let places_heading = format!("@places {}\n", app.word());
    let places = text.split_once(places_heading.as_str()).map_or("", |(_, after)| {
        after.split_once("\n@").map_or(after, |(places, _)| places)
    });
    let app_word = format!("{} ", app.word());
    let cards = text
        .split("\n@card ")
        .skip(1)
        .filter_map(|entry| entry.strip_prefix(app_word.as_str()))
        .filter_map(read_card)
        .collect();
    Guide {
        app,
        places: places.trim_end(),
        cards,
    }
}

/// A feature from its entry, after `@card <app> `: its identifier on the first line, then `title:`, `place:` and
/// `show:`, then its text.
fn read_card(entry: &'static str) -> Option<GuideCard> {
    let (identifier, rest) = entry.split_once('\n')?;
    let (title, rest) = rest.strip_prefix("title: ")?.split_once('\n')?;
    let (place, rest) = rest.strip_prefix("place: ")?.split_once('\n')?;
    let show_and_text = rest.strip_prefix("show: ")?;
    let (show, text) = show_and_text.split_once('\n').unwrap_or((show_and_text, ""));
    Some(GuideCard {
        id: FeatureId(identifier),
        title,
        place,
        show: Show::from_word(show),
        text: text.trim_end(),
    })
}

#[cfg(test)]
mod tests;
