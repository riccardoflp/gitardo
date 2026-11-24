//! The catalog of features as the checks read it: the cards of `docs/features/`, its index, and the functions and
//! tests of the code that name them.

/// A card of the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    /// Its identifier: `branch.create`.
    pub identifier: String,
    /// Its kind, as the card says: `Operation`, `Read`, `View` or `Derivation`.
    pub kind: String,
    /// The path of its function, as the card names it: `features::branch::create`.
    pub function: String,
    /// How many lines its acceptance list has.
    pub acceptance: usize,
    /// The file of the card, from the root of the workspace.
    pub file: String,
    /// The line of its heading.
    pub line: usize,
}

/// A row of the index of the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexRow {
    /// The identifier of the card.
    pub identifier: String,
    /// The kind the index gives.
    pub kind: String,
    /// The status in Gitardo Desktop: `Specified`, `Reviewed`, `Built` or `Seen`.
    pub status: String,
    /// The status in the extension, from the column after the desktop's: `Planned`, `Built`, `Seen`, `Differs (A5)`
    /// or `Host only`; none when the index has no such column.
    pub extension: Option<String>,
    /// The line of the row.
    pub line: usize,
}

/// A function of the code that says, in its doc comment, which feature it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureFunction {
    /// The identifier after `/// Feature:`.
    pub identifier: String,
    /// The name of the function that follows the doc comment, if one does.
    pub name: Option<String>,
    /// The file, from the root of the workspace.
    pub file: String,
    /// The line of `/// Feature:`.
    pub line: usize,
}

/// The cards of one file of the catalog.
pub fn parse_cards(text: &str, file: &str) -> Vec<Card> {
    let mut cards: Vec<Card> = Vec::new();
    let mut in_acceptance = false;
    for (index, line) in text.lines().enumerate() {
        if let Some(identifier) = line.strip_prefix("### `").and_then(|rest| rest.split('`').next()) {
            let card = Card {
                identifier: identifier.to_owned(),
                kind: String::new(),
                function: String::new(),
                acceptance: 0,
                file: file.to_owned(),
                line: index + 1,
            };
            cards.push(card);
            in_acceptance = false;
        } else if line.starts_with("## ") || line.starts_with("### ") {
            in_acceptance = false;
        } else if let Some(card) = cards.last_mut() {
            in_acceptance = read_card_line(card, line, in_acceptance);
        }
    }
    cards
}

/// Takes what a line of a card says into the card; returns whether the acceptance list goes on after it.
fn read_card_line(card: &mut Card, line: &str, in_acceptance: bool) -> bool {
    if let Some(kind) = line.strip_prefix("- **Kind**: ") {
        // A kind may be followed by a note in brackets: `Operation (of the engine; …)`.
        kind.split_whitespace()
            .next()
            .unwrap_or_default()
            .clone_into(&mut card.kind);
    } else if let Some(function) = line.strip_prefix("- **Function**: `") {
        let path = function.split(['(', '`']).next().unwrap_or_default();
        path.trim().clone_into(&mut card.function);
    } else if line.trim() == "Acceptance:" {
        return true;
    } else if in_acceptance && (line.starts_with("- [ ]") || line.starts_with("- [x]")) {
        // A line of the extension is tested by the extension's tests, not by the workspace's (rule R2.6).
        if !line.get(5..).unwrap_or_default().trim_start().starts_with("Extension:") {
            card.acceptance += 1;
        }
    } else if !(line.trim().is_empty() || line.starts_with(char::is_whitespace)) {
        // A line that is neither blank nor the continuation of an item ends the list.
        return false;
    }
    in_acceptance
}

/// The rows of the index of the catalog.
pub fn parse_index(text: &str) -> Vec<IndexRow> {
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let Some(rest) = line.strip_prefix("| [`") else {
            continue;
        };
        let identifier = rest.split('`').next().unwrap_or_default();
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        let (Some(kind), Some(status)) = (cells.get(3), cells.get(4)) else {
            continue;
        };
        rows.push(IndexRow {
            identifier: identifier.to_owned(),
            kind: (*kind).to_owned(),
            status: (*status).to_owned(),
            extension: cells
                .get(5)
                .filter(|cell| !cell.is_empty())
                .map(|cell| (*cell).to_owned()),
            line: index + 1,
        });
    }
    rows
}

/// The functions of a Rust file that carry `/// Feature: <identifier>`, with the name of the function after it.
pub fn parse_feature_functions(source: &str, file: &str) -> Vec<FeatureFunction> {
    let lines: Vec<&str> = source.lines().collect();
    let mut found = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some(identifier) = line.trim_start().strip_prefix("/// Feature:") else {
            continue;
        };
        found.push(FeatureFunction {
            identifier: identifier.trim().trim_end_matches('.').to_owned(),
            name: declaration_after(lines.iter().skip(index + 1).copied()).and_then(function_name),
            file: file.to_owned(),
            line: index + 1,
        });
    }
    found
}

/// The first line after a doc comment that is neither the comment nor an attribute, also one written on several
/// lines such as an `#[allow(…)]` with a reason.
fn declaration_after<'a>(lines: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let mut open_brackets = 0_usize;
    for line in lines.map(str::trim_start) {
        if open_brackets > 0 || line.starts_with("#[") {
            open_brackets = (open_brackets + line.matches('[').count()).saturating_sub(line.matches(']').count());
        } else if !(line.starts_with("///") || line.is_empty()) {
            return Some(line);
        }
    }
    None
}

/// The name of the function a line declares: `pub fn create(…)` is `create`.
fn function_name(line: &str) -> Option<String> {
    let mut words = line.split(|character: char| !(character.is_alphanumeric() || character == '_'));
    let _keyword = words.by_ref().find(|word| *word == "fn")?;
    words.find(|word| !word.is_empty()).map(str::to_owned)
}

/// What the names of the tests of a feature start with: `branch.create` gives `branch_create`, followed in each test
/// name by `__`.
pub fn test_prefix(identifier: &str) -> String {
    identifier.replace(['.', '-'], "_")
}

/// The names of the functions of some Rust code that have `__` in their name, which is how the tests of a feature are
/// named. The code is given without its comments and literals.
pub fn feature_test_names(code: &str) -> Vec<(usize, String)> {
    let words = crate::source::words(code);
    words
        .windows(2)
        .filter_map(|pair| match pair {
            [(_, "fn"), (line, name)] if name.contains("__") => Some((*line, (*name).to_owned())),
            _ => None,
        })
        .collect()
}

/// What a **Tested by** of a document names: a file of tests, and the names of tests in it that follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestedBy {
    /// The document, from the root of the repository.
    pub document: String,
    /// The line of the words **Tested by**.
    pub line: usize,
    /// The file as the document writes it: `test/…` and `src/…` from the extension's folder, `rust/…` from the root
    /// of the repository.
    pub file: String,
    /// The names given after the file, each the whole name of a test or of a step, or the start of several.
    pub names: Vec<String>,
}

impl TestedBy {
    /// Where the file is, from the root of the repository.
    pub fn path(&self) -> String {
        if self.file.starts_with("rust/") {
            self.file.clone()
        } else {
            format!("extension/{}", self.file)
        }
    }
}

/// What each **Tested by** of a document names. The passage runs from the words to the end of their sentence, over
/// the lines that go on the same item or paragraph; a file is a path of tests in backticks, and a name is in double
/// quotes or in italics after its file. Words between them, such as "the steps of the blame", name nothing a program
/// can check. `docs/catalog.py` reads them the same way.
pub fn parse_tested_by(text: &str, document: &str) -> Vec<TestedBy> {
    let lines: Vec<&str> = text.lines().collect();
    let mut found = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some(start) = line.find("Tested by") else {
            continue;
        };
        let mut passage = line.get(start + "Tested by".len()..).unwrap_or_default().to_owned();
        for following in lines.iter().skip(index + 1) {
            let trimmed = following.trim_start();
            if trimmed.is_empty()
                || ["- ", "#", "|"].iter().any(|mark| trimmed.starts_with(mark))
                || following.starts_with("```")
            {
                break;
            }
            passage.push(' ');
            passage.push_str(following.trim());
        }
        for (file, names) in named_in(&passage) {
            found.push(TestedBy {
                document: document.to_owned(),
                line: index + 1,
                file,
                names,
            });
        }
    }
    found
}

/// The files and names of one passage, up to the end of its sentence.
fn named_in(passage: &str) -> Vec<(String, Vec<String>)> {
    let characters: Vec<char> = passage.chars().collect();
    let mut files: Vec<(String, Vec<String>)> = Vec::new();
    let mut index = 0;
    while let Some(&character) = characters.get(index) {
        if character == '*' && characters.get(index + 1) == Some(&'*') {
            index += 2;
            continue;
        }
        if matches!(character, '`' | '"' | '*') {
            let Some(length) = characters.iter().skip(index + 1).position(|other| *other == character) else {
                break;
            };
            let inside: String = characters.iter().skip(index + 1).take(length).collect();
            index += length + 2;
            if character == '`' {
                if is_test_file(&inside) {
                    files.push((inside, Vec::new()));
                }
            } else {
                let name = inside.split_whitespace().collect::<Vec<_>>().join(" ");
                if let Some((_, names)) = files.last_mut().filter(|_| !name.is_empty() && name != "none") {
                    names.push(name);
                }
            }
            continue;
        }
        if character == '.' && characters.get(index + 1).is_none_or(|next| *next == ' ') {
            break;
        }
        index += 1;
    }
    files
}

/// Whether a span in backticks is a file of tests: `test/kept.test.ts`, `rust/crates/server/tests/stdio/summaries.rs`.
fn is_test_file(span: &str) -> bool {
    let rooted = ["test/", "src/", "rust/"].iter().any(|root| span.starts_with(root));
    let typed = [".ts", ".tsx", ".mjs", ".js", ".rs"]
        .iter()
        .any(|extension| span.ends_with(extension));
    let plain = span
        .chars()
        .all(|character| character.is_alphanumeric() || matches!(character, '/' | '.' | '_' | '-'));
    rooted && typed && plain
}

/// Whether some code has a string that starts with a name, as the title of a test or of a step is written there.
pub fn names_test(code: &str, name: &str) -> bool {
    let flat = code
        .replace("\\'", "'")
        .replace("\\\"", "\"")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    ['\'', '"', '`']
        .iter()
        .any(|quote| flat.contains(&format!("{quote}{name}")))
}

#[cfg(test)]
mod tests;
