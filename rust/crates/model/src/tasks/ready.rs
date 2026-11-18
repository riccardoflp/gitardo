//! When a background task counts as ready for the tasks that wait for it (`tasks.run`), from what it printed: once
//! its `endsPattern` is printed, or, without one, once it has printed nothing for two seconds after its
//! `beginsPattern` or its start.

use std::time::Duration;

use regex::Regex;

use super::{Background, TaskOutput};

/// How long a background task without an `endsPattern` prints nothing before it counts as ready.
pub const QUIET: Duration = Duration::from_secs(2);

/// What a background task printed so far, read for the lines that say it is ready. The moments are given, never read:
/// each is the time since any one moment, such as the start of the window.
#[derive(Debug, Clone)]
pub struct Readiness {
    /// The line that says the task started working; none when it has none, or one that is no regular expression here.
    begins: Option<Regex>,
    /// The line that says the task is ready; none when it has none, or one that is no regular expression here.
    ends: Option<Regex>,
    /// What it printed, in lines without the colors of a terminal.
    output: TaskOutput,
    /// How many of its lines were looked at already, the dropped ones counted; the last may still be written.
    looked_at: usize,
    /// Whether a line matched `endsPattern`.
    has_ended: bool,
    /// The last lines its terminal showed, when it is read from them.
    shown: Vec<String>,
    /// Since when it prints nothing, once that counts: from its start, or from its `beginsPattern` when it has one.
    quiet_since: Option<Duration>,
}

impl Readiness {
    /// The readiness of a background task that started at a moment. A pattern that is not a regular expression as
    /// Gitardo reads them counts as none: VS Code reads those of JavaScript, which has a few forms more.
    #[must_use]
    pub fn new(background: &Background, started: Duration) -> Self {
        let pattern = |text: &Option<String>| text.as_deref().and_then(|text| Regex::new(text).ok());
        let begins = pattern(&background.begins_pattern);
        let quiet_since = begins.is_none().then_some(started);
        Readiness {
            begins,
            ends: pattern(&background.ends_pattern),
            output: TaskOutput::default(),
            looked_at: 0,
            has_ended: false,
            shown: Vec::new(),
            quiet_since,
        }
    }

    /// Takes what the task printed at a moment, in any pieces: a line may come in two.
    pub fn printed(&mut self, text: &str, at: Duration) {
        if text.is_empty() {
            return;
        }
        self.output.push(text);
        let dropped = self.output.dropped();
        let lines = self.output.lines();
        let total = dropped + lines.len();
        let mut has_begun = false;
        for line in lines.iter().skip(self.looked_at.saturating_sub(dropped)) {
            has_begun |= self.begins.as_ref().is_some_and(|begins| begins.is_match(line));
            self.has_ended |= self.ends.as_ref().is_some_and(|ends| ends.is_match(line));
        }
        // The last line is looked at again with what is printed next, since it may not be whole yet.
        self.looked_at = total.saturating_sub(1);
        if has_begun || self.quiet_since.is_some() {
            self.quiet_since = Some(at);
        }
    }

    /// Takes the last lines the terminal of the task shows at a moment, without their colors, in place of what it
    /// printed piece by piece: lines that differ from those shown before count as printed then. A line that says the
    /// task is ready counts for as long as it is among them, so they are to be read each time the task prints.
    pub fn shows(&mut self, lines: &[String], at: Duration) {
        if self.shown == lines {
            return;
        }
        let matches = |pattern: &Option<Regex>| {
            pattern
                .as_ref()
                .is_some_and(|pattern| lines.iter().any(|line| pattern.is_match(line)))
        };
        self.has_ended |= matches(&self.ends);
        if matches(&self.begins) || self.quiet_since.is_some() {
            self.quiet_since = Some(at);
        }
        self.shown = lines.to_vec();
    }

    /// Whether the task is ready at a moment.
    #[must_use]
    pub fn is_ready(&self, now: Duration) -> bool {
        if self.ends.is_some() {
            return self.has_ended;
        }
        self.ready_at().is_some_and(|ready_at| now >= ready_at)
    }

    /// The moment the task will be ready if it prints nothing more, to look again then; none while it waits for a
    /// line: its `endsPattern`, or its `beginsPattern` not printed yet.
    #[must_use]
    pub fn ready_at(&self) -> Option<Duration> {
        if self.ends.is_some() {
            return None;
        }
        self.quiet_since.map(|since| since + QUIET)
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::time::Duration;

    use super::{Background, Readiness};

    fn seconds(seconds: u64) -> Duration {
        Duration::from_secs(seconds)
    }

    fn background(begins: Option<&str>, ends: Option<&str>) -> Background {
        Background {
            begins_pattern: begins.map(str::to_owned),
            ends_pattern: ends.map(str::to_owned),
        }
    }

    #[test]
    fn tasks_run__a_background_task_is_ready_once_its_ends_pattern_is_printed_while_it_keeps_running() {
        let mut readiness = Readiness::new(&background(None, Some("ready")), seconds(0));
        readiness.printed("starting the server\r\n", seconds(1));
        assert!(
            !readiness.is_ready(seconds(60)),
            "quiet for a minute is not ready with an endsPattern"
        );
        assert_eq!(readiness.ready_at(), None);
        readiness.printed("\u{1b}[32mserver rea", seconds(61));
        assert!(!readiness.is_ready(seconds(61)));
        readiness.printed("dy\u{1b}[0m on port 3000\n", seconds(61));
        assert!(readiness.is_ready(seconds(61)), "the line came in two pieces, in color");
        readiness.printed("GET /\n", seconds(62));
        assert!(readiness.is_ready(seconds(62)), "it stays ready while it keeps running");
    }

    #[test]
    fn tasks_run__the_ends_pattern_is_a_regular_expression_matched_on_each_line() {
        let mut readiness = Readiness::new(
            &background(Some("^Compiling"), Some(r"^Found \d+ errors?\.")),
            seconds(0),
        );
        readiness.printed("Compiling\nNot yet: Found 0 errors.\n", seconds(1));
        assert!(!readiness.is_ready(seconds(10)));
        readiness.printed("Found 0 errors. Watching for file changes.\n", seconds(11));
        assert!(readiness.is_ready(seconds(11)));
    }

    #[test]
    fn tasks_run__without_an_ends_pattern_it_is_ready_two_seconds_after_it_last_printed_since_its_start() {
        let mut readiness = Readiness::new(&background(None, None), seconds(10));
        assert_eq!(readiness.ready_at(), Some(seconds(12)));
        assert!(!readiness.is_ready(seconds(11)));
        readiness.printed("installing\n", seconds(11));
        assert!(!readiness.is_ready(seconds(12)), "it printed a second ago");
        assert_eq!(readiness.ready_at(), Some(seconds(13)));
        assert!(readiness.is_ready(seconds(13)));
        let silent = Readiness::new(&background(None, None), seconds(10));
        assert!(
            silent.is_ready(seconds(12)),
            "a task that prints nothing is ready two seconds after its start"
        );
    }

    #[test]
    fn tasks_run__with_a_begins_pattern_alone_the_two_seconds_count_from_its_line() {
        let mut readiness = Readiness::new(&background(Some("Starting compilation"), None), seconds(0));
        assert_eq!(readiness.ready_at(), None);
        assert!(
            !readiness.is_ready(seconds(30)),
            "nothing counts before its beginsPattern"
        );
        readiness.printed("preparing\n", seconds(30));
        assert!(!readiness.is_ready(seconds(40)));
        readiness.printed("Starting compilation in watch mode\n", seconds(40));
        assert!(!readiness.is_ready(seconds(41)));
        readiness.printed("src/a.ts\n", seconds(41));
        assert!(!readiness.is_ready(seconds(42)));
        assert!(readiness.is_ready(seconds(43)));
    }

    #[test]
    fn tasks_run__a_pattern_that_is_no_regular_expression_here_counts_as_none() {
        let mut readiness = Readiness::new(&background(None, Some("(?<=ready) now")), seconds(0));
        readiness.printed("ready now\n", seconds(1));
        assert!(!readiness.is_ready(seconds(2)));
        assert!(readiness.is_ready(seconds(3)), "ready as a task without an endsPattern");
    }

    #[test]
    fn tasks_run__the_last_lines_of_its_terminal_say_it_is_ready_as_what_it_printed_does() {
        let lines = |lines: &[&str]| lines.iter().map(|line| (*line).to_owned()).collect::<Vec<String>>();
        let mut readiness = Readiness::new(&background(None, Some("^ready")), seconds(0));
        readiness.shows(&lines(&["starting", "not ready"]), seconds(1));
        assert!(!readiness.is_ready(seconds(60)));
        readiness.shows(&lines(&["starting", "not ready", "ready on port 3000"]), seconds(61));
        assert!(readiness.is_ready(seconds(61)));
        readiness.shows(&lines(&["GET /"]), seconds(62));
        assert!(
            readiness.is_ready(seconds(62)),
            "it stays ready once its line scrolled away"
        );
        let mut quiet = Readiness::new(&background(None, None), seconds(0));
        quiet.shows(&lines(&["installing"]), seconds(1));
        quiet.shows(&lines(&["installing"]), seconds(2));
        assert_eq!(
            quiet.ready_at(),
            Some(seconds(3)),
            "the same lines shown again are nothing printed"
        );
        quiet.shows(&lines(&["installing", "done"]), seconds(2));
        assert!(!quiet.is_ready(seconds(3)));
        assert!(quiet.is_ready(seconds(4)));
    }
}
