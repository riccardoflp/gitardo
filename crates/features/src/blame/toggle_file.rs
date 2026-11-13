//! `blame.toggle-file`: shows or hides the blame beside the lines of the file of a File page.

use model::blame::{BlameOfFile, ShownBlame};
use model::contract::{Answers, Context, FollowUp, Outcome, Refusal, Target};

use super::{BLAME, blame_query, file_name, no_step};

/// Hides the blame of a file that shows it, until its page is closed; shows it in a file that does not, once it is
/// read. A file git has no blame for shows nothing and says why.
///
/// Feature: blame.toggle-file
#[must_use]
pub fn toggle_file(context: &Context, answers: &Answers) -> Outcome {
    let Target::LineOfFile(line) = &context.target else {
        return Outcome::Refuse(Refusal::saying(
            "Open a file of a repository to see who changed its lines.",
        ));
    };
    let name = file_name(&line.path);
    if matches!(line.shown, ShownBlame::Line(_)) {
        return no_step(
            format!("Hide the blame of {name}"),
            FollowUp::HideBlame(line.path.clone(), line.revision.clone()),
        );
    }
    let Some(lines) = answers.blame(BLAME) else {
        return Outcome::Read(BLAME, blame_query(context, line));
    };
    let Some(lines) = lines else {
        let said = format!("{name} is not in any commit yet, so there is no one to name beside its lines.");
        return Outcome::Refuse(Refusal::saying(&said));
    };
    let blame = BlameOfFile {
        path: line.path.clone(),
        revision: line.revision.clone(),
        lines: lines.to_vec(),
    };
    no_step(
        format!("Show the blame of {name}"),
        FollowUp::ShowBlame(Box::new(blame)),
    )
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answer, Answers, FollowUp, Outcome};

    use super::toggle_file;
    use crate::blame::BLAME;
    use model::blame::ShownBlame;

    use crate::blame::testing::{blamed, on_line};
    use crate::testing::plan;

    #[test]
    fn blame_toggle_file__shows_the_blame_then_hides_it() {
        let hidden = on_line(1, ShownBlame::Hidden);
        assert!(matches!(toggle_file(&hidden, &Answers::new()), Outcome::Read(BLAME, _)));
        let lines = Answers::new().with(BLAME, Answer::Blame(Some(vec![Some(blamed('a', None))])));
        let shows = plan(toggle_file(&hidden, &lines));
        let [FollowUp::ShowBlame(blame)] = shows.after.as_slice() else {
            panic!("shows the blame: {shows:?}");
        };
        assert_eq!((blame.path.as_str(), blame.lines.len()), ("src/app.rs", 1));
        let with_blame = on_line(1, ShownBlame::Line(Some(blamed('a', None))));
        let hides = plan(toggle_file(&with_blame, &Answers::new()));
        assert_eq!(hides.after, [FollowUp::HideBlame(String::from("src/app.rs"), None)]);
    }

    #[test]
    fn blame_toggle_file__says_a_file_never_committed_has_no_blame() {
        let hidden = on_line(1, ShownBlame::Hidden);
        let none = Answers::new().with(BLAME, Answer::Blame(None));
        let Outcome::Refuse(refusal) = toggle_file(&hidden, &none) else {
            panic!("shows nothing");
        };
        assert_eq!(
            refusal.message.as_str(),
            "app.rs is not in any commit yet, so there is no one to name beside its lines."
        );
    }
}
