# Gitardo

Gitardo shows the commit graph of a repository and runs the everyday operations of Git, each as the exact git commands
it shows first when they can lose work, logs as they run, and can undo. It comes as two apps:

- **[Gitardo for VS Code](extension/README.md)**, an extension published on the Marketplace: the graph in the bottom
  panel, views in the Source Control side bar, a commit or a comparison in an editor tab.
- **[Gitardo Desktop](rust/README.md)**, a desktop application for Windows and Linux, with macOS to come: the same
  features in a window of its own.

| Folder | Holds |
| --- | --- |
| [extension/](extension/) | The extension: TypeScript, its webviews, its tests and its package. |
| [rust/](rust/) | The Cargo workspace: the engine that reads git and runs the operations, and Gitardo Desktop on top of it. |
| [docs/](docs/README.md) | How both are built, the rules of their code, the decisions and the catalog of their features. |

To change the code, start from [AGENTS.md](AGENTS.md). To report a vulnerability, see [SECURITY.md](SECURITY.md).

Gitardo is released under the [MIT license](LICENSE).
