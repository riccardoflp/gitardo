//! Which names of files and of folders match what is typed in the Search page, the best first.

use std::cmp::Reverse;
use std::collections::BTreeSet;

/// What a term found together at the start of a word is worth, beside one found inside a word.
const WORD_START: i32 = 100;

/// How far from the start of what is read a term found together still counts for more than one further.
const NEAR_START: i32 = 50;

/// What a letter found apart at the start of a word is worth.
const LETTER_AT_WORD_START: i32 = 10;

/// What a letter found right after the one before it is worth.
const LETTER_AFTER_LETTER: i32 = 6;

/// The most the letters between those found take away.
const MOST_BETWEEN: i32 = 40;

/// What is typed, and how many names are wanted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RankRequest<'a> {
    /// What is typed: terms separated by spaces, each of which a name has to match.
    pub query: &'a str,
    /// Whether a term whose letters are found only apart along the whole path does not match, as in **All**.
    pub strict: bool,
    /// The most names to keep.
    pub most: usize,
}

/// A name that matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedName {
    /// Which of the names given it is.
    pub index: usize,
    /// Where the letters that matched start in it, in bytes, in order.
    pub marks: Vec<usize>,
}

/// The names that matched, the best first.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RankedNames {
    /// The names kept.
    pub names: Vec<RankedName>,
    /// How many matched in all.
    pub total: usize,
}

/// A letter of a path, as a term is matched against it.
#[derive(Debug, Clone, Copy)]
struct Letter {
    /// Where it starts in the path, in bytes.
    at: usize,
    /// The letter, small.
    folded: char,
    /// Whether it starts a word: the first of the path, one after `/`, `.`, `_`, `-` or a space, or a capital after a
    /// small letter.
    starts_word: bool,
}

/// How well a name matched, the best first when sorted.
type Rank = (Reverse<u8>, Reverse<u32>, Reverse<i32>, usize, usize);

/// The names that match what is typed, the best first, with the letters that matched: every term has to match, in the
/// name or else along the whole path, together or else in order.
///
/// Feature: file-search.rank-names
#[must_use]
pub fn rank_names(names: &[String], request: &RankRequest) -> RankedNames {
    let terms: Vec<Vec<char>> = request
        .query
        .split_whitespace()
        .map(|term| {
            term.chars()
                .map(|letter| fold(if letter == '\\' { '/' } else { letter }))
                .collect()
        })
        .collect();
    if terms.is_empty() {
        let kept = (0..names.len().min(request.most)).map(|index| RankedName {
            index,
            marks: Vec::new(),
        });
        return RankedNames {
            names: kept.collect(),
            total: names.len(),
        };
    }
    let mut matched: Vec<(Rank, RankedName)> = names
        .iter()
        .enumerate()
        .filter_map(|(index, name)| rank_of(name, index, &terms, request.strict))
        .collect();
    let total = matched.len();
    matched.sort_by_key(|(rank, _)| *rank);
    matched.truncate(request.most);
    RankedNames {
        names: matched.into_iter().map(|(_, name)| name).collect(),
        total,
    }
}

/// The folders of some files: every folder that holds one of them, each once, in order.
#[must_use]
pub fn folders_of(files: &[String]) -> Vec<String> {
    let mut folders: BTreeSet<&str> = BTreeSet::new();
    for file in files {
        let mut path = file.as_str();
        while let Some((folder, _)) = path.rsplit_once('/') {
            if !folders.insert(folder) {
                break;
            }
            path = folder;
        }
    }
    folders.into_iter().map(str::to_owned).collect()
}

/// How well one name matches the terms, and where; none when a term does not match.
fn rank_of(name: &str, index: usize, terms: &[Vec<char>], strict: bool) -> Option<(Rank, RankedName)> {
    let letters = letters(name);
    let last_part = name
        .rfind('/')
        .map_or(0, |slash| letters.partition_point(|letter| letter.at <= slash));
    let mut where_found = 0_u32;
    let mut how_well = 0_i32;
    let mut marks = Vec::new();
    for term in terms {
        let found = match_term(term, &letters, last_part, strict)?;
        where_found += u32::from(found.place);
        how_well += found.how_well;
        marks.extend(
            found
                .letters
                .iter()
                .filter_map(|at| letters.get(*at))
                .map(|letter| letter.at),
        );
    }
    marks.sort_unstable();
    marks.dedup();
    let whole = match terms {
        [term] => whole_name(term, letters.get(last_part..).unwrap_or_default()),
        _ => 0,
    };
    let rank = (
        Reverse(whole),
        Reverse(where_found),
        Reverse(how_well),
        name.len(),
        index,
    );
    Some((rank, RankedName { index, marks }))
}

/// A term found in a path.
struct TermFound {
    /// Where: 3 together in the name, 2 apart in the name, 1 together along the path, 0 apart along the path.
    place: u8,
    /// How well, among those found in the same place.
    how_well: i32,
    /// Which letters of the path matched.
    letters: Vec<usize>,
}

/// Where a term matches a path: in its last part, together or else in order, or failing that along the whole path. A
/// term with a `/` is read against the whole path only, and one found only apart along it is no match when `strict`.
fn match_term(term: &[char], letters: &[Letter], last_part: usize, strict: bool) -> Option<TermFound> {
    let found = |place: u8, (how_well, letters): (i32, Vec<usize>)| TermFound {
        place,
        how_well,
        letters,
    };
    if !term.contains(&'/') {
        if let Some(together) = together(term, letters, last_part) {
            return Some(found(3, together));
        }
        if let Some(apart) = apart(term, letters, last_part) {
            return Some(found(2, apart));
        }
    }
    if let Some(together) = together(term, letters, 0) {
        return Some(found(1, together));
    }
    if strict {
        return None;
    }
    apart(term, letters, 0).map(|apart| found(0, apart))
}

/// A term found with its letters together, from a letter of the path on: at the start of a word when it is there,
/// else where it is first.
fn together(term: &[char], letters: &[Letter], from: usize) -> Option<(i32, Vec<usize>)> {
    let read = letters.get(from..)?;
    let starts = |start: &usize| {
        read.get(*start..*start + term.len())
            .is_some_and(|window| window.iter().map(|letter| letter.folded).eq(term.iter().copied()))
    };
    let last_start = read.len().checked_sub(term.len())?;
    let first = (0..=last_start).find(starts)?;
    let at_word = (first..=last_start)
        .filter(|start| read.get(*start).is_some_and(|letter| letter.starts_word))
        .find(starts);
    let start = at_word.unwrap_or(first);
    let near = NEAR_START - i32::try_from(start).unwrap_or(NEAR_START).min(NEAR_START);
    let how_well = if at_word.is_some() { WORD_START + near } else { near };
    Some((how_well, (from + start..from + start + term.len()).collect()))
}

/// A term found with its letters in order, others between them, from a letter of the path on: each letter at the
/// start of a word when one is there that leaves room for the letters after it, else where it is first.
fn apart(term: &[char], letters: &[Letter], from: usize) -> Option<(i32, Vec<usize>)> {
    let latest = latest_places(term, letters, from)?;
    let mut places = Vec::with_capacity(term.len());
    let mut next = from;
    let mut how_well = 0;
    for (wanted, last) in term.iter().zip(latest) {
        let window = letters.get(next..=last)?;
        let first = window.iter().position(|letter| letter.folded == *wanted)?;
        let at_word = window
            .iter()
            .position(|letter| letter.folded == *wanted && letter.starts_word);
        let place = next + at_word.unwrap_or(first);
        if at_word.is_some() {
            how_well += LETTER_AT_WORD_START;
        } else if place == next && place > from {
            how_well += LETTER_AFTER_LETTER;
        }
        places.push(place);
        next = place + 1;
    }
    let between = match (places.first(), places.last()) {
        (Some(first), Some(last)) => (last + 1 - first).saturating_sub(term.len()),
        _ => 0,
    };
    how_well -= i32::try_from(between).unwrap_or(MOST_BETWEEN).min(MOST_BETWEEN);
    Some((how_well, places))
}

/// For each letter of a term, the last letter of the path it can be found at while those after it are still found
/// after it; none when the letters of the term are not in the path in order.
fn latest_places(term: &[char], letters: &[Letter], from: usize) -> Option<Vec<usize>> {
    let mut latest = Vec::with_capacity(term.len());
    let mut end = letters.len();
    for wanted in term.iter().rev() {
        let read = letters.get(from..end)?;
        let place = from + read.iter().rposition(|letter| letter.folded == *wanted)?;
        latest.push(place);
        end = place;
    }
    latest.reverse();
    Some(latest)
}

/// Whether a name is all of a term, with or without its extension (2), or starts with it (1).
fn whole_name(term: &[char], name: &[Letter]) -> u8 {
    let starts = name.len() >= term.len() && name.iter().zip(term).all(|(letter, wanted)| letter.folded == *wanted);
    if !starts {
        return 0;
    }
    let extension = name
        .iter()
        .rposition(|letter| letter.folded == '.')
        .filter(|dot| *dot > 0);
    if name.len() == term.len() || extension == Some(term.len()) {
        2
    } else {
        1
    }
}

/// The letters of a path, as the terms are matched against them.
fn letters(path: &str) -> Vec<Letter> {
    let mut before: Option<char> = None;
    path.char_indices()
        .map(|(at, letter)| {
            let starts_word = match before {
                None => true,
                Some(before) => {
                    matches!(before, '/' | '.' | '_' | '-' | ' ') || (before.is_lowercase() && letter.is_uppercase())
                }
            };
            before = Some(letter);
            Letter {
                at,
                folded: fold(letter),
                starts_word,
            }
        })
        .collect()
}

/// A letter, small, so that the case of what is typed does not matter.
fn fold(letter: char) -> char {
    letter.to_lowercase().next().unwrap_or(letter)
}
