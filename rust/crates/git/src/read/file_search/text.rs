//! The search of the text of the files, by `git grep`, read as git prints it.

use std::ffi::OsString;
use std::ops::Range;

use model::file_search::TextMatch;

use super::in_folder;
use crate::read::fields::arguments;
use crate::{Git, GitError, Taken};

/// What git prints before a match: the color Gitardo gives matches for this command, which nothing else has.
const MATCH_START: &[u8] = b"\x1b[38;2;1;2;3m";

/// What git prints after a match: the end of a color.
const MATCH_END: &[u8] = b"\x1b[m";

/// The colors of `git grep`, set for the command: the matches in one of Gitardo's own, everything else in none, so
/// that the only escape sequences in what git prints are around the matches.
const COLORS: [&str; 18] = [
    "-c",
    "color.grep.matchSelected=#010203",
    "-c",
    "color.grep.matchContext=#010203",
    "-c",
    "color.grep.filename=normal",
    "-c",
    "color.grep.linenumber=normal",
    "-c",
    "color.grep.column=normal",
    "-c",
    "color.grep.separator=normal",
    "-c",
    "color.grep.selected=normal",
    "-c",
    "color.grep.context=normal",
    "-c",
    "color.grep.function=normal",
];

/// The most characters of a line that are kept.
const MOST_CHARACTERS: usize = 240;

/// How many characters are kept before the first match of a line that is cut.
const BEFORE_MATCH: usize = 60;

/// The most characters of the paths given to one run of git, for the length a command line has on Windows.
const MOST_PATHS: usize = 24_000;

/// What git says when it was built without the regular expressions of Perl.
const NO_PERL: &str = "cannot use Perl-compatible regexes";

/// What is looked for in the text of the files, and in which.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[allow(clippy::struct_excessive_bools, reason = "each is a switch of the search, on or off")]
pub struct TextRequest {
    /// What is looked for.
    pub text: String,
    /// Whether the capitals have to be as typed.
    pub match_case: bool,
    /// Whether it has to be a word of its own.
    pub whole_word: bool,
    /// Whether it is a regular expression.
    pub regex: bool,
    /// Whether the files git ignores are searched too.
    pub ignored: bool,
    /// The folder git is run in, from the top of the worktree, whose files alone are searched; none for the top.
    pub folder: Option<String>,
    /// The pathspecs that narrow the files, read from that folder (`file-search.pathspecs`).
    pub pathspecs: Vec<String>,
    /// The files to search and no other, by their paths from the top, in place of the pathspecs.
    pub files: Option<Vec<String>>,
    /// The most lines to hand on.
    pub most: usize,
}

/// How a search of the text ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextEnd {
    /// Whether there are more lines than those handed on.
    pub more: bool,
}

/// The regular expressions git is asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    /// None: the text as it is (`-F`).
    Plain,
    /// Those of Perl (`-P`).
    Perl,
    /// The extended ones of POSIX (`-E`), for a git built without those of Perl.
    Extended,
}

/// Searches the text of the files and hands the lines found to `take` as git prints them, each with every match in
/// it, until git ends, `take` has enough, or as many lines as asked for were handed on. `git` runs in the top folder
/// of the worktree.
///
/// Feature: file-search.read-text
///
/// # Errors
///
/// When git cannot be started, is cancelled, or fails: a regular expression git cannot read fails with git's words.
pub fn read_text(
    git: &Git,
    request: &TextRequest,
    take: &mut dyn FnMut(Vec<TextMatch>) -> Taken,
) -> Result<TextEnd, GitError> {
    let mut lines = Lines::new(request.most);
    if request.text.is_empty() || request.most == 0 {
        return Ok(TextEnd { more: false });
    }
    let git = in_folder(git, request.folder.as_deref());
    for pathspecs in runs(request) {
        let engine = if request.regex { Engine::Perl } else { Engine::Plain };
        let ended = match run(&git, &text_arguments(request, engine, &pathspecs), &mut lines, take) {
            Err(error) if engine == Engine::Perl && error.stderr().is_some_and(|said| said.contains(NO_PERL)) => run(
                &git,
                &text_arguments(request, Engine::Extended, &pathspecs),
                &mut lines,
                take,
            ),
            ended => ended,
        };
        if ended? == Taken::Enough {
            break;
        }
    }
    Ok(TextEnd { more: lines.more })
}

/// `git grep --color=always --full-name -n -z -I --untracked <how> -e <text> -- <pathspecs>`, after the colors that
/// mark the matches.
#[must_use]
pub fn text_arguments(request: &TextRequest, engine: Engine, pathspecs: &[String]) -> Vec<OsString> {
    let mut found = arguments(&COLORS);
    found.extend(arguments(&[
        "grep",
        "--color=always",
        "--full-name",
        "-n",
        "-z",
        "-I",
        "--untracked",
    ]));
    if request.ignored {
        found.push(OsString::from("--no-exclude-standard"));
    }
    if !request.match_case {
        found.push(OsString::from("-i"));
    }
    if request.whole_word {
        found.push(OsString::from("-w"));
    }
    found.push(OsString::from(match engine {
        Engine::Plain => "-F",
        Engine::Perl => "-P",
        Engine::Extended => "-E",
    }));
    found.push(OsString::from("-e"));
    found.push(OsString::from(&request.text));
    found.push(OsString::from("--"));
    found.extend(pathspecs.iter().map(OsString::from));
    found
}

/// The lines [`text_arguments`] printed: for each, the path of its file, a NUL, its number, a NUL, and its text with
/// the color of a match around each match. A line that cannot be read is left out.
#[must_use]
pub fn parse_text(output: &[u8]) -> Vec<TextMatch> {
    output.split(|byte| *byte == b'\n').filter_map(parse_line).collect()
}

/// The pathspecs of each run of git: those of the request once, or the files it names one by one, as many in a run as
/// a command line holds.
fn runs(request: &TextRequest) -> Vec<Vec<String>> {
    let Some(files) = &request.files else {
        return vec![request.pathspecs.clone()];
    };
    let mut found: Vec<Vec<String>> = Vec::new();
    let mut length = 0;
    for file in files {
        let pathspec = format!(":(top,literal){file}");
        match found.last_mut() {
            Some(run) if length + pathspec.len() <= MOST_PATHS => run.push(pathspec.clone()),
            _ => {
                length = 0;
                found.push(vec![pathspec.clone()]);
            }
        }
        length += pathspec.len();
    }
    found
}

/// One run of git: its lines handed on as they come. Git exits with 1 when it found nothing, which is no error.
fn run(
    git: &Git,
    arguments: &[OsString],
    lines: &mut Lines,
    take: &mut dyn FnMut(Vec<TextMatch>) -> Taken,
) -> Result<Taken, GitError> {
    let mut wanted = Taken::More;
    let mut hand_on = |found: Vec<TextMatch>, more: bool| {
        if !found.is_empty() && take(found) == Taken::Enough {
            return Taken::Enough;
        }
        if more { Taken::Enough } else { Taken::More }
    };
    let ended = git.run_read_as_printed(arguments, &mut |part| {
        let found = lines.read(part);
        wanted = hand_on(found, lines.more);
        Ok(wanted)
    });
    match ended {
        Ok(()) | Err(GitError::Failed { code: Some(1), .. }) => {}
        Err(error) => return Err(error),
    }
    if wanted == Taken::More {
        let found = lines.rest();
        wanted = hand_on(found, lines.more);
    }
    Ok(wanted)
}

/// The lines of what git prints, read a part at a time: a line that comes in two parts is kept until it is whole.
struct Lines {
    /// The start of a line whose end has not come yet.
    pending: Vec<u8>,
    /// How many lines can still be handed on.
    left: usize,
    /// Whether a line came after the last that could be handed on.
    more: bool,
}

impl Lines {
    /// No line read yet, with the most to hand on.
    fn new(most: usize) -> Self {
        Lines {
            pending: Vec::new(),
            left: most,
            more: false,
        }
    }

    /// The whole lines of what came so far, with a part more.
    fn read(&mut self, part: &[u8]) -> Vec<TextMatch> {
        self.pending.extend_from_slice(part);
        let Some(end) = self.pending.iter().rposition(|byte| *byte == b'\n') else {
            return Vec::new();
        };
        let rest = self.pending.split_off(end + 1);
        let whole = std::mem::replace(&mut self.pending, rest);
        self.keep(parse_text(&whole))
    }

    /// The line git did not end, once git has ended.
    fn rest(&mut self) -> Vec<TextMatch> {
        let whole = std::mem::take(&mut self.pending);
        self.keep(parse_text(&whole))
    }

    /// As many of some lines as can still be handed on.
    fn keep(&mut self, mut found: Vec<TextMatch>) -> Vec<TextMatch> {
        if found.len() > self.left {
            found.truncate(self.left);
            self.more = true;
        }
        self.left -= found.len();
        found
    }
}

/// One line of what git printed, without its end.
fn parse_line(record: &[u8]) -> Option<TextMatch> {
    let mut fields = record.splitn(3, |byte| *byte == 0);
    let path = std::str::from_utf8(fields.next()?).ok()?;
    let line: usize = std::str::from_utf8(fields.next()?).ok()?.parse().ok()?;
    let printed = fields.next()?;
    let printed = printed.strip_suffix(b"\r").unwrap_or(printed);
    let (text, marks) = marked(printed);
    let (text, marks) = shaped(&text, marks);
    Some(TextMatch {
        path: path.to_owned(),
        line,
        text,
        marks,
    })
}

/// The text of a line without the colors git put around its matches, and where those matches are in it.
fn marked(printed: &[u8]) -> (String, Vec<Range<usize>>) {
    let mut text = String::new();
    let mut marks = Vec::new();
    let mut rest = printed;
    while let Some(start) = find(rest, MATCH_START) {
        text.push_str(&String::from_utf8_lossy(rest.get(..start).unwrap_or_default()));
        let after = rest.get(start + MATCH_START.len()..).unwrap_or_default();
        let length = find(after, MATCH_END).unwrap_or(after.len());
        let from = text.len();
        text.push_str(&String::from_utf8_lossy(after.get(..length).unwrap_or_default()));
        if text.len() > from {
            marks.push(from..text.len());
        }
        rest = after.get(length + MATCH_END.len()..).unwrap_or_default();
    }
    text.push_str(&String::from_utf8_lossy(rest));
    (text, marks)
}

/// Where some bytes are first in others.
fn find(bytes: &[u8], wanted: &[u8]) -> Option<usize> {
    bytes.windows(wanted.len()).position(|window| window == wanted)
}

/// A line as the results show it: without the white space at its start, and cut when it is long, from a little
/// before its first match.
fn shaped(text: &str, marks: Vec<Range<usize>>) -> (String, Vec<Range<usize>>) {
    let indent = text.len() - text.trim_start().len();
    let first = marks.first().map_or(indent, |mark| mark.start.max(indent));
    let before = text.get(indent..first).map_or(0, |before| before.chars().count());
    let long = text
        .get(indent..)
        .is_some_and(|shown| shown.chars().count() > MOST_CHARACTERS);
    let skipped = if long { before.saturating_sub(BEFORE_MATCH) } else { 0 };
    let shown = text.get(indent..).unwrap_or_default();
    let from = indent + shown.char_indices().nth(skipped).map_or(shown.len(), |(at, _)| at);
    let kept = text.get(from..).unwrap_or_default();
    let to = from
        + kept
            .char_indices()
            .nth(MOST_CHARACTERS)
            .map_or(kept.len(), |(at, _)| at);
    let marks = marks
        .into_iter()
        .map(|mark| mark.start.clamp(from, to) - from..mark.end.clamp(from, to) - from)
        .filter(|mark| !mark.is_empty())
        .collect();
    (text.get(from..to).unwrap_or_default().to_owned(), marks)
}

#[cfg(test)]
#[allow(clippy::single_range_in_vec_init, reason = "a line with one match has one range")]
mod tests {
    use super::{Engine, Lines, TextRequest, parse_text, runs, text_arguments};
    use crate::command_line;

    /// A request for a text, with nothing else asked.
    fn request(text: &str) -> TextRequest {
        TextRequest {
            text: text.to_owned(),
            most: 100,
            ..TextRequest::default()
        }
    }

    /// The arguments of a request after the colors, as the log writes them.
    fn asked(request: &TextRequest, engine: Engine) -> String {
        let line = command_line(&text_arguments(request, engine, &request.pathspecs));
        line.split_once(" grep ").map(|(_, rest)| rest.to_owned()).unwrap()
    }

    #[test]
    fn the_arguments_say_how_the_text_is_looked_for() {
        let mut request = request("a b");
        assert_eq!(
            asked(&request, Engine::Plain),
            "--color=always --full-name -n -z -I --untracked -i -F -e \"a b\" --"
        );
        request.match_case = true;
        request.whole_word = true;
        request.ignored = true;
        request.pathspecs = vec![String::from(":(glob)**/*.rs")];
        assert_eq!(
            asked(&request, Engine::Perl),
            "--color=always --full-name -n -z -I --untracked --no-exclude-standard -w -P -e \"a b\" -- :(glob)**/*.rs"
        );
        assert!(asked(&request, Engine::Extended).contains(" -E -e "));
    }

    #[test]
    fn the_files_named_one_by_one_are_given_in_as_many_runs_as_their_paths_ask() {
        let mut request = request("a");
        let long = "x".repeat(10_000);
        request.files = Some(vec![format!("{long}1"), format!("{long}2"), format!("{long}3")]);
        let runs = runs(&request);
        assert_eq!(runs.iter().map(Vec::len).collect::<Vec<_>>(), [2, 1]);
        assert!(runs[0][0].starts_with(":(top,literal)x"));
        request.files = Some(Vec::new());
        assert_eq!(super::runs(&request), Vec::<Vec<String>>::new());
    }

    #[test]
    fn a_line_is_read_with_its_matches_and_without_its_indent() {
        let output =
            b"src/a b.rs\x0012\x00\t  let \x1b[38;2;1;2;3mh\xc3\xa9\x1b[m = \x1b[38;2;1;2;3mh\xc3\xa9\x1b[m;\r\n";
        let found = parse_text(output);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].path.as_str(), found[0].line), ("src/a b.rs", 12));
        assert_eq!(found[0].text, "let hé = hé;");
        assert_eq!(found[0].marks, [4..7, 10..13]);
        assert_eq!(parse_text(b"no fields\n\n"), Vec::new());
    }

    #[test]
    fn a_long_line_is_cut_from_a_little_before_its_first_match() {
        let mut output = b"a.txt\x001\x00".to_vec();
        output.extend("é".repeat(500).bytes());
        output.extend(b"\x1b[38;2;1;2;3msocket\x1b[m");
        output.extend("z".repeat(500).bytes());
        output.push(b'\n');
        let found = parse_text(&output);
        assert_eq!(found[0].text.chars().count(), 240);
        assert_eq!(found[0].marks, [120..126]);
        assert_eq!(&found[0].text[120..126], "socket");
    }

    #[test]
    fn lines_that_come_in_parts_are_read_whole_and_no_more_than_asked() {
        let mut lines = Lines::new(2);
        assert_eq!(lines.read(b"a.txt\x001\x00one\nb.t").len(), 1);
        let second = lines.read(b"xt\x002\x00two\nc.txt\x003\x00three\n");
        assert_eq!(second.len(), 1);
        assert_eq!((second[0].path.as_str(), second[0].text.as_str()), ("b.txt", "two"));
        assert!(lines.more);
        let mut lines = Lines::new(5);
        assert_eq!(lines.read(b"a.txt\x001\x00one"), Vec::new());
        assert_eq!(lines.rest().len(), 1);
        assert!(!lines.more);
    }
}
