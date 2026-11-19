//! What the guide takes out of the text of a card: links, the names of functions and of files of the code, and the
//! numbers of decisions and questions, which say nothing to a user.

/// The text with its links made their words, its code that names a function or a file of the code taken out, and the
/// numbers of decisions and questions taken out, its spaces made single.
pub fn clean(text: &str) -> String {
    let text = without_links(text);
    let text = without_code_names(&text);
    let text = without_numbers(&text);
    let mut cleaned = text.split_whitespace().collect::<Vec<_>>().join(" ");
    loop {
        let mut tidied = cleaned.clone();
        for (before, after) in [
            (" ,", ","),
            (" .", "."),
            (" ;", ";"),
            (" :", ":"),
            ("( ", "("),
            (" )", ")"),
            ("(,", "("),
            (",)", ")"),
            (",,", ","),
            ("()", ""),
            ("  ", " "),
        ] {
            tidied = tidied.replace(before, after);
        }
        if tidied == cleaned {
            return cleaned.trim().to_owned();
        }
        cleaned = tidied;
    }
}

/// Each `[words](address)` made its words.
fn without_links(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some((before, after)) = rest.split_once('[') {
        out.push_str(before);
        let link = after
            .split_once("](")
            .and_then(|(words, tail)| tail.split_once(')').map(|(_, left)| (words, left)))
            .filter(|(words, _)| !words.contains('['));
        if let Some((words, left)) = link {
            out.push_str(words);
            rest = left;
        } else {
            out.push('[');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// Whether a span of code names a function, a type's path or a file of the code rather than something a user sees.
fn is_code_name(code: &str) -> bool {
    [
        "::",
        "(",
        ".ts",
        ".rs",
        "src/",
        "crates/",
        "Controller.",
        "EngineClient.",
    ]
    .iter()
    .any(|mark| code.contains(mark))
}

/// The spans of code that name a function or a file of the code taken out, with their backticks.
fn without_code_names(text: &str) -> String {
    let mut out = String::new();
    for (index, part) in text.split('`').enumerate() {
        if index % 2 == 0 {
            out.push_str(part);
        } else if !is_code_name(part) {
            out.push('`');
            out.push_str(part);
            out.push('`');
        }
    }
    out
}

/// The numbers of decisions and questions taken out: `D53`, `(Q570)`, `D53's`.
fn without_numbers(text: &str) -> String {
    let mut out = String::new();
    let mut word = String::new();
    for character in text.chars().chain(std::iter::once(' ')) {
        if character.is_ascii_alphanumeric() {
            word.push(character);
            continue;
        }
        if !is_decision_or_question(&word) {
            out.push_str(&word);
        }
        word.clear();
        out.push(character);
    }
    let _space = out.pop();
    out
}

/// Whether a word is `D` or `Q` and a number.
fn is_decision_or_question(word: &str) -> bool {
    let mut characters = word.chars();
    matches!(characters.next(), Some('D' | 'Q'))
        && word.len() > 1
        && characters.all(|character| character.is_ascii_digit())
}
