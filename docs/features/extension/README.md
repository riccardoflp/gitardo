# The catalog of features

Every feature, one card each, 14 in all. The template and the rules of the cards are in the
feature contract. This index is written by `catalog.py --index`: change a status here in the commit that
earns it, and run the script again after adding or renaming a card; the statuses are kept. A card with a
**Differs** line is *Differs*; when the line goes, it becomes *Documented*.

| File | Area | Cards |
| --- | --- | --- |
| [01-repositories.md](01-repositories.md) | Repositories | 6 |
| [16-undo-preview-log.md](16-undo-preview-log.md) | Undo, command preview and the log | 7 |
| [18-settings.md](18-settings.md) | Settings | 1 |

## Repositories

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`repo.select`](01-repositories.md#reposelect--choose-the-repository-shown) | Choose the repository shown | View | Documented |
| [`repo.follow-source-control`](01-repositories.md#repofollow-source-control--follow-the-repositories-view-of-vs-code) | Follow the Repositories view of VS Code | View | Documented |
| [`repo.tab-repository`](01-repositories.md#repotab-repository--each-tab-of-the-graph-shows-a-repository-of-its-own) | Each tab of the graph shows a repository of its own | View | Built |
| [`repo.follow-tab`](01-repositories.md#repofollow-tab--the-panel-and-the-side-bar-follow-the-tab-in-focus) | The panel and the side bar follow the tab in focus | View | Built |
| [`repo.tab-follows`](01-repositories.md#repotab-follows--the-tab-of-the-graph-that-follows-the-repository-chosen) | The tab of the graph that follows the repository chosen | View | Specified |
| [`repo.keep-tabs`](01-repositories.md#repokeep-tabs--the-tabs-of-the-graph-come-back-after-a-reload) | The tabs of the graph come back after a reload | View | Built |




## Undo, command preview and the log

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`preview.ask`](16-undo-preview-log.md#previewask--show-the-exact-commands-before-an-operation-runs) | Show the exact commands before an operation runs | Operation | Documented |
| [`preview.fallback`](16-undo-preview-log.md#previewfallback--offer-the-other-way-when-git-refuses) | Offer the other way when git refuses | Operation | Documented |
| [`log.write`](16-undo-preview-log.md#logwrite--write-every-command-and-its-output-to-the-log) | Write every command and its output to the log | Operation | Documented |
| [`log.show`](16-undo-preview-log.md#logshow--show-the-command-log) | Show the command log | View | Documented |
| [`log.report-failure`](16-undo-preview-log.md#logreport-failure--say-why-an-operation-failed) | Say why an operation failed | View | Documented |
| [`activity.track`](16-undo-preview-log.md#activitytrack--show-what-is-running-in-the-status-bar) | Show what is running in the status bar | View | Documented |
| [`activity.show`](16-undo-preview-log.md#activityshow--list-what-is-running) | List what is running | View | Documented |

## Settings

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`settings.read`](18-settings.md#settingsread--read-the-settings-when-they-are-needed) | Read the settings when they are needed | Read | Documented |
