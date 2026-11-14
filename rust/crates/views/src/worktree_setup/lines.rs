//! The lines of the table of the page: what git ignores in the checkout, each thing in the section
//! `worktree-setup.propose` puts it in, a name in several places as one line, and what each line gets now.

use std::collections::{BTreeMap, BTreeSet};

use model::worktree_setup::{
    ChosenLine, Gets, GetsInput, IgnoredEntry, IgnoredKind, ProposeInput, ProposedLine, Section, SetupPatterns,
    SetupPreview, line_gets, name_pattern, propose,
};

/// A line of the table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupLine<'a> {
    /// The places it stands for: one, or each place of a name in several places, in the order git lists them.
    pub places: Vec<&'a IgnoredEntry>,
    /// Whether it is a folder (or a link), not a file.
    pub is_folder: bool,
    /// The section it is in.
    pub section: Section,
    /// What is proposed for it.
    pub proposed: Gets,
    /// The pattern it is chosen by: its own, or the pattern of its name for a name in several places.
    pub pattern: String,
}

impl SetupLine<'_> {
    /// Whether it is a name in several places.
    #[must_use]
    pub fn is_name(&self) -> bool {
        self.places.len() > 1
    }

    /// What it says in the column of the paths: the path of its one place, or the pattern of the name.
    #[must_use]
    pub fn path(&self) -> &str {
        match self.places.as_slice() {
            [one] => &one.path,
            _ => &self.pattern,
        }
    }

    /// The paths of its places.
    #[must_use]
    pub fn paths(&self) -> Vec<String> {
        self.places.iter().map(|place| place.path.clone()).collect()
    }

    /// The line the buttons choose for, with the paths of its places.
    #[must_use]
    pub fn chosen<'line>(&'line self, paths: &'line [String]) -> ChosenLine<'line> {
        match self.places.as_slice() {
            [one] => ChosenLine::One {
                path: &one.path,
                is_folder: self.is_folder,
                pattern: &self.pattern,
            },
            _ => ChosenLine::Name {
                places: paths,
                is_folder: self.is_folder,
            },
        }
    }

    /// The bytes it takes on disk, what is counted of its folders included.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        self.places.iter().map(|place| bytes_of(place)).sum()
    }
}

/// The bytes of an entry: a file's, or what is counted of a folder; none for a link.
#[must_use]
pub fn bytes_of(entry: &IgnoredEntry) -> u64 {
    match &entry.kind {
        IgnoredKind::File { bytes } => *bytes,
        IgnoredKind::Folder(size) => size.bytes,
        IgnoredKind::Link => 0,
    }
}

/// Whether an entry is a folder, or a link, which is taken for one.
#[must_use]
pub fn is_folder(entry: &IgnoredEntry) -> bool {
    !matches!(entry.kind, IgnoredKind::File { .. })
}

/// The lines of what git ignores: a name in several places, such as `node_modules` in each package, as one line where
/// its first place is; each other thing on its own. Each is in the section `worktree-setup.propose` gives it.
#[must_use]
pub fn lines(entries: &[IgnoredEntry]) -> Vec<SetupLine<'_>> {
    let name_of = |entry: &IgnoredEntry| (name_pattern(&entry.path, is_folder(entry)), is_folder(entry));
    let mut places: BTreeMap<(String, bool), Vec<&IgnoredEntry>> = BTreeMap::new();
    for entry in entries {
        places.entry(name_of(entry)).or_default().push(entry);
    }
    let mut seen = BTreeSet::new();
    let mut found = Vec::new();
    for entry in entries {
        let key = name_of(entry);
        let of_name = places.get(&key).map_or(&[][..], Vec::as_slice);
        let proposal = propose(&ProposeInput {
            path: &entry.path,
            is_folder: key.1,
        });
        if of_name.len() < 2 {
            found.push(SetupLine {
                places: vec![entry],
                is_folder: key.1,
                section: proposal.section,
                proposed: proposal.proposed,
                pattern: proposal.pattern,
            });
        } else if seen.insert(key.clone()) {
            found.push(SetupLine {
                places: of_name.to_vec(),
                is_folder: key.1,
                section: proposal.section,
                proposed: proposal.proposed,
                pattern: key.0,
            });
        }
    }
    found
}

/// What the patterns of the page give each line, and what git said they take when it was last asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chosen<'a> {
    /// The patterns of the page now.
    pub patterns: &'a SetupPatterns,
    /// The patterns when git was last asked.
    pub looked: &'a SetupPatterns,
    /// What git said they take then.
    pub preview: &'a SetupPreview,
}

impl Chosen<'_> {
    /// What one place gets.
    #[must_use]
    pub fn place_gets(&self, entry: &IgnoredEntry) -> Gets {
        line_gets(&GetsInput {
            path: &entry.path,
            is_folder: is_folder(entry),
            patterns: self.patterns,
            looked: self.looked,
            preview: self.preview,
        })
    }

    /// What a line gets: what all its places get, none when they do not get the same.
    #[must_use]
    pub fn gets(&self, line: &SetupLine<'_>) -> Option<Gets> {
        let mut each = line.places.iter().map(|place| self.place_gets(place));
        let first = each.next()?;
        each.all(|gets| gets == first).then_some(first)
    }

    /// What git took of a folder whose line gets nothing: **linked** or **copied** when some path inside it is.
    #[must_use]
    pub fn part_taken(&self, line: &SetupLine<'_>) -> Option<Gets> {
        if !line.is_folder || self.gets(line) != Some(Gets::Nothing) {
            return None;
        }
        let inside = |taken: &[String]| {
            line.places.iter().any(|place| {
                taken
                    .iter()
                    .any(|path| path.strip_prefix(&place.path).is_some_and(|rest| rest.starts_with('/')))
            })
        };
        if inside(&self.preview.linked) {
            Some(Gets::Link)
        } else if inside(&self.preview.copied) {
            Some(Gets::Copy)
        } else {
            None
        }
    }
}

/// The lines with what is proposed for them, for **Use what is proposed**.
#[must_use]
pub fn proposed<'line>(lines: &'line [SetupLine<'_>], paths: &'line [Vec<String>]) -> Vec<ProposedLine<'line>> {
    lines
        .iter()
        .zip(paths)
        .map(|(line, paths)| ProposedLine {
            line: line.chosen(paths),
            proposed: line.proposed,
        })
        .collect()
}

/// Whether a section is open: as it was opened or closed by hand, otherwise open for what is not known, dependencies,
/// settings and secrets and the editor, and for any other where something is linked or copied.
#[must_use]
pub fn is_open(section: Section, toggled: &BTreeMap<Section, bool>, has_something: bool) -> bool {
    toggled.get(&section).copied().unwrap_or(
        has_something
            || matches!(
                section,
                Section::NotKnown | Section::Dependencies | Section::Secrets | Section::Editor
            ),
    )
}
