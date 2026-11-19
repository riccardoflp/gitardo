//! `cargo xtask guide`: the guide of Ask Gitardo (`help.guide`), made from the cards of `docs/features/` into
//! `crates/model/src/help/guide.txt`, which the engine holds; and the check that the file is what the catalog makes.
//!
//! For each app, the cards Built or Seen there, each with its title, its place, what **Show me** does with it, its first
//! paragraph and its entry points, when it is offered and what it refuses, or its **Extension** line; the cards *Host
//! only* in the extension by their title alone; and the names of the places of each app. Links, the names of functions
//! and the numbers of decisions and questions are taken out of what is copied.

mod clean;
mod places;

use std::io::Write;
use std::path::Path;

use crate::catalog::{IndexRow, parse_index};
use crate::check::Workspace;
use crate::error::Error;
use crate::problem::Problem;
use clean::clean;

/// Where the guide is written, from the root of the workspace.
pub const GUIDE: &str = "crates/model/src/help/guide.txt";

/// The index of the catalog, from the root of the repository.
const INDEX: &str = "docs/features/README.md";

/// The file of the cards of Ask Gitardo itself, which are left out of the guide.
const HELP_FILE: &str = "docs/features/20-help.md";

/// What the check says when the file differs.
const STALE: &str = "The guide of Ask Gitardo is not what the catalog makes: run cargo xtask guide.";

/// A card as the guide copies it.
#[derive(Debug, Default)]
struct CardText {
    /// Its identifier.
    identifier: String,
    /// Its title.
    title: String,
    /// Its kind: `Operation`, `View`, `Read` or `Derivation`.
    kind: String,
    /// Its first paragraph.
    paragraph: String,
    /// Its lines that start with a bold name, by that name: `Entry points`, `Extension`.
    lines: Vec<(String, String)>,
}

impl CardText {
    /// The line of a bold name, when the card has one.
    fn line(&self, name: &str) -> Option<&str> {
        self.lines
            .iter()
            .find(|(found, _)| found == name)
            .map(|(_, text)| text.as_str())
    }
}

/// Makes the guide and writes it.
///
/// # Errors
///
/// When the workspace cannot be read, or the file cannot be written.
pub fn run(root: &Path) -> Result<bool, Error> {
    let workspace = Workspace::read(root)?;
    let made = make(&workspace);
    let path = root.join(GUIDE);
    std::fs::write(&path, made).map_err(|source| Error::Read { path, source })?;
    writeln!(std::io::stdout().lock(), "Wrote {GUIDE}.").map_err(Error::Output)?;
    Ok(true)
}

/// Whether the file of the guide is what the catalog makes now.
pub fn check(workspace: &Workspace) -> Vec<Problem> {
    let written = text_of(workspace, GUIDE).unwrap_or_default();
    if written == make(workspace) {
        Vec::new()
    } else {
        vec![Problem::in_file("R2.1", GUIDE, String::from(STALE))]
    }
}

/// The text of a file of the workspace or of the repository.
fn text_of<'a>(workspace: &'a Workspace, relative: &str) -> Option<&'a str> {
    workspace
        .files
        .iter()
        .find(|(file, _)| file.relative == relative)
        .and_then(|(_, text)| text.as_deref())
}

/// The guide the catalog makes.
pub fn make(workspace: &Workspace) -> String {
    let index = parse_index(text_of(workspace, INDEX).unwrap_or_default());
    let mut areas: Vec<(&str, &str)> = workspace
        .files
        .iter()
        .filter(|(file, _)| is_area(&file.relative))
        .filter_map(|(file, text)| Some((file.relative.as_str(), text.as_deref()?)))
        .collect();
    areas.sort_unstable();
    let cards: Vec<(&str, CardText)> = areas
        .iter()
        .flat_map(|(file, text)| parse(text).into_iter().map(move |card| (*file, card)))
        .collect();
    let mut guide = String::from(
        "The guide of Ask Gitardo, made by cargo xtask guide from the cards of docs/features: do not edit it.\n",
    );
    guide.push_str("@places desktop\n");
    guide.push_str(&places::desktop(
        text_of(workspace, "docs/desktop/01-product.md").unwrap_or_default(),
    ));
    guide.push_str("@places extension\n");
    guide.push_str(&places::extension(
        text_of(workspace, "docs/extension/01-product.md").unwrap_or_default(),
    ));
    for app in ["desktop", "extension"] {
        for (file, card) in &cards {
            let Some(row) = index.iter().find(|row| row.identifier == card.identifier) else {
                continue;
            };
            if let Some(entry) = entry(app, file, card, row) {
                guide.push_str(&entry);
            }
        }
    }
    guide
}

/// Whether a file is an area of the catalog, other than Ask Gitardo's own.
fn is_area(relative: &str) -> bool {
    relative
        .strip_prefix("docs/features/")
        .is_some_and(|name| !name.contains('/') && name.as_bytes().first().is_some_and(u8::is_ascii_digit))
        && relative != HELP_FILE
        && crate::files::has_extension(relative, "md")
}

/// The entry of a card in the guide of an app, when the app has it.
fn entry(app: &str, file: &str, card: &CardText, row: &IndexRow) -> Option<String> {
    let status = if app == "desktop" {
        row.status.as_str()
    } else {
        row.extension.as_deref().unwrap_or_default()
    };
    let is_host_only = app == "extension" && status == "Host only";
    if !matches!(status, "Built" | "Seen") && !is_host_only {
        return None;
    }
    let place = places::of_area(file);
    let mut entry = format!(
        "@card {app} {}\ntitle: {}\nplace: {place}\n",
        card.identifier,
        clean(&card.title)
    );
    if is_host_only {
        entry.push_str("show: none\nDone by VS Code itself.\n");
        return Some(entry);
    }
    entry.push_str("show: ");
    entry.push_str(show(card));
    entry.push('\n');
    let mut body = vec![clean(&card.paragraph)];
    let extension = card.line("Extension").filter(|_| app == "extension");
    match extension {
        Some(line) => body.push(format!("In VS Code: {}", clean(line))),
        None => {
            for name in ["Entry points", "Offered when", "Refuses"] {
                if let Some(line) = card.line(name) {
                    body.push(format!("{name}: {}", clean(line)));
                }
            }
        }
    }
    for line in body.iter().filter(|line| !line.is_empty()) {
        entry.push_str(line);
        entry.push('\n');
    }
    Some(entry)
}

/// What **Show me** does with a card: an operation Quick search runs, a page opened, its place shown, or nothing.
fn show(card: &CardText) -> &'static str {
    let entry_points = card.line("Entry points").unwrap_or_default();
    let is_page =
        card.identifier.rsplit_once('.').is_some_and(|(_, last)| last == "page") || card.title.ends_with(" page");
    match card.kind.as_str() {
        "Operation" if entry_points.contains("Quick search") => "quick search",
        "Operation" => "show where",
        "View" if is_page => "page",
        _ => "none",
    }
}

/// Where the reading of a card is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reading {
    /// Before its first paragraph.
    Before,
    /// In its first paragraph.
    Paragraph,
    /// In its lines with a bold name, the last of which a line indented by two spaces goes on.
    Lines,
    /// After them, or outside any card: nothing more is copied.
    Done,
}

/// The cards of a file of the catalog, with the text the guide copies.
fn parse(text: &str) -> Vec<CardText> {
    let mut cards: Vec<CardText> = Vec::new();
    let mut reading = Reading::Done;
    for line in text.lines() {
        if let Some(heading) = line.strip_prefix("### `") {
            let (identifier, title) = heading.split_once('`').unwrap_or((heading, ""));
            let title = title.trim().trim_start_matches('—').trim();
            cards.push(CardText {
                identifier: identifier.to_owned(),
                title: title.to_owned(),
                ..CardText::default()
            });
            reading = Reading::Before;
        } else if line.starts_with("## ") {
            reading = Reading::Done;
        } else if let Some(card) = cards.last_mut() {
            reading = read_line(card, line, reading);
        }
    }
    cards
}

/// Reads one line of a card into it, and gives where the reading is after it.
fn read_line(card: &mut CardText, line: &str, reading: Reading) -> Reading {
    if reading == Reading::Done {
        return reading;
    }
    if let Some(rest) = line.strip_prefix("- **") {
        if let Some((name, text)) = rest.split_once("**:") {
            if name == "Kind" {
                text.trim().clone_into(&mut card.kind);
            }
            card.lines.push((name.to_owned(), text.trim().to_owned()));
        }
        return Reading::Lines;
    }
    let continued = line.strip_prefix("  ").map(str::trim).filter(|text| !text.is_empty());
    match (reading, continued) {
        (Reading::Lines, Some(continued)) => {
            if let Some((_, text)) = card.lines.last_mut() {
                text.push(' ');
                text.push_str(continued);
            }
            Reading::Lines
        }
        (Reading::Lines, None) => Reading::Done,
        (_, _) if line.trim().is_empty() => match reading {
            Reading::Paragraph => Reading::Lines,
            other => other,
        },
        (Reading::Before | Reading::Paragraph, _) => {
            if !card.paragraph.is_empty() {
                card.paragraph.push(' ');
            }
            card.paragraph.push_str(line.trim());
            Reading::Paragraph
        }
        (Reading::Done, _) => Reading::Done,
    }
}

#[cfg(test)]
mod tests;
