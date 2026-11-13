//! Quick search (`Ctrl+K`, D17): its picker opened from the box of the header, and what choosing an entry does. A
//! command whose card is not built yet does nothing; a branch, a tag or a commit is gone to in the timeline.

use views::Intent;
use views::feature::FeatureIntent;
use views::graph::GraphIntent;
use views::search::{QuickChoice, QuickCommand, SearchIntent};

use super::super::message::{Effect, Message};
use super::super::pages;
use super::super::state::State;
use super::super::timeline::workspace_panes;
use super::go_to;

/// Opens, closes, or runs what was chosen in Quick search.
pub fn reduce(state: &mut State, intent: SearchIntent) -> Vec<Effect> {
    match intent {
        SearchIntent::OpenQuick(typed) => {
            state.search.quick.is_open = true;
            state.search.quick.typed = typed;
            Vec::new()
        }
        SearchIntent::ChooseQuick(choice) => {
            close(state);
            choose(state, choice)
        }
        _ => {
            close(state);
            Vec::new()
        }
    }
}

/// Closes the picker and empties the box of the header.
fn close(state: &mut State) {
    state.search.quick.is_open = false;
    state.search.quick.typed.clear();
}

/// What an entry chosen does.
fn choose(state: &mut State, choice: QuickChoice) -> Vec<Effect> {
    let command = match choice {
        QuickChoice::Commit(commit) => {
            let mut effects = pages::show_timeline(state);
            effects.extend(go_to(state, commit));
            return effects;
        }
        QuickChoice::Command(command) => command,
    };
    if let Some(feature) = command.operation().filter(|_| super::view::is_built(command)) {
        let run = FeatureIntent::Run(model::contract::FeatureId(feature), model::contract::Target::Head);
        return super::super::reduce(state, Message::Intent(Intent::Feature(run)));
    }
    let intents = match command {
        QuickCommand::OpenTimeline => vec![Intent::ShowTimeline],
        QuickCommand::GoToCommit => vec![Intent::Search(SearchIntent::GoToCommit)],
        QuickCommand::OpenCommitAsPage => vec![Intent::Search(SearchIntent::OpenCommit)],
        QuickCommand::Refresh => vec![Intent::Refresh],
        QuickCommand::OpenFolder => vec![Intent::OpenFolder],
        QuickCommand::SelectRepository => vec![Intent::PickRepository],
        QuickCommand::ShowWhatIsRunning => vec![Intent::OpenActivity],
        QuickCommand::ShowBranches => {
            let mut intents = vec![Intent::ShowTimeline];
            if workspace_panes(state).is_rail_hidden {
                intents.push(Intent::Graph(GraphIntent::ToggleReferences));
            }
            intents
        }
        QuickCommand::ShowCommandLog => vec![Intent::OpenActivity, Intent::ShowCommandLog],
        QuickCommand::OpenSettings => vec![Intent::ShowPage(views::page::Page::Settings)],
        QuickCommand::Welcome => vec![Intent::ShowPage(views::page::Page::Welcome)],
        QuickCommand::ShowChanges => vec![Intent::Changes(views::changes::ChangesIntent::Open)],
        QuickCommand::ShowStashes => vec![Intent::Rail(views::RailIntent::ShowStashes)],
        QuickCommand::ReviewConflicts => return super::super::review::open(state, None),
        _ => Vec::new(),
    };
    intents
        .into_iter()
        .flat_map(|intent| super::super::reduce(state, Message::Intent(intent)))
        .collect()
}
