//! Fuzzy matching for pickers: a text matches when it has every character of the query in order, in any case, with
//! anything between them. Of all the ways a text matches, the best is kept: characters that follow each other, that
//! start a word or the text, and few characters skipped score higher. The texts are then ranked by score.

/// How a text matched a query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FuzzyMatch {
    /// How well it matched: higher is better.
    pub score: i32,
    /// Where each character of the query is in the text, as the position of its first byte, in order.
    pub positions: Vec<usize>,
}

/// The score of every character matched.
const MATCHED: i32 = 16;
/// The score of a character matched right after the one before it.
const CONSECUTIVE: i32 = 24;
/// The score of a character matched at the start of the text.
const FIRST: i32 = 24;
/// The score of a character matched at the start of a word.
const WORD_START: i32 = 20;
/// The score of an uppercase letter matched after a lowercase one, the start of a word written in camel case.
const CAMEL: i32 = 16;
/// The score of a character matched in the same case as in the query.
const SAME_CASE: i32 = 1;
/// What each character skipped between two matched ones costs.
const GAP: i32 = 3;
/// The most a gap costs, however long.
const LONGEST_GAP: i32 = 24;
/// The most the characters before the first matched one cost.
const LONGEST_LEAD: i32 = 15;

/// Whether a character separates words: what comes after it starts one.
fn is_separator(character: char) -> bool {
    character.is_whitespace() || "-_/\\.:,;()[]{}<>'\"".contains(character)
}

/// What matching a character of the text at a place scores, apart from the gap before it.
fn place_score(text: &[char], at: usize, wanted: char) -> i32 {
    let Some(&found) = text.get(at) else {
        return 0;
    };
    let previous = at.checked_sub(1).and_then(|before| text.get(before)).copied();
    let start = match previous {
        None => FIRST,
        Some(before) if is_separator(before) => WORD_START,
        Some(before) if before.is_lowercase() && found.is_uppercase() => CAMEL,
        Some(_) => 0,
    };
    let case = if found == wanted { SAME_CASE } else { 0 };
    MATCHED + start + case
}

/// Whether two characters are the same letter in any case.
fn same_letter(first: char, second: char) -> bool {
    first == second || first.to_lowercase().eq(second.to_lowercase())
}

/// How a text matches a query, when it has every character of the query in order; spaces in the query are ignored. An
/// empty query matches every text with a score of 0.
#[must_use]
pub fn fuzzy_match(query: &str, text: &str) -> Option<FuzzyMatch> {
    let wanted: Vec<char> = query.chars().filter(|character| !character.is_whitespace()).collect();
    let characters: Vec<(usize, char)> = text.char_indices().collect();
    let letters: Vec<char> = characters.iter().map(|(_, character)| *character).collect();
    if wanted.is_empty() {
        return Some(FuzzyMatch {
            score: 0,
            positions: Vec::new(),
        });
    }
    let (score, places) = best_places(&wanted, &letters)?;
    let positions = places
        .into_iter()
        .filter_map(|place| characters.get(place).map(|(byte, _)| *byte))
        .collect();
    Some(FuzzyMatch { score, positions })
}

/// The best score of matching the query at places of the text, and those places, by dynamic programming: for each
/// character of the query and each place of the text, the best score of the query so far ending there, and where the
/// character before it was.
fn best_places(wanted: &[char], text: &[char]) -> Option<(i32, Vec<usize>)> {
    let count = text.len();
    let mut best: Vec<Vec<Option<i32>>> = Vec::with_capacity(wanted.len());
    let mut from: Vec<Vec<usize>> = Vec::with_capacity(wanted.len());
    for (index, letter) in wanted.iter().enumerate() {
        let mut scores = vec![None; count];
        let mut sources = vec![0; count];
        for at in 0..count {
            if !text.get(at).is_some_and(|found| same_letter(*found, *letter)) {
                continue;
            }
            let here = place_score(text, at, *letter);
            let previous = index.checked_sub(1).and_then(|before| best.get(before));
            let candidate = match previous {
                None => Some((here - lead_cost(at), 0)),
                Some(row) => best_link(row, at).map(|(score, source)| (score + here, source)),
            };
            if let (Some((score, source)), Some(slot), Some(link)) =
                (candidate, scores.get_mut(at), sources.get_mut(at))
            {
                *slot = Some(score);
                *link = source;
            }
        }
        best.push(scores);
        from.push(sources);
    }
    let last = best.last()?;
    let (end, score) = last
        .iter()
        .enumerate()
        .filter_map(|(at, score)| score.map(|score| (at, score)))
        .max_by_key(|(at, score)| (*score, std::cmp::Reverse(*at)))?;
    let mut places = vec![end];
    for sources in from.iter().skip(1).rev() {
        let next = sources.get(*places.last()?).copied()?;
        places.push(next);
    }
    places.reverse();
    Some((score, places))
}

/// What the characters before the first matched one cost.
fn lead_cost(at: usize) -> i32 {
    i32::try_from(at).unwrap_or(i32::MAX).min(LONGEST_LEAD)
}

/// The best way to reach a place from the matches of the character before: its score with the gap paid or the bonus
/// of following it, and where it was.
fn best_link(previous: &[Option<i32>], at: usize) -> Option<(i32, usize)> {
    previous
        .iter()
        .take(at)
        .enumerate()
        .filter_map(|(source, score)| {
            let score = (*score)?;
            let skipped = at - source - 1;
            let link = if skipped == 0 {
                CONSECUTIVE
            } else {
                -(i32::try_from(skipped).unwrap_or(i32::MAX).saturating_mul(GAP)).min(LONGEST_GAP)
            };
            Some((score + link, source))
        })
        .max_by_key(|(score, source)| (*score, *source))
}

/// The texts that match a query, best first: by score, then the shorter text, then the order they were given in; with
/// nothing typed, every text in the order given. Each comes with its place in the texts given and how it matched.
#[must_use]
pub fn rank<'text>(query: &str, texts: impl IntoIterator<Item = &'text str>) -> Vec<(usize, FuzzyMatch)> {
    if query.trim().is_empty() {
        let unmatched = || FuzzyMatch {
            score: 0,
            positions: Vec::new(),
        };
        return texts
            .into_iter()
            .enumerate()
            .map(|(index, _)| (index, unmatched()))
            .collect();
    }
    let mut ranked: Vec<(usize, usize, FuzzyMatch)> = texts
        .into_iter()
        .enumerate()
        .filter_map(|(index, text)| fuzzy_match(query, text).map(|found| (index, text.len(), found)))
        .collect();
    ranked.sort_by(|first, second| {
        second
            .2
            .score
            .cmp(&first.2.score)
            .then(first.1.cmp(&second.1))
            .then(first.0.cmp(&second.0))
    });
    ranked.into_iter().map(|(index, _, found)| (index, found)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_matches_when_it_has_the_characters_in_order_in_any_case() {
        let found = fuzzy_match("flp", "feature/login-page").unwrap();
        assert_eq!(found.positions, vec![0, 8, 14]);
        assert!(fuzzy_match("FLP", "feature/login-page").is_some());
        assert_eq!(fuzzy_match("plf", "feature/login-page"), None);
        assert_eq!(fuzzy_match("", "anything").unwrap().score, 0);
        assert_eq!(fuzzy_match("x", ""), None);
        assert_eq!(fuzzy_match("ü", "Grüße").unwrap().positions, vec![2]);
    }

    #[test]
    fn the_best_way_to_match_is_kept() {
        let found = fuzzy_match("login", "fix the logo in the login page").unwrap();
        assert_eq!(
            found.positions,
            vec![20, 21, 22, 23, 24],
            "the whole word beats letters spread out"
        );
        let camel = fuzzy_match("rp", "readParser").unwrap();
        assert_eq!(camel.positions, vec![0, 4]);
    }

    #[test]
    fn texts_are_ranked_best_first_then_shorter_then_in_order() {
        let texts = ["release/2.4", "main", "feature/main-menu", "maintenance", "domain"];
        let ranked: Vec<usize> = rank("main", texts).into_iter().map(|(index, _)| index).collect();
        assert_eq!(ranked, vec![1, 3, 2, 4]);
        let all: Vec<usize> = rank("", texts).into_iter().map(|(index, _)| index).collect();
        assert_eq!(all, vec![0, 1, 2, 3, 4], "an empty query keeps every text in order");
    }
}
