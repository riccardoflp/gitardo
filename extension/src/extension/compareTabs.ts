/**
 * The tabs of the comparisons: two branches, tags or commits, or one with the working tree, each in a tab of its own,
 * narrowed to a file or a folder or not, followed as the branches move; and a file of a commit or of a comparison
 * compared with a side the user chooses, in the diff editor.
 */
import * as path from 'node:path';
import * as vscode from 'vscode';
import type { ComparisonTarget, CompareSideTarget } from '../engine/messages';
import { type Repository, reads } from '../engine/reads';
import { type CompareMode, type CompareSide, type ComparedRef, type DiffSource, type FileChange, type FilesLayout, WORKTREE } from '../shared/protocol';
import { samePath } from '../shared/paths';
import { isHash } from '../shared/refs';
import { checkedOutBranch } from '../shared/state';
import { FILES_KEY } from './commitTabs';
import { contentUri } from './content';
import type { ControllerHost } from './controllerHost';
import { errorText } from './repositorySession';
import { type RecentComparison, WORKTREE_SIDE, isSide, pickRevision } from './revisions';

/** The comparisons opened last in each repository, by its folder. */
const RECENT_KEY = 'gitardo.recentComparisons';
/** How many of them are kept. */
const RECENT_COMPARISONS = 8;

/** A comparison open in a tab: its two sides as they were chosen, and what the tab was sent last. */
export interface CompareTab {
  panel: vscode.WebviewPanel;
  repo: Repository;
  from: CompareSide;
  to: CompareSide;
  mode: CompareMode;
  /** The file or the folder the comparison is narrowed to. */
  path: string | null;
  /** The commits and the mode the tab shows, to tell whether what it shows has to be read again. */
  shown?: string;
  /** Where the changes the tab shows start from. */
  since?: string;
}

/** The title of the tab of a comparison. */
const compareTitle = (from: CompareSide, to: CompareSide, path: string | null) =>
  `${from.name} ↔ ${to.name}${path ? ` · ${path.slice(path.lastIndexOf('/') + 1)}` : ''}`;

/**
 * How two sides are compared at first: two branches or tags from where they diverged, as a pull request shows them; a
 * commit, or the working tree, directly, for how the files differ from it.
 */
const compareMode = (from: CompareSide, to: CompareSide): CompareMode =>
  from.live && to.live && from.hash !== WORKTREE && to.hash !== WORKTREE ? 'diverged' : 'direct';

/** A file or a folder as git names it, from the top of the repository, or null for all of it. */
const cleanPath = (path: unknown): string | null => {
  if (typeof path !== 'string') return null;
  const clean = path
    .trim()
    .replace(/\\/g, '/')
    .replace(/^(?:\.?\/)+|\/+$/g, '');
  return clean && clean !== '.' ? clean : null;
};

/** Opens the comparisons in tabs, and sends each what it shows. */
export class CompareTabs {
  /** The tabs of the comparisons, by their webview. */
  readonly tabs = new Map<vscode.Webview, CompareTab>();

  constructor(
    private readonly host: Pick<
      ControllerHost,
      'activity' | 'attach' | 'followers' | 'heldFor' | 'hostingOf' | 'log' | 'memento' | 'pin' | 'post' | 'requireState' | 'run' | 'showRepoOf' | 'stateOf'
    >,
  ) {}

  /**
   * Compares two branches, tags or commits, or one with the working tree, in a tab, asking which. With `from`, only
   * the other side is asked. The first question also offers the comparisons opened last.
   */
  async compare(from?: ComparedRef, on = this.host.followers): Promise<void> {
    const state = await this.host.requireState(on);
    const repo = on.repo;
    if (!state || !repo) return;
    try {
      const given = from && isSide(from.hash) && typeof from.name === 'string' ? { name: from.name, hash: from.hash } : undefined;
      const first =
        given ??
        (await pickRevision(state, repo, 'Compare', 'The branch, tag or commit to start from, or a comparison opened before', {
          worktree: true,
          recent: this.recentComparisons(repo),
        }));
      if (!first) return;
      if ('to' in first) return await this.openRecent(first, repo);
      const second = await pickRevision(state, repo, `Compare ${first.name} with`, 'The branch, tag or commit to compare it with', {
        other: first,
        worktree: true,
      });
      if (!second || 'to' in second) return;
      await this.openCompare(first, second, repo);
    } catch (error) {
      void vscode.window.showErrorMessage(`Gitardo: ${errorText(error)}`);
    }
  }

  /**
   * Compares a file or a folder as it is in the working tree with a branch, tag or commit, asking which: the given
   * one, or the file in the active editor.
   */
  async compareWith(uri = vscode.window.activeTextEditor?.document.uri): Promise<void> {
    if (!(await this.host.requireState())) return;
    if (uri?.scheme === 'file') await this.host.showRepoOf(uri);
    const { state, repo } = this.host.followers;
    if (!state || !repo) return;
    const file = uri?.scheme === 'file' ? path.relative(repo.root, uri.fsPath).replace(/\\/g, '/') : '';
    if (!uri || file.startsWith('..') || path.isAbsolute(file)) {
      void vscode.window.showWarningMessage('Gitardo: choose a file or a folder of this repository to compare.');
      return;
    }
    try {
      const name = file || path.basename(repo.root);
      const picked = await pickRevision(state, repo, `Compare ${name} with`, 'The branch, tag or commit to compare the working tree with');
      if (!picked || 'to' in picked) return;
      await this.openCompare(picked, WORKTREE_SIDE, repo, file || null);
    } catch (error) {
      void vscode.window.showErrorMessage(`Gitardo: ${errorText(error)}`);
    }
  }

  /**
   * Compares a file listed in a commit, in a comparison or in the changes not committed, with the branch, tag, commit or
   * working tree the user chooses, in the diff editor. A file the commit deleted is compared as it was before.
   */
  async compareFile(source: DiffSource, file: FileChange, names: { hash?: string; parent?: string } = {}, repo = this.host.followers.repo): Promise<void> {
    if (!repo || typeof file?.path !== 'string') return;
    // The branches offered are those of the repository the file is in, which no view may show.
    const state = this.host.heldFor(repo.root) ?? (await this.host.stateOf(repo).catch(() => null));
    if (!state) return;
    const deleted = file.kind === 'deleted';
    const head = state.head.commit;
    let at: ComparedRef | undefined;
    if (source?.kind === 'worktree') at = !deleted ? WORKTREE_SIDE : head ? { name: checkedOutBranch(state) ?? 'HEAD', hash: head } : undefined;
    else if (source?.kind === 'commit') {
      const hash = deleted ? source.parent : source.hash;
      const named = deleted ? names?.parent : names?.hash;
      if (hash === WORKTREE) at = WORKTREE_SIDE;
      else if (hash && isHash(hash)) at = { name: typeof named === 'string' && named ? named : hash.slice(0, 7), hash };
    }
    if (!at) return;
    const name = path.posix.basename(file.path);
    try {
      const picked = await pickRevision(state, repo, `Compare ${name} of ${at.name} with`, 'The branch, tag or commit to compare it with', {
        other: at,
        worktree: true,
      });
      if (!picked || 'to' in picked) return;
      // The working tree is on the right, where its file can be edited, as in every other diff.
      const [left, right] = picked.hash === WORKTREE ? [at, picked] : [picked, at];
      const uri = (side: ComparedRef) => (side.hash === WORKTREE ? vscode.Uri.file(path.join(repo.root, file.path)) : contentUri(repo.root, side.hash, file.path));
      await vscode.commands.executeCommand('vscode.diff', uri(left), uri(right), `${name} (${left.name} ↔ ${right.name})`);
    } catch (error) {
      void vscode.window.showErrorMessage(`Gitardo: ${errorText(error)}`);
    }
  }

  /** Opens a comparison opened before, its branches where they are now. */
  private async openRecent(recent: RecentComparison, repo: Repository): Promise<void> {
    const now = async (ref: ComparedRef): Promise<ComparedRef> =>
      ref.hash === WORKTREE || ref.hash.startsWith(ref.name) ? ref : { name: ref.name, hash: (await reads.resolve(repo, ref.name)) ?? ref.hash };
    await this.openCompare(await now(recent.from), await now(recent.to), repo, recent.path ?? null);
  }

  /**
   * Opens the comparison of two branches, tags or commits, or of one with the working tree, in a tab of its own, or
   * shows the tab it has. With `path`, only that file or folder.
   */
  async openCompare(from: ComparedRef, to: ComparedRef, repo = this.host.followers.repo, path: string | null = null): Promise<void> {
    if (!repo || !isSide(from?.hash) || !isSide(to?.hash) || (from.hash === WORKTREE && to.hash === WORKTREE)) return;
    const narrowed = cleanPath(path);
    const [first, second] = await Promise.all([this.compareSide(repo, from), this.compareSide(repo, to)]);
    this.rememberComparison(repo, first, second, narrowed);
    const same = (a: CompareSide, b: CompareSide) => a.name === b.name && a.live === b.live && (a.live || a.hash === b.hash);
    for (const tab of this.tabs.values()) {
      if (samePath(tab.repo.root, repo.root) && same(tab.from, first) && same(tab.to, second) && tab.path === narrowed) return tab.panel.reveal();
    }

    const panel = vscode.window.createWebviewPanel('gitardo.compare', compareTitle(first, second, narrowed), vscode.ViewColumn.Active, {
      retainContextWhenHidden: true,
      // The search of VS Code, for a word among the changes.
      enableFindWidget: true,
    });
    const { webview } = panel;
    const tab: CompareTab = { panel, repo, from: first, to: second, mode: compareMode(first, second), path: narrowed };
    const attachment = this.host.attach(webview, 'compare', { root: repo.root, files: this.host.memento.get<FilesLayout>(FILES_KEY, 'tree') });
    this.host.pin(webview, repo);
    this.tabs.set(webview, tab);
    panel.onDidDispose(() => {
      attachment.dispose();
      this.host.pin(webview, null);
      this.tabs.delete(webview);
    });
  }

  /**
   * A side of a comparison: followed when git gives its name the commit it was chosen at, fixed when it is a hash. The
   * working tree is followed as it changes.
   */
  private async compareSide(repo: Repository, ref: ComparedRef): Promise<CompareSide> {
    if (ref.hash === WORKTREE) return { name: WORKTREE_SIDE.name, hash: WORKTREE, live: true };
    const name = typeof ref.name === 'string' && ref.name ? ref.name : ref.hash.slice(0, 7);
    // A name that git reads as another commit, a tag that has the name of a branch, stands for the one chosen.
    const live = !ref.hash.startsWith(name) && (await reads.resolve(repo, name)) === ref.hash;
    return { name, hash: ref.hash, live };
  }

  /** The comparisons opened last in a repository, the latest first. */
  private recentComparisons(repo: Repository): RecentComparison[] {
    return this.host.memento.get<Record<string, RecentComparison[]>>(RECENT_KEY, {})[repo.root] ?? [];
  }

  rememberComparison(repo: Repository, from: CompareSide, to: CompareSide, path: string | null): void {
    const all = this.host.memento.get<Record<string, RecentComparison[]>>(RECENT_KEY, {});
    const entry: RecentComparison = { from: { name: from.name, hash: from.hash }, to: { name: to.name, hash: to.hash }, ...(path ? { path } : {}) };
    // The two ways of a comparison are one: the last is kept.
    const same = (other: RecentComparison) =>
      (other.path ?? null) === path &&
      ((other.from.name === from.name && other.to.name === to.name) || (other.from.name === to.name && other.to.name === from.name));
    const list = [entry, ...(all[repo.root] ?? []).filter((other) => !same(other))].slice(0, RECENT_COMPARISONS);
    void this.host.memento.update(RECENT_KEY, { ...all, [repo.root]: list });
  }

  /**
   * Sends the tab of a comparison what it shows, once its sides are looked up again, and then the lines of its files.
   * Nothing is sent when it shows that already, unless `again`; a comparison with the working tree is read again
   * whenever the repository is.
   */
  async sendCompare(tab: CompareTab, again = false): Promise<void> {
    const { panel, repo } = tab;
    const { webview } = panel;
    // Its repository is read again only while the other views show it.
    if (!again && !this.host.heldFor(repo.root)) return;
    // A tab closed while git answered is not there to be told.
    const open = () => this.tabs.get(webview) === tab;
    try {
      for (const side of [tab.from, tab.to]) {
        if (!side.live || side.hash === WORKTREE) continue;
        const hash = await reads.resolve(repo, side.name);
        if (!hash) side.gone = true;
        else {
          side.hash = hash;
          delete side.gone;
        }
      }
      const { from, to, mode, path } = tab;
      const changing = from.hash === WORKTREE || to.hash === WORKTREE;
      const key = `${from.hash} ${to.hash} ${mode} ${from.gone} ${to.gone} ${path}`;
      if (!open() || (!again && !changing && tab.shown === key)) return;
      tab.shown = key;
      panel.title = compareTitle(from, to, path);
      // What the user changed shows a wait until it is compared; the working tree compared again as it changes does not.
      if (again) this.host.post({ type: 'loading', what: 'filter' }, webview);
      const found = await this.host.activity.track(`Comparing ${panel.title}`, () => reads.compareSides(repo, from.hash, to.hash, mode, path));
      // The sides changed again meanwhile: the answer for them is the one to send, and the end of any wait.
      if (tab.shown !== key || !open()) return;
      this.host.post({ type: 'loading', what: null }, webview);
      if (!found) return this.host.post({ type: 'compareView', view: null, error: 'These are not commits of this repository.' }, webview);
      tab.since = found.since;
      this.host.post({ type: 'compareView', view: { from: { ...from }, to: { ...to }, mode, ...found } }, webview);
      const patch = await reads.comparePatch(repo, found.files, found.since, to.hash, path).catch(() => null);
      if (tab.shown === key && open()) this.host.post({ type: 'comparePatch', since: found.since, to: to.hash, patch }, webview);
    } catch (error) {
      tab.shown = undefined;
      if (!open()) return;
      this.host.log.error(errorText(error));
      this.host.post({ type: 'loading', what: null }, webview);
      this.host.post({ type: 'compareView', view: null, error: errorText(error) }, webview);
    }
  }

  /** Lets the user choose one side of a comparison again, in its tab. */
  async pickCompareSide(tab: CompareTab, which: 'from' | 'to'): Promise<void> {
    // The branches to choose from are of the repository of the tab, which the other views may no longer show.
    const state = this.host.heldFor(tab.repo.root) ?? (await this.host.stateOf(tab.repo).catch(() => null));
    if (!state) return;
    const other = which === 'from' ? tab.to : tab.from;
    const title = which === 'from' ? `Compare with ${other.name}` : `Compare ${other.name} with`;
    const picked = await pickRevision(state, tab.repo, title, `The branch, tag or commit to put in place of ${tab[which].name}`, {
      other,
      worktree: other.hash !== WORKTREE,
    });
    if (!picked || 'to' in picked) return;
    tab[which] = await this.compareSide(tab.repo, picked);
    // As when the tab opened.
    tab.mode = compareMode(tab.from, tab.to);
    this.rememberComparison(tab.repo, tab.from, tab.to, tab.path);
    await this.sendCompare(tab, true);
  }

  /**
   * Makes a file in the working tree and the index what the second side of a comparison has: its version of the file,
   * or no file when it has none.
   */
  async takeCompareFile(tab: CompareTab, file: FileChange): Promise<void> {
    const { to, repo } = tab;
    if (to.hash === WORKTREE || typeof file?.path !== 'string' || !file.path) return;
    // The engine asks before it overwrites what is not committed in the file.
    await this.host.run([{ feature: 'compare.take-file', target: { ...this.comparisonTarget(tab), file } }], repo);
  }

  /** Copies the changes of a comparison as a patch that `git apply` takes. */
  async copyComparePatch(tab: CompareTab): Promise<void> {
    if (!tab.since) return;
    const text = await reads.compareText(tab.repo, tab.since, tab.to.hash, tab.path).catch(() => '');
    if (!text) {
      void vscode.window.showInformationMessage('Gitardo: there are no changes to copy.');
      return;
    }
    await vscode.env.clipboard.writeText(text);
    const files = text.match(/^diff --git /gm)?.length ?? 0;
    void vscode.window.showInformationMessage(`Gitardo: copied the changes of ${files} ${files === 1 ? 'file' : 'files'} as a patch.`);
  }

  /**
   * Opens the comparison on the hosting service of the main remote, each side by the name it has there: a branch by its
   * upstream there, a remote branch of that remote and a tag by their names, anything else by its commit, which the
   * service knows once it is pushed (`compare.open-on-service`, which is `compare.open-on-github` on GitHub).
   */
  async openCompareOnService(tab: CompareTab): Promise<void> {
    const hosting = this.host.hostingOf(tab.repo.root);
    const { from, to } = tab;
    if (!hosting || from.hash === WORKTREE || to.hash === WORKTREE) return;
    // The engine names each side on the service and opens the address.
    await this.host.run([{ feature: 'compare.open-on-service', target: this.comparisonTarget(tab) }], tab.repo);
  }

  /** A comparison tab as the target of `compare.open-on-service` and, with a file, of `compare.take-file`. */
  private comparisonTarget(tab: CompareTab): ComparisonTarget {
    const side = (compared: CompareSide): CompareSideTarget => ({
      name: compared.name,
      side: compared.hash === WORKTREE ? { kind: 'workingTree' } : { kind: 'commit', commit: compared.hash },
      isFollowed: compared.live,
      isGone: compared.gone ?? false,
    });
    return {
      kind: 'comparison',
      first: side(tab.from),
      second: side(tab.to),
      mode: tab.mode === 'diverged' ? 'sinceDiverged' : 'everything',
      path: tab.path,
      mergeBase: null,
    };
  }

  /** Swaps the two sides of a comparison. */
  swap(tab: CompareTab): Promise<void> {
    [tab.from, tab.to] = [tab.to, tab.from];
    this.rememberComparison(tab.repo, tab.from, tab.to, tab.path);
    return this.sendCompare(tab, true);
  }

  /** Compares the two sides from where they diverged, or directly. */
  setMode(tab: CompareTab, mode: CompareMode): Promise<void> {
    tab.mode = mode === 'diverged' ? 'diverged' : 'direct';
    return this.sendCompare(tab, true);
  }

  /** Narrows a comparison to a file or a folder, or to none. */
  setPath(tab: CompareTab, path: string | null): Promise<void> {
    tab.path = cleanPath(path);
    this.rememberComparison(tab.repo, tab.from, tab.to, tab.path);
    return this.sendCompare(tab, true);
  }
}
