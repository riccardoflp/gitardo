//! Quick search (`Ctrl+K`, D17): its picker opened from the box of the header, and what choosing an entry does. A
//! command whose card is not built yet does nothing; a branch, a tag or a commit is gone to in the timeline.

use views::Intent;
use views::feature::FeatureIntent;
use views::graph::GraphIntent;
use views::search::{QuickChoice, QuickCommand, SearchIntent};
use views::settings::{SettingsAnchor, SettingsIntent, SettingsTarget};
use views::text::settings::settings;

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
            // What Quick search opens starts the way to a page again from the timeline (`ui.page-trail`).
            pages::starting_again(state, |state| choose(state, choice))
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

/// What a command of the tasks asks: the picker of **Run Task…**, the Tasks tab, or the groups imported or exported.
fn of_tasks(command: QuickCommand) -> Vec<Intent> {
    use views::tasks::{QuickRunIntent, TasksIntent};
    let asked = match command {
        QuickCommand::RunTask => TasksIntent::QuickRun(QuickRunIntent::Open),
        QuickCommand::ImportGroups => TasksIntent::ImportGroups,
        QuickCommand::ExportGroups => TasksIntent::ExportGroups,
        // **Show Tasks**, the only other command of the tasks.
        _ => {
            let tab = views::panel::PanelTab::Tasks;
            return vec![Intent::Panel(views::panel::PanelIntent::Show(tab))];
        }
    };
    vec![Intent::TasksTab(asked)]
}

/// What an entry that is not a command does: a commit gone to, a setting shown, a question asked of Ask Gitardo.
fn choose_other(state: &mut State, choice: QuickChoice) -> Result<QuickCommand, Vec<Effect>> {
    match choice {
        QuickChoice::Commit(commit) => {
            let mut effects = pages::show_timeline(state);
            effects.extend(go_to(state, commit));
            Err(effects)
        }
        QuickChoice::Settings(anchor) => Err(reveal_setting(state, anchor)),
        QuickChoice::Ask(question) => {
            let ask = Intent::Help(views::help::HelpIntent::AskAbout(question));
            Err(super::super::reduce(state, Message::Intent(ask)))
        }
        QuickChoice::Command(command) => Ok(command),
    }
}

/// What an entry chosen does.
fn choose(state: &mut State, choice: QuickChoice) -> Vec<Effect> {
    let command = match choose_other(state, choice) {
        Ok(command) => command,
        Err(effects) => return effects,
    };
    if let (Some(feature), Some(id)) = (command.file_page_operation(), state.file_in_front()) {
        let run = views::blame::BlameIntent::Run(id, model::contract::FeatureId(feature), None);
        return super::super::reduce(state, Message::Intent(Intent::Blame(run)));
    }
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
        QuickCommand::OpenWorkspaceFile => vec![Intent::OpenWorkspaceFile],
        QuickCommand::OpenRepositoryInTab => vec![Intent::PickTabToOpen],
        QuickCommand::CloseTab => vec![Intent::CloseTabShown],
        QuickCommand::ShowWhatIsRunning => vec![Intent::Panel(views::panel::PanelIntent::Show(
            views::panel::PanelTab::Activity,
        ))],
        QuickCommand::ShowBranches => show_branches(state),
        QuickCommand::FilterBranchRail => {
            let mut intents = show_branches(state);
            intents.push(Intent::Rail(views::RailIntent::OpenListFilter { window: None }));
            intents
        }
        QuickCommand::ShowCommandLog => vec![Intent::ShowCommandLog],
        QuickCommand::ShowTerminal => vec![Intent::Panel(views::panel::PanelIntent::Open)],
        QuickCommand::NewTerminal => vec![Intent::Terminal(views::terminal::TerminalIntent::New)],
        QuickCommand::RunTask | QuickCommand::ShowTasks | QuickCommand::ImportGroups | QuickCommand::ExportGroups => {
            of_tasks(command)
        }
        QuickCommand::OpenSettings => vec![Intent::ShowPage(views::page::Page::Settings)],
        QuickCommand::Welcome => vec![Intent::ShowPage(views::page::Page::Welcome)],
        QuickCommand::AskGitardo => vec![Intent::Help(views::help::HelpIntent::Open)],
        QuickCommand::ShowChanges => vec![Intent::Changes(views::changes::ChangesIntent::Open)],
        QuickCommand::ShowStashes => vec![Intent::Rail(views::RailIntent::ShowStashes)],
        QuickCommand::ReviewConflicts => return super::super::review::open(state, None),
        QuickCommand::ShowFiles => vec![Intent::Files(views::files::FilesPageIntent::Open)],
        QuickCommand::SearchInFiles => vec![Intent::FileSearch(views::file_search::FileSearchIntent::Show)],
        QuickCommand::ChooseWhatNewWorktreesGet => {
            vec![Intent::WorktreeSetup(views::worktree_setup::SetupIntent::Open)]
        }
        _ => Vec::new(),
    };
    intents
        .into_iter()
        .flat_map(|intent| super::super::reduce(state, Message::Intent(intent)))
        .collect()
}

/// **Open Settings: <section>** or **Open Settings: <title>**: the settings page shown, or opened, with its list
/// scrolled there; the search of the page emptied and the tab of the user's file shown when the setting is not
/// listed otherwise.
fn reveal_setting(state: &mut State, anchor: SettingsAnchor) -> Vec<Effect> {
    let mut intents = vec![Intent::ShowPage(views::page::Page::Settings)];
    if !state.settings_page.query.is_empty() {
        intents.push(Intent::Settings(SettingsIntent::Search(String::new())));
    }
    if let SettingsAnchor::Setting(key) = anchor {
        let is_repository = settings().any(|info| info.key == key && info.is_repository);
        if state.settings_target == SettingsTarget::Repository && !is_repository {
            intents.push(Intent::ChooseSettingsFile(SettingsTarget::User));
        }
    }
    intents.push(Intent::Settings(SettingsIntent::Reveal(anchor)));
    intents
        .into_iter()
        .flat_map(|intent| super::super::reduce(state, Message::Intent(intent)))
        .collect()
}

/// **Show branches**: the timeline, with the branch rail when it was put away.
fn show_branches(state: &State) -> Vec<Intent> {
    let mut intents = vec![Intent::ShowTimeline];
    if workspace_panes(state).is_rail_hidden {
        intents.push(Intent::Graph(GraphIntent::ToggleReferences));
    }
    intents
}
