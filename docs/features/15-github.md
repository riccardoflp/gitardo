# GitHub

When the main remote of a repository is on github.com, Gitardo reads its open pull requests and the result of their
checks, shows each beside its branch, opens a branch's pull request or GitHub's page to create one, and offers to create
one after the first push of a branch. Nothing is asked of GitHub before the user first chooses **Load pull requests
from GitHub**; the token is kept in the credential store of the system and sent only to `api.github.com`. The badge
on the labels is `graph.pull-request-badge`; **Open on GitHub** in a comparison is `compare.open-on-github`.

The cards of `hosting.` do the same for the other hosting services a remote can be on, GitLab, Bitbucket, Azure DevOps,
Gitea and Forgejo, and check out a pull request on every service that keeps its commits under a reference, GitHub
included ([D82](../08-decisions.md#d82--pull-requests-on-every-hosting-service-and-checked-out-proposed)). The cards of
`github.` keep their identifiers and stay the functions for GitHub; `compare.open-on-service` is the comparison on the
other services.

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
  1. The token kept by Gitardo in the credential store of the system, for `github.com`, which the engine reads and hands
     to the function in `TokenRequest::kept`, since `git` does not reach the store.
  2. Only when the request allows it, which only `github.load-pull-requests` does: `git credential fill`, with
     `protocol=https`, `host=github.com` and an empty line on its standard input, and `GIT_TERMINAL_PROMPT=0`, so that
     git never asks in a terminal. The value of `password=` in what it prints is the token; nothing printed, or an exit
     code other than 0, is no token.

The token never appears in the command log, in a message or in an error, and is not written to any file.

- **Extension**: under the extension the token is VS Code's sign-in to GitHub, not the credential store nor
  `git credential fill`: the engine asks it with `host/githubToken`, which `githubToken` in `src/extension/github.ts`
  answers with the session of `vscode.authentication.getSession('github', ['repo'])`, silently, or, with `ask` set,
  which only `github.load-pull-requests` does, after VS Code's sign-in. `githubSession` there tells
  `Controller.refreshPulls` whether VS Code has a session already.

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
- **Entry points**: **Load pull requests from GitHub** in the **…** menu of the header; **Sign in to GitHub** in Quick
  search.
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
- **Refuses**: when the main remote is not on GitHub, as Quick search may still ask: "The main remote of this
  repository is not on github.com."
- **Uses**: `github.detect-remote`, `github.token`, `github.read-pull-requests`, `graph.pull-request-badge`.
- **Extension**: **Sign in to GitHub**, in the **…** menu of the graph and in Quick search, runs it on the engine,
  whose token is VS Code's sign-in to GitHub (`host/githubToken` with `ask` set, `githubToken` in
  `src/extension/github.ts`): VS Code asks the user to sign in, and no box asks for a token. Backing out of the sign-in
  backs out of the feature, with no message (G4). VS Code keeps the session; nothing is kept in the credential store.
  When the engine's binary is missing, it says so (D49).

Acceptance:

- [ ] No request is sent to `api.github.com`, and `git credential fill` does not run, before **Load pull requests from
      GitHub** or **Sign in to GitHub** is chosen once.
- [ ] With a credential helper that knows `github.com`, it asks nothing, and the badges show.
- [ ] With no token found, it asks for one; backing out sends nothing and keeps nothing.
- [ ] After a restart of Gitardo, the pull requests are read again with the kept token, asking nothing.
- [ ] A token refused by GitHub is not kept, and the next **Load pull requests from GitHub** asks again.
- [ ] In a repository whose main remote is on another host, the **…** menu has no such entry, and the command in Quick
      search says why.

### `github.read-pull-requests` — Read the open pull requests

The open pull requests of the repository on GitHub, each with the result of the checks on its last commit, by the name
of its branch.

- **Kind**: Read
- **Function**: `engine::github::read_pull_requests(http: &Http, request: &PullsRequest) -> Result<PullRequests, GitHubError>`,
  with `engine::github::parse_pull_requests(body: &[u8], owner: &str) -> Result<PullRequests, GitHubError>`
- **Reads**: one `POST https://api.github.com/graphql`, with the headers `Authorization: Bearer <token>`,
  `Content-Type: application/json` and `User-Agent: gitardo-desktop`, and the body
  `{"query": <the query>, "variables": {"owner": <owner>, "repo": <repo>}}`. The query asks, of
  `repository(owner: $owner, name: $repo)`, for `pullRequests(first: 100, states: OPEN, orderBy: {field: UPDATED_AT,
  direction: DESC})`, and of each its `number`, `title`, `url`, `isDraft`, `headRefName`, `headRepositoryOwner { login }`
  and `commits(last: 1) { nodes { commit { statusCheckRollup { state } } } }`.
- **Uses**: `github.token`, `github.detect-remote`, `activity.track`.
- **Extension**: the engine's `read/github.read-pull-requests`, asked by `Controller.refreshPulls` when the repository
  is read and its main remote is on GitHub, only once VS Code has a session for GitHub, so that nothing asks the user
  to sign in (`reads.pullRequests` in `src/engine/reads.ts`); the checks become the badges' colors. Without the
  engine, no pull request is read and no badge is shown.

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
activity indicator of the status bar says **Reading the pull requests from GitHub** (`activity.track`). An error in a
request made on its own is kept quiet, with a warning line in the command log; one from `github.load-pull-requests` is
said. When the result is the same as before, nothing is drawn again. The pull requests known are forgotten when another
repository is shown.

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
- **Extension**: the same entries in the graph's menus and its badges; `Controller.openPullRequest` derives the
  address in TypeScript, from the pull requests the engine read, with `compareUrl` in `src/shared/github.ts`, and opens
  it with `vscode.env.openExternal`. It changes nothing and runs no git, so it needs no feature of the engine.

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
- **Extension**: `Controller.afterSync` derives it in TypeScript once a push of the engine gave the branch its upstream,
  and says **Pushed \<branch\>.** with **Create Pull Request**, which opens the address of `github.open-pull-request`.

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

### `hosting.detect-remote` — The hosting service of the main remote

Works out which hosting service the main remote of the repository is on, GitHub, GitLab, Bitbucket, Azure DevOps, Gitea or
Forgejo, and the project there, so that its pull requests can be opened, created, compared and checked out.

- **Kind**: Derivation
- **Function**: `model::hosting::detect_remote(remotes: &[Remote], known: &KnownHosts) -> Option<HostingRepo>`
- **Reads**: `remote.read-remotes`, when a repository is opened and after every change made by `remote.manage`.
- **Settings**: `hosting.selfHosted`.
- **Uses**: `remote.read-remotes`, `github.detect-remote`.
- **Extension**: the engine's, sent to the views with the state of the repository, in place of the repository on
  GitHub the extension works out for itself today (`parseGitHubRemote` in `src/shared/github.ts`,
  `Controller.detectGitHub`); the setting is `gitardo.hosting.selfHosted`.

Rules:

- The main remote is the one of `github.detect-remote`: `origin` when there is one, otherwise the first remote git
  lists.
- Its URL, without the spaces around it, is split into a host name and a path: `https://[user@]host[:port]/path`,
  `http://…`, `ssh://[user@]host[:port]/path` and `[user@]host:path`; `.git` and a `/` at the end are taken off. The
  port of an `ssh` URL is not that of the web and is dropped; that of an `https` URL is kept.
- The host name decides the service and the project:
  - `github.com`: GitHub, `<owner>/<repo>`, what `github.detect-remote` gives, which stays the function for it;
  - `gitlab.com`: GitLab, the whole path of the project, its groups and subgroups included (`group/sub/app`);
  - `bitbucket.org`: Bitbucket, `<workspace>/<repo>`;
  - `dev.azure.com` (`https://[<org>@]dev.azure.com/<org>/<project>/_git/<repo>`), `ssh.dev.azure.com`
    (`git@ssh.dev.azure.com:v3/<org>/<project>/<repo>`) and `<org>.visualstudio.com`
    (`https://<org>.visualstudio.com/[DefaultCollection/]<project>/_git/<repo>`): Azure DevOps, the organization,
    the project and the repository, on the web at `https://dev.azure.com/<org>/<project>/_git/<repo>`;
  - `codeberg.org`: Forgejo, `<owner>/<repo>`;
  - a host name of `hosting.selfHosted`, with the service it is given there, `gitlab`, `gitea` or `forgejo` (whose
    pages are Gitea's; `forgejo` only names it so in the menus), and the address of its web pages when it is given,
    otherwise `https://<host>`, with the port of an `https` URL, and `http` for a remote reached by `http` (Q1155);
    the path of a GitLab has two parts or more, that of a Gitea or a Forgejo two;
  - any other host name: nothing, and none of the entries of this area is offered.
- The result is the service, the address of the project on the web (`https://gitlab.com/group/sub/app`), its path there,
  the host name of its pages, which its API is asked at and its token kept for, the name of the remote and the URL it
  is fetched from. The branch `refs/remotes/<remote>/HEAD` names is read beside it, for `hosting.web-links`.
- When the result changes, the pull requests known are forgotten, as for `github.detect-remote`.

Acceptance:

- [ ] `git@gitlab.com:group/sub/app.git` gives GitLab, `group/sub/app`, at `https://gitlab.com/group/sub/app`.
- [ ] `https://bitbucket.org/team/app.git` and `git@bitbucket.org:team/app.git` give Bitbucket, `team/app`.
- [ ] `https://org@dev.azure.com/org/Project/_git/app` and `git@ssh.dev.azure.com:v3/org/Project/app` both give Azure
      DevOps, at `https://dev.azure.com/org/Project/_git/app`.
- [ ] `https://codeberg.org/owner/app.git` gives Forgejo, `owner/app`.
- [ ] `ssh://git@git.example.com:2222/team/app.git` gives nothing; with `hosting.selfHosted` naming `git.example.com` as
      `gitlab`, it gives GitLab at `https://git.example.com/team/app`.
- [ ] `https://github.com/owner/app.git` gives GitHub, `owner/app`, as `github.detect-remote` does.

### `hosting.web-links` — The pages of a hosting service

The addresses of the pages of a hosting service that Gitardo opens in the browser: a pull request, the page that creates one for
a branch, and the comparison of two sides.

- **Kind**: Derivation
- **Function**: `model::hosting::web_link(repo: &HostingRepo, page: &HostingPage) -> String`
- **Uses**: `hosting.detect-remote`; by `hosting.open-pull-request`, `hosting.offer-after-push` and `compare.open-on-service`.
- **Extension**: the engine's, in the follow-up `openUrl` the extension opens with `vscode.env.openExternal`.

Rules, `<web>` being the address of the project on the web (`hosting.detect-remote`) and each part of a branch name
between `/` percent-encoded, the `/` kept, as `github.open-pull-request` does:

| Service | Pull request `<n>` | Create one for `<branch>` | Compare `<a>` with `<b>` |
| --- | --- | --- | --- |
| GitHub | `<web>/pull/<n>` | `<web>/compare/<branch>?expand=1` | `<web>/compare/<a>...<b>`, `..` for everything that differs |
| GitLab | `<web>/-/merge_requests/<n>` | `<web>/-/merge_requests/new?merge_request%5Bsource_branch%5D=<branch>` | `<web>/-/compare/<a>...<b>`, with `?straight=true` for everything that differs |
| Bitbucket | `<web>/pull-requests/<n>` | `<web>/pull-requests/new?source=<branch>` | `<web>/branches/compare/<b>%0D<a>`, since they diverged only |
| Azure DevOps | `<web>/pullrequest/<n>` | `<web>/pullrequestcreate?sourceRef=<branch>` | `<web>/branchCompare?baseVersion=GB<a>&targetVersion=GB<b>`, `GC<hash>` for a commit |
| Gitea, Forgejo | `<web>/pulls/<n>` | `<web>/compare/<default branch>...<branch>` | `<web>/compare/<a>...<b>`, since they diverged only |

- GitHub's are those `github.open-pull-request` and `compare.open-on-github` open, which keep making them.
- `<default branch>` is the branch `refs/remotes/<remote>/HEAD` names, `main` when the repository does not have it.
- The addresses were taken from the pages of each service as they are in November 2026; each is opened by hand on the service
  before the release that brings it (Q1156).

Acceptance:

- [ ] On GitLab, the page to create a merge request for `feature/login` is
      `https://gitlab.com/group/app/-/merge_requests/new?merge_request%5Bsource_branch%5D=feature/login`.
- [ ] On Azure DevOps, merge request 7 is `https://dev.azure.com/org/Project/_git/app/pullrequest/7`.
- [ ] On Bitbucket, comparing `main` with `feature` since they diverged gives
      `https://bitbucket.org/team/app/branches/compare/feature%0Dmain`.
- [ ] A branch named `fix#1` is `fix%231` on every service.

### `hosting.open-pull-request` — Open or create the pull request of a branch, on another hosting service

What `github.open-pull-request` does for GitHub, on the other services: opens the merge request of a branch on GitLab, or
the page of the service that creates a pull request for it.

- **Kind**: Derivation
- **Function**: `model::hosting::pull_request_link(input: &HostingPullLinkInput) -> Option<PullLink>`
- **Entry points**: in the context menu of a local branch, on GitLab **Open merge request !`<number>`** or **Create
  merge request…** when it has none known; on Bitbucket, Azure DevOps, Gitea and Forgejo **Create pull request…**,
  since nothing is read of their pull requests (Q1157); a click on the number of a merge request on a label
  (`graph.pull-request-badge`).
- **Offered when**: the main remote is on one of those services (`hosting.detect-remote`), and, as in
  `github.open-pull-request`, the branch has an upstream that is not gone on that remote; for a remote branch, only
  **Open merge request** when one is known for its name.
- **Uses**: `hosting.detect-remote`, `hosting.web-links`, `hosting.read-merge-requests`, `github.open-pull-request`.
- **Extension**: the same entries in the menus and on the labels of the graph; `Controller.openPullRequest` asks the
  engine for the address rather than deriving it with `compareUrl` in `src/shared/github.ts`, and opens it with
  `vscode.env.openExternal`.

Rules:

- On GitHub, nothing: `github.open-pull-request` stays the function there.
- A branch with an open merge request known: the address GitLab gave for it.
- Otherwise: the page that creates one, from `hosting.web-links`, for the name the upstream has on its remote.
- GitLab names them **merge requests** and numbers them `!<n>`; the other services **pull requests** and `#<n>`.
- The address opens in the browser of the system; nothing changes in the repository, and nothing is logged.

Acceptance:

- [ ] On GitLab, `feature` following `origin/feature` with merge request !12 known has **Open merge request !12**,
      which opens its address.
- [ ] On GitLab, `feature` with none known has **Create merge request…**, which opens the page of `hosting.web-links`.
- [ ] On Bitbucket, a pushed branch has **Create pull request…** and no **Open pull request** entry.
- [ ] A branch with no upstream has neither entry, on any service.

### `hosting.offer-after-push` — Offer to create a pull request after the first push, on another hosting service

What `github.offer-after-push` does for GitHub, on the other services.

- **Kind**: Derivation
- **Function**: `model::hosting::offer_after_push(input: &HostingPushOutcome) -> Option<Notice>`
- **Entry points**: after `sync.push`, `sync.push-force-with-lease` or `sync.push-force`.
- **Uses**: `hosting.open-pull-request`, `hosting.detect-remote`, `github.offer-after-push`.
- **Extension**: `Controller.afterSync` shows the notice the engine derived, as it does for GitHub.

Rules: those of `github.offer-after-push`, the main remote being on GitLab, Bitbucket, Azure DevOps, Gitea or Forgejo:
"Pushed `<branch>`." with **Create merge request** on GitLab and **Create pull request** on the others, which opens
what `hosting.open-pull-request` gives for the branch. On GitHub, `github.offer-after-push` offers it.

Acceptance:

- [ ] Pushing a new branch `feature` to `origin` on GitLab shows "Pushed feature." with **Create merge request**.
- [ ] A second push of the same branch shows nothing.
- [ ] On GitLab, with merge request !3 known for `feature`, the first push shows nothing.

### `hosting.token` — Read the token of a GitLab

Finds the token that the API of a GitLab is asked with, as `github.token` finds GitHub's, without asking the user
anything.

- **Kind**: Read
- **Function**: `git::read::hosting::read_token(git: &Git, request: &HostingTokenRequest) -> Result<Option<Token>, GitError>`
- **Reads**:
  1. The token kept by Gitardo in the credential store of the system for its host name, which the engine reads and hands
     to the function in `HostingTokenRequest::kept`.
  2. Only when the request allows it, which only `hosting.load-merge-requests` does: `git credential fill`, with
     `protocol=https`, `host=<host>` and an empty line on its standard input, and `GIT_TERMINAL_PROMPT=0`. The value of
     `password=` is the token; nothing printed, or an exit code other than 0, is no token.
- **Uses**: `github.token`, whose rules it follows.
- **Extension**: VS Code has no sign-in to GitLab of its own; the engine asks the extension with the host question of
  the token, `host/githubToken` naming the host name (since 1.11, Q1505), which the extension answers from its secret
  storage (`ExtensionContext.secrets`), under `hosting.token:<host>`, and, when it is asked to, with an input box titled
  **GitLab token** that hides what is typed, keeping what is typed (Q1159; `tokenOfHost` in `src/extension/github.ts`).

The token never appears in the command log, in a message or in an error, and is not written to any file. A password
that a credential helper gives in place of a token is refused by GitLab's API, and `hosting.load-merge-requests` then
asks for a token.

Acceptance:

- [ ] With a token kept for `gitlab.com`, it gives it without running git.
- [ ] With none kept and a credential helper that knows `gitlab.com`, a request that allows it runs
      `git credential fill` with `host=gitlab.com` and gives the password it prints.
- [ ] A request that does not allow it never runs `git credential fill`.
- [ ] The token of one host name is never given for another.

### `hosting.load-merge-requests` — Load merge requests from GitLab

Reads the open merge requests of the project for the first time, asking for a token of GitLab if none is found, and
keeps that token for the next times.

- **Kind**: Operation
- **Function**: `features::hosting::load_merge_requests(context, answers) -> Outcome`
- **Entry points**: **Load merge requests from GitLab** in the **…** menu of the header; **Sign in to GitLab** in
  Quick search.
- **Extension**: **Sign in to GitLab** in the **…** menu of the graph and in the Command Palette, run on the engine;
  the token is asked in VS Code's input box and kept in the secret storage of the extension (`hosting.token`). When the
  engine's binary is missing, it says so (D49).
- **Offered when**: the main remote is on GitLab (`hosting.detect-remote`); in the **…** menu, only then.
- **Reads**: `hosting.token`, allowed to run `git credential fill`.
- **Asks**: only when no token was found: the token, in an input box titled **GitLab token** that hides what is
  typed, with the prompt **A personal access token of `<host>` with the scope `read_api`**; it cannot be empty.
- **Plan**: no step: nothing changes in the repository. The token found or given is kept in the credential store of the
  system for its host name, and `hosting.read-merge-requests` runs at once.
- **Danger**: None.
- **Undo**: Nothing to undo: it changes nothing in the repository.
- **After**: the labels of the branches that have an open merge request show its number (`graph.pull-request-badge`);
  from then on they are read again on their own. When GitLab refuses the token (status 401), the kept token is taken
  out of the store and Gitardo says "Could not read merge requests. GitLab answered 401 Unauthorized".
- **Refuses**: "The main remote of this repository is not on a GitLab." when Quick search asks it elsewhere.
- **Uses**: `hosting.detect-remote`, `hosting.token`, `hosting.read-merge-requests`, `graph.pull-request-badge`,
  `github.load-pull-requests`, whose rules it follows.

Acceptance:

- [ ] No request is sent to GitLab, and `git credential fill` does not run, before **Load merge requests from
      GitLab** or **Sign in to GitLab** is chosen once.
- [ ] With no token found, it asks for one; backing out sends nothing and keeps nothing.
- [ ] A token refused by GitLab is not kept, and the next **Load merge requests from GitLab** asks again.
- [ ] In a repository whose main remote is on GitHub, the **…** menu has no such entry.

### `hosting.read-merge-requests` — Read the open merge requests of GitLab

The open merge requests of the project on GitLab, each with the state of the pipeline of its last commit, by the name
of its branch: what `github.read-pull-requests` gives for GitHub, for the same labels.

- **Kind**: Read
- **Function**: `engine::hosting::read_merge_requests(http: &Http, request: &MergeRequestsRequest) -> Result<PullRequests, HostingError>`,
  with `engine::hosting::parse_merge_requests(body: &[u8]) -> Result<PullRequests, HostingError>`
- **Reads**: one `POST https://<host>/api/graphql` (under the address of the pages of a GitLab of one's own when
  `hosting.selfHosted` gives one with a path: `https://git.example.com/gitlab/api/graphql`), with the headers `Authorization: Bearer <token>`,
  `Content-Type: application/json` and `User-Agent: gitardo-desktop`, and a query that asks of
  `project(fullPath: $path)` for `mergeRequests(state: opened, first: 100, sort: UPDATED_DESC)`, and of each its `iid`,
  `title`, `webUrl`, `draft`, `sourceBranch`, `sourceProjectId`, `targetProjectId` and `headPipeline { status }`.
- **Uses**: `hosting.token`, `hosting.detect-remote`, `activity.track`, `github.read-pull-requests`, whose rules of when it
  runs it follows.
- **Extension**: the engine's read, asked by the extension when the repository is read and its main remote is on
  GitLab, once it has a token for its host name, so that nothing asks the user (`src/engine/reads.ts`).

Rules of the result:

- A merge request whose source project is not its target project comes from a fork and is left out, as
  `github.read-pull-requests` leaves out those of GitHub.
- Each branch name has one merge request, the one updated last.
- The pipeline is **passed** for `SUCCESS`; **failed** for `FAILED`; **running** for `CREATED`,
  `WAITING_FOR_RESOURCE`, `PREPARING`, `PENDING`, `RUNNING` and `SCHEDULED`; none for any other state or no pipeline.
- An answer whose status is not 2xx is an error, "GitLab answered `<status>` `<reason>`"; one with `errors`, the message
  of the first.
- Only the GitLab of the main remote is asked, and while it runs for more than half a second the activity indicator says
  **Reading the merge requests from GitLab**.

Acceptance:

- [ ] A recorded answer with !3 (`feature`, `SUCCESS`), !4 (`fix`, from a fork) and !5 (`feature`, updated before !3)
      gives one entry, `feature` → !3, passed.
- [ ] `PENDING` gives running, `CANCELED` gives none, a draft is marked draft.
- [ ] A 401 answer gives "GitLab answered 401 Unauthorized".
- [ ] No request goes to any host name but that of the main remote.

### `hosting.read-pull-request-list` — Read the open pull requests to pick from

The open pull requests of the repository, those from forks included, with their number, title, author and branch, for
the pick of `hosting.checkout-pull-request`.

- **Kind**: Read
- **Function**: `engine::hosting::read_pull_request_list(http: &Http, request: &PullListRequest) -> Result<Vec<PullListEntry>, HostingError>`
- **Reads**:
  - GitHub: the request of `github.read-pull-requests`, its query asking also for `author { login }`,
    `isCrossRepository` and `headRepository { nameWithOwner }`, and leaving no pull request out;
  - GitLab: the request of `hosting.read-merge-requests`, asking also for `author { username }` and
    `sourceProject { fullPath }`, and leaving none out;
  - Gitea, Forgejo, Bitbucket and Azure DevOps: nothing is read (Q1157).

  The token is the one kept for the service, read without running git's credential helper; with none, nothing is read
  and the pick says "Sign in to `<service>` to list the open pull requests." (Q1503)
- **Uses**: `github.token`, `hosting.token`, `github.read-pull-requests`, `hosting.read-merge-requests`.
- **Extension**: the engine's read, asked by the extension for its quick pick.

It is read each time the pick opens, and is not kept.

Acceptance:

- [ ] A recorded answer of GitHub with #3 from the repository and #4 from the fork `contributor/app` gives both, #4
      with `contributor/app` and its branch.
- [ ] A recorded answer of GitLab gives each merge request with its author and its source project.
- [ ] On Bitbucket it reads nothing and gives an empty list.

### `hosting.checkout-pull-request` — Check out a pull request

Checks out the commits of a pull request in a local branch, also one from a fork, to try it or review it.

- **Kind**: Operation
- **Function**: `features::hosting::checkout_pull_request(context, answers) -> Outcome`
- **Entry points**: **Check out a pull request…** (**Check out a merge request…** on GitLab) in Quick search and in the
  **…** menu of the header; **Check out pull request #`<n>`** in the context menu of a remote branch whose label shows a
  pull request (`graph.pull-request-badge`), in the timeline and in the branch rail.
- **Extension**: **Gitardo: Check Out Pull Request…** in the Command Palette and in the **…** menu of the graph, and the
  entry on the remote branch (`remoteBranchMenu` in `src/webview/menus.ts`), sending `feature/run` of
  `hosting.checkout-pull-request` to Gitardo's engine (`Controller.runOnEngine` in `src/extension/controller.ts`); the
  pick is a quick pick with a box to type in, the name an input box checked with `question/validate`. When the engine's
  binary is missing, it says so (D49).
- **Offered when**: the main remote is on GitHub, GitLab, Gitea or Forgejo, which keep the commits of each pull request
  under a reference of their own; no merge, rebase, cherry-pick, revert, patch or bisect is under way.
- **Asks**:
  1. From Quick search and the header: a search titled **Check out a pull request** (**merge request**), with the
     placeholder **Pick one, or type its number**: the open ones (`hosting.read-pull-request-list`), newest first, each
     `#12 Add the parser` described with its author and its branch, `contributor:fix-parser` for one from a fork. A
     number typed, `12`, `#12` or `!12`, is taken whether or not it is listed; on Gitea and Forgejo, where nothing is
     listed, it is the only way. In the window, the number not listed is typed after **Type its number…**, the last
     choice of the pick, in a box with the placeholder `#12` (`!12` on GitLab), which comes at once when nothing is
     listed and says why (Q1500).
  2. For a pull request from a fork, or one whose branch is not known: the name of the local branch, in an input box
     titled **Check out #`<n>` as**, starting from `<owner of the fork>/<branch>`, or `pull/<n>` (`merge-request/<n>`
     on GitLab) when the branch is not known (Q1151); checked by `branch.validate-name`. A name a local branch has is
     taken, and the box says "`<name>` exists: it is brought up to the pull request if it can be fast-forwarded."
- **Reads**: `hosting.read-pull-request-list` for the pick; the state of the repository, for the local branches and the
  worktrees that have them.
- **Plan**:
  - A pull request from a branch of the repository itself: `git fetch <remote> <branch>`, which brings its remote
    branch up to date, then the plan of `branch.checkout-remote` for `<remote>/<branch>`, so that the local branch
    follows it and a push goes to the pull request.
  - From a fork, or not known: `git fetch <remote> refs/pull/<n>/head:refs/heads/<name>` on GitHub, Gitea and Forgejo,
    `git fetch <remote> refs/merge-requests/<n>/head:refs/heads/<name>` on GitLab, which git refuses when the branch
    `<name>` would not be fast-forwarded; then `git checkout <name>`, then the update of the submodules
    (`submodule.update-on-checkout`).
  - When `<name>` is the branch checked out: `git pull --ff-only --no-rebase <remote> refs/pull/<n>/head` (or the
    reference of GitLab), and no checkout.
  - A pull request from a branch of the repository whose local branch exists already is fetched as one from a fork,
    into that branch, which keeps its upstream (Q1502).
- **Danger**: None, or Confirm when the update of the submodules would leave commits behind.
- **Undo**: By the reflog: Undo checks out again what was checked out before (`undo.last`); the branch stays, and
  `branch.delete` removes it.
- **After**: the tip of the pull request is selected; for one from a fork, Gitardo says "Checked out #`<n>` as
  `<name>`. It has no upstream: a push would make a branch `<name>` on `<remote>`." (Q1152)
- **On failure**: when the service has no pull request of that number, git's "couldn't find remote ref
  refs/pull/`<n>`/head" is said under **Check out pull request #`<n>`** (Q1501); when the branch would not be
  fast-forwarded, git's refusal is shown and nothing changed.
- **Refuses**: *Finish or abort the `<operation>` first.*; *End the bisect first.*; "`<name>` is checked out in
  another worktree: check out the pull request there." when it is. What is typed in the pick and is neither a pull
  request listed nor a number is refused while it is typed: "Type the number of a pull request".
- **Settings**: `submodules.updateOnCheckout`.
- **Uses**: `hosting.detect-remote`, `hosting.read-pull-request-list`, `branch.checkout-remote`, `branch.validate-name`,
  `branch.delete`, `submodule.update-on-checkout`, `graph.pull-request-badge`, `undo.last`, `sync.push`.

Acceptance:

- [ ] On GitHub, #4 from the fork `contributor/app`, branch `fix-parser`, proposes `contributor/fix-parser`, and runs
      `git fetch origin refs/pull/4/head:refs/heads/contributor/fix-parser` and
      `git checkout contributor/fix-parser`; the tip of #4 is selected.
- [ ] On GitLab, `!7` typed runs `git fetch origin refs/merge-requests/7/head:refs/heads/merge-request/7`.
- [ ] #3 from the branch `feature` of the repository runs `git fetch origin feature`, then
      `git checkout --track origin/feature`, and `feature` follows `origin/feature` after.
- [ ] Checking out #4 again after the fork pushed more commits brings `contributor/fix-parser` up to them.
- [ ] A number with no pull request says so, and no branch is made.
- [ ] Undo right after goes back to the branch checked out before.
- [ ] On Bitbucket, neither entry is offered.
- [ ] Extension: **Gitardo: Check Out Pull Request…**, `4` typed, runs the same fetch and checkout in the engine, and
      the log has them.
