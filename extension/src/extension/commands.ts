/**
 * The commands of the toolbar, of the Command Palette and of the menus that ask in VS Code before the engine runs them,
 * or after: switching worktrees, the reflog, the pull requests, the remotes, the identities and the co-authors, drafting
 * a commit message, fetching, pulling and pushing and what is said after, checking tags on a remote, patches, and the
 * entries of the Explorer's and Source Control's menus.
 */
import * as path from 'node:path';
import * as vscode from 'vscode';
import { syncRuns } from '../engine/actions';
import { EngineError, ErrorCode } from '../engine/connection';
import { readCommits } from '../engine/graph';
import type { Plan } from '../engine/messages';
import { type Repository, reads } from '../engine/reads';
import { sayEngineMissing } from '../engine/vscode';
import { pullWords, readsPullRequests } from '../shared/hosting';
import { samePath } from '../shared/paths';
import type { PullAction, RepoState, SyncAction, UiCommand } from '../shared/protocol';
import { COMPARE_REWRITE, SEE_WHAT_CHANGED, forcePushed } from '../shared/rangeDiff';
import { isHash, worktreeLabel, worktreeStatus } from '../shared/refs';
import { checkedOutBranch, worktreeBranch } from '../shared/state';
import { checkedMarks } from '../shared/tagMarks';
import { closeInVSCodeGit } from './actions';
import { pickCoAuthors } from './coAuthors';
import type { ControllerHost } from './controllerHost';
import { type ViewGroup, errorText } from './repositorySession';

const REFLOG_ENTRIES = 200;
/** How many subjects of the last commits a draft of a commit message is written with. */
const DRAFT_SUBJECTS = 10;

/** What the Pull button runs: the settings add `--rebase` and `--autostash`, unless switched off. */
export function defaultPull(): PullAction {
  const config = vscode.workspace.getConfiguration('gitardo.pull');
  const rebase = config.get<boolean>('rebase') !== false;
  const autostash = config.get<boolean>('autostash') !== false;
  if (rebase) return autostash ? 'pullRebaseAutostash' : 'pullRebase';
  return autostash ? 'pullAutostash' : 'pull';
}

/** Runs the commands, on the repository of a group of views: the panel's unless another is given. */
export class Commands {
  constructor(
    private readonly host: Pick<
      ControllerHost,
      | 'action'
      | 'fileHistory'
      | 'followers'
      | 'goToCommit'
      | 'heldFor'
      | 'log'
      | 'markTags'
      | 'openLargeFiles'
      | 'openWorktreeSetup'
      | 'pickRepo'
      | 'post'
      | 'refresh'
      | 'requireState'
      | 'reveal'
      | 'run'
      | 'showRepoOf'
    >,
  ) {}

  /** Runs a command of the toolbar or of the Command Palette, on the repository of a group: the panel's unless another is given. */
  async command(command: UiCommand, on = this.host.followers): Promise<void> {
    try {
      if (command === 'showLog') return this.host.log.show();
      if (command === 'activeFileHistory') return await this.host.fileHistory();
      if (command === 'signInGitHub') return await this.signIn(on);
      if (command === 'signInGitLab') return await this.runHostingFeature('hosting.load-merge-requests', on);
      if (command === 'checkOutPullRequest') return await this.runHostingFeature('hosting.checkout-pull-request', on);
      if (command === 'reflog') return await this.browseReflog(on);
      if (command === 'goToCommit') return await this.host.goToCommit(on);
      if (command === 'cleanupBranches') return await this.cleanupBranches(on);
      if (command === 'draftMessage') return await this.draftMessage(on);
      if (command === 'switchIdentity') return await this.switchIdentity(on);
      if (command === 'selectRepository') return await this.host.pickRepo(on);
      if (command === 'remotes') return await this.manageRemotes(false, on);
      if (command === 'switchWorktree') return await this.switchWorktree(on);
      if (command === 'worktreeSetup') return await this.host.openWorktreeSetup(on);
      if (command === 'largeFiles') return await this.host.openLargeFiles(on);
    } catch (error) {
      void vscode.window.showErrorMessage(`Gitardo: ${errorText(error)}`);
    }
  }

  /**
   * Lists the worktrees with how they stand: the branch against its upstream, the files not committed, the pull request.
   * The one picked opens where the settings say; the last entry creates another.
   */
  private async switchWorktree(on: ViewGroup): Promise<void> {
    const state = await this.host.requireState(on);
    if (!state) return;
    const create = { label: '$(add) Create worktree…', path: '', current: false };
    const worktrees = state.worktrees
      .filter((worktree) => !worktree.isMissing)
      .map((worktree) => {
        const { ahead, pull } = worktreeStatus(worktree, state);
        const description = [
          [worktreeBranch(worktree) ?? `detached at ${(worktree.commit ?? '0000000').slice(0, 7)}`, ahead].filter(Boolean).join(' '),
          worktree.uncommitted ? `$(edit) ${worktree.uncommitted}` : undefined,
          pull && `$(git-pull-request) #${pull.number}`,
          worktree.isCurrent && 'this window',
        ];
        return {
          label: `$(${worktree.isMain ? 'home' : 'worktree'}) ${worktreeLabel(worktree.folder, state.worktreesRoot)}`,
          description: description.filter(Boolean).join(' · '),
          detail: worktree.folder,
          path: worktree.folder,
          current: worktree.isCurrent,
        };
      });
    // The one open here is listed last: it is where the user already is.
    worktrees.sort((a, b) => Number(a.current) - Number(b.current));
    const picked = await vscode.window.showQuickPick([...worktrees, { label: '', kind: vscode.QuickPickItemKind.Separator, path: '', current: false }, create], {
      title: 'Switch Worktree',
      placeHolder: 'The worktree to open',
      matchOnDescription: true,
      matchOnDetail: true,
    });
    if (!picked || picked.current) return;
    await this.host.action(picked === create ? { kind: 'worktreeAdd' } : { kind: 'worktreeOpen', path: picked.path }, on);
  }

  /** Lists where HEAD has been, and offers what can be done with the chosen position, even if no branch reaches it any more. */
  private async browseReflog(on: ViewGroup): Promise<void> {
    const state = await this.host.requireState(on);
    if (!state || !on.repo) return;
    const entries = await reads.reflog(on.repo, 'HEAD', REFLOG_ENTRIES);
    const entry = await vscode.window.showQuickPick(
      entries.map((item) => ({
        label: item.subject,
        description: `${item.hash.slice(0, 7)} · ${item.selector}`,
        detail: new Date(item.date * 1000).toLocaleString(),
        item,
      })),
      { title: 'Reflog: where HEAD has been', placeHolder: 'Pick an entry to go back to it or recover it', matchOnDescription: true },
    );
    if (!entry) return;
    const { hash } = entry.item;
    const short = hash.slice(0, 7);
    const choice = await vscode.window.showQuickPick(
      [
        { label: 'Show in the graph', run: () => this.host.reveal(hash, on) },
        { label: 'Create branch here…', run: () => this.host.action({ kind: 'createBranch', at: hash }, on) },
        { label: `Reset ${checkedOutBranch(state) ?? 'HEAD'} to here…`, run: () => this.host.action({ kind: 'reset', hash }, on) },
        { label: 'Check out (detached)', run: () => this.host.action({ kind: 'checkout', target: 'detached', ref: hash }, on) },
        { label: 'Cherry-pick', run: () => this.host.action({ kind: 'cherryPick', hash, isMerge: false }, on) },
      ],
      { title: `${short}: ${entry.item.subject}` },
    );
    await choice?.run();
  }

  /** Offers to delete the local branches that are merged into the current one, or whose upstream is gone. */
  private async cleanupBranches(on: ViewGroup): Promise<void> {
    const state = await this.host.requireState(on);
    if (!state || !on.repo) return;
    // Once the plan is confirmed, VS Code's Git first lets go of the worktrees it removes.
    await this.host.run([{ feature: 'branch.clean-up', target: { kind: 'head' } }], on.repo, { confirmed: (plan) => this.closeRemoved(plan) }, undefined, on);
  }

  /** Asks the language model for a commit message and hands it to the Changes view. */
  private async draftMessage(on: ViewGroup): Promise<void> {
    const state = await this.host.requireState(on);
    const repo = on.repo;
    if (!state || !repo) return;
    this.host.post({ type: 'drafting', active: true });
    try {
      const message = await vscode.window.withProgress(
        { location: vscode.ProgressLocation.Window, title: 'Gitardo: drafting commit message', cancellable: true },
        (_progress, token) => this.draftWith(repo, state, token),
      );
      this.host.post({ type: 'drafting', active: false, message });
    } catch (error) {
      this.host.post({ type: 'drafting', active: false });
      throw error;
    }
  }

  // AI and GitHub through the engine.

  /**
   * The draft of a commit message, by the engine (`commit.draft-message`), from the subjects of the last commits the
   * timeline lists, without the draft and without stashes.
   */
  private async draftWith(repo: Repository, state: RepoState, token: vscode.CancellationToken): Promise<string> {
    const client = reads.client();
    const head = state.head.commit;
    const asked = { limit: DRAFT_SUBJECTS, filter: null, scope: null, names: [], stashes: false, firstParent: false, head, refs: null, stashList: null };
    const subjects = (await readCommits(client, repo.root, asked, DRAFT_SUBJECTS)).commits.map((commit) => commit.subject);
    const aborted = new AbortController();
    const listener = token.onCancellationRequested(() => aborted.abort());
    try {
      const drafted = await client.draftMessage(repo.root, subjects, aborted.signal);
      if (drafted.kind === 'failed') throw new Error(drafted.message);
      return drafted.message;
    } catch (error) {
      // Cancelled, the box keeps what it had, as when the extension drafted it.
      if (error instanceof EngineError && error.code === ErrorCode.cancelled) throw new vscode.CancellationError();
      throw error;
    } finally {
      listener.dispose();
    }
  }

  /**
   * Runs an operation of the pull requests of the hosting service of the main remote on the engine, which refuses it,
   * saying why, when the remote is not on a service that has it (`hosting.load-merge-requests`,
   * `hosting.checkout-pull-request`).
   */
  private async runHostingFeature(feature: 'hosting.load-merge-requests' | 'hosting.checkout-pull-request', on: ViewGroup): Promise<void> {
    const repo = on.repo;
    if (!repo) return;
    await this.host.run([{ feature, target: { kind: 'head' } }], repo, {}, undefined, on);
  }

  /**
   * Signs in to the hosting service of the main remote of a group's repository, and loads its pull requests with the
   * engine's `github.load-pull-requests` or `hosting.load-merge-requests`, which read them again.
   */
  private async signIn(on: ViewGroup): Promise<void> {
    const hosting = on.session?.hosting;
    const repo = on.repo;
    if (!hosting || !readsPullRequests(hosting.service)) {
      void vscode.window.showWarningMessage('Gitardo: the main remote of this repository is not on github.com or on a GitLab.');
      return;
    }
    if (!repo) return;
    const gitLab = hosting.service === 'gitLab';
    const words = `${pullWords(hosting.service)}s`;
    try {
      await this.host.run([{ feature: gitLab ? 'hosting.load-merge-requests' : 'github.load-pull-requests', target: { kind: 'head' } }], repo, {}, undefined, on);
    } catch (error) {
      this.host.log.warn(`Could not read ${words}: ${errorText(error)}`);
      void vscode.window.showErrorMessage(`Gitardo: could not read ${words}. ${errorText(error)}`);
    }
  }

  /**
   * Opens the pull request of a branch in the browser, or the page of the hosting service that creates one, whose
   * address the engine gives (`hosting.open-pull-request`, `hosting.web-links`).
   */
  async openPullRequest(branch: string, on: ViewGroup): Promise<void> {
    const repo = on.repo;
    if (!on.session?.hosting || !repo) return;
    const known = on.session.pulls[branch]?.url;
    const url = known ?? (await reads.webLink(repo.root, { kind: 'create', branch }).catch((error) => void this.host.log.warn(`No address: ${errorText(error)}`)));
    if (url) await vscode.env.openExternal(vscode.Uri.parse(url));
  }

  /** Checks out the pull request a remote branch has on its label, by its number (`hosting.checkout-pull-request`). */
  async checkOutPullRequest(number: number, remoteBranch: string, on: ViewGroup): Promise<void> {
    const repo = on.repo;
    if (!repo || !on.session?.hosting?.canCheckOut) return;
    const answers = { number: { kind: 'text' as const, text: String(number) } };
    await this.host.run([{ feature: 'hosting.checkout-pull-request', target: { kind: 'remoteBranch', name: remoteBranch }, answers }], repo, {}, undefined, on);
  }

  /** Lists the remotes, to fetch, rename, repoint or remove one, or to add another. */
  private async manageRemotes(add = false, on = this.host.followers): Promise<void> {
    const state = await this.host.requireState(on);
    const repo = on.repo;
    if (!state || !repo) return;
    // The remote that pull requests are read from may be another one after.
    if (!(await this.host.run([{ feature: add ? 'remote.add' : 'remote.manage', target: { kind: 'head' } }], repo, {}, undefined, on))) return;
    await on.session?.detectHosting();
    await this.host.refresh(on);
  }

  /** Chooses who commits are made as in this repository: a profile from the settings, a new one, or the global identity again. */
  private async switchIdentity(on: ViewGroup): Promise<void> {
    const state = await this.host.requireState(on);
    const repo = on.repo;
    if (!state || !repo) return;
    // The engine's, with the entry that opens the setting at the end of its pick (B9).
    const editList = { label: 'Edit the list of identities…', description: 'gitardo.identities', run: () => this.openIdentities() };
    await this.host.run([{ feature: 'identity.switch', target: { kind: 'head' } }], repo, { choices: { 'identity-pick': [editList] } }, undefined, on);
  }

  /** Opens the setting that lists the identities to commit as. */
  private async openIdentities(): Promise<void> {
    await vscode.commands.executeCommand('workbench.action.openSettings', 'gitardo.identities');
  }

  /** Asks for the co-authors of the next commit, the authors of the history read as the pick opens, and gives them to the box. */
  async pickCoAuthors(repo: Repository, chosen: string[], webview: vscode.Webview): Promise<void> {
    const authors = reads.authors(repo);
    const email = this.host.heldFor(repo.root)?.identity.email ?? null;
    const picked = await pickCoAuthors(authors, email, Array.isArray(chosen) ? chosen : []);
    if (picked) this.host.post({ type: 'coAuthors', coAuthors: picked }, webview);
  }

  /**
   * Asks which language model resolves conflicts, and how much it thinks: `ai.choose-model` of the engine, which lists
   * VS Code's models and leaves the two settings to write.
   */
  async chooseConflictModel(on = this.host.followers): Promise<void> {
    const repo = (await this.host.requireState(on)) && on.repo;
    if (repo) await this.host.run([{ feature: 'ai.choose-model', target: { kind: 'head' } }], repo, {}, undefined, on);
  }

  /**
   * **Check tags on `<remote>`**: the remote of `remote.default` asked for its tags by the engine, and the tags it lacks
   * marked in the references and on the labels until VS Code is closed (`tag.remote-marks`).
   */
  async checkTags(state: RepoState, repo: Repository): Promise<void> {
    const remote = state.remotes.includes('origin') ? 'origin' : state.remotes[0];
    if (!remote) {
      void vscode.window.showWarningMessage('Gitardo: this repository has no remote.');
      return;
    }
    if (!reads.available) return sayEngineMissing();
    try {
      const title = `Gitardo: asking ${remote} for its tags`;
      const comparison = await vscode.window.withProgress({ location: vscode.ProgressLocation.Window, title }, () => reads.remoteTags(repo, remote));
      this.host.markTags(repo.root, checkedMarks(comparison, state.tags));
    } catch (error) {
      void vscode.window.showErrorMessage(`Gitardo: ${errorText(error)}`);
    }
  }

  /**
   * Copies commits as one patch that `git am` takes, oldest first, each with its author, date and message, or saves it
   * to a file the user chooses. Git makes no patch of a merge commit.
   */
  async formatPatch(repo: Repository, hashes: unknown, save: boolean): Promise<void> {
    if (!Array.isArray(hashes) || !hashes.length || !hashes.every(isHash)) return;
    // The engine makes the patch, and puts it on the clipboard or in the file the save dialog chose.
    await this.host.run([{ feature: save ? 'patch.save' : 'patch.copy', target: { kind: 'commits', commits: hashes } }], repo);
  }

  /** Pulls the way the settings choose. */
  pull(): Promise<void> {
    return this.sync(defaultPull());
  }

  /** Fetches, pulls, pushes or syncs the repository a group shows: the panel's unless another is given. */
  async sync(action: SyncAction, on = this.host.followers): Promise<void> {
    const state = await this.host.requireState(on);
    if (!state) return;
    // Fetch, pull, push and Sync are the engine's.
    const repo = on.repo;
    if (repo && (await this.host.run(syncRuns(action, defaultPull()), repo, {}, undefined, on))) await this.afterSync(action, state, on);
  }

  /** What is said after a pull or a push, on either path, from the state before it and the state read after. */
  private async afterSync(action: SyncAction, before: RepoState, on: ViewGroup): Promise<void> {
    const after = on.state && samePath(on.state.root, before.root) ? on.state : null;
    // The changes that were set aside no longer fit on what came in: git leaves them in conflict in the files, and
    // keeps the stash it made, which nothing says unless its output is read.
    const kept = after?.stashes[0];
    if (kept && after.stashes.length > before.stashes.length && /autostash/i.test(kept.message)) {
      void vscode.window
        .showWarningMessage(
          `Gitardo: pulled, but your changes do not fit on what came in. They are in conflict in the files, and whole in the stash "${kept.message}": resolve the conflicts, then drop the stash.`,
          'Show Stashes',
        )
        .then((choice) => choice && vscode.commands.executeCommand('gitardo.openStashes'));
    }
    // A fetch that found the upstream of a branch force pushed says so, with what it changed (M36).
    const forced = after && forcePushed(before, after);
    if (forced && on.repo) {
      const repo = on.repo;
      void vscode.window.showInformationMessage(`Gitardo: ${forced} was force pushed.`, SEE_WHAT_CHANGED).then((choice) => {
        if (choice) void this.host.run([{ feature: COMPARE_REWRITE, target: { kind: 'remoteBranch', name: forced } }], repo, {}, undefined, on);
      });
    }
    // The first push of a branch is the natural moment to open a pull request for it: it had no upstream, and has one now.
    const branch = checkedOutBranch(before);
    const hadUpstream = before.branches.some((each) => each.isCheckedOut && each.upstream);
    const hasUpstream = after?.branches.some((each) => each.isCheckedOut && each.name === branch && each.upstream);
    const hosting = on.session?.hosting;
    if ((action.startsWith('push') || action === 'sync') && branch && !hadUpstream && hasUpstream && hosting && !on.session?.pulls[branch]) {
      const button = hosting.service === 'gitLab' ? 'Create Merge Request' : 'Create Pull Request';
      const choice = await vscode.window.showInformationMessage(`Pushed ${branch}.`, button);
      if (choice) await this.openPullRequest(branch, on);
    }
  }

  /**
   * Stashes from the list of changes VS Code keeps, in the repository these files are of: the files themselves, only
   * what is staged, or everything.
   */
  async stashFromScm(uris: vscode.Uri[], what: 'files' | 'staged' | 'all'): Promise<void> {
    if (!(await this.host.requireState())) return;
    const local = uris.filter((uri) => uri.scheme === 'file');
    if (local[0]) await this.host.showRepoOf(local[0]);
    const { state, repo } = this.host.followers;
    if (!state || !repo) return;
    if (what !== 'files') return this.host.action({ kind: 'stashPush', staged: what === 'staged' || undefined });
    const files = local
      .map((uri) => path.relative(repo.root, uri.fsPath).replace(/\\/g, '/'))
      .filter((file) => file && !file.startsWith('..') && !path.isAbsolute(file));
    if (!files.length) return;
    // A renamed file is stashed with the name it had, or git would leave half of the rename behind.
    const before = new Map([...state.workingTree.staged, ...state.workingTree.unstaged].map((change) => [change.path, change.originalPath]));
    const paths = [...new Set(files.flatMap((file) => [file, before.get(file)]).filter((file) => typeof file === 'string'))];
    return this.host.action({ kind: 'stashPush', paths });
  }

  /**
   * **Store with Git LFS…** on a file of the Explorer: has the engine ask how its files are named, by its extension,
   * the file alone, its folder or a pattern typed, and write the pattern in the `.gitattributes` at the top of the
   * repository it is in, which the views show first (`lfs.track`). What follows, **Store them with Git LFS in the next
   * commit** for the files committed before, is offered as the engine says (`lfs.store-committed`).
   */
  async storeWithLfsFromExplorer(uri: vscode.Uri | undefined): Promise<void> {
    if (uri?.scheme !== 'file' || !(await this.host.requireState())) return;
    await this.host.showRepoOf(uri);
    const repo = this.host.followers.repo;
    const relative = repo ? path.relative(repo.root, uri.fsPath).replace(/\\/g, '/') : '';
    if (!repo || !relative || relative.startsWith('..') || path.isAbsolute(relative)) {
      void vscode.window.showWarningMessage(`Gitardo: ${path.basename(uri.fsPath)} is in no repository of the workspace.`);
      return;
    }
    await this.host.run([{ feature: 'lfs.track', target: { kind: 'file', path: relative } }], repo);
  }

  /**
   * Ignores, stops tracking, hides from git or shows to git again the file or the folder the Explorer's menu was opened
   * on, in the repository it is in, which the views show first (`changes.ignore`, `changes.stop-tracking`,
   * `changes.skip`, `changes.unskip`). One path at a time, as in the desktop (Q1173).
   */
  async leaveAloneFromExplorer(kind: 'ignore' | 'stopTracking' | 'hideFromGit' | 'showToGit', uri: vscode.Uri | undefined): Promise<void> {
    if (uri?.scheme !== 'file' || !(await this.host.requireState())) return;
    await this.host.showRepoOf(uri);
    const relative = this.host.followers.repo ? path.relative(this.host.followers.repo.root, uri.fsPath).replace(/\\/g, '/') : '';
    if (!relative || relative.startsWith('..') || path.isAbsolute(relative)) {
      void vscode.window.showWarningMessage(`Gitardo: ${path.basename(uri.fsPath)} is in no repository of the workspace.`);
      return;
    }
    await this.host.action({ kind, path: relative });
  }

  /** Closes in VS Code's Git the worktrees a plan of the engine removes, which Windows would not let git delete. */
  private async closeRemoved(plan: Plan): Promise<void> {
    const removed = plan.steps.flatMap((step) => {
      const command = step.kind === 'git' && Array.isArray(step.arguments) ? (step.arguments as string[]) : [];
      return command[0] === 'worktree' && command[1] === 'remove' ? [command[command.length - 1]] : [];
    });
    if (removed.length) await closeInVSCodeGit(removed, this.host.log);
  }
}
