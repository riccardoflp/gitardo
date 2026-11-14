//! How the buttons of the page change the patterns, by the pattern of a path or of a name, keeping the patterns that
//! take more than the line (`worktree-setup.choose`), and what a line gets as the page says it.

use super::{Gets, SetupPreview};

/// The link and copy patterns of the page.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SetupPatterns {
    /// `worktrees.linkFolders`.
    pub link: Vec<String>,
    /// `worktrees.copyFiles`.
    pub copy: Vec<String>,
}

/// The line of the page a choice is made on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChosenLine<'a> {
    /// Something in one place, chosen by its own pattern (`worktree-setup.propose`): `node_modules/`, `/.env`.
    One {
        /// Its path from the top, with `/`.
        path: &'a str,
        /// Whether it is a folder.
        is_folder: bool,
        /// The pattern it is chosen by.
        pattern: &'a str,
    },
    /// A name in several places, chosen for all of them by the pattern of the name.
    Name {
        /// The paths of its places.
        places: &'a [String],
        /// Whether they are folders.
        is_folder: bool,
    },
    /// One place of a name in several places, chosen for that one alone by the pattern of its path.
    Place {
        /// Its path.
        path: &'a str,
        /// Whether it is a folder.
        is_folder: bool,
        /// The paths of the other places of the name.
        others: &'a [String],
    },
}

/// What [`choose`] looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChooseInput<'a> {
    /// The patterns of the page.
    pub patterns: &'a SetupPatterns,
    /// The line chosen on.
    pub line: ChosenLine<'a>,
    /// What it is to get.
    pub gets: Gets,
}

/// The patterns once a line is chosen to get something: the patterns of its path and of its name are taken out of
/// both lists, and its pattern put in the list of what it gets; a pattern that takes more than it, such as `*.local`,
/// is left. A place of a name the name's pattern gave something else makes that pattern give way to a pattern of the
/// path of each other place, so that they keep what they had.
///
/// Feature: worktree-setup.choose
#[must_use]
pub fn choose(input: &ChooseInput<'_>) -> SetupPatterns {
    let mut patterns = input.patterns.clone();
    match input.line {
        ChosenLine::One {
            path,
            is_folder,
            pattern,
        } => {
            let mut own = path_patterns(path, is_folder);
            own.extend(name_patterns(path, is_folder));
            take_out(&mut patterns, &own);
            put(&mut patterns, input.gets, pattern);
        }
        ChosenLine::Name { places, is_folder } => {
            let Some(first) = places.first() else {
                return patterns;
            };
            let mut own = name_patterns(first, is_folder);
            for place in places {
                own.extend(path_patterns(place, is_folder));
            }
            take_out(&mut patterns, &own);
            put(&mut patterns, input.gets, &name_pattern(first, is_folder));
        }
        ChosenLine::Place {
            path,
            is_folder,
            others,
        } => choose_place(&mut patterns, (path, is_folder, others), input.gets),
    }
    patterns
}

/// Chooses for one place of a name in several places.
fn choose_place(patterns: &mut SetupPatterns, (path, is_folder, others): (&str, bool, &[String]), gets: Gets) {
    let names = name_patterns(path, is_folder);
    let by_name = gets_of(patterns, &names);
    if by_name == Some(gets) {
        take_out(patterns, &path_patterns(path, is_folder));
        return;
    }
    if let Some(other) = by_name {
        take_out(patterns, &names);
        for place in others {
            let theirs = path_patterns(place, is_folder);
            if gets_of(patterns, &theirs).is_none() {
                put(patterns, other, &own_path_pattern(place, is_folder));
            }
        }
    }
    take_out(patterns, &path_patterns(path, is_folder));
    put(patterns, gets, &own_path_pattern(path, is_folder));
}

/// A line with what is proposed for it, for [`use_proposed`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProposedLine<'a> {
    /// The line: a name in several places, or something in one place.
    pub line: ChosenLine<'a>,
    /// What is proposed for it.
    pub proposed: Gets,
}

/// **Use what is proposed**: each name in several places gets its proposal by the pattern of the name, and each other
/// line its proposal by its own pattern.
#[must_use]
pub fn use_proposed(patterns: &SetupPatterns, lines: &[ProposedLine<'_>]) -> SetupPatterns {
    lines.iter().fold(patterns.clone(), |patterns, line| {
        choose(&ChooseInput {
            patterns: &patterns,
            line: line.line,
            gets: line.proposed,
        })
    })
}

/// What [`line_gets`] looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetsInput<'a> {
    /// The path of the line.
    pub path: &'a str,
    /// Whether it is a folder.
    pub is_folder: bool,
    /// The patterns of the page now.
    pub patterns: &'a SetupPatterns,
    /// The patterns when git was last asked what they take.
    pub looked: &'a SetupPatterns,
    /// What git said they take then.
    pub preview: &'a SetupPreview,
}

/// What a line gets: from its own patterns when it has one, and otherwise from what git said the patterns take when
/// it was last asked; a pattern of its own taken out since leaves it nothing.
#[must_use]
pub fn line_gets(input: &GetsInput<'_>) -> Gets {
    let mut own = path_patterns(input.path, input.is_folder);
    own.extend(name_patterns(input.path, input.is_folder));
    if let Some(gets) = gets_of(input.patterns, &own) {
        return gets;
    }
    if gets_of(input.looked, &own).is_some() {
        return Gets::Nothing;
    }
    let takes = |paths: &[String]| paths.iter().any(|taken| is_same_or_inside(input.path, taken));
    if takes(&input.preview.linked) {
        Gets::Link
    } else if takes(&input.preview.copied) {
        Gets::Copy
    } else {
        Gets::Nothing
    }
}

/// Whether a path is a folder taken, or inside one.
fn is_same_or_inside(path: &str, taken: &str) -> bool {
    path == taken || path.strip_prefix(taken).is_some_and(|rest| rest.starts_with('/'))
}

/// The patterns of a path: `/packages/api/.env`; for a folder also with a `/` after it.
#[must_use]
pub fn path_patterns(path: &str, is_folder: bool) -> Vec<String> {
    let path = path.trim_matches('/');
    let mut patterns = vec![format!("/{path}")];
    if is_folder {
        patterns.push(format!("/{path}/"));
    }
    if path.contains('/') {
        // A pattern with a `/` in its middle is from the top already, with or without one before it.
        patterns.push(path.to_owned());
        if is_folder {
            patterns.push(format!("{path}/"));
        }
    }
    patterns
}

/// The pattern of the path of a line: `/packages/api/node_modules/`, `/.env`.
#[must_use]
pub fn own_path_pattern(path: &str, is_folder: bool) -> String {
    let path = path.trim_matches('/');
    if is_folder {
        format!("/{path}/")
    } else {
        format!("/{path}")
    }
}

/// The patterns of the name of a path: `.env`; for a folder also `build/`.
fn name_patterns(path: &str, is_folder: bool) -> Vec<String> {
    let name = path.trim_matches('/').rsplit('/').next().unwrap_or(path).to_owned();
    if is_folder {
        vec![name.clone(), format!("{name}/")]
    } else {
        vec![name]
    }
}

/// The pattern of a name in several places: `node_modules/`, `.env.local`.
#[must_use]
pub fn name_pattern(path: &str, is_folder: bool) -> String {
    let name = path.trim_matches('/').rsplit('/').next().unwrap_or(path);
    if is_folder { format!("{name}/") } else { name.to_owned() }
}

/// What some patterns give, by the list one of them is in: the folders to link first.
fn gets_of(patterns: &SetupPatterns, own: &[String]) -> Option<Gets> {
    let has = |list: &[String]| list.iter().any(|pattern| own.contains(pattern));
    if has(&patterns.link) {
        Some(Gets::Link)
    } else if has(&patterns.copy) {
        Some(Gets::Copy)
    } else {
        None
    }
}

/// Takes patterns out of both lists.
fn take_out(patterns: &mut SetupPatterns, own: &[String]) {
    patterns.link.retain(|pattern| !own.contains(pattern));
    patterns.copy.retain(|pattern| !own.contains(pattern));
}

/// Puts a pattern at the end of the list of what is chosen, once; nothing for **Nothing**.
fn put(patterns: &mut SetupPatterns, gets: Gets, pattern: &str) {
    let list = match gets {
        Gets::Link => &mut patterns.link,
        Gets::Copy => &mut patterns.copy,
        Gets::Nothing => return,
    };
    if !list.iter().any(|known| known == pattern) {
        list.push(pattern.to_owned());
    }
}

#[cfg(test)]
mod tests;
