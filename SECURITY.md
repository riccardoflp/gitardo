# Security

## Supported versions

Fixes go into the next release of Gitardo on the Visual Studio Marketplace. Only the latest release is supported:
update the extension before reporting.

## Reporting a vulnerability

Please do not open a public issue for a vulnerability. Report it privately through
**TO BE CHOSEN BY THE MAINTAINER**, with the version of Gitardo, of VS Code and of git, your operating system, and the
steps that show the problem.

Once a fix is released, the CHANGELOG says what was fixed, and you are credited if you wish.

## What counts

Gitardo runs git and, when you ask, a language model and GitHub, with your rights on your machine. A vulnerability is
anything that lets a repository, a remote, GitHub or a model's answer make Gitardo do what you did not ask: run a
command or an option you did not choose, change or delete your work without asking, show content that runs script in
its views, or send your token or your code somewhere. What Gitardo protects and what it does not is described in
[docs/10-security.md](docs/10-security.md).
