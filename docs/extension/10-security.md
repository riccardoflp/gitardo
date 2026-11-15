# Security

What Gitardo protects, what could go wrong, and what it does about it. It runs with the user's rights on the user's
machine, as VS Code does; it has one secret it uses (the GitHub token, which VS Code keeps), talks to two services when
the user lets it, and reads a great deal that it does not control: every repository it is opened on.

*Source: the Threat Modeling Manifesto (2020) for the four questions; STRIDE (Adam Shostack, "Threat Modeling", 2014)
for finding threats; CWE-88 (argument injection) and CWE-79 (cross-site scripting) for the two classic ones here.*

## What we are working on

The [context diagram](02-architecture.md#context) is the picture: Gitardo between VS Code and git, with the repository,
GitHub and the language model outside the trust boundary. What is worth protecting:

- **The user's work**: commits, branches, stashes, uncommitted changes. Losing them is the harm Gitardo is most able to
  do, and the one most of its design is about (see the [promises](04-feature-contract.md#the-promises-every-card-is-checked-against)).
- **The user's machine**: Gitardo runs git, and in one place commands of the user's settings, with the user's rights.
- **The GitHub token**, with the `repo` scope.
- **The user's code**, which leaves the machine only when the user asks a language model about it.

Who is not trusted: the contents of a repository (a clone of someone else's project, its file names, messages, branch
names, `.gitmodules`, hooks, `.vscode/settings.json`), the answers of GitHub, and the answers of the language model.

## What can go wrong

| | STRIDE | The threat, concretely | Where | Mitigation | Status |
| --- | --- | --- | --- | --- | --- |
| T1 | Tampering, Elevation | A file named `--output=…` or a branch name that git reads as an option changes what a command does. | Every command built from a name | R8.1 (no shell), R8.3 (`arg()` refuses a leading `-`; files after `--` and as `:(literal)` when they are patterns) | Mitigated |
| T2 | Information disclosure | The GitHub token leaks: logged, shown in a command preview, sent to another host. | `src/extension/github.ts`, `src/engine/client.ts` | R13.1: asked of VS Code at each use, sent only in the `Authorization` header to `https://api.github.com/graphql`, never logged. The engine asks it with the host question `host/githubToken`, only when the extension said at `initialize` that it answers it, for each read of the pull requests, never to sign in; it keeps it for that request only and sends it only as a secret header, which its log and its errors never show | Mitigated |
| T3 | Information disclosure | Code leaves the machine without the user knowing. | The language model; GitHub | R13.3: the model is asked only when the user asks for a draft or a resolution, and the AI resolution is off by default (`conflict.resolve-ai`); GitHub is asked only for the open pull requests of a repository on github.com, with a session the user already granted | Mitigated |
| T4 | Tampering, Elevation | A commit message, branch or file name containing markup runs script in a webview. | The webviews | R13.2: React escapes all text; the two uses of `innerHTML` in `src/flow/webview` write fixed markup; the Content Security Policy allows only the extension's scripts and a nonce | Mitigated |
| T5 | Information disclosure | Gitardo calls a service on its own, as telemetry would. | Network | R13.3: no telemetry, no call but the two above | Mitigated |
| T6 | Elevation of privilege | A cloned repository's `.vscode/settings.json` sets `gitardo.worktrees.setupCommands`, and creating a worktree runs them. | `worktree.create` | VS Code runs Gitardo only in a trusted workspace (no `untrustedWorkspaces` capability); the commands are said in the question before the worktree is created (R13.4) | Mitigated while the workspace is trusted only by choice; see the card |
| T7 | Tampering | A wrong resolution, from the rules or the model, is written into the user's files. | `conflict.resolve-rules`, `conflict.resolve-ai`, `conflict.review` | Proposals are written only once accepted; files are written all or none, only if still what they were read as (`writeFiles`); `undo.last` marks the conflicts again | Mitigated |
| T8 | Elevation of privilege | Git hooks of the repository run when Gitardo commits, checks out, merges or rebases. | git | Git's own behavior, the same as in a terminal; Gitardo runs only in a trusted workspace | Not defended (below) |
| T9 | Denial of service | A huge repository, diff or conflict makes the views hang. | Reads, the tab of a commit, the rules of conflicts | Pages of commits, lists cut at 100, diffs capped at 20,000 lines, rules give up past 50,000 lines (see [12-quality.md](12-quality.md)) | Mitigated in part |
| T10 | Repudiation | Afterwards, no one can tell what Gitardo ran. | The log | `log.write` writes every command; the output channel is lost when the window closes, and the reflog keeps what moved | Accepted |
| T11 | Spoofing | An answer that is not GitHub's says where a pull request is, and its URL is opened. | `src/extension/controller.ts`, `openPullRequest` | HTTPS to `api.github.com` with the system's certificates; the URL is opened by VS Code's `openExternal`, which asks before leaving for a host the user has not trusted | Mitigated by VS Code |
| T12 | Information disclosure | The messages of an interactive rebase stay in the temporary folder after the rebase stopped or was cancelled. | `interactiveRebase` | None yet | Open ([RS7](13-risks.md)) |
| T13 | Tampering | The nonce of the webviews' Content Security Policy is made with `Math.random`, not a cryptographic source. | `Controller.html` | The webview loads only the extension's own files; no content of a repository is a script | Accepted |
| T14 | Tampering, Elevation | Another program runs as Gitardo's engine: a binary put in the package's `bin/`, or one that `gitardo.engine.path` names. | `src/engine/binary.ts` | The binary in `bin/` is the package's own, and changing it needs the rights that change the extension; `gitardo.engine.path`, a setting for development, is read only from the user's own settings, never from a workspace's `.vscode/settings.json` | Mitigated |

## What we do about it

Each mitigation is a rule of [03-code-rules.md](03-code-rules.md) (R8 for running git, R13 for secrets and untrusted
input), a line of a card's acceptance, or a decision. The threats that are open are rows of [the risks](13-risks.md).

## Secrets

| Secret | Kept by | Read by | Never in |
| --- | --- | --- | --- |
| GitHub token (`repo` scope) | VS Code's authentication provider | `githubSession` at each request, silently unless the user runs **Gitardo: Sign In to GitHub**; the engine, through `host/githubToken`, for one read of the pull requests | The log, a command preview, the workspace state, the webviews, any request but GitHub's, the engine's memory once its read is answered |
| Credentials of remotes | git's credential helper, the SSH agent | git | Gitardo: it never asks for them, and `GIT_TERMINAL_PROMPT=0` keeps git from asking on a terminal it does not have |

## Input that is not trusted

| Source | What Gitardo does with it |
| --- | --- |
| Names in the repository: files, branches, tags, remotes, stashes | Passes them to git as separate arguments, after `--` or as `:(literal)` where they are paths; refuses a name it builds into a command when it starts with `-`; shows them as text. |
| Commit messages, author names | Shows them as text; the message rules only cut and join lines. |
| `.gitmodules`, submodule URLs | Shows them; the URL is used only by git's own `git submodule` commands, with git's protections. |
| What git prints | Parsed from machine-readable formats where git has one (R8.4). |
| Conflicted files | Read, compared and written only as text; the rules give up on very large ones. |
| Settings of the workspace | Read only in a trusted workspace; the setup commands are said before they run. |
| What Gitardo's engine writes | Read as framed JSON-RPC messages; a message that cannot be read is said in the log and dropped; what it writes on its standard error is copied into the log as text. The engine reads its standard input only from the extension that started it. |
| GitHub's answers | Parsed as JSON into the pull requests of the branches; a failure is a warning in the log. |
| The language model's answers | Shown as proposals; written only once the user accepts them. |

## What is not defended

- A repository whose hooks, filters or configuration run code: that is git's behavior, and opening the folder in a
  trusted workspace is the user's choice.
- A user, or a program, with the rights of the user on the machine: they can do anything Gitardo can.
- The language model's provider, once the user chose to send it a conflict or a diff.
- VS Code itself and its extension host.

## When to look at this page again

Before a release; and in the commit that adds a service Gitardo talks to, a secret, a new kind of file it writes, or a
place where it runs something other than git.
