//! Finding a text in a file (`file-view.find`): every place it is, as plain text or as a regular expression, with the
//! case of letters mattering or not, and as a whole word or anywhere.

use std::ops::Range;

use regex::RegexBuilder;

/// What is looked for, and how.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FindQuery<'a> {
    /// What is typed in the box.
    pub text: &'a str,
    /// Whether `Parse` finds only `Parse`, not `parse`.
    pub match_case: bool,
    /// Whether a match must be a whole word: `parse` does not find the start of `parser`.
    pub whole_word: bool,
    /// Whether what is typed is a regular expression.
    pub is_regex: bool,
}

/// Why nothing can be found: what is typed is not a regular expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InvalidPattern;

/// Every place of a text where a query is found, in order, as ranges of its bytes; none for an empty query. A regular
/// expression that matches an empty text, such as `a*`, finds only its matches that are not empty.
///
/// # Errors
///
/// When the query is a regular expression that is not valid, or too large to be compiled.
pub fn find_all(text: &str, query: &FindQuery<'_>) -> Result<Vec<Range<usize>>, InvalidPattern> {
    if query.text.is_empty() {
        return Ok(Vec::new());
    }
    let pattern = if query.is_regex {
        query.text.to_owned()
    } else {
        regex::escape(query.text)
    };
    let pattern = if query.whole_word {
        format!(r"\b(?:{pattern})\b")
    } else {
        pattern
    };
    let found = RegexBuilder::new(&pattern)
        .case_insensitive(!query.match_case)
        .multi_line(true)
        .build()
        .map_err(|_| InvalidPattern)?;
    Ok(found
        .find_iter(text)
        .filter(|found| !found.is_empty())
        .map(|found| found.range())
        .collect())
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::*;

    fn query(text: &str) -> FindQuery<'_> {
        FindQuery {
            text,
            match_case: false,
            whole_word: false,
            is_regex: false,
        }
    }

    /// Ranges of bytes, by their starts and ends.
    fn ranges(pairs: &[(usize, usize)]) -> Vec<Range<usize>> {
        pairs.iter().map(|(start, end)| *start..*end).collect()
    }

    #[test]
    fn a_plain_query_finds_every_place_whatever_the_case_and_its_special_characters_are_plain() {
        assert_eq!(find_all("parse Parse PARSE", &query("parse")).unwrap().len(), 3);
        assert_eq!(find_all("a.b axb", &query("a.b")).unwrap(), ranges(&[(0, 3)]));
        assert_eq!(find_all("anything", &query("")).unwrap().len(), 0);
    }

    #[test]
    fn match_case_and_whole_word_narrow_what_is_found() {
        let exact = FindQuery {
            match_case: true,
            ..query("Parse")
        };
        assert_eq!(find_all("parse Parse", &exact).unwrap(), ranges(&[(6, 11)]));
        let whole = FindQuery {
            whole_word: true,
            ..query("parse")
        };
        assert_eq!(find_all("parser parse reparse", &whole).unwrap(), ranges(&[(7, 12)]));
    }

    #[test]
    fn a_regular_expression_finds_its_matches_and_an_invalid_one_is_refused() {
        let numbers = FindQuery {
            is_regex: true,
            ..query(r"\d+")
        };
        assert_eq!(find_all("a 12 b 345", &numbers).unwrap(), ranges(&[(2, 4), (7, 10)]));
        let starts = FindQuery {
            is_regex: true,
            ..query("^fn")
        };
        assert_eq!(
            find_all("fn a\n  fn b\nfn c", &starts).unwrap(),
            ranges(&[(0, 2), (12, 14)])
        );
        let invalid = FindQuery {
            is_regex: true,
            ..query("(unclosed")
        };
        assert_eq!(find_all("(unclosed", &invalid), Err(InvalidPattern));
        let empty = FindQuery {
            is_regex: true,
            ..query("x*")
        };
        assert_eq!(find_all("axxb", &empty).unwrap(), ranges(&[(1, 3)]));
    }
}
