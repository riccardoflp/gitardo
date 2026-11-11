# GitHub

When the main remote of a repository is on github.com, Gitardo reads its open pull requests and the result of their
checks, shows each beside its branch, opens a branch's pull request or GitHub's page to create one, and offers to create
one after the first push of a branch. Nothing is asked of GitHub before the user first chooses **Load pull requests
from GitHub**; the token is kept in the credential store of the system and sent only to `api.github.com`. The badge
on the labels is `graph.pull-request-badge`; **Open on GitHub** in a comparison is `compare.open-on-github`.

### `github.detect-remote` — The repository on GitHub

Works out whether the main remote of the repository is a repository on github.com, and which.

- **Kind**: Derivation
- **Function**: `model::github::detect_remote(remotes: &[Remote]) -> Option<GitHubRepo>`
- **Reads**: `remote.read-remotes`, when a repository is opened and after every change made by `remote.manage`.
- **Uses**: `remote.read-remotes`.

Rules:

- The main remote is `origin` when there is one, otherwise the first remote git lists. No other remote is looked at.
- Its URL, without the spaces around it, is on GitHub when it is one of:
  - `https://github.com/<owner>/<repo>` or `http://github.com/<owner>/<repo>`, with or without a user before the host
    (`https://user@github.com/…`);
  - `ssh://git@github.com/<owner>/<repo>`;
  - `git@github.com:<owner>/<repo>`;

  each with or without `.git` and a `/` at the end. The result is the owner and the repository, `.git` taken off.
- Any other host, GitHub Enterprise included, is not GitHub: no pull request is read, and the entries of this area are
  not offered.
- When the result changes, the pull requests known are forgotten, and are read again on the next occasion.

Acceptance:

- [ ] `https://github.com/owner/app.git`, `git@github.com:owner/app.git` and `ssh://git@github.com/owner/app` all give
      `owner` and `app`.
- [ ] `https://user@github.com/owner/app/` gives `owner` and `app`.
- [ ] `https://gitlab.com/owner/app.git` and `git@example.com:owner/app.git` give nothing.
- [ ] With `upstream` on GitHub and `origin` on another host, the result is nothing: `origin` is the main remote.
- [ ] With only a remote `fork` on GitHub, the result is its repository.
- [ ] Renaming the remote on GitHub to something other than `origin`, when another remote is `origin`, makes the badges
      go.

### `github.token` — Read the GitHub token

Finds the token that `api.github.com` is asked with, without asking the user anything.

- **Kind**: Read
- **Function**: `git::read::github::read_token(git: &Git, request: &TokenRequest) -> Result<Option<Token>, GitError>`
- **Reads**:
  1. The token kept by Gitardo in the credential store of the system, for `github.com`.
  2. Only when the request allows it, which only `github.load-pull-requests` does: `git credential fill`, with
     `protocol=https`, `host=github.com` and an empty line on its standard input, and `GIT_TERMINAL_PROMPT=0`, so that
     git never asks in a terminal. The value of `password=` in what it prints is the token; nothing printed, or an exit
     code other than 0, is no token.

The token never appears in the command log, in a message or in an error, and is not written to any file.

Acceptance:

- [ ] With a token kept in the credential store, it gives it without running git.
- [ ] With none kept and a credential helper that knows `github.com`, a request that allows it runs
      `git credential fill` and gives the password it prints.
- [ ] A request that does not allow it never runs `git credential fill`.
- [ ] With no helper and nothing kept, it gives no token, and no prompt of git shows.
- [ ] The command log at `debug` shows `git credential fill`, never what it printed.

### `github.load-pull-requests` — Load pull requests from GitHub

Reads the open pull requests of the repository for the first time, asking for a GitHub token if none is found, and
keeps that token for the next times.

- **Kind**: Operation
- **Function**: `features::github::load_pull_requests(context, answers) -> Outcome`
- **Entry points**: **Load pull requests from GitHub** in the **…** menu of the toolbar; **Sign in to GitHub** in the
  command palette.
- **Offered when**: the main remote is on GitHub (`github.detect-remote`); in the **…** menu, only then.
- **Reads**: `github.token`, allowed to run `git credential fill`.
- **Asks**: only when no token was found: the token, in an input box titled **GitHub token** that hides what is typed,
  with the prompt **A token that can read the pull requests of `<owner>/<repo>`**; it cannot be empty.
- **Plan**: no step: nothing changes in the repository. The token found or given is kept in the credential store of the
  system, for `github.com`, and `github.read-pull-requests` runs at once, whatever was read last.
- **Danger**: None.
- **Undo**: Nothing to undo: it changes nothing in the repository.
- **After**: the labels of the branches that have an open pull request show it (`graph.pull-request-badge`); from then
  on the pull requests are read again on their own (`github.read-pull-requests`). When GitHub refuses the token (status
  401), the kept token is taken out of the credential store, so that the next **Load pull requests from GitHub** asks
  for one again, and Gitardo says "Could not read pull requests. GitHub answered 401 Unauthorized".
- **Refuses**: when the main remote is not on GitHub, as the command palette may still ask: "The main remote of this
  repository is not on github.com."
- **Uses**: `github.detect-remote`, `github.token`, `github.read-pull-requests`, `graph.pull-request-badge`.

Acceptance:

- [ ] No request is sent to `api.github.com`, and `git credential fill` does not run, before **Load pull requests from
      GitHub** or **Sign in to GitHub** is chosen once.
- [ ] With a credential helper that knows `github.com`, it asks nothing, and the badges show.
- [ ] With no token found, it asks for one; backing out sends nothing and keeps nothing.
- [ ] After a restart of Gitardo, the pull requests are read again with the kept token, asking nothing.
- [ ] A token refused by GitHub is not kept, and the next **Load pull requests from GitHub** asks again.
- [ ] In a repository whose main remote is on another host, the **…** menu has no such entry, and the palette command
      says why.

### `github.read-pull-requests` — Read the open pull requests

The open pull requests of the repository on GitHub, each with the result of the checks on its last commit, by the name
of its branch.

- **Kind**: Read
- **Function**: `net::github::read_pull_requests(http: &Http, request: &PullsRequest) -> Result<PullRequests, GitHubError>`,
  with `net::github::parse_pull_requests(body: &[u8], owner: &str) -> Result<PullRequests, GitHubError>`
- **Reads**: one `POST https://api.github.com/graphql`, with the headers `Authorization: Bearer <token>`,
  `Content-Type: application/json` and `User-Agent: gitardo-desktop`, and the body
  `{"query": <the query>, "variables": {"owner": <owner>, "repo": <repo>}}`. The query asks, of
  `repository(owner: $owner, name: $repo)`, for `pullRequests(first: 100, states: OPEN, orderBy: {field: UPDATED_AT,
  direction: DESC})`, and of each its `number`, `title`, `url`, `isDraft`, `headRefName`, `headRepositoryOwner { login }`
  and `commits(last: 1) { nodes { commit { statusCheckRollup { state } } } }`.
- **Uses**: `github.token`, `github.detect-remote`, `activity.track`.

Rules of the result:

- A pull request whose head repository's owner is not the owner of the repository, compared without regard to case, is
  left out: a pull request from a fork says nothing about the branches here. One with no head repository is left out too.
- Each branch name has one pull request: the first in the list, the one updated last.
- Its checks are **passed** for `SUCCESS`; **failed** for `FAILURE` and `ERROR`; **running** for `PENDING` and
  `EXPECTED`; and none when the last commit has no checks or another state.
- An answer whose status is not 2xx is an error, "GitHub answered `<status>` `<reason>`"; an answer with `errors` is
  an error with the message of the first one.

When it runs:

- At once, from `github.load-pull-requests`.
- After each successful fetch, pull or push of `sync.`, whatever was read last.
- After each read of the repository while it is shown, when a token is kept and the last request is more than 120
  seconds old; never with no token kept.

Only the main remote's repository is asked, and only `api.github.com`. While it runs for more than half a second, the
status bar says **Reading the pull requests from GitHub** (`activity.track`). An error in a request made on its own is
kept quiet, with a warning line in the command log; one from `github.load-pull-requests` is said. When the result is
the same as before, nothing is drawn again. The pull requests known are forgotten when another repository is shown.

Acceptance:

- [ ] A recorded answer with pull requests #3 (`feature`, `SUCCESS`), #4 (`fix`, from a fork) and #5 (`feature`,
      updated before #3) gives one entry, `feature` → #3, checks passed.
- [ ] `ERROR` gives failed, `EXPECTED` gives running, no `statusCheckRollup` gives none.
- [ ] A draft pull request is marked draft.
- [ ] A 403 answer gives the error "GitHub answered 403 Forbidden"; a `{"errors":[{"message":"Bad credentials"}]}`
      answer gives "Bad credentials".
- [ ] Two reads of the repository 30 seconds apart send one request, not two.
- [ ] A push that succeeded is followed by a request, even 10 seconds after the last.
- [ ] No request goes to any host but `api.github.com`.

### `github.open-pull-request` — Open or create the pull request of a branch

Opens the pull request of a branch in the browser, or GitHub's page to create one.

- **Kind**: Derivation
- **Function**: `model::github::pull_request_link(input: &PullLinkInput) -> Option<PullLink>`
- **Entry points**: in the context menu of a local branch, **Open pull request #`<number>`**, or **Create pull
  request…** when it has none; in the context menu of a remote branch, **Open pull request #`<number>`**; a click on the
  badge of a label (`graph.pull-request-badge`).
- **Offered when**: the main remote is on GitHub. For a local branch: it has an upstream that is not gone, and the name
  that upstream has on its remote is the name looked up. For a remote branch: only when a pull request is known for its
  name without its remote; there is no **Create pull request…** on a remote branch.
- **Uses**: `github.read-pull-requests`, `github.detect-remote`.

Rules:

- A branch with an open pull request known: the address of that pull request, as GitHub gave it.
- Otherwise: `https://github.com/<owner>/<repo>/compare/<branch>?expand=1`, where each part of the branch name between
  `/` is percent-encoded and the `/` are kept: GitHub's page to open a pull request of that branch.
- The address opens in the browser of the system; nothing changes in the repository, and nothing is logged.

Acceptance:

- [ ] `feature`, following `origin/feature`, with pull request #12 known, has **Open pull request #12**, which opens its
      address.
- [ ] `feature/login` with no pull request has **Create pull request…**, which opens
      `https://github.com/owner/app/compare/feature/login?expand=1`.
- [ ] A branch named `fix#1` opens `…/compare/fix%231?expand=1`.
- [ ] A local branch with no upstream, or whose upstream is gone, has neither entry.
- [ ] `origin/feature` with no pull request known has no entry; with #7 known, **Open pull request #7**.
- [ ] Before the pull requests are loaded, a pushed branch has **Create pull request…**.

### `github.offer-after-push` — Offer to create a pull request after the first push

The first push of a branch is when a pull request is usually wanted: Gitardo offers to open GitHub's page to create it.

- **Kind**: Derivation
- **Function**: `model::github::offer_after_push(input: &PushOutcome) -> Option<Notice>`
- **Entry points**: after `sync.push`, `sync.push-force-with-lease` or `sync.push-force`.
- **Uses**: `github.open-pull-request`, `github.detect-remote`.

Rules:

- It offers only when the push succeeded, a branch is checked out, that branch had no upstream before the push (so that
  the push set it), the main remote is on GitHub, and no open pull request is known for the branch's name.
- The notice says "Pushed `<branch>`." with **Create pull request**, which opens what `github.open-pull-request` gives
  for the branch: the page to create one.
- Otherwise it offers nothing.

Acceptance:

- [ ] Pushing a new branch `feature` to `origin` on GitHub shows "Pushed feature." with **Create pull request**, which
      opens `https://github.com/<owner>/<repo>/compare/feature?expand=1`.
- [ ] A second push of the same branch shows nothing.
- [ ] Pushing a new branch in a repository whose main remote is not on GitHub shows nothing.
- [ ] A push that failed shows nothing.
