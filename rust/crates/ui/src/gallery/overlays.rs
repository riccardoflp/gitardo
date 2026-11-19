//! What opens over the gallery and waits for an answer: a dialog that asks for a name, the dialog of Pull with its
//! choices, the dialog that says a commit needs a message, a confirmation of something that cannot be undone, Quick
//! search, and pickers of one branch or of several.

use render::Rect;

use super::{Field, GalleryIntent, GalleryState, Overlay, Switch};
use crate::frame::{TextKind, Ui};
use crate::layout::{Area, Size, columns};
use crate::widget::Tone;
use crate::widget::choice::Choice;
use crate::widget::dialog::{ConfirmAnswer, ConfirmStyle, Confirmation, Dialog, DialogButton, DialogRole};
use crate::widget::input_box::InputBox;
use crate::widget::label::Label;
use crate::widget::metrics;
use crate::widget::picker::{Picker, PickerAnswer, PickerItem, PickerMode};

/// The branches the pickers offer, with what they say about each.
const PICKER_BRANCHES: [PickerItem<'static>; 10] = [
    PickerItem {
        label: "main",
        description: "origin/main, yesterday",
    },
    PickerItem {
        label: "feature/login-page",
        description: "2 ahead, checked out",
    },
    PickerItem {
        label: "feature/search",
        description: "1 ahead, 4 behind",
    },
    PickerItem {
        label: "fix/parser",
        description: "merged",
    },
    PickerItem {
        label: "release/2.4",
        description: "tag v2.4.0",
    },
    PickerItem {
        label: "spike/graph-colors",
        description: "5 ahead",
    },
    PickerItem {
        label: "feature/file-history",
        description: "worktree",
    },
    PickerItem {
        label: "docs/readme",
        description: "merged",
    },
    PickerItem {
        label: "refactor/engine-plans",
        description: "2 weeks ago",
    },
    PickerItem {
        label: "hotfix/fetch-timeout",
        description: "origin/hotfix/fetch-timeout",
    },
];

/// What Quick search offers: commits, branches, files and commands.
const QUICK_SEARCH: [PickerItem<'static>; 8] = [
    PickerItem {
        label: "Add the login page",
        description: "commit 4b830a6",
    },
    PickerItem {
        label: "feature/login-page",
        description: "branch, checked out",
    },
    PickerItem {
        label: "src/login/page.rs",
        description: "file",
    },
    PickerItem {
        label: "Fetch",
        description: "command",
    },
    PickerItem {
        label: "Switch to a branch",
        description: "command, Ctrl+P",
    },
    PickerItem {
        label: "Create a branch",
        description: "command",
    },
    PickerItem {
        label: "Find in page",
        description: "command, Ctrl+F",
    },
    PickerItem {
        label: "main",
        description: "branch",
    },
];

/// The ways Pull brings the commits of the upstream.
const STRATEGIES: [&str; 3] = ["Merge", "Rebase", "Fast-forward"];

/// The key of the pickers.
pub(super) const PICKER: &str = "picker";

/// Declares the overlay open, if any.
pub(super) fn overlay(state: &GalleryState, ui: &mut Ui<'_>, intents: &mut Vec<GalleryIntent>) {
    match state.overlay {
        None => {}
        Some(Overlay::CreateBranch) => create_branch(state, ui, intents),
        Some(Overlay::ConfirmDelete) => confirm_delete(ui, intents),
        Some(Overlay::PickBranch) => pick(ui, PickerMode::Single, intents),
        Some(Overlay::PickBranches) => pick(ui, PickerMode::Multiple, intents),
        Some(Overlay::QuickSearch) => quick_search(ui, intents),
        Some(Overlay::Pull) => pull(state, ui, intents),
        Some(Overlay::EmptyMessage) => empty_message(ui, intents),
    }
}

/// A dialog that asks for the name of a new branch.
fn create_branch(state: &GalleryState, ui: &mut Ui<'_>, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let control = metrics::control_height(&theme);
    let row = theme.sizes.row_height;
    let buttons = [
        DialogButton {
            label: "Cancel",
            role: DialogRole::Cancel,
        },
        DialogButton {
            label: "Create branch",
            role: DialogRole::Primary,
        },
    ];
    let dialog = Dialog {
        title: "Create a branch",
        body_height: row + control + theme.sizes.spacing.medium,
        buttons: &buttons,
    };
    let response = ui.dialog("create branch", &dialog, |ui, body| {
        let mut area = Area::with_gap(body.rect, theme.sizes.spacing.medium);
        let _ = ui.label(area.top(row), &Label::new("Name of the new branch").tone(Tone::Muted));
        let input = InputBox::new(&state.new_branch, "Branch name");
        let field = area.top(control);
        if body.is_opening {
            let id = ui.id("name");
            ui.request_focus(id);
        }
        if let Some(text) = ui.input_box("name", field, &input).changed_text {
            intents.push(GalleryIntent::Typed(Field::NewBranch, text));
        }
    });
    if response.is_cancelled {
        intents.push(GalleryIntent::Close);
    } else if response.pressed.is_some() {
        intents.push(GalleryIntent::Did(format!("Created {}", state.new_branch.trim())));
    }
}

/// A confirmation of the deletion of a branch, with the commands that will run.
fn confirm_delete(ui: &mut Ui<'_>, intents: &mut Vec<GalleryIntent>) {
    let commands = [
        "git branch --delete --force feature/login-page",
        "git push origin --delete feature/login-page",
    ];
    let confirmation = Confirmation {
        title: "Delete feature/login-page?",
        message: "The branch and its remote branch on origin are deleted. Its commits are not merged anywhere: Undo \
                  can bring the branch back, the remote branch only by pushing it again.",
        commands: &commands,
        confirm: "Delete",
        other: None,
        style: ConfirmStyle::Danger,
    };
    match ui.confirm("confirm delete", &confirmation) {
        Some(ConfirmAnswer::Confirmed) => intents.push(GalleryIntent::Did(String::from("Deleted feature/login-page"))),
        Some(ConfirmAnswer::Cancelled | ConfirmAnswer::Other) => intents.push(GalleryIntent::Close),
        None => {}
    }
}

/// A picker of one branch or of several.
fn pick(ui: &mut Ui<'_>, mode: PickerMode, intents: &mut Vec<GalleryIntent>) {
    let placeholder = match mode {
        PickerMode::Single => "Switch to a branch",
        PickerMode::Multiple => "Choose the branches to show in the timeline",
    };
    let picker = Picker {
        placeholder,
        items: &PICKER_BRANCHES,
        mode,
        query: "",
        checked: &[],
    };
    let label = |index: usize| PICKER_BRANCHES.get(index).map_or("", |item| item.label);
    match ui.picker(PICKER, &picker) {
        Some(PickerAnswer::Chose(index)) => intents.push(GalleryIntent::Did(format!("Switched to {}", label(index)))),
        Some(PickerAnswer::ChoseMany(chosen)) => {
            let names: Vec<&str> = chosen.into_iter().map(label).collect();
            intents.push(GalleryIntent::Did(format!("Showing {}", names.join(", "))));
        }
        Some(PickerAnswer::Cancelled) => intents.push(GalleryIntent::Close),
        None => {}
    }
}

/// Quick search: commits, branches, files and commands, filtered by what is typed.
fn quick_search(ui: &mut Ui<'_>, intents: &mut Vec<GalleryIntent>) {
    let picker = Picker {
        placeholder: "Commits, branches, files, commands",
        items: &QUICK_SEARCH,
        mode: PickerMode::Single,
        query: "",
        checked: &[],
    };
    match ui.picker(PICKER, &picker) {
        Some(PickerAnswer::Chose(index)) => {
            let label = QUICK_SEARCH.get(index).map_or("", |item| item.label);
            intents.push(GalleryIntent::Did(format!("Opened {label}")));
        }
        Some(PickerAnswer::Cancelled) => intents.push(GalleryIntent::Close),
        // A picker of one item never answers with several.
        Some(PickerAnswer::ChoseMany(_)) | None => {}
    }
}

/// The dialog of Pull: how it brings the commits, and what it does before and around it.
fn pull(state: &GalleryState, ui: &mut Ui<'_>, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let control = metrics::control_height(&theme);
    let gap = theme.sizes.spacing.medium;
    let buttons = [
        DialogButton {
            label: "Cancel",
            role: DialogRole::Cancel,
        },
        DialogButton {
            label: "Pull",
            role: DialogRole::Primary,
        },
    ];
    let dialog = Dialog {
        title: "Pull feature/login-page",
        body_height: theme.sizes.row_height + 3.0 * control + 3.0 * gap,
        buttons: &buttons,
    };
    let response = ui.dialog("pull", &dialog, |ui, body| pull_choices(state, ui, body.rect, intents));
    if response.is_cancelled {
        intents.push(GalleryIntent::Close);
    } else if response.pressed.is_some() {
        let way = STRATEGIES
            .get(state.pull.strategy)
            .copied()
            .unwrap_or_default()
            .to_lowercase();
        intents.push(GalleryIntent::Did(format!("Pulled feature/login-page, {way}")));
    }
}

/// The choices in the body of the dialog of Pull: the way, as radio buttons, then check boxes and a toggle switch.
fn pull_choices(state: &GalleryState, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let control = metrics::control_height(&theme);
    let gap = theme.sizes.spacing.medium;
    let mut area = Area::with_gap(rect, gap);
    let words = Label::new("Bring the commits of origin/feature/login-page by").tone(Tone::Muted);
    let _ = ui.label(area.top(theme.sizes.row_height), &words);
    let radios = columns(
        area.top(control),
        gap,
        [Size::Fill(1.0), Size::Fill(1.0), Size::Fill(1.3)],
    );
    for (index, (rect, label)) in radios.into_iter().zip(STRATEGIES).enumerate() {
        let choice = Choice::radio(label, state.pull.strategy == index);
        if ui.choice(("strategy", index), rect, &choice).clicked {
            intents.push(GalleryIntent::Chose(index));
        }
    }
    let [fetch, prune] = columns(area.top(control), gap, [Size::Fill(1.0), Size::Fill(1.0)]);
    let switches = [
        (
            fetch,
            Switch::FetchFirst,
            Choice::check_box("Fetch before pulling", state.pull.fetch_first),
        ),
        (
            prune,
            Switch::Prune,
            Choice::check_box("Prune remote branches", state.pull.prune),
        ),
        (
            area.top(control),
            Switch::Autostash,
            Choice::toggle("Keep the changes aside while pulling", state.pull.autostash),
        ),
    ];
    for (rect, switch, choice) in switches {
        if ui.choice(switch, rect, &choice).clicked {
            intents.push(GalleryIntent::Toggled(switch));
        }
    }
}

/// The dialog that says a commit needs a message.
fn empty_message(ui: &mut Ui<'_>, intents: &mut Vec<GalleryIntent>) {
    let buttons = [DialogButton {
        label: "Write the message",
        role: DialogRole::Primary,
    }];
    let text = "Say in the message of the draft what the commit does, then commit again.";
    let style = ui.text_style(TextKind::Body);
    let lines = ui.fonts().wrap(text, style, ui.dialog_body_width()).len();
    let dialog = Dialog {
        title: "The commit needs a message",
        body_height: crate::number::float_of_count(lines) * ui.line_height(style),
        buttons: &buttons,
    };
    let response = ui.dialog("empty message", &dialog, |ui, body| {
        let _ = ui.paragraph(body.rect, text, (TextKind::Body, Tone::Normal));
    });
    if response.is_cancelled || response.pressed.is_some() {
        intents.push(GalleryIntent::Close);
    }
}
