#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

use std::path::PathBuf;
use std::time::Duration;

use super::{HiddenTab, schedule_hidden};
use crate::repo::schedule::{ReadAction, ReadEvent};
use crate::repo::watch::{Change, ReadCause, watch_changes};

/// A commit made in a terminal, as the watcher of the tab sees it.
fn commit_in_a_terminal() -> ReadEvent {
    let request = watch_changes(&Change::GitFolder(PathBuf::from("refs/heads/main"))).unwrap();
    ReadEvent::Changed {
        request,
        at: Duration::from_secs(1),
    }
}

/// Tells a tab an event, and gives it with what it asks.
fn tell(tab: &HiddenTab, event: &ReadEvent) -> (HiddenTab, Vec<ReadAction>) {
    schedule_hidden(tab, event)
}

/// The number of the read an action starts.
fn started(actions: &[ReadAction]) -> Option<u64> {
    actions.iter().find_map(|action| match action {
        ReadAction::Start(read) => Some(read.number),
        _ => None,
    })
}

#[test]
fn repo_watch_hidden__a_commit_in_a_terminal_in_a_tab_not_shown_reads_nothing_and_marks_it_out_of_date() {
    let tab = HiddenTab::left(None);
    let (tab, actions) = tell(&tab, &commit_in_a_terminal());
    assert_eq!(actions, [], "nothing is read");
    assert_eq!(
        tab.out_of_date(),
        Some(&ReadCause::GitFile(PathBuf::from("refs/heads/main")))
    );
    let (tab, actions) = tell(&tab, &ReadEvent::Asked(ReadCause::RepositoryChosen));
    assert_eq!(actions, [], "its turn reads only a tab never read");
    assert!(tab.out_of_date().is_some());
}

#[test]
fn repo_watch_hidden__eight_tabs_never_read_are_read_one_after_the_other() {
    let mut tabs = vec![HiddenTab::new(); 7];
    let mut order = Vec::new();
    // The window gives the next tab its turn once the read before ends.
    for (place, tab) in tabs.iter_mut().enumerate() {
        assert!(tab.is_waiting());
        let (reading, actions) = tell(tab, &ReadEvent::Asked(ReadCause::RepositoryChosen));
        let number = started(&actions).unwrap();
        assert!(reading.is_reading());
        let (read, actions) = tell(&reading, &ReadEvent::Finished(number));
        assert_eq!(actions, [ReadAction::Show(number)]);
        assert_eq!(read.out_of_date(), None);
        assert!(!read.is_waiting() && !read.is_reading());
        *tab = read;
        order.push(place);
    }
    assert_eq!(order, (0..7).collect::<Vec<_>>());
}

#[test]
fn repo_watch_hidden__a_tab_left_with_nothing_changed_is_up_to_date_and_one_changed_during_its_read_is_not() {
    assert_eq!(HiddenTab::left(None).out_of_date(), None);
    let left = HiddenTab::left(Some(ReadCause::Focus));
    assert_eq!(left.out_of_date(), Some(&ReadCause::Focus));
    let (reading, actions) = tell(&HiddenTab::new(), &ReadEvent::Asked(ReadCause::RepositoryChosen));
    let number = started(&actions).unwrap();
    let (reading, _) = tell(&reading, &commit_in_a_terminal());
    let (read, _) = tell(&reading, &ReadEvent::Finished(number));
    assert!(
        read.out_of_date().is_some(),
        "the change may be missing from what it read"
    );
}

#[test]
fn repo_watch_hidden__while_an_operation_runs_no_tab_not_shown_is_read() {
    let (reading, actions) = tell(&HiddenTab::new(), &ReadEvent::Asked(ReadCause::RepositoryChosen));
    let number = started(&actions).unwrap();
    let (stopped, actions) = tell(&reading, &ReadEvent::OperationStarted);
    assert_eq!(actions, [ReadAction::Discard(number)]);
    assert!(stopped.is_waiting(), "it waits for its turn again");
    let (still, actions) = tell(&stopped, &ReadEvent::Asked(ReadCause::RepositoryChosen));
    assert_eq!(actions, [], "not while the operation runs");
    let (after, _) = tell(&still, &ReadEvent::OperationEnded);
    let (_, actions) = tell(&after, &ReadEvent::Asked(ReadCause::RepositoryChosen));
    assert!(started(&actions).is_some());
}

#[test]
fn repo_watch_hidden__a_read_of_the_tab_shown_stops_the_read_of_a_tab_not_shown() {
    let (reading, actions) = tell(&HiddenTab::new(), &ReadEvent::Asked(ReadCause::RepositoryChosen));
    let number = started(&actions).unwrap();
    let (stopped, actions) = tell(&reading, &ReadEvent::Stopped);
    assert_eq!(actions, [ReadAction::Discard(number)]);
    assert!(stopped.is_waiting());
    let (_, actions) = tell(&stopped, &ReadEvent::Finished(number));
    assert_eq!(actions, [], "what the read stopped found is not shown");
}
