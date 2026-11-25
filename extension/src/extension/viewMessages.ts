/**
 * What the views ask of the extension, each message to the part of the controller that answers it, about the repository
 * the group of the view shows, or the one of the tab of a commit, of a comparison and the like. A new message is a case
 * here (`docs/extension/02-architecture.md`, Adding to the UI).
 */
import * as path from 'node:path';
import * as vscode from 'vscode';
import { reads } from '../engine/reads';
import { GRAPH_COLUMNS, WORKTREE, type WebviewMessage } from '../shared/protocol';
import { FILES_KEY } from './commitTabs';
import type { ControllerHost, ControllerParts } from './controllerHost';
import { keepOpen, pageSize } from './repositorySession';
import { editSetupJson } from './worktreeSettings';

/** The most commits the view of the commits of a branch is given, however many it asks for. */
const LOG_LIMIT = 10_000;

/** Answers the messages of every view. */
export class ViewMessages {
  /** The read of the signature each view waits for, cancelled when it asks for another (`details.signature`). */
  readonly signatureReads = new Map<vscode.Webview, AbortController>();

  constructor(
    private readonly host: ControllerHost,
    private readonly parts: ControllerParts,
  ) {}

  async onMessage(webview: vscode.Webview, message: WebviewMessage): Promise<void> {
    // What a view asks is about the repository its group shows, and what the tab of a commit asks about its commit's.
    const on = this.host.groupOf(webview) ?? this.host.followers;
    const repo = this.host.pinned.get(webview) ?? on.repo;
    const compareTab = this.parts.compareTabs.tabs.get(webview);
    const review = this.host.reviewTab?.webview === webview ? this.host.reviewTab : undefined;
    // The tab of the review reads the files in conflict by itself.
    if (review && message.type === 'ready') return review.reload();
    // The tab of what a rewrite changed is sent its pairs, which it does not read again with the repository.
    if (message.type === 'ready' && this.parts.rangeDiffTab.shows(webview)) return this.parts.rangeDiffTab.ready();
    switch (message.type) {
      case 'focus':
        return this.host.setFocused(webview, message.focused === true);
      case 'ready':
        // A view that has just loaded has nothing yet: it is sent every part it shows.
        this.host.sent.delete(webview);
        // The tab of a commit reads its commit by itself, and nothing in it goes stale: the repository is not read for it.
        if (this.host.views.get(webview) === 'commit') return this.host.post({ type: 'state', state: on.state }, webview);
        // The tab of a comparison is sent its comparison, and the state for what each side is.
        if (compareTab) {
          this.host.post({ type: 'state', state: on.state }, webview);
          return this.parts.compareTabs.sendCompare(compareTab, true);
        }
        // The Repositories view is sent its rows as they are, and what the panel shows is read as for any view.
        if (this.host.views.get(webview) === 'repositories') {
          this.host.repositoriesView.ready(webview);
          return this.host.readForView(this.host.followers);
        }
        // What was read before it opened, as at the start of VS Code, is shown at once, and read again if it is not fresh.
        if (this.host.located && !this.host.locating && !on.session?.loading) this.host.post({ type: 'state', state: on.state }, webview);
        if (on.waiting) this.host.post({ type: 'loading', what: on.waiting.what }, webview);
        if (this.host.views.get(webview) === 'changes' && this.parts.unsaved.paths.length) this.host.post({ type: 'unsaved', paths: this.parts.unsaved.paths }, webview);
        await this.host.readForView(on);
        return this.parts.history.flushPending(on);
      case 'setFilter':
        return this.parts.history.setFilter(message.filter, on);
      case 'setScope':
        return this.parts.history.setScope(message.scope, on);
      case 'setListed':
        return this.parts.history.setListed(message.listed, on);
      case 'setShowStashes':
        return vscode.workspace.getConfiguration('gitardo').update('graph.showStashes', message.show === true, vscode.ConfigurationTarget.Global);
      case 'setDateSeparators':
        return vscode.workspace.getConfiguration('gitardo').update('graph.dateSeparators', message.show === true, vscode.ConfigurationTarget.Global);
      case 'setCombineUpstream':
        return vscode.workspace.getConfiguration('gitardo').update('graph.combineUpstream', message.combine === true, vscode.ConfigurationTarget.Global);
      case 'setBranchSort': {
        const sort = message.sort === 'date' ? 'date' : 'name';
        return vscode.workspace.getConfiguration('gitardo').update('branches.sortOrder', sort, vscode.ConfigurationTarget.Global);
      }
      case 'setGraphColumns': {
        const columns = GRAPH_COLUMNS.filter((column) => message.columns.includes(column));
        return vscode.workspace.getConfiguration('gitardo').update('graph.columns', columns, vscode.ConfigurationTarget.Global);
      }
      case 'setFirstParent':
        return vscode.workspace.getConfiguration('gitardo').update('graph.firstParent', message.only === true, vscode.ConfigurationTarget.Global);
      case 'setOneLane':
        return vscode.workspace.getConfiguration('gitardo').update('graph.oneLane', message.one === true, vscode.ConfigurationTarget.Global);
      case 'setCompactRows':
        return vscode.workspace.getConfiguration('gitardo').update('graph.compactRows', message.compact === true, vscode.ConfigurationTarget.Global);
      case 'setTraceSelected':
        return vscode.workspace.getConfiguration('gitardo').update('graph.traceSelected', message.trace === true, vscode.ConfigurationTarget.Global);
      case 'setFavorite':
        return this.parts.history.setFavorite(message.ref, message.favorite === true, on);
      case 'goTo':
        return this.parts.history.goTo(message.revision, webview, on);
      case 'find':
        return this.parts.history.find(message.search, webview, on);
      case 'command':
        return this.parts.commands.command(message.command, on);
      case 'pullRequest':
        return this.parts.commands.openPullRequest(message.branch, on);
      case 'checkOutPullRequest':
        return this.parts.commands.checkOutPullRequest(message.number, message.remoteBranch, on);
      case 'syntaxTheme':
        this.host.colored.add(webview);
        return this.host.post({ type: 'syntaxTheme', theme: (await this.host.themes.resolveActive()) ?? null }, webview);
      case 'fileIcons':
        this.host.iconed.add(webview);
        return this.host.post({ type: 'fileIcons', theme: await this.host.fileIcons.for(webview) }, webview);
      case 'getWorktreeSetup':
        return this.parts.worktreeSetup.send(webview, message.rescan === true);
      case 'getLargeFiles':
        return this.parts.largeFiles.opened(webview);
      case 'largeFiles':
        return this.parts.largeFiles.ask(webview, message.ask);
      case 'openLargeFiles':
        if (typeof message.root !== 'string') return;
        await this.parts.repositories.selectRepo(message.root);
        return this.host.openLargeFiles();
      case 'previewWorktreeSetup':
        return repo && this.parts.worktreeSetup.preview(webview, repo, message);
      case 'saveWorktreeSetup':
        return repo && this.parts.worktreeSetup.save(webview, repo, message);
      case 'editWorktreeSetup':
        return repo && editSetupJson(repo.root, message.target === 'user' ? 'user' : 'workspace');
      case 'reviewConflicts':
        return this.host.reviewConflicts(Array.isArray(message.paths) ? message.paths : undefined, message.ai === true, on);
      case 'getReview':
        return review?.reload();
      case 'writeResolutions':
        return Array.isArray(message.accepted) ? review?.write(message.accepted) : undefined;
      case 'previewResolutions':
        return Array.isArray(message.accepted) ? review?.preview(message.accepted) : undefined;
      case 'chooseResolution':
        return message.conflict && review?.choose(message.conflict, message.choice ?? null);
      case 'askAI':
        return review?.ask(Array.isArray(message.conflicts) ? message.conflicts : undefined);
      case 'stopAsking':
        return review?.stop();
      case 'chooseConflictModel':
        return this.parts.commands.chooseConflictModel(on);
      case 'openConflict':
        return repo && this.parts.diffs.openConflict(repo, message.path, message.line, message.editor);
      case 'refresh':
        return this.host.refreshAll(on);
      case 'loadMore': {
        // While a read adds its pages, the next ones come without asking (`graph.load-more`); and a list that asked while
        // the last page came has nothing left to ask for once the read ended at the end of the history.
        // Nor has a view that is still sent the commits read, behind its first page: they come without asking too.
        if (!on.repo || on.graph.isReading(on.repo.root) || on.state?.hasMore === false || this.host.behind.has(webview)) return;
        on.cancelGraphRead();
        const listed = on.state?.rows.filter((row) => row.hash !== WORKTREE && row.stash === undefined).length ?? 0;
        on.limit = Math.max(on.limit, listed) + pageSize();
        return this.host.reload(on, 'more');
      }
      case 'getLog': {
        const limit = Math.min(Math.max(Math.trunc(Number(message.limit)) || 1, 1), LOG_LIMIT);
        const log = on.repo && (await reads.branchLog(on.repo, limit).catch(() => null));
        return log ? this.host.post({ type: 'log', log }, webview) : undefined;
      }
      case 'getDetail': {
        const detail = (repo && (await reads.commitDetail(repo, message.hash).catch(() => null))) ?? null;
        return this.host.post({ type: 'detail', hash: message.hash, detail }, webview);
      }
      case 'getSignature': {
        // The read of the commit selected before is no longer waited for: going down the commits reads the last alone.
        this.signatureReads.get(webview)?.abort();
        const reading = new AbortController();
        this.signatureReads.set(webview, reading);
        const signature = (repo && (await reads.signature(repo, message.hash, reading.signal).catch(() => null))) ?? null;
        if (reading.signal.aborted) return;
        this.signatureReads.delete(webview);
        return this.host.post({ type: 'signature', hash: message.hash, signature }, webview);
      }
      case 'pickCoAuthors':
        return repo && this.parts.commands.pickCoAuthors(repo, message.chosen, webview);
      case 'getPatch': {
        const detail = repo && (await reads.commitDetail(repo, message.hash).catch(() => null));
        const patch = (repo && detail && (await reads.commitPatch(repo, detail).catch(() => null))) ?? null;
        return this.host.post({ type: 'patch', hash: message.hash, patch }, webview);
      }
      case 'openCommit':
        return this.parts.commitTabs.openCommit(message.hash, repo);
      case 'reveal':
        // In the graph of the group that asks; from a tab of a commit, in the panel, pointed at the repository the commit
        // is of when it shows another.
        if (this.host.pinned.has(webview)) {
          if (repo && repo.root !== this.host.followers.repo?.root) await this.parts.repositories.selectRepo(repo.root);
          return this.parts.history.reveal(message.hash);
        }
        return this.parts.history.reveal(message.hash, on);
      case 'setFilesLayout':
        return void this.host.memento.update(FILES_KEY, message.layout === 'list' ? 'list' : 'tree');
      case 'compare':
        return this.parts.compareTabs.compare(message.from, on);
      case 'openCompare':
        return this.parts.compareTabs.openCompare(message.from, message.to, repo);
      case 'pickCompareSide':
        return compareTab && this.parts.compareTabs.pickCompareSide(compareTab, message.side === 'from' ? 'from' : 'to');
      case 'swapCompare':
        return compareTab && this.parts.compareTabs.swap(compareTab);
      case 'setCompareMode':
        return compareTab && this.parts.compareTabs.setMode(compareTab, message.mode);
      case 'setComparePath':
        return compareTab && this.parts.compareTabs.setPath(compareTab, message.path);
      case 'takeCompareFile':
        return compareTab ? this.parts.compareTabs.takeCompareFile(compareTab, message.file) : undefined;
      case 'copyComparePatch':
        return compareTab ? this.parts.compareTabs.copyComparePatch(compareTab) : undefined;
      case 'formatPatch':
        return repo && this.parts.commands.formatPatch(repo, message.hashes, message.save === true);
      case 'openCompareOnService':
        return compareTab ? this.parts.compareTabs.openCompareOnService(compareTab) : undefined;
      case 'show':
        return this.parts.history.show(message.hash, on);
      case 'trace':
        return this.parts.history.trace(message.hash, on);
      case 'getComparison': {
        const comparison = on.repo && (await reads.compare(on.repo, message.from, message.to).catch(() => null));
        return comparison ? this.host.post({ type: 'comparison', comparison }, webview) : undefined;
      }
      case 'selectRepo':
        return this.parts.repositories.selectRepo(message.root, on);
      case 'openInTab':
        return typeof message.root === 'string' ? this.parts.repositories.openTab(message.root) : undefined;
      case 'closeTab':
        return void (this.host.goneTabs.get(webview) ?? on.panel)?.dispose();
      case 'keepMessage':
        return this.keepMessage(webview, message.root, message.message, message.shown);
      case 'openDiff':
        return this.parts.diffs.openDiff(message.source, message.file, repo);
      case 'readImages':
        return this.parts.diffs.readImages(message.source, message.file, webview, repo);
      case 'compareFile':
        return this.parts.compareTabs.compareFile(message.source, message.file, message.names, repo);
      case 'openFile':
        return this.parts.diffs.openFile(message.file, repo);
      case 'openChange':
        return vscode.workspace.getConfiguration('gitardo').get('changes.openOnClick') === 'diff'
          ? this.parts.diffs.openDiff({ kind: 'change' }, message.file, repo)
          : this.parts.diffs.openFile(message.file, repo);
      case 'openChanges':
        return this.parts.diffs.openChanges(message.title, message.source, message.files, repo);
      case 'save':
        return this.parts.unsaved.save(message.paths, repo);
      case 'compareWithSaved':
        if (!repo || typeof message.path !== 'string') return;
        return void vscode.commands.executeCommand('workbench.files.action.compareWithSaved', vscode.Uri.file(path.join(repo.root, message.path)));
      case 'sync':
        return this.parts.commands.sync(message.action, on);
      case 'action':
        return this.host.action(message.action, on);
      case 'copy':
        return void vscode.env.clipboard.writeText(message.text);
    }
  }

  /**
   * Keeps the message the commit box had in the repository it was pointed away from, and gives it back the one it had in
   * the repository shown, when it kept one (`repo.keep-open`).
   */
  private keepMessage(webview: vscode.Webview, root: unknown, message: unknown, shown: unknown): void {
    if (!keepOpen() || typeof root !== 'string' || typeof message !== 'string' || typeof shown !== 'string') return;
    this.host.kept.keepMessage(root, message);
    const back = this.host.kept.message(shown);
    if (back !== undefined) this.host.post({ type: 'keptMessage', root: shown, message: back }, webview);
  }
}
