# Every feature is a function

This is the contract every feature of Gitardo Desktop follows, and the template of the cards in
[features/](../features/README.md). Read it before you write a card or the code of one.

## Why

A feature written as one function, with everything it needs given to it and everything it does returned from it, can be
read in one place, tested without a window or a repository, previewed before it runs, logged as it runs and undone after.
The rest of the application is the machinery that feeds these functions and carries out what they return. That
machinery is written once; the features are many.

## The four kinds

| Kind | What it is | Signature | Lives in |
| --- | --- | --- | --- |
| **Read** | Asks git or the disk something and returns data. Never changes anything. | `fn read_<what>(git: &Git, request: &<What>Request) -> Result<<What>, GitError>` | `crates/git/src/read/` |
| **Operation** | Something the user does to a repository, a remote or the disk. Returns questions, then a plan. | `fn <verb>_<object>(context: &Context, answers: &Answers) -> Outcome` | `crates/features/src/<area>/` |
| **View** | How state is shown and what the user can do there. Returns the intents of the user. | `fn <name>_view(state: &<Name>State, ui: &mut Ui) -> Vec<Intent>` | `crates/views/src/<name>/` |
| **Derivation** | Turns data into other data: the lanes of the graph, a diff, a proposal for a conflict, the text of a blame line. | `fn <verb>_<object>(input: &<Input>) -> <Output>` | `crates/model/`, `crates/text/` |

A card in the catalog says which kind it is. A feature the user sees is usually one **Operation** or **View** plus the
**Reads** and **Derivations** it uses; the card names them.

## Operations in detail

An operation never runs anything itself. It is called again and again by the engine, each time with what has been
learned so far, and each time says what it needs next:

```rust
/// What an operation needs next, or what it has decided to do.
pub enum Outcome {
    /// Ask the user something; the answer comes back in `answers` under `question.key`.
    Ask(Question),
    /// Read something more from the repository; the result comes back in `answers` under `query.key`.
    Read(Query),
    /// Do not run: why, in words for the user.
    Refuse(Refusal),
    /// Run this.
    Run(Plan),
}
```

- `Context` is an immutable snapshot: the repository state as last read, the settings, the selection or the item the
  feature was invoked on (`Target`), the current time and the platform. A feature never reads the clock, the
  environment, the disk or a global.
- `Answers` holds the answers of the questions asked and the results of the queries read, by key. It starts empty.
- The function is pure: the same `context` and `answers` always give the same `Outcome`. This is what makes every
  feature testable as a table of inputs and expected outcomes.
- An operation asks at most what it cannot know. A question has a default whenever one is sensible.

### Question

```rust
pub struct Question {
    pub key: AnswerKey,
    pub kind: QuestionKind,   // Text { validate }, Pick { items, allow_custom }, Search { items }, PickMany { items }, Confirm { danger }
    pub title: Text,          // "Name of the new branch"
    pub detail: Option<Text>, // Says what will happen with the answer.
    pub default: Option<Answer>,
}
```

Questions are shown by the application in its own dialogs, pickers and input boxes. Validation of typed text is a pure
function the question carries, so that "what git does not know is refused while you type" is a validation that may run
a `Query` in the background.

### Plan

```rust
pub struct Plan {
    pub title: Text,               // "Create branch feature/login at 3f2a1c9"
    pub steps: Vec<Step>,          // What runs, in order. Exactly what the preview shows and the log records.
    pub danger: Danger,            // None, Confirm(reason), Destructive(reason): decides when the user is asked.
    pub undo: UndoRecord,          // How Undo takes it back, or Undo::ByReflog, or Undo::Impossible(reason).
    pub after: Vec<FollowUp>,      // What happens once it succeeded: select a commit, open a page, copy to the
                                   // clipboard, open a URL, offer a next step, say something.
    pub on_failure: Vec<FollowUp>, // What happens when a step fails: offer the stashes, open the conflicts.
}

pub enum Step {
    Git { arguments: Vec<OsString>, directory: PathBuf, input: Option<Vec<u8>> },
    Write { path: PathBuf, contents: Vec<u8> },
    Link { from: PathBuf, to: PathBuf },
    Copy { from: PathBuf, to: PathBuf },
    Unlink { path: PathBuf },
    Delete { path: PathBuf },
    Run { command: String, directory: PathBuf },
    Task { command: String, directory: PathBuf, name: String },
}
```

Every `Step` has one line in the command log, in a fixed form: the git command as it runs, `write <path>`,
`symlink <folder>`, `copy <path>`, `unlink <path>`, `delete <path>`, `run <command>`. There is no other way for a
feature to change anything. In particular, no feature calls `std::fs` or `std::process`: only the engine does.

### Danger and confirmation

The engine, not the feature, decides whether to show the plan before running it, from the setting `commandPreview` and
from `Plan::danger`:

| `commandPreview` | `Danger::None` | `Danger::Confirm` | `Danger::Destructive` |
| --- | --- | --- | --- |
| `off` | runs | asks, showing the steps | asks, showing the steps and the reason in a warning |
| `all` | asks, showing the steps | asks | asks with the warning |

The list of what is `Destructive` is in [16-undo-preview-log.md](../features/16-undo-preview-log.md). A card that marks an
operation destructive says why.

## Reads in detail

A read is the only place git output is parsed. It is split in two functions, so that parsing is tested on recorded
output without git:

```rust
pub fn read_branches(git: &Git, request: &BranchesRequest) -> Result<Vec<Branch>, GitError> {
    let output = git.run_read(&branches_arguments(request))?;
    parse_branches(&output)
}

fn branches_arguments(request: &BranchesRequest) -> Vec<OsString> { /* … */ }
pub fn parse_branches(output: &[u8]) -> Result<Vec<Branch>, GitError> { /* … */ }
```

Reads always ask git for a machine format (`-z`, `--porcelain=v2`, `--format` with NUL separators) and never parse what
git prints for people. They run with `GIT_OPTIONAL_LOCKS=0`, so that a read never takes a lock another program waits
for.

## Views in detail

A view is a function of its state. It draws with the `Ui` of the application and returns what the user did as
`Intent`s; it does not change state, run git or call an operation. The application turns an `Intent` into a call of the
feature it names, or into a change of the view's own state (a fold, a width, a scroll position) through
`reduce(state, intent)`.

## Identifiers

Every feature has an identifier, `area.action`, lower case, with words joined by `-`: `branch.create`,
`graph.fold-merge`, `conflicts.resolve-by-rules`. The identifier

- is the heading of its card in the catalog;
- names the function, with `-` as `_` and the area as its module: `features::branch::create`;
- is written in the doc comment of the function: `/// Feature: branch.create`;
- prefixes the name of each of its tests: `branch_create__refuses_a_name_git_does_not_accept`.

`cargo xtask check` fails when a card has no function, a function no card, or a card no test. The catalog and the code
cannot drift apart.

## The card

Each card in [features/](../features/README.md) follows this template. Leave out a line that does not apply rather than
writing "none", except **Undo** and **Danger** on an operation, which are always said.

````markdown
### `area.action` — Title in sentence case

What it is for, in one or two sentences, from the user's side.

- **Kind**: Operation | Read | View | Derivation
- **Function**: `features::area::action(context, answers) -> Outcome`
- **Entry points**: where the user finds it (the header, the context menu of a commit, Quick search, a key).
- **Offered when**: the conditions under which the entry points show it; otherwise it is not there.
- **Asks**: the questions, in order, with their defaults.
- **Reads**: the queries it needs before it can decide, and why.
- **Plan**: the steps, as they appear in the log.
- **Danger**: None | Confirm, because… | Destructive, because…
- **Undo**: how Undo takes it back, or why it cannot.
- **After**: what is selected, said or offered once it has run.
- **Refuses**: when, and what it says.
- **Settings**: the settings that change what it does.
- **Uses**: other features, reads and derivations by identifier.

Acceptance:

- [ ] A sentence a test can check, from the user's side.
- [ ] …
````

The acceptance list is what "done" means for that feature. Each line becomes at least one test.
