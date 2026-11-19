//! The patterns offered to ignore a file or a folder, the files each can be written in, and the line written there
//! (`changes.ignore-patterns`).

use std::path::PathBuf;

/// What the patterns are worked out from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoreInput {
    /// The path from the top of the repository, with `/`, without `/` at its end.
    pub path: String,
    /// Whether it is a folder.
    pub is_folder: bool,
    /// The exclude file of the repository (`git rev-parse --git-path info/exclude`), in full.
    pub exclude: PathBuf,
}

/// A pattern offered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnorePattern {
    /// What it ignores, in words: **This file**, **All `.log` files**.
    pub words: String,
    /// The pattern as the `.gitignore` of the repository would hold it, shown beside its words.
    pub shown: String,
    kind: PatternKind,
}

/// A file a line can go in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnorePlace {
    /// Where, in words: **The `.gitignore` of the repository**.
    pub words: String,
    /// The file: from the top of the repository with `/` for a `.gitignore`, in full for the exclude file.
    pub file: PathBuf,
    /// Whether it is the exclude file, which is not committed.
    pub is_exclude: bool,
    /// The folder its patterns are relative to, from the top, empty for the top.
    base: String,
}

/// The patterns and the places offered for a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoreChoices {
    /// The patterns, in order.
    pub patterns: Vec<IgnorePattern>,
    /// The places, in order.
    pub places: Vec<IgnorePlace>,
}

/// What a pattern names.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PatternKind {
    /// The path itself, and whether it is a folder.
    This(String, bool),
    /// Every file of an extension.
    Extension(String),
    /// The folder that holds the file.
    Holder(String),
    /// Every folder of a name.
    EveryFolder(String),
    /// A pattern typed, written as it is.
    Typed(String),
}

impl IgnorePattern {
    /// A pattern typed, written as it is in every place.
    #[must_use]
    pub fn typed(pattern: &str) -> IgnorePattern {
        IgnorePattern {
            words: pattern.to_owned(),
            shown: pattern.to_owned(),
            kind: PatternKind::Typed(pattern.to_owned()),
        }
    }

    /// The line this pattern is written as in a place, none when it cannot be written there: a path outside the
    /// folder of that `.gitignore`, or the folder of that `.gitignore` itself.
    #[must_use]
    pub fn line_in(&self, place: &IgnorePlace) -> Option<String> {
        match &self.kind {
            PatternKind::This(path, is_folder) => {
                let inside = from_base(&place.base, path).filter(|inside| !inside.is_empty())?;
                Some(format!("/{}{}", escaped(&inside), if *is_folder { "/" } else { "" }))
            }
            PatternKind::Extension(extension) => Some(format!("*.{}", escaped(extension))),
            PatternKind::Holder(folder) => {
                let inside = from_base(&place.base, folder).filter(|inside| !inside.is_empty())?;
                Some(format!("/{}/", escaped(&inside)))
            }
            PatternKind::EveryFolder(name) => Some(format!("{}/", escaped(name))),
            PatternKind::Typed(pattern) => Some(pattern.clone()),
        }
    }
}

/// The patterns that ignore a file or a folder, and the files each can be written in.
///
/// Feature: changes.ignore-patterns
#[must_use]
pub fn ignore_patterns(input: &IgnoreInput) -> IgnoreChoices {
    let path = input.path.trim_matches('/');
    let (holder, name) = path.rsplit_once('/').unwrap_or(("", path));
    let mut patterns = Vec::new();
    if input.is_folder {
        let this = PatternKind::This(path.to_owned(), true);
        patterns.push(pattern("This folder", format!("/{}/", escaped(path)), this));
        patterns.push(pattern(
            &format!("Every folder named {name}"),
            format!("{}/", escaped(name)),
            PatternKind::EveryFolder(name.to_owned()),
        ));
    } else {
        let this = PatternKind::This(path.to_owned(), false);
        patterns.push(pattern("This file", format!("/{}", escaped(path)), this));
        if let Some((_, extension)) = name.rsplit_once('.').filter(|(stem, _)| !stem.is_empty()) {
            let words = format!("All .{extension} files");
            patterns.push(pattern(
                &words,
                format!("*.{}", escaped(extension)),
                PatternKind::Extension(extension.to_owned()),
            ));
        }
        if !holder.is_empty() {
            let words = format!("The folder {holder}");
            patterns.push(pattern(
                &words,
                format!("/{}/", escaped(holder)),
                PatternKind::Holder(holder.to_owned()),
            ));
        }
    }
    let mut places = vec![place(
        "The .gitignore of the repository",
        PathBuf::from(".gitignore"),
        "",
    )];
    if !holder.is_empty() {
        let words = format!("The .gitignore of {holder}");
        places.push(place(&words, PathBuf::from(format!("{holder}/.gitignore")), holder));
    }
    places.push(IgnorePlace {
        words: String::from("Only on this computer"),
        file: input.exclude.clone(),
        is_exclude: true,
        base: String::new(),
    });
    IgnoreChoices { patterns, places }
}

/// A pattern offered.
fn pattern(words: &str, shown: String, kind: PatternKind) -> IgnorePattern {
    IgnorePattern {
        words: words.to_owned(),
        shown,
        kind,
    }
}

/// A `.gitignore` offered.
fn place(words: &str, file: PathBuf, base: &str) -> IgnorePlace {
    IgnorePlace {
        words: words.to_owned(),
        file,
        is_exclude: false,
        base: base.to_owned(),
    }
}

/// A path from the top, from a folder instead: none when it is not in it.
fn from_base(base: &str, path: &str) -> Option<String> {
    if base.is_empty() {
        return Some(path.to_owned());
    }
    if path == base {
        return Some(String::new());
    }
    path.strip_prefix(base)?.strip_prefix('/').map(str::to_owned)
}

/// A path written so that it names that path only: `*`, `?`, `[`, `]` and `\` with `\` before them, and a space at its end
/// too.
fn escaped(path: &str) -> String {
    let mut written = String::with_capacity(path.len());
    for character in path.chars() {
        if matches!(character, '*' | '?' | '[' | ']' | '\\') {
            written.push('\\');
        }
        written.push(character);
    }
    if written.ends_with(' ') {
        let _space = written.pop();
        written.push_str("\\ ");
    }
    written
}

/// The text of a file once a line is added at its end, on a line of its own with the line ending the file uses; none
/// when the file has the line already, once the spaces at the end of its lines are left aside. A file that is not
/// there is made with the line alone.
#[must_use]
pub fn with_line(text: Option<&str>, line: &str) -> Option<String> {
    let Some(text) = text else {
        return Some(format!("{line}\n"));
    };
    if text.lines().any(|existing| existing.trim_end() == line.trim_end()) {
        return None;
    }
    let ending = if text.lines().next().is_some() && text.split('\n').next().is_some_and(|first| first.ends_with('\r'))
    {
        "\r\n"
    } else {
        "\n"
    };
    let mut written = text.to_owned();
    if !written.is_empty() && !written.ends_with('\n') {
        written.push_str(ending);
    }
    written.push_str(line);
    written.push_str(ending);
    Some(written)
}

#[cfg(test)]
#[path = "ignore_tests.rs"]
mod tests;
