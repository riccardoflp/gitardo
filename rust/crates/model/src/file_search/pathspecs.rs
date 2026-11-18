//! **Include**, **Exclude** and **Hidden** of the Search page, as the pathspecs git is given.

/// The most patterns one pattern with braces is read as; the alternatives after them are left out.
const MOST_ALTERNATIVES: usize = 64;

/// What narrows a search to some files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Filters<'a> {
    /// The patterns of the files searched, separated by commas; empty for all of them.
    pub include: &'a str,
    /// The patterns of the files left out.
    pub exclude: &'a str,
    /// Whether the files and the folders whose name starts with a dot are searched too.
    pub hidden: bool,
}

/// The pathspecs that make git list and search the files the filters ask for: those of **Include**, then those of
/// **Exclude**, then, with **Hidden** off, the two that leave out every name that starts with a dot. They are read
/// from the folder git is run in.
///
/// Feature: file-search.pathspecs
#[must_use]
pub fn pathspecs(filters: &Filters) -> Vec<String> {
    let mut found = Vec::new();
    for pattern in patterns(filters.include) {
        found.extend(globs(&pattern).into_iter().map(|glob| format!(":(glob){glob}")));
    }
    for pattern in patterns(filters.exclude) {
        found.extend(globs(&pattern).into_iter().map(|glob| format!(":(exclude,glob){glob}")));
    }
    if !filters.hidden {
        found.push(String::from(":(exclude,glob)**/.*"));
        found.push(String::from(":(exclude,glob)**/.*/**"));
    }
    found
}

/// The patterns of a box: cut at the commas outside braces, each alternative in braces a pattern of its own.
fn patterns(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for part in split_outside_braces(text, ',') {
        let part = part.trim().replace('\\', "/");
        if part.is_empty() {
            continue;
        }
        let mut alternatives = Vec::new();
        expand(&part, &mut alternatives);
        found.extend(alternatives);
    }
    found
}

/// A text cut at a separator, where it is not inside braces.
fn split_outside_braces(text: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0_usize;
    let mut start = 0;
    for (at, letter) in text.char_indices() {
        match letter {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ if letter == separator && depth == 0 => {
                parts.extend(text.get(start..at));
                start = at + letter.len_utf8();
            }
            _ => {}
        }
    }
    parts.extend(text.get(start..));
    parts
}

/// Adds a pattern to those found, once for each alternative of its braces: git has no braces in its globs.
fn expand(pattern: &str, found: &mut Vec<String>) {
    if found.len() >= MOST_ALTERNATIVES {
        return;
    }
    let Some((before, inside, after)) = first_braces(pattern) else {
        found.push(pattern.to_owned());
        return;
    };
    for alternative in split_outside_braces(inside, ',') {
        expand(&format!("{before}{alternative}{after}"), found);
    }
}

/// The first braces of a pattern that close: what is before them, in them and after them.
fn first_braces(pattern: &str) -> Option<(&str, &str, &str)> {
    let open = pattern.find('{')?;
    let mut depth = 0_usize;
    for (at, letter) in pattern.char_indices().skip_while(|(at, _)| *at < open) {
        match letter {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if depth == 0 {
            return Some((pattern.get(..open)?, pattern.get(open + 1..at)?, pattern.get(at + 1..)?));
        }
    }
    None
}

/// The globs of git for one pattern: a name is looked for anywhere, a path from the top of what is searched; a pattern
/// that ends with `/` finds folders only, and any other a file of that name or what a folder of that name holds.
fn globs(pattern: &str) -> Vec<String> {
    let folders_only = pattern.ends_with('/');
    let pattern = pattern.trim_end_matches('/');
    let (from_top, pattern) = match pattern.strip_prefix("./").or_else(|| pattern.strip_prefix('/')) {
        Some(rest) => (true, rest.trim_start_matches('/')),
        None => (pattern.contains('/'), pattern),
    };
    if pattern.is_empty() || pattern == "." {
        return Vec::new();
    }
    let anywhere = if from_top { "" } else { "**/" };
    let inside = format!("{anywhere}{pattern}/**");
    if folders_only {
        vec![inside]
    } else {
        vec![format!("{anywhere}{pattern}"), inside]
    }
}
