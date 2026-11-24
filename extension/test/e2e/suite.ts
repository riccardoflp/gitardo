// End-to-end checks that run inside a real Extension Host, against the repositories test/e2e/run.mjs prepared.
// They drive the extension through its commands and its controller, with VS Code's prompts answered by stubs.

import * as assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { CONFLICT, INDEX, contentUri } from '../../src/extension/content';
import type { EngineClient } from '../../src/engine/client';
import { reads } from '../../src/engine/reads';
import type { GitardoApi } from '../../src/extension/extension';
import { type DiffDescriptor, describeDiff } from '../../src/flow/sides';
import { git } from '../git';
import { keptTab } from '../../src/extension/sessions';
import { samePath } from '../../src/shared/paths';
import { WORKTREE, type CompareMode, type CompareSide, type Filter, type HostMessage, type RebaseStep, type RepoState, type SentState, type ToViews, type ViewKind, type WebviewMessage } from '../../src/shared/protocol';
import type { Hosting } from '../../src/shared/hosting';
import type { ConflictReview } from '../../src/shared/resolution';
import { subjectAndBody } from '../../src/shared/message';
import { buildLabels } from '../../src/webview/labels';
import { recentBranches } from '../../src/shared/refs';
import { CHANGE_LETTERS, type ChangeKind, checkedOutBranch, type FileChange } from '../../src/shared/state';

/** The parts of the controller that are private to everyone but these tests. */
interface Internals {
  state: RepoState | null;
  /** How many commits are asked of git, and what narrows them. */
  limit: number;
  filter: Filter | null;
  views: Map<unknown, ViewKind>;
  /** The tabs of the comparisons, by their webview. */
  compareTabs: { tabs: Map<unknown, CompareTab> };
  hosting: Hosting | null;
  onMessage(webview: unknown, message: WebviewMessage): Promise<void>;
  post(message: ToViews, to?: unknown): void;
  scheduleRefresh(cause: string, delay?: number): void;
  /** The read of the repository under way, while there is one. */
  loading?: Promise<void>;
  /** Gitardo's engine, when the extension has it running. */
  engine?: { running?: unknown };
  /** The repositories left that keep what they showed. */
  kept: { has(root: string): boolean; outOfDate(root: string): string | undefined };
  /** Looks for the repositories of the workspace again, as when its folders change. */
  locate(): Promise<void>;
  /** The workspace state of the extension. */
  memento: { get<T>(key: string): T | undefined };
  /** The host of the Repositories view, with what its pages were last sent, as one text. */
  repositoriesView: { sent?: string };
  /** The views of the panel and of the side bar, and the session of the repository they show (D89). */
  followers: { session?: object };
  /** The group each view that shows a repository is in: the panel and the side bar, or a tab of the graph (D89). */
  groups: Map<unknown, ViewGroup>;
  /** The session of each repository a group of views shows. */
  sessions: { all(): { root: string }[] };
}

/** A group of views, as the controller keeps it: the panel and the side bar, or a tab of the graph in the editor. */
interface ViewGroup {
  session?: { root: string };
  state: RepoState | null;
  filter: Filter | null;
  panel?: vscode.WebviewPanel;
}

/** A comparison open in a tab, as the controller keeps it. */
interface CompareTab {
  panel: vscode.WebviewPanel;
  from: CompareSide;
  to: CompareSide;
  mode: CompareMode;
  path: string | null;
}

interface Result {
  name: string;
  ok: boolean;
  error?: string;
}

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

async function until<T>(what: string, check: () => T | undefined | null | false, timeout = 15_000): Promise<T> {
  const end = Date.now() + timeout;
  for (;;) {
    const value = check();
    if (value) return value;
    if (Date.now() > end) throw new Error(`Timed out waiting for ${what}`);
    await sleep(100);
  }
}

/** The API of VS Code's Git extension, which opens worktrees as repositories of their own. */
interface VSCodeGitApi {
  repositories: { rootUri: vscode.Uri; ui: { readonly selected: boolean } }[];
  openRepository(root: vscode.Uri): Promise<unknown>;
}

async function vscodeGit(): Promise<VSCodeGitApi> {
  const extension = vscode.extensions.getExtension<{ getAPI(version: 1): VSCodeGitApi }>('vscode.git');
  assert.ok(extension, "VS Code's Git extension");
  return (await extension.activate()).getAPI(1);
}

export async function run(): Promise<void> {
  const results: Result[] = [];
  const report = () => {
    const file = process.env.GITARDO_E2E_RESULTS;
    if (file) fs.writeFileSync(file, JSON.stringify(results, null, 2));
  };
  const step = async (name: string, body: () => Promise<void>) => {
    try {
      await body();
      results.push({ name, ok: true });
    } catch (error) {
      results.push({ name, ok: false, error: error instanceof Error ? (error.stack ?? error.message) : String(error) });
    }
    report();
  };

  const workspace = vscode.workspace.workspaceFolders![0].uri.fsPath;
  const app = path.join(workspace, 'app');
  const lib = path.join(workspace, 'lib');
  const write = (file: string, content: string) => fs.writeFileSync(path.join(app, file), content);

  // By its name, whoever publishes it.
  const extension = vscode.extensions.all.find((candidate) => candidate.packageJSON.name === 'gitardo') as vscode.Extension<GitardoApi> | undefined;
  assert.ok(extension, 'the extension is installed');
  const { controller, log, fileBlame, flowDiff } = await extension.activate();
  const internals = controller as unknown as Internals;
  // The functions of the extension this suite calls itself, such as describeDiff, are bundled into it with reads of
  // their own: they ask the engine of the window, as the extension's do.
  reads.use(() => internals.engine?.running as EngineClient | undefined);
  const state = () => {
    assert.ok(internals.state, `a repository is loaded${lastLogged ? `; logged last: ${lastLogged}` : ''}`);
    return internals.state;
  };
  const subjects = () => state().rows.map((row) => row.subject);

  // Everything the webviews say to the extension, and the extension to them.
  const received: WebviewMessage[] = [];
  const posted: ToViews[] = [];
  /** The view each of them was for, or nothing when it was for all. */
  const addressed: unknown[] = [];
  /** What the views ask that a step wants left unanswered for a while. */
  const unanswered = new Set<WebviewMessage['type']>();
  const onMessage = internals.onMessage.bind(controller);
  internals.onMessage = (webview, message) => {
    received.push(message);
    return unanswered.has(message.type) ? Promise.resolve() : onMessage(webview, message);
  };
  const post = internals.post.bind(controller);
  internals.post = (message, to) => {
    posted.push(message);
    addressed.push(to);
    post(message, to);
  };
  const fromWebview = (message: WebviewMessage) => internals.onMessage([...internals.views.keys()][0], message);
  const kinds = () => [...internals.views.values()].sort();
  const view = (kind: ViewKind) => {
    const found = [...internals.views.entries()].find(([, candidate]) => candidate === kind);
    assert.ok(found, `the ${kind} view is open`);
    return found[0] as vscode.Webview;
  };

  // The command log, which must show every command exactly as it was run.
  const logged: string[] = [];
  const logInfo = log.info.bind(log);
  log.info = (message: string, ...rest: unknown[]) => {
    logged.push(message);
    logInfo(message, ...rest);
  };

  /**
   * The commands of the log, each file written by its name: the engine logs the whole path of a file it writes, the
   * extension the path from the top of the repository.
   */
  const commandsLogged = () => logged.filter((line) => line.startsWith('> ')).map((line) => line.replace(/^> write .*[\\/]/, '> write '));

  // What the extension logs as an error, and shows as one.
  const errors: string[] = [];
  /** The last error or warning logged, which the steps that clear `errors` leave: it says why a step found no state. */
  let lastLogged: string | undefined;
  const logError = log.error.bind(log);
  log.error = (message: string | Error, ...rest: unknown[]) => {
    errors.push(String(message));
    lastLogged = `error: ${String(message)}`;
    logError(message, ...rest);
  };
  const logWarn = log.warn.bind(log);
  log.warn = (message: string, ...rest: unknown[]) => {
    lastLogged = `warning: ${message}`;
    logWarn(message, ...rest);
  };

  /** What the tab of the review of the conflicts was sent last. */
  const lastReview = () => posted.reduce<ConflictReview | null | undefined>((last, message) => (message.type === 'review' ? message.review : last), undefined);

  // Prompts are answered by these, which each test sets as it needs.
  const ui = vscode.window as unknown as Record<string, (...args: never[]) => unknown>;
  const asked: { message: string; detail: string; items: string[] }[] = [];
  let answer: (message: string, detail: string, items: string[]) => string | undefined = () => undefined;
  const run = (_message: string, _detail: string, items: string[]) => (items.includes('Run') ? 'Run' : undefined);
  const message = (text: string, ...rest: unknown[]) => {
    const options = typeof rest[0] === 'object' && rest[0] !== null ? (rest.shift() as { detail?: string }) : {};
    const items = rest as string[];
    asked.push({ message: text, detail: options.detail ?? '', items });
    return Promise.resolve(answer(text, options.detail ?? '', items));
  };
  ui.showInformationMessage = message as never;
  ui.showWarningMessage = message as never;
  ui.showErrorMessage = ((text: string) => {
    errors.push(text);
    return Promise.resolve(undefined);
  }) as never;
  let inputs: string[] = [];
  ui.showInputBox = (() => Promise.resolve(inputs.shift())) as never;
  type Item = vscode.QuickPickItem;
  let choose: (items: Item[], options: vscode.QuickPickOptions) => Item | Item[] | undefined = () => undefined;
  ui.showQuickPick = ((items: Item[], options: vscode.QuickPickOptions = {}) => Promise.resolve(choose(items, options))) as never;
  const byLabel = (label: string | RegExp) => (items: Item[]) => {
    const found = items.find((item) => (typeof label === 'string' ? item.label === label : label.test(item.label)));
    assert.ok(found, `"${label}" is among ${items.map((item) => item.label).join(', ')}`);
    return found;
  };

  await step('registers every command it contributes', async () => {
    const contributed = (extension.packageJSON.contributes.commands as { command: string }[]).map((entry) => entry.command);
    const registered = new Set(await vscode.commands.getCommands(true));
    assert.deepEqual(
      contributed.filter((command) => !registered.has(command)),
      [],
    );
    assert.ok(contributed.length >= 20);
  });

  // Without Gitardo's engine for the system the extension reads and runs nothing (D49): every view says so, with a
  // button that shows the log, and so does every command that needs the engine. That is all there is to check.
  if (!internals.engine?.running) {
    await step('says in every view that the engine is not available, with a button that shows the log', async () => {
      for (const name of ['Graph', 'Repositories', 'Changes', 'Commits', 'SideGraph', 'Branches', 'Stashes']) await vscode.commands.executeCommand(`gitardo.open${name}`);
      await until('the webviews', () => kinds().join() === 'branches,changes,commits,graph,repositories,sideGraph,stashes');
      // The views are sent no state; `post` adds that it is for want of the engine, which the UI suite sees drawn.
      await until('the views told there is no state', () => posted.some((message) => message.type === 'state' && message.state === null));
      assert.equal(internals.state, null);
      assert.ok(!posted.some((message) => message.type === 'state' && message.state !== null));
      // The button of the views.
      await fromWebview({ type: 'command', command: 'showLog' });
    });

    await step('says it, with Show the log, for the commands that need the engine', async () => {
      const said: { text: string; items: string[] }[] = [];
      ui.showErrorMessage = ((text: string, ...items: string[]) => {
        said.push({ text, items });
        return Promise.resolve(undefined);
      }) as never;
      for (const command of ['gitardo.fetch', 'gitardo.undo', 'gitardo.createBranch']) {
        said.length = 0;
        await vscode.commands.executeCommand(command);
        await until(`what ${command} says`, () => said.length > 0);
        assert.deepEqual(said[0], { text: "Gitardo: Gitardo's engine is not available on this system.", items: ['Show the log'] });
      }
    });
    return;
  }

  await step('reads the first repository when VS Code starts, before any of its views is open', async () => {
    assert.equal(internals.views.size, 0);
    await until('the repository state', () => internals.state);
    assert.equal(state().name, 'app');
    assert.deepEqual(subjects(), ['Add feature', 'Change a', 'Add b', 'Add a']);
  });

  await step('the graph view loads its UI and gets the state of the first repository', async () => {
    await vscode.commands.executeCommand('gitardo.openGraph');
    // The UI says `ready` once React has mounted, which proves the bundle ran under the content security policy.
    await until('the graph webview to say it is ready', () => received.some((incoming) => incoming.type === 'ready'));
    await until('the repository state', () => internals.state);
    assert.equal(state().name, 'app');
    assert.equal(checkedOutBranch(state()), 'main');
    assert.deepEqual(subjects(), ['Add feature', 'Change a', 'Add b', 'Add a']);
    assert.deepEqual(
      state().repos.map((repo) => repo.name),
      ['workspace/app', 'workspace/lib'],
    );
    assert.deepEqual([...internals.views.values()], ['graph']);
  });

  await step('the six views of the side bar load too, and the graph opens as an editor tab', async () => {
    const before = received.filter((incoming) => incoming.type === 'ready').length;
    // The stashes start folded, and a folded view is not loaded: its command unfolds it.
    for (const name of ['Repositories', 'Changes', 'Commits', 'SideGraph', 'Branches', 'Stashes']) await vscode.commands.executeCommand(`gitardo.open${name}`);
    await until('the webviews of the side bar', () => kinds().join() === 'branches,changes,commits,graph,repositories,sideGraph,stashes');
    await vscode.commands.executeCommand('gitardo.openInEditor');
    await until('the new webviews to say they are ready', () => received.filter((incoming) => incoming.type === 'ready').length >= before + 7);
    assert.deepEqual(kinds(), ['branches', 'changes', 'commits', 'graph', 'graph', 'repositories', 'sideGraph', 'stashes']);
    await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
    await until('the editor tab to detach', () => internals.views.size === 7);
  });

  await step('notices commits and files that come from outside the editor', async () => {
    await git(app, ['commit', '--allow-empty', '-m', 'From the terminal']);
    await until('the new commit', () => subjects()[0] === 'From the terminal');
    write('outside.txt', 'written by another program\n');
    await until('the new file', () => state().changes === 1);
    assert.equal(state().rows[0].subject, 'Uncommitted changes (1)');
  });

  await step('stages and commits without asking when the preview is off', async () => {
    asked.length = 0;
    await controller.action({ kind: 'stageAll' });
    assert.deepEqual(state().workingTree.staged, [{ kind: 'added', path: 'outside.txt', originalPath: null }]);
    await controller.action({ kind: 'commit', message: 'Add a file', amend: false });
    assert.equal(subjects()[0], 'Add a file');
    assert.equal(state().changes, 0);
    assert.ok(posted.some((outgoing) => outgoing.type === 'committed'));
    assert.deepEqual(asked, []);
  });

  await step('shows the exact commands and waits when the preview is on', async () => {
    await vscode.workspace.getConfiguration('gitardo').update('commandPreview', 'all', vscode.ConfigurationTarget.Global);
    try {
      write('a.txt', 'one\ntwo\nthree\n');
      await vscode.commands.executeCommand('gitardo.refresh');
      asked.length = 0;
      // Declined: nothing runs.
      answer = () => undefined;
      await controller.action({ kind: 'stage', paths: ['a.txt'] });
      assert.equal(state().workingTree.staged.length, 0);
      assert.equal(asked.length, 1);
      assert.equal(asked[0].message, 'Stage a.txt?');
      assert.match(asked[0].detail, /git add -- a\.txt$/);
      assert.ok(asked[0].detail.includes(state().root));
      // Approved: it runs.
      assert.ok(!logged.includes('> git add -- a.txt'), 'nothing ran while the question was open');
      answer = run;
      await controller.action({ kind: 'stage', paths: ['a.txt'] });
      assert.equal(state().workingTree.staged.length, 1);
      // What was shown is what ran, and what the log says ran.
      assert.ok(logged.includes('> git add -- a.txt'));
      await controller.action({ kind: 'unstageAll' });
    } finally {
      await vscode.workspace.getConfiguration('gitardo').update('commandPreview', undefined, vscode.ConfigurationTarget.Global);
    }
  });

  await step('always asks before an operation that loses work', async () => {
    asked.length = 0;
    answer = () => undefined;
    await controller.action({ kind: 'discardAll' });
    assert.equal(asked.length, 1);
    assert.match(asked[0].detail, /will be lost[\s\S]*git restore -- a\.txt/);
    assert.equal(state().changes, 1);
    answer = run;
    await controller.action({ kind: 'discardAll' });
    assert.equal(state().changes, 0);
    assert.equal(fs.readFileSync(path.join(app, 'a.txt'), 'utf8'), 'one\ntwo\n');
  });

  await step('asks for names and options with input boxes and pickers', async () => {
    inputs = ['topic'];
    choose = byLabel('Create only');
    await controller.action({ kind: 'createBranch', at: 'HEAD' });
    assert.ok(state().branches.some((branch) => branch.name === 'topic' && !branch.isCheckedOut));

    inputs = ['v1', 'First release'];
    await controller.action({ kind: 'createTag', at: 'HEAD' });
    assert.deepEqual(
      state().tags.map((tag) => tag.name),
      ['v1'],
    );

    // Backing out of a prompt does nothing.
    inputs = [];
    await controller.action({ kind: 'createBranch', at: 'HEAD' });
    assert.equal(state().branches.length, 4);
  });

  await step('creates a worktree by name, renames it with its branch and removes both', async () => {
    const branches = state().branches.length;
    const home = path.join(workspace, 'app.worktrees').replace(/\\/g, '/');
    assert.equal(typeof state().agentsWindow, 'boolean');
    try {
      asked.length = 0;
      answer = () => undefined;
      inputs = ['agents/first'];
      choose = byLabel('main');
      await controller.action({ kind: 'worktreeAdd' });
      const created = state().worktrees.find((worktree) => worktree.branch === 'refs/heads/agents/first');
      assert.ok(created, JSON.stringify(state().worktrees));
      // A folder in a folder, as the name of the branch has it.
      assert.equal(created.folder.toLowerCase(), `${home}/agents/first`.toLowerCase());
      // The views name it by its path from there.
      assert.equal(state().worktreesRoot?.toLowerCase(), home.toLowerCase());
      assert.ok(!created.isMain && !created.isCurrent);
      assert.ok(state().worktrees.find((worktree) => worktree.isCurrent)?.isMain);
      // It offers to open it, in the Agents window where VS Code has one, and this first time to choose what new worktrees get.
      const offer = await until('the offer to open it', () => asked.find((question) => /^Worktree created in /.test(question.message)));
      assert.deepEqual(offer.items, [...(state().agentsWindow ? ['Open in Agents Window'] : []), 'Open in New Window', 'Choose What It Gets…']);

      // Renamed, with the branch taking the name as it was typed. The engine writes the name of the branch between
      // backticks, the extension without.
      inputs = ['agents-two', 'agents-two'];
      choose = byLabel(/^Rename the branch `?agents\/first`? too$/);
      await controller.action({ kind: 'worktreeRename', path: created.folder });
      const renamed = state().worktrees.find((worktree) => worktree.branch === 'refs/heads/agents-two');
      assert.ok(renamed, JSON.stringify(state().worktrees));
      assert.equal(renamed.folder.toLowerCase(), `${home}/agents-two`.toLowerCase());
      assert.ok(!fs.existsSync(created.folder));
      // The folder that held only it went with it.
      assert.ok(!fs.existsSync(`${home}/agents`));

      // The main worktree cannot be renamed, which is said rather than tried.
      errors.length = 0;
      asked.length = 0;
      await controller.action({ kind: 'worktreeRename', path: state().worktrees.find((worktree) => worktree.isMain)!.folder });
      // The engine says it as a refusal, a warning; the extension as an error.
      const mainSaid = [...errors, ...asked.map((question) => question.message)];
      assert.ok(mainSaid.some((text) => /cannot move the main worktree/.test(text)), JSON.stringify(mainSaid));
      errors.length = 0;

      // Removed with its branch, which is merged: it was made from main and has nothing of its own.
      choose = byLabel(/and delete `?agents-two/);
      answer = run;
      await controller.action({ kind: 'worktreeRemove', path: renamed.folder });
      assert.equal(state().worktrees.length, 1);
      assert.equal(state().branches.length, branches);
      assert.ok(!fs.existsSync(renamed.folder));
    } finally {
      answer = () => undefined;
      fs.rmSync(home, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
    }
  });

  await step('names a new branch from the format, at the branch checked out, a tag or a commit, or as it is typed', async () => {
    const config = vscode.workspace.getConfiguration('gitardo');
    const global = vscode.ConfigurationTarget.Global;
    // Answers to the input boxes, where KEEP takes what the box proposes, and the labels to pick in turn.
    const KEEP = Symbol('keep');
    let typed: (string | typeof KEEP)[] = [];
    const showInputBox = ui.showInputBox;
    ui.showInputBox = ((options: vscode.InputBoxOptions = {}) => {
      const next = typed.shift();
      return Promise.resolve(next === KEEP ? options.value : next);
    }) as never;
    // The engine asks what ${base} is after the inputs, the extension before them (B10 of the differences): each pick
    // takes the first label still to pick that the picker shows.
    let picks: (string | RegExp)[] = [];
    choose = (items) => {
      const index = picks.findIndex((label) => items.some((item) => (typeof label === 'string' ? item.label === label : label.test(item.label))));
      return byLabel(picks.splice(Math.max(index, 0), 1)[0])(items);
    };
    const made: string[] = [];
    const created = (name: string) => {
      made.push(name);
      assert.ok(
        state().branches.some((branch) => branch.name === name && !branch.isCheckedOut),
        JSON.stringify(state().branches.map((branch) => branch.name)),
      );
    };
    await config.update('branches.format', '${input:kind}/${base}-${input:ticket}', global);
    await config.update(
      'branches.inputs',
      [
        { id: 'ticket', type: 'promptString', description: 'Ticket number', pattern: '^\\d+$' },
        { id: 'kind', type: 'pickString', description: 'Type of change', options: ['feat', 'fix'] },
      ],
      global,
    );
    try {
      // From the toolbar, ${base} is the branch checked out.
      picks = ['Use the format', 'feat', 'Create only'];
      typed = ['1', KEEP];
      await controller.action({ kind: 'createBranch', at: 'HEAD' });
      created('feat/main-1');

      // On a tag, the tag; the ticket used last is proposed.
      picks = ['Use the format', 'fix', 'Create only'];
      typed = [KEEP, KEEP];
      await controller.action({ kind: 'createBranch', at: 'v1' });
      created('fix/v1-1');

      // At a commit, the commit or one of the branches that have it.
      const short = state().head.commit!.slice(0, 7);
      picks = ['Use the format', short, 'feat', 'Create only'];
      typed = ['2', KEEP];
      await controller.action({ kind: 'createBranch', at: state().head.commit! });
      created(`feat/${short}-2`);

      // Or the name is typed, as without a format.
      picks = ['Type a branch name', 'Create only'];
      typed = ['typed-branch'];
      await controller.action({ kind: 'createBranch', at: 'HEAD' });
      created('typed-branch');

      // The settings the format had before, among the worktrees, are still read where the new ones are not set.
      await config.update('branches.format', undefined, global);
      await config.update('branches.inputs', undefined, global);
      await config.update('worktrees.branchFormat', 'old-${input:ticket}', global);
      await config.update('worktrees.inputs', [{ id: 'ticket', type: 'promptString' }], global);
      picks = ['Use the format', 'Create only'];
      typed = ['3', KEEP];
      await controller.action({ kind: 'createBranch', at: 'HEAD' });
      created('old-3');
    } finally {
      ui.showInputBox = showInputBox;
      choose = () => undefined;
      for (const key of ['branches.format', 'branches.inputs', 'worktrees.branchFormat', 'worktrees.inputs']) await config.update(key, undefined, global);
      if (made.length) await git(app, ['branch', '-D', ...made]);
      await vscode.commands.executeCommand('gitardo.refresh');
    }
  });

  await step('names the branch of a new worktree from the format, asking for its inputs, or as it is typed', async () => {
    const config = vscode.workspace.getConfiguration('gitardo');
    const home = path.join(workspace, 'app.worktrees').replace(/\\/g, '/');
    const branches = state().branches.length;
    // Answers to the input boxes, where KEEP takes what the box proposes, and the labels to pick in turn.
    const KEEP = Symbol('keep');
    let typed: (string | typeof KEEP)[] = [];
    const proposed: string[] = [];
    const showInputBox = ui.showInputBox;
    ui.showInputBox = ((options: vscode.InputBoxOptions = {}) => {
      proposed.push(options.value ?? '');
      const next = typed.shift();
      return Promise.resolve(next === KEEP ? options.value : next);
    }) as never;
    // The engine asks what ${base} is at a commit after the inputs, the extension before them (B10 of the
    // differences): each pick takes the first label still to pick that the picker shows.
    let picks: (string | RegExp)[] = [];
    const titles: string[] = [];
    choose = (items, options) => {
      titles.push(options.title ?? '');
      const index = picks.findIndex((label) => items.some((item) => (typeof label === 'string' ? item.label === label : label.test(item.label))));
      return byLabel(picks.splice(Math.max(index, 0), 1)[0])(items);
    };
    await config.update('branches.format', 'PRJ-${input:ticket}-${input:kind}-${base}', vscode.ConfigurationTarget.Global);
    await config.update(
      'branches.inputs',
      [
        { id: 'ticket', type: 'promptString', description: 'Ticket number', pattern: '^\\d+$' },
        { id: 'kind', type: 'pickString', description: 'Type of change', options: ['feat', 'fix'], allowCustom: true },
      ],
      vscode.ConfigurationTarget.Global,
    );
    try {
      answer = () => undefined;
      // The format, picked first, from main: the ticket typed, the kind picked, and the name it makes kept.
      picks = ['Use the format', 'main', 'fix'];
      typed = ['123', KEEP];
      await controller.action({ kind: 'worktreeAdd' });
      const made = state().worktrees.find((worktree) => worktree.branch === 'refs/heads/PRJ-123-fix-main');
      assert.ok(made, JSON.stringify(state().worktrees));
      assert.equal(made.folder.toLowerCase(), `${home}/PRJ-123-fix-main`.toLowerCase());

      // From a remote branch, ${base} has no remote; the ticket used last is proposed, and a kind can be typed.
      picks = ['Use the format', 'origin/main', 'Type another value…'];
      proposed.length = 0;
      typed = [KEEP, 'hotfix', KEEP];
      await controller.action({ kind: 'worktreeAdd' });
      assert.equal(proposed[0], '123');
      assert.ok(state().worktrees.some((worktree) => worktree.branch === 'refs/heads/PRJ-123-hotfix-main'), JSON.stringify(state().worktrees));

      // A typed ticket that is not a number is refused.
      errors.length = 0;
      picks = ['Use the format', 'main'];
      typed = ['12a'];
      await controller.action({ kind: 'worktreeAdd' });
      assert.ok(errors.some((text) => /It has to match/.test(text)), JSON.stringify(errors));
      errors.length = 0;

      // At a commit, ${base} is the commit or one of the branches that have it.
      const head = state().head.commit!;
      picks = ['Use the format', head.slice(0, 7), 'feat'];
      typed = ['7', KEEP];
      await controller.action({ kind: 'worktreeAdd', ref: head, isBranch: false });
      assert.ok(state().worktrees.some((worktree) => worktree.branch === `refs/heads/PRJ-7-feat-${head.slice(0, 7)}`), JSON.stringify(state().worktrees));
      assert.ok(titles.includes(`Create worktree at ${head.slice(0, 7)}: what \${base} is`), JSON.stringify(titles));

      // Or the name is typed, as without a format.
      picks = ['Type a branch name', 'main'];
      typed = ['typed/name'];
      await controller.action({ kind: 'worktreeAdd' });
      assert.ok(state().worktrees.some((worktree) => worktree.branch === 'refs/heads/typed/name'), JSON.stringify(state().worktrees));

      // On a branch, the branch itself, in a folder of its name, or a new branch from the format.
      await git(app, ['branch', 'side/work']);
      await vscode.commands.executeCommand('gitardo.refresh');
      picks = [/^Use the branch `?side\/work`?$/];
      typed = [KEEP];
      await controller.action({ kind: 'worktreeAdd', ref: 'side/work', isBranch: true });
      const side = state().worktrees.find((worktree) => worktree.branch === 'refs/heads/side/work');
      assert.ok(side, JSON.stringify(state().worktrees));
      assert.equal(side.folder.toLowerCase(), `${home}/side/work`.toLowerCase());

      // Removed from the menu of its branch, which says whether the branch goes too: here it stays, without a question.
      answer = run;
      picks = [];
      await controller.action({ kind: 'worktreeRemove', path: side.folder, deleteBranch: false });
      assert.ok(!state().worktrees.some((worktree) => worktree.branch === 'refs/heads/side/work'));
      assert.ok(state().branches.some((branch) => branch.name === 'side/work'));
      assert.ok(!fs.existsSync(`${home}/side`), 'the folder that held only it went too');
      answer = () => undefined;
    } finally {
      ui.showInputBox = showInputBox;
      choose = () => undefined;
      await config.update('branches.format', undefined, vscode.ConfigurationTarget.Global);
      await config.update('branches.inputs', undefined, vscode.ConfigurationTarget.Global);
      // Windows can hold a folder that was just made for a moment: it is deleted with retries, and git forgets it after.
      fs.rmSync(home, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
      await git(app, ['worktree', 'prune']);
      const made = (await git(app, ['for-each-ref', '--format=%(refname:short)', 'refs/heads'])).stdout.split('\n');
      for (const branch of made) {
        if (/^(PRJ-|typed\/|side\/)/.test(branch)) await git(app, ['branch', '-D', branch]);
      }
      await vscode.commands.executeCommand('gitardo.refresh');
      fs.rmSync(home, { recursive: true, force: true });
    }
    assert.equal(state().branches.length, branches);
    assert.equal(state().worktrees.length, 1);
  });

  await step('links and copies into a new worktree what the settings of VS Code ask, and unlinks before removing it', async () => {
    const config = vscode.workspace.getConfiguration('git');
    const home = path.join(workspace, 'app.worktrees').replace(/\\/g, '/');
    const exclude = path.join(app, '.git', 'info', 'exclude');
    const excluded = fs.existsSync(exclude) ? fs.readFileSync(exclude, 'utf8') : null;
    const branches = state().branches.length;
    let made: string | undefined;
    // Ignored through the repository's own exclude file, which leaves what is committed alone.
    fs.mkdirSync(path.dirname(exclude), { recursive: true });
    fs.writeFileSync(exclude, `${excluded ?? ''}\nnode_modules/\n.env\n`);
    fs.mkdirSync(path.join(app, 'node_modules', 'pkg'), { recursive: true });
    write('node_modules/pkg/index.js', 'module\n');
    write('.env', 'SECRET=1\n');
    await config.update('worktreeSymlinkFolders', ['node_modules'], vscode.ConfigurationTarget.Global);
    await config.update('worktreeIncludeFiles', ['.env'], vscode.ConfigurationTarget.Global);
    try {
      logged.length = 0;
      answer = () => undefined;
      inputs = ['linked/one'];
      choose = byLabel('main');
      await controller.action({ kind: 'worktreeAdd' });
      made = state().worktrees.find((worktree) => worktree.branch === 'refs/heads/linked/one')?.folder;
      assert.ok(made, JSON.stringify(state().worktrees));
      // After git made it, each one said in the log as it is made.
      const steps = logged.filter((line) => line.startsWith('> '));
      assert.match(steps[0], /^> git worktree add /);
      assert.deepEqual(steps.slice(1), ['> symlink node_modules', '> copy .env']);
      assert.ok(fs.lstatSync(path.join(made, 'node_modules')).isSymbolicLink());
      assert.equal(fs.readFileSync(path.join(made, 'node_modules/pkg/index.js'), 'utf8'), 'module\n');
      assert.ok(!fs.lstatSync(path.join(made, '.env')).isSymbolicLink());
      assert.equal(fs.readFileSync(path.join(made, '.env'), 'utf8'), 'SECRET=1\n');

      // VS Code's Git extension opens worktrees as repositories, and watches their folders, which Windows then keeps:
      // it closes this one first.
      const vscodeRepos = await vscodeGit();
      await vscodeRepos.openRepository(vscode.Uri.file(made));
      const openInGit = () => vscodeRepos.repositories.some((repository) => samePath(repository.rootUri.fsPath, made!));
      await until('the worktree open in VS Code', () => openInGit());
      // Git for Windows would delete what is in the junction with the worktree, in this checkout: it goes first.
      logged.length = 0;
      answer = run;
      await controller.action({ kind: 'worktreeRemove', path: made, deleteBranch: true });
      // The engine may write the path of a link with the separators of the system.
      const removal = logged
        .filter((line) => /^> (git\.close|unlink|git worktree remove) /.test(line))
        .map((line) => line.replace(/\\/g, '/').toLowerCase());
      // Elsewhere than Windows git would refuse the worktree for the link, which `node_modules/` does not ignore.
      assert.deepEqual(
        removal,
        [`> git.close ${made}`, `> unlink ${made}/node_modules`, `> git worktree remove ${made}`].map((line) => line.toLowerCase()),
      );
      assert.ok(!openInGit());
      assert.ok(!state().worktrees.some((worktree) => worktree.branch === 'refs/heads/linked/one'));
      assert.ok(!fs.existsSync(made), JSON.stringify({ logged, errors }));
      assert.equal(fs.readFileSync(path.join(app, 'node_modules/pkg/index.js'), 'utf8'), 'module\n');
      assert.equal(state().branches.length, branches);
    } finally {
      answer = () => undefined;
      choose = () => undefined;
      await config.update('worktreeSymlinkFolders', undefined, vscode.ConfigurationTarget.Global);
      await config.update('worktreeIncludeFiles', undefined, vscode.ConfigurationTarget.Global);
      // The link goes before its folder does, so that nothing of this checkout goes with it.
      const link = made && path.join(made, 'node_modules');
      if (link && fs.lstatSync(link, { throwIfNoEntry: false })?.isSymbolicLink()) fs.unlinkSync(link);
      fs.rmSync(home, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
      await git(app, ['worktree', 'prune']);
      await git(app, ['branch', '-D', 'linked/one']).catch(() => undefined);
      fs.rmSync(path.join(app, 'node_modules'), { recursive: true, force: true });
      fs.rmSync(path.join(app, '.env'), { force: true });
      if (excluded === null) fs.rmSync(exclude, { force: true });
      else fs.writeFileSync(exclude, excluded);
      await vscode.commands.executeCommand('gitardo.refresh');
    }
  });

  await step('chooses what new worktrees get in a tab of its own, saves it for the workspace folder, and brings it into worktrees', async () => {
    const gitardo = () => vscode.workspace.getConfiguration('gitardo', vscode.Uri.file(app));
    const exclude = path.join(app, '.git', 'info', 'exclude');
    const excluded = fs.existsSync(exclude) ? fs.readFileSync(exclude, 'utf8') : null;
    const home = path.join(workspace, 'app.worktrees').replace(/\\/g, '/');
    let made: string | undefined;
    fs.writeFileSync(exclude, `${excluded ?? ''}\nnode_modules/\n.env\n*.log\n`);
    fs.mkdirSync(path.join(app, 'node_modules', 'pkg'), { recursive: true });
    write('node_modules/pkg/index.js', 'module\n');
    write('.env', 'SECRET=2\n');
    write('build.log', 'not wanted\n');
    try {
      posted.length = 0;
      await vscode.commands.executeCommand('gitardo.worktreeSetup');
      const tab = (await until('the tab', () => [...internals.views].find(([, kind]) => kind === 'worktreeSetup')))[0];
      // The page asks for what it shows as it loads: what git ignores, each with what is proposed for it.
      type Setup = Extract<HostMessage, { type: 'worktreeSetup' }>;
      const shown = await until('what the tab shows', () => posted.find((message): message is Setup => message.type === 'worktreeSetup' && !!message.info));
      const proposed = Object.fromEntries(shown.info!.entries.map((entry) => [entry.path, entry.suggestion]));
      assert.deepEqual(proposed, { '.env': 'copy', 'build.log': 'none', node_modules: 'link' });
      assert.deepEqual(shown.info!.effective, { link: [], copy: [], linkFrom: 'none', copyFrom: 'none' });
      assert.equal(shown.info!.workspace?.name, path.basename(workspace));

      // What the patterns take, as the page asks while they are typed.
      const patterns = { link: ['node_modules/'], copy: ['/.env', '*.missing'] };
      await internals.onMessage(tab, { type: 'previewWorktreeSetup', patterns });
      type Preview = Extract<HostMessage, { type: 'worktreeSetupPreview' }>;
      // The page asks for its own as well, so the answer is the one to these patterns.
      const answered = (message: HostMessage): message is Preview =>
        message.type === 'worktreeSetupPreview' && JSON.stringify(message.patterns) === JSON.stringify(patterns);
      const preview = (await until('what the patterns take', () => posted.find(answered))).preview!;
      assert.deepEqual([preview.link, preview.copy, preview.counts.copy], [['node_modules'], ['.env'], { '/.env': 1, '*.missing': 0 }]);

      // Saved for the workspace folder: Gitardo's settings, which apply from then on, written by the engine's plan and
      // logged (M48).
      posted.length = 0;
      logged.length = 0;
      await internals.onMessage(tab, { type: 'saveWorktreeSetup', target: 'workspace', patterns });
      assert.deepEqual(gitardo().inspect('worktrees.linkFolders')?.workspaceFolderValue, ['node_modules/']);
      assert.deepEqual(gitardo().inspect('worktrees.copyFiles')?.workspaceFolderValue, ['/.env', '*.missing']);
      assert.ok(logged.includes('> set gitardo.worktrees.linkFolders ["node_modules/"] in the workspace folder'), JSON.stringify(logged));
      const saved = await until('the tab to show what is saved', () => posted.find((message): message is Setup => message.type === 'worktreeSetup' && !!message.info));
      assert.deepEqual([saved.info!.effective.linkFrom, saved.info!.effective.copyFrom], ['workspace', 'workspace']);

      // From the Explorer, a file copied by its own path is saved where the copies are set, then taken out again.
      const log = vscode.Uri.file(path.join(app, 'build.log'));
      logged.length = 0;
      asked.length = 0;
      await vscode.commands.executeCommand('gitardo.copyIntoWorktrees', log);
      assert.deepEqual(gitardo().inspect('worktrees.copyFiles')?.workspaceFolderValue, ['/.env', '*.missing', '/build.log']);
      assert.ok(logged.includes('> set gitardo.worktrees.copyFiles ["/.env","*.missing","/build.log"] in the workspace folder'), JSON.stringify(logged));
      await until('what became of it', () =>
        asked.find((question) => question.message === 'Gitardo: build.log will be copied into new worktrees. Saved in the settings of the workspace folder.'),
      );
      await vscode.commands.executeCommand('gitardo.stopCopyingIntoWorktrees', log);
      assert.deepEqual(gitardo().inspect('worktrees.copyFiles')?.workspaceFolderValue, ['/.env', '*.missing']);

      // A new worktree gets it, and no longer offers to choose it.
      logged.length = 0;
      asked.length = 0;
      answer = () => undefined;
      inputs = ['setup/one'];
      choose = byLabel('main');
      await controller.action({ kind: 'worktreeAdd' });
      made = state().worktrees.find((worktree) => worktree.branch === 'refs/heads/setup/one')?.folder;
      assert.ok(made, JSON.stringify(state().worktrees));
      assert.deepEqual(
        logged.filter((line) => line.startsWith('> ') && !line.startsWith('> git ')),
        ['> symlink node_modules', '> copy .env'],
      );
      assert.ok(!fs.existsSync(path.join(made, 'build.log')));
      const offer = await until('the offer to open it', () => asked.find((question) => /^Worktree created in /.test(question.message)));
      assert.ok(!offer.items.includes('Choose What It Gets…'), JSON.stringify(offer.items));

      // Brought again into it, from its context menu: only what it no longer has.
      fs.rmSync(path.join(made, '.env'));
      logged.length = 0;
      answer = run;
      await controller.action({ kind: 'worktreeSetUp', path: made });
      assert.equal(fs.readFileSync(path.join(made, '.env'), 'utf8'), 'SECRET=2\n');
      assert.ok(logged.includes('> copy .env'), JSON.stringify(logged));
      // The extension says what the worktree has already; the engine brings only what it lacks (E3 of the differences).

      await controller.action({ kind: 'worktreeRemove', path: made, deleteBranch: true });
      assert.ok(!fs.existsSync(made), JSON.stringify({ logged, errors }));
      assert.equal(fs.readFileSync(path.join(app, 'node_modules/pkg/index.js'), 'utf8'), 'module\n');

      // Taken out of the workspace folder, nothing is set there any more.
      await internals.onMessage(tab, { type: 'saveWorktreeSetup', target: 'workspace', patterns: null });
      assert.equal(gitardo().inspect('worktrees.linkFolders')?.workspaceFolderValue, undefined);
      assert.equal(gitardo().inspect('worktrees.copyFiles')?.workspaceFolderValue, undefined);
    } finally {
      answer = () => undefined;
      choose = () => undefined;
      await gitardo().update('worktrees.linkFolders', undefined, vscode.ConfigurationTarget.WorkspaceFolder);
      await gitardo().update('worktrees.copyFiles', undefined, vscode.ConfigurationTarget.WorkspaceFolder);
      const link = made && path.join(made, 'node_modules');
      if (link && fs.lstatSync(link, { throwIfNoEntry: false })?.isSymbolicLink()) fs.unlinkSync(link);
      fs.rmSync(home, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
      await git(app, ['worktree', 'prune']);
      await git(app, ['branch', '-D', 'setup/one']).catch(() => undefined);
      for (const file of ['node_modules', '.env', 'build.log']) fs.rmSync(path.join(app, file), { recursive: true, force: true });
      if (excluded === null) fs.rmSync(exclude, { force: true });
      else fs.writeFileSync(exclude, excluded);
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');
      await vscode.commands.executeCommand('gitardo.refresh');
    }
  });

  await step('runs the commands for new worktrees in tasks, in order, before opening the worktree, and stops at one that fails', async () => {
    const gitardo = () => vscode.workspace.getConfiguration('gitardo', vscode.Uri.file(app));
    const home = path.join(workspace, 'app.worktrees').replace(/\\/g, '/');
    const commands = vscode.commands as unknown as Record<string, (...args: unknown[]) => Thenable<unknown>>;
    const executeCommand = commands.executeCommand;
    const opened: string[] = [];
    commands.executeCommand = (id: unknown, ...args: unknown[]) => {
      if (id !== 'vscode.openFolder') return executeCommand(id, ...args);
      opened.push((args[0] as vscode.Uri).fsPath.replace(/\\/g, '/').toLowerCase());
      return Promise.resolve();
    };
    const ref = async (name: string) => (await git(app, ['rev-parse', '--verify', '--quiet', name]).catch(() => null))?.stdout.trim();
    await git(app, ['branch', 'commands/one', 'main']);
    await git(app, ['branch', 'commands/two', 'main']);
    await vscode.commands.executeCommand('gitardo.refresh');
    try {
      // No quotes, nothing but a command and its arguments: the same in every shell. The second needs the first.
      const made = ['git update-ref refs/setup/one HEAD', 'git update-ref refs/setup/two refs/setup/one'];
      // Saved from the tab, with the patterns, and without what is empty.
      await vscode.commands.executeCommand('gitardo.worktreeSetup');
      const tab = (await until('the tab', () => [...internals.views].find(([, kind]) => kind === 'worktreeSetup')))[0];
      await internals.onMessage(tab, { type: 'saveWorktreeSetup', target: 'workspace', patterns: { link: [], copy: [] }, commands: [...made, ' '] });
      assert.deepEqual(gitardo().inspect('worktrees.setupCommands')?.workspaceFolderValue, made);
      logged.length = 0;
      asked.length = 0;
      await controller.action({ kind: 'openInWorktree', name: 'commands/one' });
      const one = `${home}/commands/one`.toLowerCase();
      await until('the worktree to open once the commands are done', () => opened.includes(one), 60_000);
      assert.equal(await ref('refs/setup/two'), await ref('main'));
      assert.deepEqual(logged.filter((line) => line.startsWith('> run ')), made.map((command) => `> run ${command}`));
      assert.equal(asked.length, 0, JSON.stringify(asked));

      // One that fails stops those after it, and the worktree is only offered.
      const failing = ['git rev-parse --verify --quiet refs/setup/missing', 'git update-ref refs/setup/after HEAD'];
      await gitardo().update('worktrees.setupCommands', failing, vscode.ConfigurationTarget.WorkspaceFolder);
      opened.length = 0;
      await controller.action({ kind: 'openInWorktree', name: 'commands/two' });
      const stopped = await until('the warning that a command failed', () => asked.find((question) => /failed with exit code/.test(question.message)), 60_000);
      assert.match(stopped.message, /git rev-parse --verify --quiet refs\/setup\/missing failed with exit code 1 in two, so the command after it did not run/);
      assert.ok(stopped.items.includes('Open in New Window'), JSON.stringify(stopped.items));
      assert.deepEqual(opened, []);
      assert.equal(await ref('refs/setup/after'), undefined);
    } finally {
      commands.executeCommand = executeCommand;
      for (const key of ['worktrees.setupCommands', 'worktrees.linkFolders', 'worktrees.copyFiles']) {
        await gitardo().update(key, undefined, vscode.ConfigurationTarget.WorkspaceFolder);
      }
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');
      for (const name of ['one', 'two']) await git(app, ['worktree', 'remove', '--force', `${home}/commands/${name}`]).catch(() => undefined);
      fs.rmSync(home, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
      await git(app, ['worktree', 'prune']);
      for (const name of ['commands/one', 'commands/two']) await git(app, ['branch', '-D', name]).catch(() => undefined);
      for (const name of ['one', 'two', 'after']) await git(app, ['update-ref', '-d', `refs/setup/${name}`]).catch(() => undefined);
      await vscode.commands.executeCommand('gitardo.refresh');
    }
  });

  await step('shows another worktree in the views of this window, works in it there and goes back', async () => {
    const home = path.join(workspace, 'app.worktrees').replace(/\\/g, '/');
    const dir = `${home}/shown`;
    const same = (a: string, b: string) => a.replace(/\\/g, '/').toLowerCase() === b.replace(/\\/g, '/').toLowerCase();
    const branches = state().branches.length;
    const back = state().repos[0];
    await git(app, ['worktree', 'add', '-b', 'shown/one', dir, 'main']);
    await vscode.commands.executeCommand('gitardo.refresh');
    try {
      await controller.repositories.selectRepo(dir);
      assert.ok(same(state().root, dir), state().root);
      assert.equal(checkedOutBranch(state()), 'shown/one');
      assert.deepEqual(state().home, back);
      // The window's own folder is listed as another worktree, which is neither renamed nor removed from here.
      const window = state().worktrees.find((worktree) => same(worktree.folder, app));
      assert.ok(window && !window.isCurrent);
      errors.length = 0;
      await controller.action({ kind: 'worktreeRemove', path: window.folder, deleteBranch: false });
      assert.ok(errors.some((text) => /folder open in this window/.test(text)), JSON.stringify(errors));
      errors.length = 0;

      // What is done in the views is done in that worktree.
      fs.writeFileSync(path.join(dir, 'shown.txt'), 'shown\n');
      await vscode.commands.executeCommand('gitardo.refresh');
      assert.deepEqual(state().workingTree.untracked, ['shown.txt']);
      await controller.action({ kind: 'stage', paths: ['shown.txt'] });
      assert.equal((await git(dir, ['diff', '--cached', '--name-only'])).stdout.trim(), 'shown.txt');
      assert.equal((await git(app, ['diff', '--cached', '--name-only'])).stdout.trim(), '');

      // And back to the folder of the window.
      await controller.repositories.selectRepo(back.root);
      assert.equal(state().root, back.root);
      assert.equal(state().home, null);
    } finally {
      if (!same(state().root, back.root)) await controller.repositories.selectRepo(back.root);
      await git(app, ['worktree', 'remove', '--force', dir]).catch(() => undefined);
      fs.rmSync(home, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
      await git(app, ['worktree', 'prune']);
      await git(app, ['branch', '-D', 'shown/one']).catch(() => undefined);
      await vscode.commands.executeCommand('gitardo.refresh');
    }
    assert.equal(state().branches.length, branches);
    assert.equal(state().worktrees.length, 1);
  });

  await step("follows the repository selected in VS Code's Repositories view, selects there the one shown, and shows a worktree of another", async () => {
    const vscodeApi = await vscodeGit();
    const opened = (root: string) => vscodeApi.repositories.find((repository) => samePath(repository.rootUri.fsPath, root));
    const selected = () => vscodeApi.repositories.filter((repository) => repository.ui.selected).map((repository) => path.basename(repository.rootUri.fsPath));
    // The view makes the commands that select in it; the worktrees of the steps before, gone from disk, are closed.
    await vscode.commands.executeCommand('workbench.view.scm');
    // VS Code's Git may keep a worktree a step deleted with git open for a long while, and may list one only after a
    // while, under load: each one gone from disk is closed as it is seen, as Gitardo closes those it removes itself,
    // asked again every two seconds until it is gone from the list. Closing them once before waiting failed now and
    // then, when the list had not caught up yet.
    const asked = new Map<string, number>();
    const closeGone = () => {
      for (const repository of vscodeApi.repositories) {
        const root = repository.rootUri.fsPath;
        if (fs.existsSync(root) || Date.now() - (asked.get(root) ?? 0) < 2000) continue;
        asked.set(root, Date.now());
        vscode.commands.executeCommand('git.close', repository).then(undefined, () => undefined);
      }
    };
    await until("VS Code's Git extension to have app and lib open, alone", () => (closeGone(), opened(app) && opened(lib) && vscodeApi.repositories.length === 2), 30_000).catch(
      (error: Error) => {
        const listed = vscodeApi.repositories.map((repository) => `${repository.rootUri.fsPath}${fs.existsSync(repository.rootUri.fsPath) ? '' : ' (gone)'}`);
        throw new Error(`${error.message}; it has ${listed.join(', ')}`);
      },
    );
    const scm = vscode.workspace.getConfiguration('scm');
    const home = path.join(workspace, 'lib.worktrees').replace(/\\/g, '/');
    const beside = `${home}/beside`;
    try {
      // With several repositories listed at once, Gitardo leaves the view as it is.
      await until('app to be the repository Source Control is on', () => opened(app)?.ui.selected);
      await controller.repositories.selectRepo(lib);
      await sleep(1500);
      assert.deepEqual(selected(), ['app']);

      // With one at a time, choosing one in Gitardo selects it in the view.
      await scm.update('repositories.selectionMode', 'single', vscode.ConfigurationTarget.Workspace);
      // The view keeps the first repository it lists, which may move Source Control.
      await sleep(500);
      await controller.repositories.selectRepo(app);
      await controller.repositories.selectRepo(lib);
      await until(`lib to be selected in the Repositories view, not ${selected()}`, () => opened(lib)?.ui.selected && !opened(app)?.ui.selected);

      // Chosen there, as a click on it would, it is shown in Gitardo.
      const prefix = 'workbench.scm.action.toggleRepositoryVisibility.scm';
      const toggles = (await vscode.commands.getCommands(true))
        .filter((id) => id.startsWith(prefix))
        .sort((a, b) => Number(a.slice(prefix.length)) - Number(b.slice(prefix.length)));
      // The Git extension hands out new objects each time it is asked: a repository is found by its root.
      const roots = vscodeApi.repositories.map((repository) => repository.rootUri.fsPath);
      assert.equal(toggles.length, roots.length, JSON.stringify({ toggles, roots }));
      await vscode.commands.executeCommand(toggles[roots.findIndex((root) => samePath(root, app))]);
      await until('Gitardo to show app', () => samePath(state().root, app));
      assert.deepEqual(selected(), ['app']);

      // A worktree of lib, which is not in the workspace, is shown as one of lib.
      await git(lib, ['worktree', 'add', '-b', 'beside', beside]);
      await controller.repositories.selectRepo(beside);
      assert.ok(samePath(state().root, beside), state().root);
      assert.equal(checkedOutBranch(state()), 'beside');
      assert.ok(state().home && samePath(state().home!.root, lib), JSON.stringify(state().home));
    } finally {
      await scm.update('repositories.selectionMode', undefined, vscode.ConfigurationTarget.Workspace);
      if (!samePath(state().root, app)) await controller.repositories.selectRepo(app);
      await git(lib, ['worktree', 'remove', '--force', beside]).catch(() => undefined);
      fs.rmSync(home, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
      await git(lib, ['worktree', 'prune']);
      await git(lib, ['branch', '-D', 'beside']).catch(() => undefined);
    }
    assert.ok(samePath(state().root, app), state().root);
  });

  // What a repository keeps while another is shown (repo.keep-open), and how it follows its changes meanwhile
  // (repo.watch-hidden), with gitardo.repositories.keepOpen on and off.
  const keepOpen = (on: boolean | undefined) => vscode.workspace.getConfiguration('gitardo').update('repositories.keepOpen', on, vscode.ConfigurationTarget.Global);
  /** The reads of the repository each title names, as the status bar says them, from the time this is called. */
  const readsFrom = () => {
    const tasks: { title: string; task: { detail?: string } }[] = [];
    const start = controller.activity.start;
    // Each turn of a read says which repository it reads, the first one too: a read of another that follows one under
    // way is a turn of the same task.
    controller.activity.start = (title: string, detail?: string) => {
      const task = start.call(controller.activity, title, detail);
      const describe = task.describe.bind(task);
      task.describe = (said: string | undefined, named = task.title) => {
        if (named.startsWith('Reading ')) tasks.push({ title: named, task });
        describe(said, named);
      };
      return task;
    };
    return {
      of: (repository: string) => tasks.filter(({ title }) => title === `Reading ${repository}`),
      stop: () => {
        controller.activity.start = start;
      },
    };
  };
  const nameOf = (root: string) => state().repos.find((repo) => samePath(repo.root, root))?.name ?? '';
  /** What the views were told from then on of the reads of the graph and of the repositories shown. */
  const toldFrom = (from: number) =>
    posted.slice(from).flatMap((message) => {
      if (message.type === 'loading') return [`loading ${message.what}`];
      if (message.type === 'state') return [`state ${message.state ? path.basename(message.state.root) : 'none'}`];
      return [];
    });

  await step('with keeping repositories open on, shows a repository chosen again at once as it was left, and reads nothing for it when nothing changed', async () => {
    const counted = readsFrom();
    // The window getting the focus asks for a read, as it should, and the windows of the runners get it at any moment:
    // that is not what this step looks at, so it asks for none here.
    const schedule = internals.scheduleRefresh;
    internals.scheduleRefresh = (cause, delay) => {
      if (cause !== 'VS Code got the focus') schedule.call(controller, cause, delay);
    };
    await keepOpen(true);
    try {
      // What app shows: the history of a file, the local branches only, and more than a page of it asked for.
      await controller.history.setFilter({ kind: 'path', path: 'b.txt' });
      await controller.history.setScope({ kind: 'kinds', kinds: ['branch'] });
      internals.limit = 2500;
      await vscode.commands.executeCommand('gitardo.refresh');
      const left = state();
      assert.equal(left.keepOpen, true);
      assert.deepEqual(left.filter, { kind: 'path', path: 'b.txt' });

      await controller.repositories.selectRepo(lib);
      assert.ok(samePath(state().root, lib), state().root);
      // Shown for the first time, lib is read, nothing narrowed, as before.
      assert.equal(state().filter, null);
      assert.ok(counted.of(nameOf(lib)).length > 0, 'lib is read');
      assert.ok(internals.kept.has(app), 'app keeps what it showed');

      const from = posted.length;
      const appReads = counted.of(nameOf(app)).length;
      // Why app would be read once shown, if something changed in it meanwhile.
      const changed = internals.kept.outOfDate(app);
      await controller.repositories.selectRepo(app);
      assert.ok(samePath(state().root, app), state().root);
      assert.deepEqual(
        state().rows.map((row) => row.hash),
        left.rows.map((row) => row.hash),
      );
      assert.deepEqual(state().filter, left.filter);
      assert.deepEqual(state().scope, left.scope);
      assert.deepEqual(internals.filter, left.filter);
      assert.equal(internals.limit, 2500);
      // At once: the views are sent its state, and are not told to wait for its commits.
      assert.deepEqual(toldFrom(from), ['state app'], `app was marked out of date: ${changed}`);
      await sleep(1500);
      assert.equal(counted.of(nameOf(app)).length, appReads, 'app is not read again');
      assert.ok(!internals.kept.has(app), 'app, shown, keeps nothing apart');
      assert.ok(internals.kept.has(lib), 'lib keeps what it showed');
      // Choosing the repository shown reads nothing either.
      await controller.repositories.selectRepo(app);
      assert.equal(counted.of(nameOf(app)).length, appReads);
    } finally {
      counted.stop();
      internals.scheduleRefresh = schedule;
      // What app was narrowed to is kept for it: it is taken off there, whichever repository a failure left shown.
      if (!samePath(state().root, app)) await controller.repositories.selectRepo(app);
      await controller.history.setFilter(null);
      await controller.history.setScope(null);
      await keepOpen(undefined);
    }
  });

  await step('with keeping repositories open on, keeps the message of the Changes view of each repository and gives it back when it is shown again', async () => {
    const changes = view('changes');
    await keepOpen(true);
    try {
      const told = (from: number) => posted.slice(from).filter((message) => message.type === 'keptMessage');
      // The box was pointed from app to lib: lib had none kept, so the box keeps what it has.
      let from = posted.length;
      await internals.onMessage(changes, { type: 'keepMessage', root: app, message: 'Fix the parser', shown: lib });
      assert.deepEqual(told(from), []);
      // And back to app: the message app had comes back, and the one of lib is kept.
      from = posted.length;
      await internals.onMessage(changes, { type: 'keepMessage', root: lib, message: 'Update the docs', shown: app });
      assert.deepEqual(told(from), [{ type: 'keptMessage', root: app, message: 'Fix the parser' }]);
      from = posted.length;
      await internals.onMessage(changes, { type: 'keepMessage', root: app, message: '', shown: lib });
      assert.deepEqual(told(from), [{ type: 'keptMessage', root: lib, message: 'Update the docs' }]);
    } finally {
      await keepOpen(undefined);
    }
    // With the setting off, nothing is kept and nothing given back.
    const from = posted.length;
    await internals.onMessage(changes, { type: 'keepMessage', root: lib, message: 'Update the docs', shown: app });
    assert.ok(!posted.slice(from).some((message) => message.type === 'keptMessage'));
  });

  await step('with keeping repositories open off, its default, coming back to a repository shows it as choosing a repository did before', async () => {
    assert.equal(vscode.workspace.getConfiguration('gitardo').get('repositories.keepOpen'), false);
    assert.equal(state().keepOpen, false);
    const counted = readsFrom();
    try {
      await controller.history.setFilter({ kind: 'path', path: 'b.txt' });
      await controller.repositories.selectRepo(lib);
      assert.ok(!internals.kept.has(app), 'app keeps nothing');
      const from = posted.length;
      const appReads = counted.of(nameOf(app)).length;
      await controller.repositories.selectRepo(app);
      // Its commits are read again, nothing narrowed, and the views wait for them as before.
      assert.equal(state().filter, null);
      const told = toldFrom(from);
      assert.deepEqual(
        told.filter((line) => line.startsWith('loading')),
        ['loading filter', 'loading null'],
      );
      assert.ok(told.includes('state app'), told.join(', '));
      assert.ok(counted.of(nameOf(app)).length > appReads, 'app is read again');
    } finally {
      counted.stop();
      await controller.history.setFilter(null);
    }
  });

  await step('turning keeping repositories open off forgets at once what the repositories not shown kept, and tells the views', async () => {
    await keepOpen(true);
    try {
      await controller.repositories.selectRepo(lib);
      await controller.repositories.selectRepo(app);
      assert.ok(internals.kept.has(lib), 'lib keeps what it showed');
      const from = posted.length;
      await keepOpen(false);
      await until('lib to keep nothing', () => !internals.kept.has(lib));
      assert.ok(posted.slice(from).some((message) => message.type === 'state' && message.state?.keepOpen === false));
    } finally {
      await keepOpen(undefined);
    }
  });

  await step('with keeping repositories open on, a commit made in a terminal in a repository not shown reads nothing, and showing it reads it once', async () => {
    const counted = readsFrom();
    await keepOpen(true);
    const first = (await git(lib, ['rev-parse', 'HEAD'])).stdout.trim();
    try {
      await controller.repositories.selectRepo(lib);
      const before = state().rows.map((row) => row.subject);
      await controller.repositories.selectRepo(app);
      assert.ok(internals.kept.has(lib), 'lib keeps what it showed');
      await git(lib, ['-c', 'user.name=Test Author', '-c', 'user.email=author@example.com', 'commit', '--allow-empty', '-m', 'From the terminal in lib']);
      const libReads = counted.of(nameOf(lib)).length;
      await until('lib to be marked out of date', () => internals.kept.outOfDate(lib));
      // Longer than a read of a change waits for.
      await sleep(1500);
      assert.equal(counted.of(nameOf(lib)).length, libReads, 'lib is not read while app is shown');

      // What it kept, at once, then the commit after one read, which the status bar says the reason for.
      const shown: string[][] = [];
      const listening = controller.onDidChangeState((each) => {
        if (each && samePath(each.root, lib)) shown.push(each.rows.map((row) => row.subject));
      });
      try {
        await controller.repositories.selectRepo(lib);
      } finally {
        listening.dispose();
      }
      assert.deepEqual(shown[0], before);
      await until('the new commit of lib', () => state().rows[0]?.subject === 'From the terminal in lib');
      const reads = counted.of(nameOf(lib)).slice(libReads);
      assert.equal(reads.length, 1, `lib read ${reads.length} times`);
      assert.match(reads[0].task.detail ?? '', /^Because .* changed\.$/);
      await sleep(1500);
      assert.equal(counted.of(nameOf(lib)).length, libReads + 1, 'lib is read once');
    } finally {
      counted.stop();
      if (!samePath(state().root, app)) await controller.repositories.selectRepo(app);
      await keepOpen(undefined);
      await git(lib, ['reset', '--hard', first]);
    }
  });

  await step('lists the worktrees of a repository of the workspace under it in the list of the repositories', async () => {
    const login = path.join(workspace, 'app-login');
    await git(app, ['worktree', 'add', '-b', 'login-page', login, 'main']);
    try {
      // The repositories of the workspace are looked for again, as when its folders change.
      await internals.locate();
      assert.deepEqual(
        state().repos.map((repo) => `${'  '.repeat(repo.depth)}${repo.name}`),
        ['workspace/app', '  workspace/app-login', 'workspace/lib'],
      );
    } finally {
      await git(app, ['worktree', 'remove', '--force', login]).catch(() => undefined);
      fs.rmSync(login, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
      await git(app, ['worktree', 'prune']);
      await git(app, ['branch', '-D', 'login-page']).catch(() => undefined);
      await internals.locate();
    }
    assert.deepEqual(
      state().repos.map((repo) => repo.name),
      ['workspace/app', 'workspace/lib'],
    );
    assert.ok(samePath(state().root, app), state().root);
  });

  await step('lists the repositories in the Repositories view with their worktrees and what each is on, counts a file changed in one not shown, and shows the row clicked', async () => {
    const login = path.join(workspace, 'app-login');
    type Rows = Extract<HostMessage, { type: 'repositories' }>;
    /** What the view was last sent. */
    const sent = (): Rows | undefined => (internals.repositoriesView.sent ? (JSON.parse(internals.repositoriesView.sent) as Rows) : undefined);
    const rows = () => sent()?.rows ?? [];
    const names = () => rows().map((row) => `${'  '.repeat(row.depth)}${row.kind} ${row.name}${row.isShown ? ' *' : ''}`);
    const row = (folder: string) => rows().find((each) => samePath(each.folder, folder));
    const listed = (expected: string[]) =>
      until(`the rows ${expected.join()}`, () => names().join() === expected.join()).catch((error: Error) => {
        throw new Error(`${error.message}; the rows are ${names().join()}`);
      });
    // The view is opened and in sight: the engine follows the folders, and is told what changes in them, only then.
    await vscode.commands.executeCommand('gitardo.openRepositories');
    await listed(['repository workspace/app *', 'repository workspace/lib']);
    await until('what lib is on', () => row(lib)?.summary);
    await git(app, ['worktree', 'add', '-b', 'login-page', login, 'main']);
    try {
      await internals.locate();
      await vscode.commands.executeCommand('gitardo.refresh');
      await listed(['repository workspace/app *', '  worktree app-login', 'repository workspace/lib']);
      // What each is on: the one shown from its own read, the others from the engine's.
      assert.deepEqual(row(app)?.summary?.head, { kind: 'branch', name: 'main' });
      await until('what the worktree is on', () => row(login)?.summary);
      assert.deepEqual(row(login)?.summary, { head: { kind: 'branch', name: 'login-page' }, changed: 0, ahead: null, behind: null });
      // The menu of its row is made of what the repository shown says of its worktrees.
      assert.ok(sent()?.shown?.worktrees.some((worktree) => samePath(worktree.folder, login)));

      // A file changed in a worktree that is not shown is counted on its row.
      fs.writeFileSync(path.join(login, 'seen.txt'), 'seen\n');
      await until('the file counted on the row of the worktree', () => row(login)?.summary?.changed === 1, 30_000);

      // A click on a row shows it in every view, and marks it.
      await internals.onMessage(view('repositories'), { type: 'selectRepo', root: login });
      await until('the worktree shown', () => samePath(state().root, login));
      assert.equal(checkedOutBranch(state()), 'login-page');
      await listed(['repository workspace/app', '  worktree app-login *', 'repository workspace/lib']);
      assert.equal(row(login)?.summary?.changed, 1);
      await internals.onMessage(view('repositories'), { type: 'selectRepo', root: lib });
      await until('lib shown', () => samePath(state().root, lib));
      await listed(['repository workspace/app', '  worktree app-login', 'repository workspace/lib *']);
      // The one left is read by the engine again, and says what it said while it was shown.
      await until('what the worktree left is on', () => row(login)?.summary?.changed === 1, 30_000);
    } finally {
      if (!samePath(state().root, app)) await controller.repositories.selectRepo(app);
      await git(app, ['worktree', 'remove', '--force', login]).catch(() => undefined);
      fs.rmSync(login, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
      await git(app, ['worktree', 'prune']);
      await git(app, ['branch', '-D', 'login-page']).catch(() => undefined);
      await internals.locate();
      await vscode.commands.executeCommand('gitardo.refresh');
    }
    await listed(['repository workspace/app *', 'repository workspace/lib']);
  });

  await step('opens a branch in its worktree, made for it with nothing asked, and switches between worktrees', async () => {
    const home = path.join(workspace, 'app.worktrees').replace(/\\/g, '/');
    const branches = state().branches.length;
    // The windows that would open are only noted.
    const commands = vscode.commands as unknown as Record<string, (...args: unknown[]) => Thenable<unknown>>;
    const executeCommand = commands.executeCommand;
    const opened: string[] = [];
    commands.executeCommand = (id: unknown, ...args: unknown[]) => {
      if (id !== 'vscode.openFolder') return executeCommand(id, ...args);
      opened.push((args[0] as vscode.Uri).fsPath.replace(/\\/g, '/').toLowerCase());
      return Promise.resolve();
    };
    const head = state().head.commit!;
    try {
      await git(app, ['push', 'origin', `${head}:refs/heads/remote/only`]);
      await vscode.commands.executeCommand('gitardo.refresh');
      assert.ok(state().remoteBranches.some((ref) => ref.name === 'origin/remote/only'));

      // A local branch: its worktree is made, named after it, and opened.
      asked.length = 0;
      await controller.action({ kind: 'openInWorktree', name: 'merged' });
      const merged = state().worktrees.find((worktree) => worktree.branch === 'refs/heads/merged');
      assert.ok(merged, JSON.stringify(state().worktrees));
      assert.equal(merged.folder.toLowerCase(), `${home}/merged`.toLowerCase());
      await until('the worktree to open', () => opened.includes(`${home}/merged`.toLowerCase()));
      assert.equal(asked.length, 0, JSON.stringify(asked));

      // Again: it is there, so it only opens.
      opened.length = 0;
      await controller.action({ kind: 'openInWorktree', name: 'merged' });
      assert.deepEqual(opened, [`${home}/merged`.toLowerCase()]);
      assert.equal(state().worktrees.length, 2);

      // A remote branch: a local branch by its name tracks it there.
      await controller.action({ kind: 'openInWorktree', name: 'origin/remote/only', remote: true });
      const remote = state().worktrees.find((worktree) => worktree.branch === 'refs/heads/remote/only');
      assert.ok(remote, JSON.stringify(state().worktrees));
      assert.equal(remote.folder.toLowerCase(), `${home}/remote/only`.toLowerCase());
      assert.equal(state().branches.find((branch) => branch.name === 'remote/only')?.upstream?.reference, 'refs/remotes/origin/remote/only');

      // The branch checked out here has no worktree to open.
      errors.length = 0;
      asked.length = 0;
      await controller.action({ kind: 'openInWorktree', name: checkedOutBranch(state())! });
      // The engine says it as a refusal, a warning; the extension as an error.
      const hereSaid = [...errors, ...asked.map((question) => question.message)];
      assert.ok(hereSaid.some((text) => /checked out in this window/.test(text)), JSON.stringify(hereSaid));
      errors.length = 0;

      // What is not committed in a worktree is counted.
      fs.writeFileSync(path.join(merged.folder, 'new.txt'), 'new\n');
      await vscode.commands.executeCommand('gitardo.refresh');
      assert.equal(state().worktrees.find((worktree) => worktree.branch === 'refs/heads/merged')?.uncommitted, 1);

      // Switch Worktree lists them all with how they stand, the one of this window last, and creates another from there.
      opened.length = 0;
      let listed: string[] = [];
      let described: string[] = [];
      choose = (items) => {
        listed = items.map((item) => item.label);
        described = items.map((item) => item.description ?? '');
        return byLabel(/ merged$/)(items);
      };
      await vscode.commands.executeCommand('gitardo.switchWorktree');
      assert.deepEqual(listed, ['$(worktree) merged', '$(worktree) remote/only', '$(home) app', '', '$(add) Create worktree…']);
      assert.equal(described[0], 'merged · $(edit) 1');
      assert.match(described[2], / · this window$/);
      assert.deepEqual(opened, [`${home}/merged`.toLowerCase()]);
      opened.length = 0;
      choose = byLabel(/ app$/);
      await vscode.commands.executeCommand('gitardo.switchWorktree');
      assert.deepEqual(opened, []);
    } finally {
      commands.executeCommand = executeCommand;
      choose = () => undefined;
      // Each removal makes the views refresh, which runs `git status` in the other worktrees: on Windows a folder that a process
      // runs in cannot be deleted, so a removal can fail. What it leaves is deleted with retries and pruned below.
      for (const worktree of state().worktrees.filter((each) => !each.isMain)) {
        await git(app, ['worktree', 'remove', '--force', worktree.folder]).catch(() => undefined);
      }
      fs.rmSync(home, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
      await git(app, ['worktree', 'prune']);
      await git(app, ['branch', '-D', 'remote/only']).catch(() => undefined);
      await git(app, ['push', 'origin', '--delete', 'remote/only']);
      await vscode.commands.executeCommand('gitardo.refresh');
    }
    assert.equal(state().branches.length, branches);
    assert.equal(state().worktrees.length, 1);
  });

  await step('serves file contents at a commit and in the index, and opens diffs', async () => {
    const head = state().head.commit!;
    const at = async (ref: string) => (await vscode.workspace.openTextDocument(contentUri(state().root, ref, 'a.txt'))).getText();
    assert.equal(await at(head), 'one\ntwo\n');
    assert.equal(await at(state().rows.at(-1)!.hash), 'one\n');
    write('a.txt', 'one\ntwo\nstaged\n');
    await controller.action({ kind: 'stage', paths: ['a.txt'] });
    assert.equal(await at(INDEX), 'one\ntwo\nstaged\n');

    // The diffs of the changes view are VS Code's, served by its Git extension once it has the repository, so that its
    // own commands know them. A file with all of it staged shows HEAD against the index.
    const vscodeGit = vscode.extensions.getExtension<{ getAPI(version: 1): { repositories: { rootUri: vscode.Uri }[] } }>('vscode.git')!;
    await until("the repository in VS Code's Git", () =>
      vscodeGit.isActive && vscodeGit.exports.getAPI(1).repositories.some((repository) => samePath(repository.rootUri.fsPath, app)),
    );
    await fromWebview({ type: 'openDiff', source: { kind: 'change' }, file: { kind: 'modified', path: 'a.txt', originalPath: null } });
    const tab = await until('the diff editor', () => vscode.window.tabGroups.activeTabGroup.activeTab);
    assert.ok(tab.input instanceof vscode.TabInputTextDiff, 'a diff editor is open');
    assert.deepEqual([tab.input.original.scheme, JSON.parse(tab.input.original.query).ref], ['git', 'HEAD']);
    assert.deepEqual([tab.input.modified.scheme, JSON.parse(tab.input.modified.query).ref], ['git', '~']);
    assert.match(tab.label, /a\.txt \(Index\)/);
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');

    // With changes not staged, the index against the file: VS Code's Stage Selected Ranges stages the lines selected.
    write('a.txt', 'one\ntwo\nstaged\nnot staged\n');
    await until('the change read', () => state().workingTree.unstaged.some((file) => file.path === 'a.txt'));
    await fromWebview({ type: 'openDiff', source: { kind: 'change' }, file: { kind: 'modified', path: 'a.txt', originalPath: null } });
    const unstaged = await until('the diff editor', () => vscode.window.tabGroups.activeTabGroup.activeTab);
    assert.ok(unstaged.input instanceof vscode.TabInputTextDiff, 'a diff editor is open');
    assert.deepEqual([unstaged.input.original.scheme, JSON.parse(unstaged.input.original.query).ref], ['git', '~']);
    assert.equal((await vscode.workspace.openTextDocument(unstaged.input.original)).getText(), 'one\ntwo\nstaged\n');
    assert.equal(unstaged.input.modified.scheme, 'file');
    assert.match(unstaged.label, /a\.txt \(Working Tree\)/);
    const index = async () => (await git(app, ['show', ':a.txt'])).stdout;
    const editor = await until('the file in the diff editor', () =>
      vscode.window.activeTextEditor?.document.uri.fsPath === path.join(app, 'a.txt') ? vscode.window.activeTextEditor : undefined,
    );
    editor.selection = new vscode.Selection(3, 0, 3, 0);
    // The diff of the editor is worked out a moment after it opens: until then the command finds no change there.
    for (let tries = 0; ; tries++) {
      await vscode.commands.executeCommand('git.stageSelectedRanges');
      if ((await index()) === 'one\ntwo\nstaged\nnot staged\n') break;
      assert.ok(tries < 50, `the selected line staged: ${JSON.stringify(await index())}`);
      await sleep(200);
    }
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');

    // A click in the changes view opens the file, or its diff when the settings say so; the button of the row, the file.
    const active = () => until('an editor', () => vscode.window.tabGroups.activeTabGroup.activeTab);
    await fromWebview({ type: 'openChange', file: { kind: 'modified', path: 'a.txt', originalPath: null } });
    const opened = await active();
    assert.ok(opened.input instanceof vscode.TabInputText && opened.input.uri.fsPath === path.join(app, 'a.txt'), 'the file is open');
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
    await vscode.workspace.getConfiguration('gitardo').update('changes.openOnClick', 'diff', vscode.ConfigurationTarget.Global);
    try {
      await fromWebview({ type: 'openChange', file: { kind: 'modified', path: 'a.txt', originalPath: null } });
      assert.ok((await active()).input instanceof vscode.TabInputTextDiff, 'its diff is open');
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');
      await fromWebview({ type: 'openFile', file: { kind: 'modified', path: 'a.txt', originalPath: null } });
      assert.ok((await active()).input instanceof vscode.TabInputText, 'the button opens the file');
    } finally {
      await vscode.workspace.getConfiguration('gitardo').update('changes.openOnClick', undefined, vscode.ConfigurationTarget.Global);
    }
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
    await controller.action({ kind: 'unstageAll' });
    answer = run;
    await controller.action({ kind: 'discardAll' });
  });

  await step('switches a diff between the diff editor and the Flow Diff, which stages and unstages a change of it', async () => {
    interface Panel {
      descriptor: DiffDescriptor;
      currentModel: { chunks: { staged?: boolean }[] } | undefined;
      initSent: boolean;
      onMessage(message: { type: string; chunkId?: number }): void;
    }
    const index = async () => (await git(app, ['show', ':a.txt'])).stdout;
    const diffTabs = () => vscode.window.tabGroups.all.flatMap((group) => group.tabs).filter((tab) => tab.input instanceof vscode.TabInputTextDiff);
    write('a.txt', 'one\ntwo\nthree\n');
    try {
      await fromWebview({ type: 'openDiff', source: { kind: 'worktree' }, file: { kind: 'modified', path: 'a.txt', originalPath: null } });
      await until('the diff editor', () => diffTabs().length === 1);

      await vscode.commands.executeCommand('gitardo.openInFlowDiff');
      const panel = await until('the Flow Diff', () => flowDiff.panels()[0] as unknown as Panel | undefined);
      assert.equal(diffTabs().length, 0, 'the Flow Diff takes the place of the diff editor');
      // Gitardo names HEAD by its hash: it is HEAD again, which lets the index take part.
      assert.deepEqual(panel.descriptor.left, { kind: 'commit', ref: 'HEAD' });
      assert.deepEqual(panel.descriptor.right, { kind: 'worktree' });
      // The page says it is ready once its module ran, which proves the Monaco bundle loads under the content security policy.
      await until('the Flow Diff page to load', () => panel.initSent);

      errors.length = 0;
      panel.onMessage({ type: 'stageChunk', chunkId: 0 });
      // One operation runs at a time: another one asked meanwhile is refused, and that is said, as Gitardo Desktop says it.
      panel.onMessage({ type: 'unstageChunk', chunkId: 0 });
      await until('the refusal to be said', () => errors.length > 0);
      // The repository is named from the folder of the workspace around it, where its path is seen inside that folder.
      assert.equal(errors.length, 1, errors.join('\n'));
      assert.match(errors[0], /^Gitardo: diff\.stage-change is running in (workspace\/)?app\. Try again once it ends\.$/);
      await until('the change to show as staged', () => panel.currentModel?.chunks[0]?.staged === true);
      assert.equal(await index(), 'one\ntwo\nthree\n');
      // As Gitardo's own operations: in its log, exactly as it ran.
      assert.ok(logged.includes('> git apply --cached --unidiff-zero --whitespace=nowarn -'));
      // The panel follows the index as soon as git writes it, while the engine still reads the repository after the
      // operation; an unstage asked before it ends would be refused as above.
      await until('the operation to end', () => !(controller as unknown as { running: boolean }).running);

      panel.onMessage({ type: 'unstageChunk', chunkId: 0 });
      await until('the change to show as unstaged again', () => panel.currentModel?.chunks[0]?.staged === false);
      assert.equal(await index(), 'one\ntwo\n');

      await vscode.commands.executeCommand('gitardo.flowDiff.openInDiffEditor');
      const [tab] = await until('the diff editor again', () => (diffTabs().length ? diffTabs() : undefined));
      assert.equal(flowDiff.panels().length, 0);
      const input = tab.input as vscode.TabInputTextDiff;
      assert.equal(input.original.scheme, 'gitardo');
      assert.equal(input.modified.fsPath, path.join(app, 'a.txt'));

      // A diff of the Source Control view of VS Code: the index against the file.
      const fromGit = vscode.Uri.from({ scheme: 'git', path: input.modified.path, query: JSON.stringify({ path: input.modified.fsPath, ref: '~' }) });
      const described = await describeDiff(fromGit, input.modified);
      assert.deepEqual(described && { left: described.left, right: described.right, path: described.path }, {
        left: { kind: 'index' },
        right: { kind: 'worktree' },
        path: 'a.txt',
      });
    } finally {
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');
      answer = run;
      await controller.action({ kind: 'discardAll' });
    }
  });

  await step('stages, unstages and reverts the lines selected in the Flow Diff', async () => {
    interface Panel {
      currentModel: { rows: { right: { text?: string }; chunkId?: number }[]; chunks: unknown[] } | undefined;
      initSent: boolean;
      onMessage(message: { type: 'lines'; action: 'stage' | 'unstage' | 'revert'; rows: { start: number; end: number } }): void;
    }
    // Read at once, for `until` to wait on.
    const index = () => execFileSync('git', ['show', ':a.txt'], { cwd: app, encoding: 'utf8' });
    const rowOf = (panel: Panel, text: string) => panel.currentModel?.rows.findIndex((row) => row.right.text === text) ?? -1;
    const idle = () => until('the operation to end', () => !(controller as unknown as { running: boolean }).running);
    write('a.txt', 'one\ntwo\nA\nB\nC\nD\nE\n');
    try {
      await fromWebview({ type: 'openDiff', source: { kind: 'worktree' }, file: { kind: 'modified', path: 'a.txt', originalPath: null } });
      await vscode.commands.executeCommand('gitardo.openInFlowDiff');
      const panel = await until('the Flow Diff', () => flowDiff.panels()[0] as unknown as Panel | undefined);
      await until('the Flow Diff page to load', () => panel.initSent && rowOf(panel, 'B') >= 0);

      // The second and third of the five lines added, B and C, selected and staged: the index has those two alone.
      const rows = { start: rowOf(panel, 'B'), end: rowOf(panel, 'C') + 1 };
      panel.onMessage({ type: 'lines', action: 'stage', rows });
      await until('two lines staged', () => index() === 'one\ntwo\nB\nC\n');
      assert.ok(logged.includes('> git apply --cached --unidiff-zero --whitespace=nowarn -'));
      await idle();

      panel.onMessage({ type: 'lines', action: 'unstage', rows: { start: rowOf(panel, 'B'), end: rowOf(panel, 'C') + 1 } });
      await until('the two lines unstaged', () => index() === 'one\ntwo\n');
      await idle();

      // A line changed, reverted: the file has its old text again, saved.
      write('a.txt', 'one\nTWO\n');
      await until('the change of the line in the page', () => rowOf(panel, 'TWO') >= 0);
      panel.onMessage({ type: 'lines', action: 'revert', rows: { start: rowOf(panel, 'TWO'), end: rowOf(panel, 'TWO') + 1 } });
      await until('the line reverted', () => fs.readFileSync(path.join(app, 'a.txt'), 'utf8') === 'one\ntwo\n');
    } finally {
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');
      answer = run;
      await controller.action({ kind: 'discardAll' });
    }
  });

  await step('remembers how a conflict is resolved, and says a merge met twice was resolved as before', async () => {
    const refreshed = async () => vscode.commands.executeCommand('gitardo.refresh');
    const start = (await git(app, ['rev-parse', 'HEAD'])).stdout.trim();
    // Its own configuration says it, whatever the user's says.
    await git(app, ['config', '--local', 'rerere.enabled', 'false']);
    await refreshed();
    choose = byLabel(/leave the file for me to look at/);
    answer = run;
    try {
      await vscode.commands.executeCommand('gitardo.rememberResolutions');
      assert.equal((await git(app, ['config', '--local', '--get', 'rerere.enabled'])).stdout.trim(), 'true');
      await until('the state to say git remembers them', () => state().rerere?.isOn);

      await git(app, ['checkout', '-q', '-b', 'rerere-side']);
      write('r.txt', 'one\ntheirs\n');
      await git(app, ['add', 'r.txt']);
      await git(app, ['commit', '-q', '-m', 'Theirs']);
      await git(app, ['checkout', '-q', 'main']);
      write('r.txt', 'one\nours\n');
      await git(app, ['add', 'r.txt']);
      await git(app, ['commit', '-q', '-m', 'Ours']);
      await git(app, ['merge', 'rerere-side']).catch(() => undefined);
      write('r.txt', 'one\nboth\n');
      await git(app, ['add', 'r.txt']);
      await git(app, ['commit', '-q', '--no-edit']);
      await git(app, ['reset', '-q', '--hard', 'HEAD~1']);
      // Met again: git writes what was resolved, and leaves the file in conflict.
      await git(app, ['merge', 'rerere-side']).catch(() => undefined);
      await refreshed();
      await until('r.txt resolved as before', () => state().resolvedAsBefore?.some((file) => file.path === 'r.txt'));

      // Show what git wrote: the markers on the left, what git wrote on the right.
      await fromWebview({ type: 'openDiff', source: { kind: 'resolvedAsBefore' }, file: { kind: 'modified', path: 'r.txt', originalPath: null } });
      const tab = await until('the diff of what git wrote', () =>
        vscode.window.tabGroups.all.flatMap((group) => group.tabs).find((each) => each.input instanceof vscode.TabInputTextDiff),
      );
      const input = tab.input as vscode.TabInputTextDiff;
      assert.match(input.original.query, /:marked/);
      assert.match(Buffer.from(await vscode.workspace.fs.readFile(input.original)).toString('utf8'), /<<<<<<< /);
      assert.equal(fs.readFileSync(input.modified.fsPath, 'utf8'), 'one\nboth\n');

      // Forget this resolution…: it warns, then the file has its markers again.
      answer = (_message, _detail, items) => items.find((item) => item !== 'Cancel');
      await controller.action({ kind: 'forgetResolution', path: 'r.txt' });
      assert.match(fs.readFileSync(path.join(app, 'r.txt'), 'utf8'), /<<<<<<< /);
      answer = run;
      await controller.action({ kind: 'abort' });
    } finally {
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');
      choose = () => undefined;
      answer = run;
      await git(app, ['merge', '--abort']).catch(() => undefined);
      await git(app, ['reset', '-q', '--hard', start]).catch(() => undefined);
      await git(app, ['branch', '-D', 'rerere-side']).catch(() => undefined);
      await git(app, ['config', '--local', '--unset', 'rerere.enabled']).catch(() => undefined);
      // With rerere.enabled unset, a folder rr-cache still turns it on: the steps after would see conflicts resolved as before.
      fs.rmSync(path.join(app, '.git', 'rr-cache'), { recursive: true, force: true });
      await refreshed();
    }
  });

  await step('says where the history of a shallow clone ends, and loads more of it, then the whole of it', async () => {
    const refreshed = async () => vscode.commands.executeCommand('gitardo.refresh');
    // The commit origin/main points at, its parents taken as not fetched, as a clone made with --depth leaves them.
    const boundary = (await git(app, ['rev-parse', 'origin/main'])).stdout.trim();
    const shallowFile = path.join(app, '.git', 'shallow');
    answer = run;
    try {
      fs.writeFileSync(shallowFile, `${boundary}\n`);
      await refreshed();
      await until('the state to say where the history ends', () => state().shallow?.includes(boundary));
      const commands = (extension.packageJSON.contributes.commands as { command: string; title: string }[]).filter((entry) => /^gitardo\.load(More|Whole)History$/.test(entry.command));
      assert.deepEqual(commands.map((entry) => entry.title), ['Load More History', 'Load the Whole History']);

      // Load more history, as the last row of the graph and the Command Palette ask it, with 3 commits further back.
      logged.length = 0;
      inputs = ['3'];
      await fromWebview({ type: 'action', action: { kind: 'deepen' } });
      assert.ok(commandsLogged().includes('> git fetch --deepen=3 origin'), commandsLogged().join('\n'));
      // A second operation asked while the first runs would be refused (M42).
      await until('the deepening to end', () => !(controller as unknown as { running: boolean }).running);
      // The history of the suite's repository is short enough for three more commits to reach its start: it is made
      // shallow again before it is loaded whole.
      fs.writeFileSync(shallowFile, `${boundary}\n`);
      await refreshed();
      await until('the state to say where the history ends again', () => state().shallow?.includes(boundary));

      // Load the whole history: the repository is a clone like any other, and the graph has no row of its end.
      logged.length = 0;
      await vscode.commands.executeCommand('gitardo.loadWholeHistory');
      assert.ok(commandsLogged().includes('> git fetch --unshallow origin'), commandsLogged().join('\n'));
      // Git takes the file away once the fetch is done; the state says so after the read that follows.
      await until('git to take the shallow file away', () => !fs.existsSync(shallowFile));
      await until('the state to say the history is whole', () => !state().shallow?.length);
    } finally {
      inputs = [];
      fs.rmSync(shallowFile, { force: true });
      await refreshed();
    }
  });

  await step('copies the whole message of a commit, its body included', async () => {
    const start = (await git(app, ['rev-parse', 'HEAD'])).stdout.trim();
    write('body.txt', 'body\n');
    await git(app, ['add', 'body.txt']);
    await git(app, ['commit', '-q', '-m', 'Add a body', '-m', 'The first line of the body\nand the second.']);
    try {
      const hash = (await git(app, ['rev-parse', 'HEAD'])).stdout.trim();
      await vscode.env.clipboard.writeText('');
      await fromWebview({ type: 'action', action: { kind: 'copyMessage', hash } });
      assert.equal(await vscode.env.clipboard.readText(), 'Add a body\n\nThe first line of the body\nand the second.');
    } finally {
      await git(app, ['reset', '-q', '--hard', start]);
      await vscode.commands.executeCommand('gitardo.refresh');
    }
  });

  await step('gives a commit with eight labels all of them in their order, checks out the one chosen from their list, and turns compact rows off and on', async () => {
    const start = (await git(app, ['rev-parse', 'HEAD'])).stdout.trim();
    const branches = ['labels/one', 'labels/two', 'labels/three', 'labels/four'];
    const tags = ['labels-1', 'labels-2', 'labels-3', 'labels-4'];
    const settings = () => vscode.workspace.getConfiguration('gitardo');
    answer = run;
    try {
      for (const branch of branches) await git(app, ['branch', branch]);
      for (const tag of tags) await git(app, ['tag', tag]);
      await vscode.commands.executeCommand('gitardo.refresh');
      await until('the eight labels', () => tags.every((tag) => state().tags.some((each) => each.name === tag)) && branches.every((branch) => state().branches.some((each) => each.name === branch)));

      // The narrow Graph view of the side bar is sent every one of them, which its row fits and counts in `+N`, and its
      // list under the pointer names in their order: the branches, then the tags.
      const side = view('sideGraph');
      const sent: Extract<HostMessage, { type: 'state' }>[] = [];
      const postMessage = side.postMessage.bind(side);
      side.postMessage = (outgoing: HostMessage) => {
        if (outgoing.type === 'state') sent.push(outgoing);
        return postMessage(outgoing);
      };
      try {
        await internals.onMessage(side, { type: 'ready' });
        const told = (await until('the state of the side bar', () => sent.find((outgoing) => outgoing.state?.branches))).state as RepoState;
        assert.equal(told.compactRows, true, 'compact rows are on by default');
        const labels = buildLabels({ ...state(), ...told }).get(start) ?? [];
        const ours = labels.filter((label) => label.name.startsWith('labels'));
        assert.deepEqual(
          ours.map((label) => `${label.kind} ${label.name}`),
          [...branches.map((branch) => `branch ${branch}`).sort(), ...tags.map((tag) => `tag ${tag}`)],
        );
        assert.ok(labels.length >= 9, `main and the eight: ${labels.map((label) => label.name).join(', ')}`);
      } finally {
        side.postMessage = postMessage;
      }

      // The menu of a label of the list, and its double click, check that branch out.
      await fromWebview({ type: 'action', action: { kind: 'checkout', target: 'branch', ref: 'labels/four' } });
      await until('labels/four checked out', () => checkedOutBranch(state()) === 'labels/four');

      // Compact rows off writes the user's settings, and every view of the graph is told; on again, as by default.
      await fromWebview({ type: 'setCompactRows', compact: false });
      await until('compact rows off', () => settings().inspect('graph.compactRows')?.globalValue === false && state().compactRows === false);
      await fromWebview({ type: 'setCompactRows', compact: true });
      await until('compact rows on', () => state().compactRows === true);
    } finally {
      await settings().update('graph.compactRows', undefined, vscode.ConfigurationTarget.Global);
      await git(app, ['checkout', '-q', 'main']);
      await git(app, ['branch', '-D', ...branches]);
      await git(app, ['tag', '-d', ...tags]);
      await vscode.commands.executeCommand('gitardo.refresh');
    }
  });

  await step('answers the webviews: commit details and comparisons', async () => {
    const [newest, older] = state().rows;
    posted.length = 0;
    await fromWebview({ type: 'getDetail', hash: newest.hash });
    const detail = posted.find((outgoing) => outgoing.type === 'detail');
    assert.ok(detail?.type === 'detail' && detail.detail);
    assert.deepEqual(detail.detail.files, [{ kind: 'added', path: 'outside.txt', originalPath: null }]);

    await fromWebview({ type: 'getComparison', from: state().rows.at(-1)!.hash, to: older.hash });
    const comparison = posted.find((outgoing) => outgoing.type === 'comparison');
    assert.ok(comparison?.type === 'comparison');
    assert.ok(comparison.comparison.files.some((file) => file.path === 'b.txt' && file.kind === 'added'));
  });

  await step('gives the commits of the graph to the views that show it, and spares the others', async () => {
    const told = async (kind: ViewKind) => {
      const webview = view(kind);
      const sent: Extract<HostMessage, { type: 'state' }>[] = [];
      const postMessage = webview.postMessage.bind(webview);
      webview.postMessage = (outgoing: HostMessage) => {
        if (outgoing.type === 'state') sent.push(outgoing);
        return postMessage(outgoing);
      };
      try {
        // What a view says when it has just loaded, and has nothing: it is sent every part it shows.
        await internals.onMessage(webview, { type: 'ready' });
        return (await until(`the state for the ${kind} view`, () => sent.find((outgoing) => outgoing.state))).state!;
      } finally {
        webview.postMessage = postMessage;
      }
    };
    const commits = state().rows.length;
    assert.ok(commits >= 5);
    const graph = await told('sideGraph');
    assert.equal((graph.rows?.length ?? 0) + (graph.draft ? 1 : 0), commits);
    // The view of the commits asks for its own, which are not the ones of the graph.
    for (const kind of ['commits', 'branches', 'stashes', 'changes'] as const) {
      const seen = await told(kind);
      assert.equal(seen.rows, undefined, `no commits for the ${kind} view`);
      assert.equal(seen.draft, undefined, `no row of the uncommitted changes for the ${kind} view`);
      if (kind === 'stashes') assert.equal(seen.branches, undefined, 'no branches for the stashes view');
      else
        assert.deepEqual(
          seen.branches?.map((branch) => branch.name),
          ['feature', 'main', 'merged', 'topic'],
        );
    }
  });

  await step('sends the Branches view nothing when a file is saved', async () => {
    const branches = view('branches');
    const sent: HostMessage[] = [];
    const postMessage = branches.postMessage.bind(branches);
    branches.postMessage = (outgoing: HostMessage) => {
      sent.push(outgoing);
      return postMessage(outgoing);
    };
    const file = path.join(app, 'saved.txt');
    try {
      // A file changed already, changed again and saved: what git says of the working tree is the same.
      write('saved.txt', 'one\n');
      await until('the new file in the state', () => internals.state?.workingTree.untracked.includes('saved.txt'));
      await until('no read under way', () => !internals.loading);
      const document = await vscode.workspace.openTextDocument(file);
      const editor = await vscode.window.showTextDocument(document);
      await editor.edit((edit) => edit.insert(new vscode.Position(0, 0), 'two\n'));
      const before = internals.state;
      const from = sent.length;
      await document.save();
      // The file is not one git ignores: it is read again, and the state is a new one.
      await until('the read after the save', () => internals.state !== before && !internals.loading);
      await sleep(500);
      assert.deepEqual(
        sent.slice(from).filter((outgoing) => outgoing.type === 'state'),
        [],
      );
    } finally {
      branches.postMessage = postMessage;
      await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
      fs.rmSync(file, { force: true });
      await until('the file gone from the state', () => !internals.state?.workingTree.untracked.includes('saved.txt'));
    }
  });

  await step('reads the repository once for five views opened at once, and not at all when nothing changed', async () => {
    // The session of the repository the panel shows reads it, for every view on it.
    const session = internals.followers.session;
    assert.ok(session, 'the panel shows a repository');
    const sessionReads = session as unknown as { readState: (...args: unknown[]) => Promise<unknown> };
    const readState = sessionReads.readState;
    let count = 0;
    sessionReads.readState = (...args: unknown[]) => {
      count++;
      return readState.apply(session, args);
    };
    const five = (['sideGraph', 'commits', 'branches', 'stashes', 'changes'] as const).map(view);
    try {
      // The changes of the steps before, which the watcher of macOS tells of late and git may still be writing after a
      // fetch, are read first: the repository is quiet once its state saw every change heard of for a second and a half.
      const fresh = session as unknown as { isFresh: boolean };
      let quietSince = Date.now();
      await until('the changes of the steps before to be read', () => {
        if (internals.loading || !fresh.isFresh) quietSince = Date.now();
        return Date.now() - quietSince > 1_500;
      });
      count = 0;
      // The state held was read after the last change heard of: each view is sent it, and nothing is read.
      await Promise.all(five.map((webview) => internals.onMessage(webview, { type: 'ready' })));
      assert.equal(count, 0, 'no read for views opened while nothing changed');
      // A change heard of, whose read has not started yet: the five views wait for one read.
      internals.scheduleRefresh('a change of this test', 60_000);
      await Promise.all(five.map((webview) => internals.onMessage(webview, { type: 'ready' })));
      assert.equal(count, 1, 'one read for five views');
    } finally {
      delete (sessionReads as { readState?: unknown }).readState;
    }
  });

  // The comparisons open in tabs.
  const tabs = () => [...internals.compareTabs.tabs.values()];
  /** The sides of the comparison opened last, and how it compares them. */
  const compared = () => {
    const tab = tabs().at(-1);
    return tab ? { from: tab.from, to: tab.to, mode: tab.mode } : null;
  };
  /** What the page of a comparison was told last, of what it shows or of its lines. */
  const told = <T extends 'compareView' | 'comparePatch' = 'compareView'>(tab: CompareTab, type = 'compareView' as T) => {
    for (let at = posted.length - 1; at >= 0; at--) {
      const outgoing = posted[at];
      if (outgoing.type === type && addressed[at] === tab.panel.webview) return outgoing as Extract<HostMessage, { type: T }>;
    }
  };
  const closeAll = async () => {
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
    await until('the tabs to close', () => tabs().length === 0);
  };
  const forget = () => {
    posted.length = 0;
    addressed.length = 0;
  };

  await step('compares branches, tags and typed revisions in a tab of their own, which follows the branches as they move', async () => {
    const head = state().head.commit!;
    const hashOf = (name: string) => state().branches.find((branch) => branch.name === name)!.commit;
    const tag = state().tags[0];
    const revParse = async (revision: string) => (await git(app, ['rev-parse', revision])).stdout.trim();

    // The command: both sides are asked, and a tab shows how they compare.
    const titles: string[] = [];
    let offered: string[] = [];
    choose = (items, options) => {
      titles.push(options.title ?? '');
      offered = items.map((item) => item.label);
      return byLabel(titles.length === 1 ? '$(git-branch) feature' : '$(target) main')(items);
    };
    forget();
    await vscode.commands.executeCommand('gitardo.compare');
    assert.deepEqual(titles, ['Compare', 'Compare feature with']);
    // Everything but the side that is already chosen.
    assert.deepEqual(offered, [
      '$(edit) Enter a hash or a revision…',
      '$(target) main',
      '$(files) Working tree',
      'Branches',
      '$(git-branch) merged',
      '$(git-branch) topic',
      'Remote branches',
      '$(cloud) origin/main',
      'Tags',
      '$(tag) v1',
    ]);
    const tab = await until('the tab of the comparison', () => tabs()[0]);
    assert.equal(tab.panel.title, 'feature ↔ main');
    // The page is told the session of VS Code, for what it remembers only until VS Code is closed.
    assert.match(tab.panel.webview.html, new RegExp(`data-view="compare" data-session="${vscode.env.sessionId}" data-root="[^"]+" data-files="tree"`));
    // Two branches are followed, and compared from where they diverged.
    assert.deepEqual(compared(), {
      from: { name: 'feature', hash: hashOf('feature'), live: true },
      to: { name: 'main', hash: head, live: true },
      mode: 'diverged',
    });
    // The page asks for what it shows: how far apart the two are and the files that differ, then their lines.
    const base = (await git(app, ['merge-base', 'feature', 'main'])).stdout.trim();
    const [behind, ahead] = (await git(app, ['rev-list', '--left-right', '--count', 'feature...main'])).stdout.trim().split(/\s+/).map(Number);
    const shown = await until('the comparison', () => told(tab));
    assert.ok(shown.view);
    assert.deepEqual([shown.view.base, shown.view.since, shown.view.ahead, shown.view.behind], [base, base, ahead, behind]);
    // And the commits each has that the other has not, matched with the other side's.
    const onlyIn = (side: 'from' | 'to') => shown.view!.commits.filter((commit) => commit.side === side).map((commit) => commit.hash);
    const only = async (range: string) => (await git(app, ['rev-list', range])).stdout.trim().split('\n').filter(Boolean);
    assert.deepEqual(onlyIn('to').sort(), (await only('feature..main')).sort());
    assert.deepEqual(onlyIn('from').sort(), (await only('main..feature')).sort());
    assert.ok(shown.view.matched);
    const changed = (await git(app, ['diff', '--name-only', base, head])).stdout.trim().split('\n');
    assert.deepEqual(shown.view.files.map((file) => file.path), changed);
    const lines = await until('the lines', () => posted.find((outgoing, at) => outgoing.type === 'comparePatch' && addressed[at] === tab.panel.webview));
    assert.ok(lines.type === 'comparePatch' && lines.patch?.length === changed.length);

    // Everything that differs, and the other way round.
    forget();
    await internals.onMessage(tab.panel.webview, { type: 'setCompareMode', mode: 'direct' });
    assert.equal(told(tab)?.view?.since, hashOf('feature'));
    await internals.onMessage(tab.panel.webview, { type: 'swapCompare' });
    assert.deepEqual([told(tab)?.view?.from.name, told(tab)?.view?.to.name, told(tab)?.view?.since], ['main', 'feature', head]);
    assert.equal(tab.panel.title, 'main ↔ feature');
    // While the two are compared again the tab is told to wait, and that the wait is over before the answer.
    forget();
    await internals.onMessage(tab.panel.webview, { type: 'swapCompare' });
    const waited = posted.flatMap((outgoing, at) => {
      if (addressed[at] !== tab.panel.webview) return [];
      if (outgoing.type === 'loading') return [`loading ${outgoing.what}`];
      return outgoing.type === 'compareView' ? ['compareView'] : [];
    });
    assert.deepEqual(waited, ['loading filter', 'loading null', 'compareView']);

    // Asked for again, from the button of a comparison in a view, the two have their tab: that one is shown.
    await internals.onMessage(view('branches'), { type: 'openCompare', from: { name: 'feature', hash: hashOf('feature') }, to: { name: 'main', hash: head } });
    assert.equal(tabs().length, 1);

    // One side chosen again in the page.
    choose = (items, options) => {
      titles.push(options.title ?? '');
      return byLabel('$(git-branch) topic')(items);
    };
    await internals.onMessage(tab.panel.webview, { type: 'pickCompareSide', side: 'to' });
    assert.equal(titles.at(-1), 'Compare feature with');
    assert.deepEqual(compared()?.to, { name: 'topic', hash: hashOf('topic'), live: true });
    assert.equal(told(tab)?.view?.to.name, 'topic');
    assert.equal(tab.panel.title, 'feature ↔ topic');

    // A branch that moves is followed; one that is deleted stays where it was.
    const before = await revParse('HEAD~1');
    await git(app, ['branch', 'compared', before]);
    // Named by a branch that git gives another commit, a side is the commit it was chosen at.
    await internals.onMessage(view('branches'), { type: 'openCompare', from: { name: 'feature', hash: hashOf('feature') }, to: { name: 'compared', hash: head } });
    const fixed = await until('the tab of the comparison', () => tabs()[1]);
    assert.deepEqual([fixed.to.live, fixed.mode], [false, 'direct']);
    await internals.onMessage(view('branches'), { type: 'openCompare', from: { name: 'feature', hash: hashOf('feature') }, to: { name: 'compared', hash: before } });
    const followed = await until('the other tab', () => tabs()[2]);
    assert.deepEqual([followed.to.live, followed.mode], [true, 'diverged']);
    await until('its comparison', () => told(followed));
    await git(app, ['branch', '--force', 'compared', head]);
    await controller.refresh();
    await until('the comparison of where the branch is now', () => told(followed)?.view?.to.hash === head);
    await git(app, ['branch', '--delete', '--force', 'compared']);
    await controller.refresh();
    await until('the branch to be gone', () => told(followed)?.view?.to.gone);
    assert.equal(followed.to.hash, head);
    // A tab closed while git answers for it is left alone.
    errors.length = 0;
    const answering = internals.onMessage(followed.panel.webview, { type: 'setCompareMode', mode: 'direct' });
    followed.panel.dispose();
    await answering;
    assert.deepEqual(errors, []);
    await closeAll();

    // Asked by the branches, with something typed: a revision is followed as well.
    choose = (items, options) => byLabel(options.title === 'Compare' ? /Enter a hash/ : '$(git-branch) topic')(items);
    inputs = ['HEAD~1'];
    await internals.onMessage(view('branches'), { type: 'compare' });
    const parent = (await git(app, ['rev-parse', 'HEAD~1'])).stdout.trim();
    await until('the tab', () => tabs().length === 1);
    assert.deepEqual(compared(), {
      from: { name: 'HEAD~1', hash: parent, live: true },
      to: { name: 'topic', hash: hashOf('topic'), live: true },
      mode: 'diverged',
    });

    // "Compare with…" in a menu gives one side; a hash that is typed is named by its short form, and stays that commit.
    choose = byLabel(/Enter a hash/);
    inputs = [parent];
    await internals.onMessage(view('sideGraph'), { type: 'compare', from: { name: tag.name, hash: tag.commit } });
    await until('the tab', () => tabs().length === 2);
    assert.deepEqual(compared(), {
      from: { name: 'v1', hash: tag.commit, live: true },
      to: { name: parent.slice(0, 7), hash: parent, live: false },
      mode: 'direct',
    });
    await closeAll();

    // What git does not know is said, and backing out does nothing.
    choose = byLabel(/Enter a hash/);
    inputs = ['no-such-thing'];
    asked.length = 0;
    await vscode.commands.executeCommand('gitardo.compare');
    assert.deepEqual(
      asked.map((question) => question.message),
      ['Gitardo: "no-such-thing" is not a commit of this repository.'],
    );
    choose = () => undefined;
    await vscode.commands.executeCommand('gitardo.compare');
    assert.equal(tabs().length, 0);

    // A branch clicked in the side bar is shown where the commits are, and nowhere else.
    forget();
    await internals.onMessage(view('branches'), { type: 'show', hash: hashOf('feature') });
    assert.deepEqual(posted, [
      { type: 'reveal', hash: hashOf('feature') },
      { type: 'reveal', hash: hashOf('feature') },
    ]);
    assert.deepEqual(addressed.map((webview) => internals.views.get(webview)).sort(), ['graph', 'sideGraph']);

    // A branch the pointer rests on in the side bar has its history brought out where the commits are, until it leaves.
    forget();
    await internals.onMessage(view('branches'), { type: 'trace', hash: hashOf('feature') });
    await internals.onMessage(view('branches'), { type: 'trace', hash: null });
    await internals.onMessage(view('branches'), { type: 'trace', hash: 'not a hash' });
    assert.deepEqual(posted, [
      { type: 'trace', hash: hashOf('feature') },
      { type: 'trace', hash: hashOf('feature') },
      { type: 'trace', hash: null },
      { type: 'trace', hash: null },
    ]);
    assert.deepEqual(addressed.map((webview) => internals.views.get(webview)).sort(), ['graph', 'graph', 'sideGraph', 'sideGraph']);
  });

  await step('compares a file with the working tree from the Explorer, takes it from the other side, copies a patch, opens GitHub and the comparisons opened before', async () => {
    const hashOf = (name: string) => state().branches.find((branch) => branch.name === name)!.commit;
    const show = async (revision: string) => (await git(app, ['show', revision])).stdout;
    const original = fs.readFileSync(path.join(app, 'a.txt'), 'utf8');

    // From the Explorer: the file as it is now, with what is not committed, compared with a branch.
    write('a.txt', `${original}local\n`);
    await controller.refresh();
    const titles: string[] = [];
    let offered: string[] = [];
    choose = (items, options) => {
      titles.push(options.title ?? '');
      offered = items.map((item) => item.label);
      return byLabel('$(git-branch) feature')(items);
    };
    forget();
    await vscode.commands.executeCommand('gitardo.compareWith', vscode.Uri.file(path.join(app, 'a.txt')));
    assert.deepEqual(titles, ['Compare a.txt with']);
    assert.ok(!offered.includes('$(files) Working tree'));
    const tab = await until('the tab of the comparison', () => tabs()[0]);
    assert.equal(tab.panel.title, 'feature ↔ Working tree · a.txt');
    // How the files differ from it, not what the branch did since the two diverged.
    assert.deepEqual([tab.from, tab.to, tab.mode, tab.path], [{ name: 'feature', hash: hashOf('feature'), live: true }, { name: 'Working tree', hash: WORKTREE, live: true }, 'direct', 'a.txt']);
    const shown = await until('the comparison', () => told(tab));
    assert.deepEqual([shown.view?.path, shown.view?.conflicts, shown.view?.files.map((file) => file.path)], ['a.txt', null, ['a.txt']]);
    const added = (patch?: Extract<HostMessage, { type: 'comparePatch' }>) => patch?.patch?.[0]?.hunks?.flatMap((hunk) => hunk.lines).filter((line) => line.startsWith('+'));
    assert.ok(added(await until('the lines', () => told(tab, 'comparePatch')))?.includes('+local'));
    // Followed as the files change.
    write('a.txt', `${original}local\nmore\n`);
    await controller.refresh();
    await until('the lines of the file as it is now', () => added(told(tab, 'comparePatch'))?.includes('+more'));

    // Narrowed in the page: to nothing, which is all of it, and to a file however it is written.
    await internals.onMessage(tab.panel.webview, { type: 'setComparePath', path: ' ./ ' });
    assert.deepEqual([tab.path, told(tab)?.view?.path, tab.panel.title], [null, null, 'feature ↔ Working tree']);
    await internals.onMessage(tab.panel.webview, { type: 'setComparePath', path: '.\\a.txt' });
    assert.deepEqual([tab.path, told(tab)?.view?.files.map((file) => file.path)], ['a.txt', ['a.txt']]);

    // The other way round, the file taken from the branch: what is not committed in it is lost, so it is asked first.
    await internals.onMessage(tab.panel.webview, { type: 'swapCompare' });
    asked.length = 0;
    answer = run;
    await internals.onMessage(tab.panel.webview, { type: 'takeCompareFile', file: { kind: 'modified', path: 'a.txt', originalPath: null } });
    answer = () => undefined;
    assert.deepEqual(
      asked.map((question) => question.message),
      ['Take a.txt from feature?'],
    );
    assert.match(asked[0].detail, /changes that are not committed are lost/);
    assert.equal(fs.readFileSync(path.join(app, 'a.txt'), 'utf8'), await show('feature:a.txt'));
    await until('the file to be the same', () => told(tab)?.view?.files.length === 0);
    await git(app, ['checkout', 'HEAD', '--', 'a.txt']);
    await controller.refresh();
    // The working tree has no version to give.
    await internals.onMessage(tab.panel.webview, { type: 'swapCompare' });
    asked.length = 0;
    await internals.onMessage(tab.panel.webview, { type: 'takeCompareFile', file: { kind: 'modified', path: 'a.txt', originalPath: null } });
    assert.equal(asked.length, 0);
    await closeAll();

    // Two branches: the changes copied as a patch, and the comparison opened on GitHub, each side by its name there.
    forget();
    await internals.onMessage(view('branches'), { type: 'openCompare', from: { name: 'origin/main', hash: (await git(app, ['rev-parse', 'origin/main'])).stdout.trim() }, to: { name: 'topic', hash: hashOf('topic') } });
    const branches = await until('the tab of the comparison', () => tabs()[0]);
    await until('the comparison', () => told(branches));
    asked.length = 0;
    await internals.onMessage(branches.panel.webview, { type: 'copyComparePatch' });
    const text = await vscode.env.clipboard.readText();
    const files = text.match(/^diff --git /gm)?.length ?? 0;
    assert.equal(files, told(branches)?.view?.files.length);
    assert.deepEqual(
      asked.map((question) => question.message),
      [`Gitardo: copied the changes of ${files} ${files === 1 ? 'file' : 'files'} as a patch.`],
    );
    const opened: string[] = [];
    const env = vscode.env as unknown as Record<string, unknown>;
    const openExternal = env.openExternal;
    env.openExternal = (uri: vscode.Uri) => {
      opened.push(uri.toString(true));
      return Promise.resolve(true);
    };
    const hosting = internals.hosting;
    internals.hosting = { service: 'gitHub', host: 'github.com', remote: 'origin', canCheckOut: true, comparesSinceDiverged: false };
    // Area 6 of stage 4: the engine reads the remotes itself, so origin is on GitHub while it opens the comparison.
    const originUrl = (await git(app, ['remote', 'get-url', 'origin'])).stdout.trim();
    if (internals.engine?.running) await git(app, ['remote', 'set-url', 'origin', 'https://github.com/octo/app.git']);
    try {
      await internals.onMessage(branches.panel.webview, { type: 'openCompareOnService' });
      // A branch with no upstream there is not on GitHub by its name: its commit is, once pushed.
      await internals.onMessage(branches.panel.webview, { type: 'swapCompare' });
      await internals.onMessage(branches.panel.webview, { type: 'setCompareMode', mode: 'direct' });
      await internals.onMessage(branches.panel.webview, { type: 'openCompareOnService' });
    } finally {
      env.openExternal = openExternal;
      internals.hosting = hosting;
      await git(app, ['remote', 'set-url', 'origin', originUrl]);
    }
    assert.deepEqual(opened, [`https://github.com/octo/app/compare/main...${hashOf('topic')}`, `https://github.com/octo/app/compare/${hashOf('topic')}..main`]);
    await closeAll();

    // The comparisons opened last come first when one is asked for, the two ways of one as one, and open as they were.
    let first: string[] = [];
    choose = (items, options) => {
      if (options.title === 'Compare') first = items.map((item) => `${item.label}${item.description ? ` · ${item.description}` : ''}`);
      return byLabel('$(history) feature ↔ Working tree')(items);
    };
    await vscode.commands.executeCommand('gitardo.compare');
    assert.deepEqual(first.slice(0, 4), ['Opened before', '$(history) topic ↔ origin/main', '$(history) feature ↔ Working tree · a.txt', '$(history) feature ↔ Working tree']);
    const again = await until('the tab of the comparison', () => tabs()[0]);
    assert.deepEqual([again.from.name, again.to.hash, again.path], ['feature', WORKTREE, 'a.txt']);
    await closeAll();
  });

  await step('compares a file of a commit, or of the changes, with a branch, tag, commit or the working tree', async () => {
    // The diffs that would open are only noted.
    const commands = vscode.commands as unknown as Record<string, (...args: unknown[]) => Thenable<unknown>>;
    const executeCommand = commands.executeCommand;
    const diffs: string[][] = [];
    const side = (uri: vscode.Uri) => (uri.scheme === 'file' ? path.basename(uri.fsPath) : (JSON.parse(uri.query) as { ref: string }).ref);
    commands.executeCommand = (id: unknown, ...args: unknown[]) => {
      if (id !== 'vscode.diff') return executeCommand(id, ...args);
      diffs.push([side(args[0] as vscode.Uri), side(args[1] as vscode.Uri), args[2] as string]);
      return Promise.resolve();
    };
    const titles: string[] = [];
    let offered: string[] = [];
    const picking = (label: string) => (items: Item[], options: vscode.QuickPickOptions) => {
      titles.push(options.title ?? '');
      offered = items.map((item) => item.label);
      return byLabel(label)(items);
    };
    try {
      const change = (await git(app, ['log', '--all', '--format=%H', '--grep=^Change a$'])).stdout.trim();
      const parent = (await git(app, ['rev-parse', `${change}^`])).stdout.trim();
      const feature = (await git(app, ['rev-parse', 'feature'])).stdout.trim();
      const short = change.slice(0, 7);
      const source = { kind: 'commit', hash: change, parent } as const;

      // The branch on the left, the file of the commit on the right.
      choose = picking('$(git-branch) feature');
      await fromWebview({ type: 'compareFile', source, file: { kind: 'modified', path: 'a.txt', originalPath: null } });
      assert.deepEqual(titles, [`Compare a.txt of ${short} with`]);
      assert.deepEqual(diffs, [[feature, change, `a.txt (feature ↔ ${short})`]]);

      // The working tree stays on the right, where the file can be edited.
      diffs.length = 0;
      choose = picking('$(files) Working tree');
      await fromWebview({ type: 'compareFile', source, file: { kind: 'modified', path: 'a.txt', originalPath: null } });
      assert.deepEqual(diffs, [[change, 'a.txt', `a.txt (${short} ↔ Working tree)`]]);

      // A file the commit deleted, as it was before.
      diffs.length = 0;
      choose = picking('$(git-branch) feature');
      await fromWebview({ type: 'compareFile', source, file: { kind: 'deleted', path: 'a.txt', originalPath: null } });
      assert.deepEqual(diffs, [[feature, parent, `a.txt (feature ↔ ${parent.slice(0, 7)})`]]);

      // A file of a comparison, as its second side has it, called by the name of the side; deleted, by the first.
      diffs.length = 0;
      titles.length = 0;
      const names = { hash: 'to', parent: 'from' };
      await fromWebview({ type: 'compareFile', source: { kind: 'commit', hash: change, parent }, file: { kind: 'modified', path: 'a.txt', originalPath: null }, names });
      await fromWebview({ type: 'compareFile', source: { kind: 'commit', hash: change, parent }, file: { kind: 'deleted', path: 'a.txt', originalPath: null }, names });
      assert.deepEqual(titles, ['Compare a.txt of to with', 'Compare a.txt of from with']);
      assert.deepEqual(diffs, [
        [feature, change, 'a.txt (feature ↔ to)'],
        [feature, parent, 'a.txt (feature ↔ from)'],
      ]);

      // A file of the changes not committed: the working tree is not offered as the other side.
      diffs.length = 0;
      await fromWebview({ type: 'compareFile', source: { kind: 'worktree' }, file: { kind: 'modified', path: 'a.txt', originalPath: null } });
      assert.ok(!offered.includes('$(files) Working tree'));
      assert.deepEqual(diffs, [[feature, 'a.txt', 'a.txt (feature ↔ Working tree)']]);
    } finally {
      commands.executeCommand = executeCommand;
      choose = () => undefined;
    }
  });

  await step('narrows the graph to the history of a file, switching repository if needed', async () => {
    await vscode.commands.executeCommand('gitardo.fileHistory', vscode.Uri.file(path.join(app, 'b.txt')));
    assert.deepEqual(state().filter, { kind: 'path', path: 'b.txt' });
    // On every branch: `feature` changes the file too.
    assert.deepEqual(subjects(), ['Add feature', 'Add b']);
    await controller.history.setFilter(null);
    assert.equal(state().filter, null);

    await vscode.commands.executeCommand('gitardo.fileHistory', vscode.Uri.file(path.join(lib, 'lib.txt')));
    assert.equal(state().name, 'lib');
    assert.deepEqual(subjects(), ['Add lib']);
    await controller.repositories.selectRepo(state().repos[0].root);
    assert.equal(state().name, 'app');
    assert.equal(state().filter, null);
  });

  await step('narrows the graph to the history of a folder, and shows all of it for the root folder of a repository', async () => {
    const before = (await git(app, ['rev-parse', 'HEAD'])).stdout.trim();
    try {
      fs.mkdirSync(path.join(app, 'docs', 'deep'), { recursive: true });
      write('docs/deep/guide.txt', 'guide\n');
      await git(app, ['add', 'docs']);
      await git(app, ['commit', '-m', 'Add docs']);
      await vscode.commands.executeCommand('gitardo.folderHistory', vscode.Uri.file(path.join(app, 'docs')));
      assert.deepEqual(state().filter, { kind: 'path', path: 'docs', folder: true });
      assert.deepEqual(subjects(), ['Add docs']);

      await vscode.commands.executeCommand('gitardo.folderHistory', vscode.Uri.file(lib));
      assert.equal(state().name, 'lib');
      assert.equal(state().filter, null);
      await vscode.commands.executeCommand('gitardo.folderHistory', vscode.Uri.file(app));
      assert.equal(state().name, 'app');
      assert.equal(state().filter, null);

      // Outside of every repository there is no history to show.
      asked.length = 0;
      await vscode.commands.executeCommand('gitardo.folderHistory', vscode.Uri.file(workspace));
      assert.ok(asked.some((each) => each.message.includes('choose a folder of this repository')), asked.map((each) => each.message).join('\n'));
      assert.equal(state().filter, null);
    } finally {
      await controller.history.setFilter(null);
      await git(app, ['reset', '--hard', before]);
      await vscode.commands.executeCommand('gitardo.refresh');
    }
  });

  await step('narrows the graph to the history of the selected lines, as the commit checked out has them', async () => {
    const before = (await git(app, ['rev-parse', 'HEAD'])).stdout.trim();
    try {
      write('lines.txt', 'one\ntwo\n');
      await git(app, ['add', 'lines.txt']);
      await git(app, ['commit', '-m', 'Add lines']);
      write('lines.txt', 'one\nTWO\n');
      await git(app, ['commit', '-am', 'Change two']);
      await vscode.commands.executeCommand('gitardo.refresh');
      const head = state().head.commit!;
      const editor = await vscode.window.showTextDocument(vscode.Uri.file(path.join(app, 'lines.txt')));

      // The line of the cursor.
      editor.selection = new vscode.Selection(1, 2, 1, 2);
      await vscode.commands.executeCommand('gitardo.lineHistory');
      assert.deepEqual(state().filter, { kind: 'lines', path: 'lines.txt', start: 2, end: 2, commit: head });
      assert.deepEqual(subjects(), ['Change two', 'Add lines']);
      // A selection of whole lines ends at the start of the next one, which it leaves out.
      editor.selection = new vscode.Selection(0, 0, 1, 0);
      await vscode.commands.executeCommand('gitardo.lineHistory');
      assert.deepEqual(state().filter, { kind: 'lines', path: 'lines.txt', start: 1, end: 1, commit: head });
      assert.deepEqual(subjects(), ['Add lines']);

      // With a line inserted above and not saved, the line below is the second one of the commit.
      await editor.edit((edit) => edit.insert(new vscode.Position(0, 0), 'zero\n'));
      editor.selection = new vscode.Selection(2, 0, 2, 0);
      await vscode.commands.executeCommand('gitardo.lineHistory');
      assert.deepEqual(state().filter, { kind: 'lines', path: 'lines.txt', start: 2, end: 2, commit: head, editor: { start: 3, end: 3 } });
      assert.deepEqual(subjects(), ['Change two', 'Add lines']);

      // The line inserted has no history: that is said, and the graph is left as it was.
      asked.length = 0;
      editor.selection = new vscode.Selection(0, 0, 0, 0);
      await vscode.commands.executeCommand('gitardo.lineHistory');
      assert.ok(asked.some((each) => each.message.includes('added after the last commit')), asked.map((each) => each.message).join('\n'));
      assert.equal(state().filter?.kind, 'lines');
      await vscode.commands.executeCommand('workbench.action.files.revert');
    } finally {
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');
      await controller.history.setFilter(null);
      await git(app, ['reset', '--hard', before]);
      await vscode.commands.executeCommand('gitardo.refresh');
    }
  });

  await step('narrows the graph to the branches chosen, remembers them for each repository, and shows them all for a commit elsewhere', async () => {
    const graph = view('graph');
    const everything = subjects();
    assert.equal(state().scope, null);
    assert.ok(everything.includes('Add feature'));

    // As a view asks: only what is checked out.
    await internals.onMessage(graph, { type: 'setScope', scope: { kind: 'current' } });
    assert.deepEqual(state().scope, { kind: 'current' });
    assert.ok(!subjects().includes('Add feature') && subjects().includes('Add b'), `current: ${subjects()}`);

    // Or the branches chosen one by one, of which only the ones that are there are taken.
    await internals.onMessage(graph, { type: 'setScope', scope: { kind: 'refs', refs: ['refs/heads/feature', 'refs/heads/gone', '--all'] } });
    assert.deepEqual(state().scope, { kind: 'refs', refs: ['refs/heads/feature'] });
    assert.equal(subjects()[0], 'Add feature');

    // The history of a file is the one it has among them, and leaves them as they were when it is closed.
    await controller.history.setFilter({ kind: 'path', path: 'b.txt' });
    assert.deepEqual(subjects(), ['Add feature', 'Add b']);
    await internals.onMessage(graph, { type: 'setScope', scope: { kind: 'current' } });
    assert.deepEqual(subjects(), ['Add b']);
    assert.deepEqual(state().filter, { kind: 'path', path: 'b.txt' });
    await controller.history.setFilter(null);
    assert.deepEqual(state().scope, { kind: 'current' });

    // Each repository has its own, and finds it again.
    await controller.repositories.selectRepo(state().repos[1].root);
    assert.equal(state().name, 'lib');
    assert.equal(state().scope, null);
    await controller.repositories.selectRepo(state().repos[0].root);
    assert.equal(state().name, 'app');
    assert.deepEqual(state().scope, { kind: 'current' });

    // A commit on a branch that is not shown is not hidden from who asks for it: the branches are all shown again.
    const feature = (await git(app, ['rev-parse', 'feature'])).stdout.trim();
    await internals.onMessage(graph, { type: 'goTo', revision: feature });
    assert.equal(state().scope, null);
    assert.ok(state().rows.some((row) => row.hash === feature));

    // A branch deleted while it is one of those shown is no longer one of them.
    await git(app, ['branch', 'scoped', 'HEAD~1']);
    await vscode.commands.executeCommand('gitardo.refresh');
    await internals.onMessage(graph, { type: 'setScope', scope: { kind: 'refs', refs: ['refs/heads/scoped', 'refs/heads/main'] } });
    assert.deepEqual(state().scope, { kind: 'refs', refs: ['refs/heads/scoped', 'refs/heads/main'] });
    await git(app, ['branch', '-D', 'scoped']);
    await vscode.commands.executeCommand('gitardo.refresh');
    assert.deepEqual(state().scope, { kind: 'refs', refs: ['refs/heads/main'] });

    await internals.onMessage(graph, { type: 'setScope', scope: null });
    assert.equal(state().scope, null);
    assert.deepEqual(subjects(), everything);
  });

  await step('lists in the references what their own filter chooses, kept for each repository, and the branches shown in the graph and it never change each other', async () => {
    const graph = view('graph');
    const everything = subjects();
    assert.equal(state().listed, null);
    /** What the workspace state keeps for app. */
    const kept = () => Object.entries(internals.memento.get<Record<string, unknown>>('gitardo.branchesListed') ?? {}).find(([root]) => samePath(root, app))?.[1];

    // Local in the filter of the references: the graph is not narrowed, and its own filter is as it was.
    await internals.onMessage(graph, { type: 'setListed', listed: { kind: 'kinds', kinds: ['branch'] } });
    assert.deepEqual(state().listed, { kind: 'kinds', kinds: ['branch'] });
    assert.equal(state().scope, null);
    assert.deepEqual(subjects(), everything);
    assert.deepEqual(kept(), { kind: 'kinds', kinds: ['branch'] });

    // Narrowing the graph leaves the filter of the references as it was.
    await internals.onMessage(graph, { type: 'setScope', scope: { kind: 'refs', refs: ['refs/heads/main'] } });
    assert.deepEqual(state().scope, { kind: 'refs', refs: ['refs/heads/main'] });
    assert.deepEqual(state().listed, { kind: 'kinds', kinds: ['branch'] });
    await internals.onMessage(graph, { type: 'setScope', scope: null });
    assert.deepEqual(state().listed, { kind: 'kinds', kinds: ['branch'] });

    // Each repository has its own, and finds it again.
    await controller.repositories.selectRepo(lib);
    assert.equal(state().listed, null);
    await controller.repositories.selectRepo(app);
    assert.deepEqual(state().listed, { kind: 'kinds', kinds: ['branch'] });

    // Branches ticked one by one: one deleted is no longer listed, and with none left everything is.
    await git(app, ['branch', 'old', 'HEAD~1']);
    await vscode.commands.executeCommand('gitardo.refresh');
    await internals.onMessage(graph, { type: 'setListed', listed: { kind: 'refs', refs: ['refs/heads/main', 'refs/heads/old', '--all'] } });
    assert.deepEqual(state().listed, { kind: 'refs', refs: ['refs/heads/main', 'refs/heads/old'] });
    await git(app, ['branch', '-D', 'old']);
    await vscode.commands.executeCommand('gitardo.refresh');
    assert.deepEqual(state().listed, { kind: 'refs', refs: ['refs/heads/main'] });
    assert.equal(state().scope, null);

    // Show all: everything listed, and nothing kept for the repository.
    await internals.onMessage(graph, { type: 'setListed', listed: null });
    assert.equal(state().listed, null);
    assert.equal(kept(), undefined);
    assert.deepEqual(subjects(), everything);
  });

  await step('lists the branches checked out last, and remembers the favorites of each repository', async () => {
    const graph = view('graph');
    // What was checked out before is where one is likely to go back to: not what is checked out now.
    await git(app, ['checkout', '-q', 'feature']);
    await git(app, ['checkout', '-q', 'main']);
    await vscode.commands.executeCommand('gitardo.refresh');
    assert.equal(recentBranches(state())[0], 'feature');
    assert.ok(!recentBranches(state()).includes('main'), `recent: ${recentBranches(state())}`);

    // A favorite is told to the views at once, with the rows they already have, and only a branch that is there is one.
    assert.deepEqual(state().favorites, []);
    posted.length = 0;
    await internals.onMessage(graph, { type: 'setFavorite', ref: 'refs/heads/feature', favorite: true });
    await internals.onMessage(graph, { type: 'setFavorite', ref: 'refs/remotes/origin/main', favorite: true });
    await internals.onMessage(graph, { type: 'setFavorite', ref: 'refs/heads/nowhere', favorite: true });
    assert.deepEqual(state().favorites, ['refs/heads/feature', 'refs/remotes/origin/main']);
    assert.ok(posted.some((outgoing) => outgoing.type === 'state'));

    // Each repository has its own, and finds them again.
    await controller.repositories.selectRepo(state().repos[1].root);
    assert.deepEqual(state().favorites, []);
    await controller.repositories.selectRepo(state().repos[0].root);
    assert.deepEqual(state().favorites, ['refs/heads/feature', 'refs/remotes/origin/main']);

    // A branch that is deleted is no longer one, and the others stay when one is taken out.
    await git(app, ['branch', 'liked']);
    await vscode.commands.executeCommand('gitardo.refresh');
    await internals.onMessage(graph, { type: 'setFavorite', ref: 'refs/heads/liked', favorite: true });
    assert.equal(state().favorites.length, 3);
    await git(app, ['branch', '-D', 'liked']);
    await vscode.commands.executeCommand('gitardo.refresh');
    assert.deepEqual(state().favorites, ['refs/heads/feature', 'refs/remotes/origin/main']);
    await internals.onMessage(graph, { type: 'setFavorite', ref: 'refs/heads/feature', favorite: false });
    await internals.onMessage(graph, { type: 'setFavorite', ref: 'refs/remotes/origin/main', favorite: false });
    assert.deepEqual(state().favorites, []);
  });

  /** The commits of what is checked out, as their view in the side bar is given them when it asks. */
  const branchLog = async () => {
    const commits = view('commits');
    posted.length = 0;
    addressed.length = 0;
    await internals.onMessage(commits, { type: 'getLog', limit: 100 });
    const index = posted.findIndex((outgoing) => outgoing.type === 'log');
    const answer = posted[index];
    assert.ok(answer?.type === 'log', 'the view is answered');
    assert.equal(addressed[index], commits);
    return answer.log;
  };

  await step('lists the commits of what is checked out apart from the graph, with their files and the ones that are not pushed', async () => {
    const history = (await git(app, ['log', '--format=%H', 'HEAD'])).stdout.split('\n').filter(Boolean);
    assert.ok(history.length >= 5);
    const log = await branchLog();
    assert.equal(log.root, state().root);
    assert.deepEqual(
      log.commits.map((commit) => commit.hash),
      history,
    );
    assert.equal(log.hasMore, false);
    assert.equal(log.upstream, 'origin/main');
    assert.equal(state().branches.find((branch) => branch.isCheckedOut)!.upstream!.ahead, 2);
    assert.deepEqual(log.unpushed, history.slice(0, 2));

    // What the graph is narrowed to is nothing to them.
    await controller.history.setFilter({ kind: 'path', path: 'b.txt' });
    try {
      assert.ok(state().rows.length < history.length, 'the graph has fewer commits');
      assert.deepEqual(
        (await branchLog()).commits.map((commit) => commit.hash),
        history,
      );
    } finally {
      await controller.history.setFilter(null);
    }

    // A commit unfolds to the files it changed.
    const first = log.commits.at(-1)!;
    posted.length = 0;
    await internals.onMessage(view('commits'), { type: 'getDetail', hash: first.hash });
    const detail = posted.find((outgoing) => outgoing.type === 'detail');
    assert.ok(detail?.type === 'detail' && detail.detail);
    assert.deepEqual(detail.detail.files, [{ kind: 'added', path: 'a.txt', originalPath: null }]);
    assert.deepEqual(errors, []);
  });

  await step('pushes, fetches and pulls with a rebase against the remote', async () => {
    assert.equal(state().branches.find((branch) => branch.isCheckedOut)!.upstream!.ahead, 2);
    await controller.commands.sync('push');
    assert.equal(state().branches.find((branch) => branch.isCheckedOut)!.upstream!.ahead, 0);
    assert.deepEqual((await branchLog()).unpushed, []);
    await controller.commands.sync('fetch');
    await controller.commands.sync('pullRebase');
    assert.deepEqual(errors, []);
  });

  await step('pulls with the changes that are not committed set aside, and says when they do not fit on what came in', async () => {
    const start = state().head.commit!;
    const stashes = state().stashes.length;
    const remote = path.join(workspace, '..', 'remote.git');
    const other = path.join(workspace, '..', 'other');
    await git(workspace, ['clone', remote, other]);
    /** Someone else changes a file and pushes. */
    const theirs = async (file: string, content: string, message: string) => {
      fs.writeFileSync(path.join(other, file), content);
      await git(other, ['add', '--all']);
      await git(other, ['-c', 'user.name=Someone Else', '-c', 'user.email=else@gitardo.invalid', 'commit', '-m', message]);
      await git(other, ['push', 'origin', 'HEAD:main']);
    };
    const read = (file: string) => fs.readFileSync(path.join(app, file), 'utf8');
    const original = read('a.txt');
    try {
      // A change in a file the pull does not touch is there after it.
      await theirs('theirs.txt', 'theirs\n', 'Their file');
      write('a.txt', `${original}mine\n`);
      logged.length = 0;
      asked.length = 0;
      await controller.commands.sync('pullAutostash');
      assert.ok(logged.includes('> git pull --autostash'), logged.join('\n'));
      assert.equal(subjects().filter((subject) => subject === 'Their file').length, 1);
      assert.equal(read('a.txt'), `${original}mine\n`);
      assert.equal(state().stashes.length, stashes);
      assert.equal(asked.length, 0);

      // A change where the pull changes too is left in conflict and in the stash git made, which is said.
      write('a.txt', original);
      write('theirs.txt', 'mine\n');
      await theirs('theirs.txt', 'THEIRS\n', 'Their change');
      await controller.commands.sync('pullRebaseAutostash');
      assert.ok(logged.includes('> git pull --rebase --autostash'), logged.join('\n'));
      assert.equal(subjects().filter((subject) => subject === 'Their change').length, 1);
      assert.equal(state().stashes.length, stashes + 1);
      assert.match(state().stashes[0].message, /autostash/);
      assert.match(read('theirs.txt'), /<<<<<<</);
      assert.ok(asked.some((question) => /do not fit on what came in/.test(question.message) && question.items.includes('Show Stashes')), JSON.stringify(asked));
      assert.deepEqual(errors, []);
    } finally {
      // As it was before, here and on the remote, for the steps that follow.
      await git(app, ['reset', '--hard', start]);
      if (state().stashes.length > stashes) await git(app, ['stash', 'drop']);
      await git(app, ['push', '--force', 'origin', `${start}:main`]);
      await vscode.commands.executeCommand('gitardo.refresh');
    }
    assert.equal(state().head.commit, start);
    assert.equal(state().branches.find((branch) => branch.isCheckedOut)!.upstream!.ahead, 0);
  });

  await step('fast-forwards a branch that is not checked out to what its upstream has on the remote', async () => {
    const start = state().head.commit!;
    const remote = path.join(workspace, '..', 'remote.git');
    const other = path.join(workspace, '..', 'fast-forward');
    await git(app, ['push', 'origin', `${start}:refs/heads/lagging`]);
    await git(app, ['branch', '--track', 'lagging', 'origin/lagging']);
    // Someone else pushes to it, and nothing is fetched here before the fast-forward.
    await git(workspace, ['clone', '--branch', 'lagging', remote, other]);
    fs.writeFileSync(path.join(other, 'lagging.txt'), 'theirs\n');
    await git(other, ['add', '--all']);
    await git(other, ['-c', 'user.name=Someone Else', '-c', 'user.email=else@gitardo.invalid', 'commit', '-m', 'On lagging']);
    await git(other, ['push', 'origin', 'lagging']);
    const theirs = (await git(other, ['rev-parse', 'HEAD'])).stdout.trim();
    await vscode.commands.executeCommand('gitardo.refresh');
    try {
      logged.length = 0;
      await controller.action({ kind: 'fastForward', name: 'lagging' });
      assert.ok(logged.includes('> git fetch origin refs/heads/lagging:refs/heads/lagging'), logged.join('\n'));
      assert.equal(state().branches.find((branch) => branch.name === 'lagging')!.commit, theirs);
      assert.equal(checkedOutBranch(state()), 'main');
      assert.equal(state().head.commit, start);
      assert.deepEqual(errors, []);
    } finally {
      await git(app, ['branch', '-D', 'lagging']);
      await git(app, ['push', 'origin', '--delete', 'lagging']);
      await vscode.commands.executeCommand('gitardo.refresh');
    }
  });

  await step('the Pull button pulls the way the settings choose: with a rebase and the changes set aside, unless told otherwise', async () => {
    const setting = (name: string, value: boolean | undefined) =>
      vscode.workspace.getConfiguration('gitardo').update(name, value, vscode.ConfigurationTarget.Global);
    /** What the button is set to, once the views were told. */
    const chosen = (action: string) => until(`the button set to ${action}`, () => state().defaultPull === action);
    /** The pull that the command of the button runs. */
    const pulled = async () => {
      logged.length = 0;
      await vscode.commands.executeCommand('gitardo.pull');
      return logged.filter((line) => line.startsWith('> git pull'));
    };
    try {
      assert.equal(state().defaultPull, 'pullRebaseAutostash');
      assert.deepEqual(await pulled(), ['> git pull --rebase --autostash']);
      await setting('pull.rebase', false);
      await chosen('pullAutostash');
      assert.deepEqual(await pulled(), ['> git pull --autostash']);
      await setting('pull.autostash', false);
      await chosen('pull');
      assert.deepEqual(await pulled(), ['> git pull']);
      await setting('pull.rebase', undefined);
      await chosen('pullRebase');
      assert.deepEqual(await pulled(), ['> git pull --rebase']);
      // The pull with no option stays a command of its own, whatever the button is set to.
      await setting('pull.autostash', undefined);
      await chosen('pullRebaseAutostash');
      logged.length = 0;
      await vscode.commands.executeCommand('gitardo.pullPlain');
      assert.deepEqual(logged.filter((line) => line.startsWith('> git pull')), ['> git pull']);
      assert.deepEqual(errors, []);
    } finally {
      await setting('pull.rebase', undefined);
      await setting('pull.autostash', undefined);
    }
  });

  await step('warns about what a force push overwrites', async () => {
    // Rewrite the last commit, so that the remote has one the local branch does not.
    await git(app, ['commit', '--amend', '--allow-empty', '-m', 'Add a file, reworded']);
    await vscode.commands.executeCommand('gitardo.refresh');
    asked.length = 0;
    answer = () => undefined;
    await controller.commands.sync('pushForceWithLease');
    assert.equal(asked.length, 1);
    assert.match(asked[0].detail, /will be overwritten:\n\w+ Add a file\n/);
    assert.match(asked[0].detail, /git push --force-with-lease origin main$/);
    answer = run;
    await controller.commands.sync('pushForceWithLease');
    assert.deepEqual(state().branches.find((branch) => branch.isCheckedOut)!.upstream, {
      reference: 'refs/remotes/origin/main',
      remote: 'origin',
      remoteBranch: 'refs/heads/main',
      ahead: 0,
      behind: 0,
      isGone: false,
    });
  });

  await step('undoes the last commit, and restores a deleted branch', async () => {
    write('undo.txt', 'x\n');
    await controller.action({ kind: 'stageAll' });
    await controller.action({ kind: 'commit', message: 'To be undone', amend: false });
    answer = run;
    await controller.action({ kind: 'undo' });
    assert.notEqual(subjects()[1], 'To be undone');
    assert.deepEqual(state().workingTree.staged, [{ kind: 'added', path: 'undo.txt', originalPath: null }]);
    await controller.action({ kind: 'unstageAll' });
    await controller.action({ kind: 'discardAll' });

    await controller.action({ kind: 'deleteBranch', name: 'topic' });
    assert.ok(!state().branches.some((branch) => branch.name === 'topic'));
    await controller.action({ kind: 'undo' });
    assert.ok(state().branches.some((branch) => branch.name === 'topic'));
  });

  await step('rebases a stack of branches with the ones above it moved too, and one Undo puts them all back (M36)', async () => {
    const was = (await git(app, ['rev-parse', '--abbrev-ref', 'HEAD'])).stdout.trim();
    const commitOf = async (ref: string) => (await git(app, ['rev-parse', ref])).stdout.trim();
    await git(app, ['branch', 'stack-base']);
    await git(app, ['switch', '--quiet', '-c', 'stack-a']);
    write('stack-a.txt', 'a\n');
    await git(app, ['add', 'stack-a.txt']);
    await git(app, ['commit', '--quiet', '-m', 'Add stack a']);
    await git(app, ['switch', '--quiet', '-c', 'stack-b']);
    write('stack-b.txt', 'b\n');
    await git(app, ['add', 'stack-b.txt']);
    await git(app, ['commit', '--quiet', '-m', 'Add stack b']);
    await git(app, ['switch', '--quiet', 'stack-base']);
    write('stack-base.txt', 'base\n');
    await git(app, ['add', 'stack-base.txt']);
    await git(app, ['commit', '--quiet', '-m', 'Add stack base']);
    await git(app, ['switch', '--quiet', 'stack-b']);
    await vscode.commands.executeCommand('gitardo.refresh');
    const before = [await commitOf('stack-a'), await commitOf('stack-b')];
    answer = run;
    try {
      await controller.action({ kind: 'rebase', onto: 'stack-base' });
      assert.equal(await commitOf('stack-a^'), await commitOf('stack-base'), 'stack-a moved with stack-b');
      assert.equal(await commitOf('stack-b^'), await commitOf('stack-a'));
      await controller.action({ kind: 'undo' });
      assert.deepEqual([await commitOf('stack-a'), await commitOf('stack-b')], before);
    } finally {
      await git(app, ['switch', '--quiet', was]);
      await git(app, ['branch', '--quiet', '-D', 'stack-a', 'stack-b', 'stack-base']);
      await vscode.commands.executeCommand('gitardo.refresh');
    }
  });

  await step('offers the forced delete when git refuses an unmerged branch', async () => {
    asked.length = 0;
    answer = run;
    await controller.action({ kind: 'deleteBranch', name: 'feature' });
    assert.equal(asked.length, 1);
    assert.match(asked[0].detail, /not fully merged[\s\S]*git branch -D feature$/);
    assert.ok(!state().branches.some((branch) => branch.name === 'feature'));
    await controller.action({ kind: 'undo' });
    assert.ok(state().branches.some((branch) => branch.name === 'feature'));
  });

  await step('cleans up merged branches from a picker, with their worktrees', async () => {
    // `merged` has a worktree, which goes with it.
    const home = path.join(workspace, 'app.worktrees').replace(/\\/g, '/');
    const dir = `${home}/merged`;
    await git(app, ['worktree', 'add', dir, 'merged']);
    await vscode.commands.executeCommand('gitardo.refresh');
    let offered: string[] = [];
    choose = (items) => {
      offered = items.map((item) => `${item.label}${item.picked ? ' (ticked)' : ''}: ${item.description}`);
      return items.filter((item) => item.picked);
    };
    answer = run;
    try {
      await vscode.commands.executeCommand('gitardo.cleanupBranches');
      // `topic` points at the commit that was reworded since, so it is not merged and not offered.
      // The engine joins what it says of a branch with commas, the extension with dots (stage 4, area 2).
      assert.equal(offered.length, 1);
      assert.match(offered[0], /^merged \(ticked\): merged into main(, | · )with its worktree merged$/);
      assert.deepEqual(
        state().branches.map((branch) => branch.name),
        ['feature', 'main', 'topic'],
      );
      assert.equal(state().worktrees.length, 1);
      assert.ok(!fs.existsSync(dir));
    } finally {
      fs.rmSync(home, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
      await git(app, ['worktree', 'prune']);
    }
  });

  await step('browses the reflog and shows the chosen commit in the graph', async () => {
    let entries = 0;
    choose = (items, options) => {
      if (options.title?.startsWith('Reflog')) {
        entries = items.length;
        return items[1];
      }
      return byLabel('Show in the graph')(items);
    };
    posted.length = 0;
    await vscode.commands.executeCommand('gitardo.reflog');
    assert.ok(entries > 5, `the reflog has entries (${entries})`);
    assert.ok(posted.some((outgoing) => outgoing.type === 'reveal'));
  });

  await step('goes to a commit by its hash: one that is loaded, one further back, one that is filtered out, one no branch reaches', async () => {
    const graph = view('graph');
    const loaded = (hash: string) => state().rows.some((row) => row.hash === hash);
    /** The commits the views were told to select, and the kinds of the views that were told. */
    const revealed = () =>
      posted.flatMap((outgoing, index) => (outgoing.type === 'reveal' ? [`${outgoing.hash} in ${internals.views.get(addressed[index])}`] : []));
    const forget = () => {
      posted.length = 0;
      addressed.length = 0;
      asked.length = 0;
    };
    const oldest = state().rows.at(-1)!.hash;
    const pages = internals.limit;

    // The start of a hash is enough, as typed in the search box of a view: only that view goes there.
    forget();
    await internals.onMessage(graph, { type: 'goTo', revision: oldest.slice(0, 8) });
    assert.deepEqual(revealed(), [`${oldest} in graph`]);

    // Further back than the first page: the read goes on to the end of the history as git prints it (D54), and the
    // commit is shown once its page came. The views, whose lists end in sight, would ask for more themselves: they are
    // not answered, to see that nothing needs to.
    unanswered.add('loadMore');
    try {
      await vscode.commands.executeCommand('gitardo.refresh');
      forget();
      await internals.onMessage(view('sideGraph'), { type: 'goTo', revision: oldest });
      assert.ok(loaded(oldest), 'the history is loaded up to the commit');
      assert.deepEqual(revealed(), [`${oldest} in sideGraph`]);
      assert.equal(internals.limit, pages);
    } finally {
      unanswered.delete('loadMore');
      internals.limit = pages;
    }

    // In a list that leaves it out, the list gives way; in one that has it, the list stays.
    await controller.history.setFilter({ kind: 'path', path: 'b.txt' });
    const inFile = state().rows[0].hash;
    assert.ok(!loaded(oldest));
    await internals.onMessage(graph, { type: 'goTo', revision: inFile });
    assert.deepEqual(state().filter, { kind: 'path', path: 'b.txt' });
    await internals.onMessage(graph, { type: 'goTo', revision: oldest });
    assert.equal(state().filter, null);
    assert.ok(loaded(oldest));

    // A commit that no branch or tag reaches is shown with its own history.
    const lost = (await git(app, ['commit-tree', 'HEAD^{tree}', '-p', 'HEAD', '-m', 'Lost'])).stdout.trim();
    forget();
    await internals.onMessage(graph, { type: 'goTo', revision: lost.slice(0, 12) });
    assert.deepEqual(state().filter, { kind: 'ref', ref: lost, lost: true });
    assert.equal(subjects()[0], 'Lost');
    assert.deepEqual(revealed(), [`${lost} in graph`]);
    await controller.history.setFilter(null);

    // What is not a commit is said, and nothing moves.
    forget();
    await internals.onMessage(graph, { type: 'goTo', revision: 'fffffffffff' });
    assert.deepEqual(revealed(), []);
    assert.match(asked[0].message, /"fffffffffff" is not a commit of this repository/);

    // The command asks for anything git understands, and shows it in the graph.
    const third = (await git(app, ['rev-parse', 'HEAD~2'])).stdout.trim();
    inputs = ['HEAD~2'];
    forget();
    await vscode.commands.executeCommand('gitardo.goToCommit');
    assert.ok(revealed().includes(`${third} in graph`), `revealed: ${revealed()}`);
  });

  await step('finds commits in the whole history for a view to highlight where they are, without replacing the graph', async () => {
    const commits = view('sideGraph');
    const find = async (search: Extract<Filter, { query: string }>) => {
      posted.length = 0;
      addressed.length = 0;
      await internals.onMessage(commits, { type: 'find', search });
      const index = posted.findIndex((outgoing) => outgoing.type === 'found');
      const found = posted[index];
      assert.ok(found?.type === 'found', 'the view is answered');
      assert.equal(addressed[index], commits);
      assert.deepEqual(found.search, search);
      return found;
    };
    const hashes = (wanted: (subject: string) => boolean = () => true) =>
      state()
        .rows.filter((row) => wanted(row.subject))
        .map((row) => row.hash);
    const everything = hashes().length;

    const added = await find({ kind: 'message', query: 'ADD' });
    assert.ok(added.hashes.length > 2 && !added.more);
    assert.deepEqual(added.hashes, hashes((subject) => /add/i.test(subject)));
    // The graph is as it was.
    assert.equal(state().filter, null);
    assert.equal(hashes().length, everything);

    // In the history of a file, only the commits of that history.
    await controller.history.setFilter({ kind: 'path', path: 'b.txt' });
    assert.ok(hashes().length < everything);
    assert.deepEqual((await find({ kind: 'author', query: 'gitardo test' })).hashes, hashes());
    assert.deepEqual(state().filter, { kind: 'path', path: 'b.txt' });

    // A list of results gives way to the graph the new ones are shown in.
    await controller.history.setFilter({ kind: 'message', query: 'feature' });
    const two = await find({ kind: 'pickaxe', query: 'two' });
    assert.equal(state().filter, null);
    assert.ok(two.hashes.length > 0 && two.hashes.every((hash) => hashes().includes(hash)));

    // A search git refuses is said, and finds nothing.
    const before = errors.length;
    assert.deepEqual((await find({ kind: 'regex', query: '[' })).hashes, []);
    assert.match(errors.at(-1)!, /the search failed/);
    // Provoked on purpose.
    errors.length = before;
  });

  await step('switches identity for the repository', async () => {
    choose = byLabel('Enter a name and email…');
    inputs = ['Other Me', 'other@gitardo.invalid'];
    await vscode.commands.executeCommand('gitardo.switchIdentity');
    assert.deepEqual(state().identity, { name: 'Other Me', email: 'other@gitardo.invalid' });
  });

  await step('opens all the changes between two commits in one editor', async () => {
    const from = state().rows.at(-1)!.hash;
    const to = state().head.commit!;
    // The files git says differ, as the engine lists them.
    const named: Record<string, ChangeKind> = { A: 'added', M: 'modified', D: 'deleted', T: 'typeChanged' };
    const files: FileChange[] = (await git(app, ['diff', '--name-status', '--no-renames', from, to])).stdout
      .split('\n')
      .filter(Boolean)
      .map((line) => {
        const [letter, file] = line.split('\t');
        return { path: file, originalPath: null, kind: named[letter[0]] };
      });
    assert.ok(files.length > 1);
    await fromWebview({ type: 'openChanges', source: { kind: 'commit', hash: to, parent: from }, files, title: 'Everything since the start' });
    const tab = await until('the editor with all the changes', () => vscode.window.tabGroups.activeTabGroup.activeTab);
    assert.equal(tab.label, `Everything since the start (${files.length} files)`);
    // Not one diff, and not a file: the stable API has no type of its own for this editor either.
    assert.ok(!(tab.input instanceof vscode.TabInputTextDiff) && !(tab.input instanceof vscode.TabInputText));
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
  });

  await step('opens a commit in an editor tab of its own, with what it changed, whichever repository the views show by then', async () => {
    const hashes = (await git(app, ['log', '--all', '--format=%H %P', '--grep=^Change a$'])).stdout.trim().split('\n');
    const [hash, parent] = hashes.at(-1)!.split(' ');
    const short = hash.slice(0, 7);
    const tabs = () => vscode.window.tabGroups.all.flatMap((group) => group.tabs).filter((tab) => tab.input instanceof vscode.TabInputWebview);
    const commitViews = () => [...internals.views.entries()].filter(([, kind]) => kind === 'commit').map(([webview]) => webview as vscode.Webview);
    const told = (type: HostMessage['type'], to: unknown) => posted.filter((outgoing, index) => outgoing.type === type && addressed[index] === to);

    received.length = 0;
    posted.length = 0;
    addressed.length = 0;
    // As a double click on its row asks.
    await fromWebview({ type: 'openCommit', hash });
    const tab = await until('the tab of the commit', () => tabs().find((candidate) => candidate.label.startsWith(short)));
    assert.equal(tab.label, `${short} Change a`);
    assert.ok(tab.isActive);
    const [webview] = commitViews();
    assert.match(webview.html, new RegExp(`data-view="commit" data-session="${vscode.env.sessionId}" data-commit="${hash}" data-root="[^"]+" data-files="tree"`));

    // The page asks for its commit by itself: it loaded under the content security policy, and read which one it is of.
    await until('the page to ask for the changes', () => received.some((incoming) => incoming.type === 'getPatch' && incoming.hash === hash));
    const detail = await until('the details', () => told('detail', webview)[0]);
    assert.ok(detail.type === 'detail' && detail.detail);
    assert.deepEqual([subjectAndBody(detail.detail.message).subject, detail.detail.parents], ['Change a', [parent]]);
    const patch = await until('the changes', () => told('patch', webview)[0]);
    assert.ok(patch.type === 'patch' && patch.patch);
    assert.deepEqual(
      patch.patch.map((file) => [CHANGE_LETTERS[file.file.kind], file.file.path, file.added, file.removed, file.hunks?.[0].lines]),
      [['M', 'a.txt', 1, 0, [' one', '+two']]],
    );
    // It is told the state as well, for the labels of its commit.
    assert.equal(told('state', webview).length, 1);

    // Asked for again, the commit has its tab: that one is shown.
    await vscode.commands.executeCommand('gitardo.openGraph');
    await controller.commitTabs.openCommit(hash);
    assert.equal(commitViews().length, 1);
    assert.equal(tabs().filter((candidate) => candidate.label.startsWith(short)).length, 1);

    // Its parent, from the link in the page, has another. So has a revision typed in the prompt of the command.
    await internals.onMessage(webview, { type: 'openCommit', hash: parent });
    await until('the tab of the parent', () => tabs().some((candidate) => candidate.label === `${parent.slice(0, 7)} Add b`));
    await internals.onMessage(webview, { type: 'setFilesLayout', layout: 'list' });
    inputs = ['feature'];
    await vscode.commands.executeCommand('gitardo.openCommit');
    await until('the tab of the typed revision', () => tabs().some((candidate) => /^\w{7} Add feature$/.test(candidate.label)));
    assert.equal(commitViews().length, 3);
    // The layout chosen in one tab is the one the next opens with.
    assert.match(commitViews()[2].html, /data-files="list"/);
    await internals.onMessage(webview, { type: 'setFilesLayout', layout: 'tree' });

    // The other views move on to another repository: the tab is still of its own.
    await controller.repositories.selectRepo(state().repos[1].root);
    assert.equal(state().name, 'lib');
    posted.length = 0;
    addressed.length = 0;
    await internals.onMessage(webview, { type: 'getDetail', hash });
    const [again] = told('detail', webview);
    assert.ok(again.type === 'detail' && again.detail && subjectAndBody(again.detail.message).subject === 'Change a');
    await internals.onMessage(webview, { type: 'openDiff', source: { kind: 'commit', hash, parent }, file: { kind: 'modified', path: 'a.txt', originalPath: null } });
    const diff = await until('the diff editor', () => {
      const active = vscode.window.tabGroups.activeTabGroup.activeTab;
      return active?.input instanceof vscode.TabInputTextDiff ? active.input : undefined;
    });
    assert.equal((await vscode.workspace.openTextDocument(diff.modified)).getText(), 'one\ntwo\n');
    assert.equal((await vscode.workspace.openTextDocument(diff.original)).getText(), 'one\n');

    // And its button for the graph brings the graph back to the repository of the commit.
    await internals.onMessage(webview, { type: 'reveal', hash });
    assert.equal(state().name, 'app');
    assert.ok(posted.some((outgoing) => outgoing.type === 'reveal' && outgoing.hash === hash));

    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
    await until('the tabs to detach', () => commitViews().length === 0);
    // A closed tab is not kept for the next time.
    await controller.commitTabs.openCommit(hash);
    await until('a new tab', () => commitViews().length === 1);
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
    await until('the tab to detach', () => commitViews().length === 0);

    // What is not a commit of the repository is said, and opens nothing.
    asked.length = 0;
    await controller.commitTabs.openCommit('0123456789abcdef0123456789abcdef01234567');
    assert.match(asked.at(-1)!.message, /0123456 is not a commit of this repository/);
    assert.equal(commitViews().length, 0);
  });

  await step('adds, renames and removes a remote, and offers to add one when there is none', async () => {
    const pickRemote = (name: string, then: string) => (items: Item[], options: vscode.QuickPickOptions) =>
      byLabel(options.title?.startsWith('Remotes of') ? name : then)(items);

    choose = byLabel('Add remote…');
    inputs = ['backup', 'https://example.invalid/backup.git'];
    await vscode.commands.executeCommand('gitardo.remotes');
    assert.deepEqual(state().remotes, ['backup', 'origin']);
    assert.ok(logged.includes('> git remote add backup https://example.invalid/backup.git'));

    choose = pickRemote('backup', 'Rename…');
    inputs = ['mirror'];
    await vscode.commands.executeCommand('gitardo.remotes');
    assert.deepEqual(state().remotes, ['mirror', 'origin']);

    choose = pickRemote('mirror', 'Remove');
    asked.length = 0;
    answer = run;
    await vscode.commands.executeCommand('gitardo.remotes');
    assert.equal(asked[0].message, 'Remove remote mirror?');
    assert.deepEqual(state().remotes, ['origin']);

    await controller.repositories.selectRepo(state().repos[1].root);
    assert.deepEqual(state().remotes, []);
    asked.length = 0;
    answer = () => undefined;
    await controller.commands.sync('push');
    // The engine writes it as the desktop does, in capitals where the extension had none (stage 4, area 2).
    assert.equal(asked.length, 1);
    assert.match(asked[0].message, /^Gitardo: [Tt]his repository has no remote\.$/);
    assert.equal(asked[0].items.length, 1);
    assert.match(asked[0].items[0], /^Add [Rr]emote…$/);
    await controller.repositories.selectRepo(state().repos[0].root);
  });

  await step('stashes a single file and leaves the rest alone', async () => {
    write('a.txt', 'one\ntwo\nstash me\n');
    write('keep.txt', 'stays\n');
    await vscode.commands.executeCommand('gitardo.refresh');
    inputs = ['only a'];
    await controller.action({ kind: 'stashPush', paths: ['a.txt'] });
    assert.equal(state().stashes.length, 1);
    assert.match(state().stashes[0].message, /only a$/);
    assert.deepEqual([state().workingTree.unstaged, state().workingTree.untracked], [[], ['keep.txt']]);
    assert.equal(fs.readFileSync(path.join(app, 'a.txt'), 'utf8'), 'one\ntwo\n');

    // Asked first, with what it would stop on.
    asked.length = 0;
    answer = run;
    await controller.action({ kind: 'stashPop', ref: 'stash@{0}' });
    assert.deepEqual(
      asked.map((question) => question.message),
      ['Pop stash@{0}?'],
    );
    assert.ok(asked[0].detail.includes('No conflicts expected.'), asked[0].detail);
    assert.equal(state().stashes.length, 0);
    assert.equal(state().changes, 2);
    await controller.action({ kind: 'discardAll' });
    assert.equal(state().changes, 0);
  });

  await step('the buttons added to the list of changes of VS Code stash a file, what is staged or everything, in the repository of the file', async () => {
    // What VS Code hands to the command of a button: the file it is on, or the group with its files.
    const file = (root: string, name: string) => ({ resourceUri: vscode.Uri.file(path.join(root, name)) });
    const stashed = () => logged.filter((line) => line.startsWith('> git stash push'));
    const before = errors.length;
    const pop = () => {
      answer = run;
      return controller.action({ kind: 'stashPop', ref: 'stash@{0}' });
    };
    write('a.txt', 'one\ntwo\nstash me\n');
    write('keep.txt', 'stays\n');
    await vscode.commands.executeCommand('gitardo.refresh');
    try {
      logged.length = 0;
      inputs = ['from the list'];
      await vscode.commands.executeCommand('gitardo.scmStashFile', file(app, 'a.txt'));
      assert.deepEqual(stashed(), ['> git stash push --include-untracked -m "from the list" -- a.txt']);
      assert.equal(state().stashes.length, 1);
      assert.deepEqual([state().workingTree.unstaged, state().workingTree.untracked], [[], ['keep.txt']]);
      await pop();
      assert.equal(state().changes, 2);

      // On the group of the changes: everything, untracked files too.
      logged.length = 0;
      inputs = [''];
      await vscode.commands.executeCommand('gitardo.scmStashAll', { resourceStates: [file(app, 'a.txt'), file(app, 'keep.txt')] });
      assert.deepEqual(stashed(), ['> git stash push --include-untracked']);
      assert.equal(state().changes, 0);
      await pop();
      assert.equal(state().changes, 2);

      // On the group of what is staged: that, and nothing else.
      await controller.action({ kind: 'stage', paths: ['a.txt'] });
      logged.length = 0;
      inputs = ['staged'];
      await vscode.commands.executeCommand('gitardo.scmStashStaged', { resourceStates: [file(app, 'a.txt')] });
      assert.deepEqual(stashed(), ['> git stash push --staged -m staged']);
      assert.deepEqual(state().workingTree.staged, []);
      assert.deepEqual([state().workingTree.unstaged, state().workingTree.untracked], [[], ['keep.txt']]);
      await pop();
      assert.equal(state().changes, 2);

      // A file of the other repository is stashed there, which the views then show.
      fs.writeFileSync(path.join(lib, 'lib.txt'), 'lib\nmore\n');
      inputs = ['in lib'];
      await vscode.commands.executeCommand('gitardo.scmStashFile', file(lib, 'lib.txt'));
      assert.equal(state().name, 'lib');
      assert.equal(state().stashes.length, 1);
      assert.equal(state().changes, 0);
      await pop();
      await git(lib, ['checkout', '--', 'lib.txt']);
      assert.equal(errors.length, before);
    } finally {
      await controller.repositories.selectRepo(state().repos[0].root);
      answer = run;
      await controller.action({ kind: 'discardAll' });
    }
    assert.equal(state().name, 'app');
    assert.equal(state().changes, 0);
    assert.equal(state().stashes.length, 0);
  });

  await step('cherry-picks several commits oldest first, whatever order they were selected in', async () => {
    await git(app, ['checkout', '-b', 'picks']);
    const made: string[] = [];
    for (const name of ['one', 'two']) {
      write(`pick-${name}.txt`, `${name}\n`);
      await git(app, ['add', '--all']);
      await git(app, ['commit', '-m', `Pick ${name}`]);
      made.push((await git(app, ['rev-parse', 'HEAD'])).stdout.trim());
    }
    await git(app, ['checkout', 'main']);
    await vscode.commands.executeCommand('gitardo.refresh');
    assert.equal(checkedOutBranch(state()), 'main');
    const before = state().head.commit!;

    await vscode.workspace.getConfiguration('gitardo').update('commandPreview', 'all', vscode.ConfigurationTarget.Global);
    try {
      asked.length = 0;
      answer = run;
      await controller.action({ kind: 'cherryPickMany', hashes: [made[1], made[0]] });
      // The engine names the branch they are picked onto, and lists the commands only (area 3 of stage 4).
      assert.match(asked[0].message, /^Cherry-pick 2 commits( onto main)?\?$/);
      if (!asked[0].message.includes('onto')) assert.match(asked[0].detail, /In this order:\n\w{7} Pick one\n\w{7} Pick two\n/);
      assert.ok(asked[0].detail.endsWith(`git cherry-pick ${made[0]} ${made[1]}`));
    } finally {
      await vscode.workspace.getConfiguration('gitardo').update('commandPreview', undefined, vscode.ConfigurationTarget.Global);
    }
    assert.equal(state().operation, null);
    const log = async () => (await git(app, ['log', '--format=%s', '-2', 'main'])).stdout.trim().split('\n');
    assert.deepEqual(await log(), ['Pick two', 'Pick one'], JSON.stringify({ errors, logged: logged.slice(-12), asked }));

    // Undo takes back the whole series, not just its last commit; undoing again brings it back.
    asked.length = 0;
    await controller.action({ kind: 'undo' });
    assert.match(asked[0].message, /^Undo "Cherry-pick 2 commits( onto main)?"\?$/);
    assert.match(asked[0].detail, new RegExp(`git reset --keep ${before}$`));
    assert.equal(state().head.commit, before);
    if (internals.engine?.running) {
      // The engine's Undo takes its entry out of the journal, so that undoing again takes back the operation before
      // (D9 of the differences): the series is put back as a test would.
      await git(app, ['cherry-pick', made[0], made[1]]);
      await vscode.commands.executeCommand('gitardo.refresh');
    } else {
      await controller.action({ kind: 'undo' });
    }
    assert.deepEqual(await log(), ['Pick two', 'Pick one']);
    await git(app, ['branch', '-D', 'picks']);
    await vscode.commands.executeCommand('gitardo.refresh');
  });

  await step('continues a series of cherry-picks past a conflict resolved by keeping what the branch has', async () => {
    const before = state().head.commit!;
    await git(app, ['checkout', '-b', 'clashing']);
    const made: string[] = [];
    for (const [name, content] of [['clash', 'theirs'], ['after', 'after']]) {
      write(`${name}.txt`, `${content}\n`);
      await git(app, ['add', '--all']);
      await git(app, ['commit', '-m', `Pick ${name}`]);
      made.push((await git(app, ['rev-parse', 'HEAD'])).stdout.trim());
    }
    await git(app, ['checkout', 'main']);
    write('clash.txt', 'ours\n');
    await git(app, ['add', '--all']);
    await git(app, ['commit', '-m', 'Clash on main']);
    await vscode.commands.executeCommand('gitardo.refresh');
    try {
      answer = run;
      await controller.action({ kind: 'cherryPickMany', hashes: made });
      assert.equal(state().operation, 'cherryPick');
      assert.deepEqual(state().workingTree.conflicted.map((file) => file.path), ['clash.txt']);
      const failed = errors.length;

      // Keeping main's side leaves the first commit with nothing to commit: continuing skips it and picks the next.
      await controller.action({ kind: 'takeSide', path: 'clash.txt', side: 'ours' });
      await controller.action({ kind: 'continue' });
      assert.equal(state().operation, null);
      assert.equal(errors.length, failed, errors.slice(failed).join('\n'));
      assert.deepEqual((await git(app, ['log', '--format=%s', '-2', 'main'])).stdout.trim().split('\n'), ['Pick after', 'Clash on main']);
    } finally {
      await git(app, ['cherry-pick', '--abort']).catch(() => undefined);
      await git(app, ['reset', '--hard', before]);
      await git(app, ['branch', '-D', 'clashing']);
      await vscode.commands.executeCommand('gitardo.refresh');
    }
  });

  await step('merges in the way chosen, refuses a fast-forward only that cannot be one, and undoes a squash, which moves no branch', async () => {
    const before = state().head.commit!;
    await git(app, ['checkout', '-b', 'merging']);
    write('merge-way.txt', 'merged\n');
    await git(app, ['add', '--all']);
    await git(app, ['commit', '-m', 'Add a file to merge']);
    const tip = (await git(app, ['rev-parse', 'HEAD'])).stdout.trim();
    await git(app, ['checkout', 'main']);
    await vscode.commands.executeCommand('gitardo.refresh');
    try {
      answer = run;
      let offered: string[] = [];
      const way = (label: string) => (items: Item[]) => {
        offered = items.map((item) => item.label);
        return byLabel(label)(items);
      };

      choose = way('Merge (fast-forward only)');
      await controller.action({ kind: 'merge', ref: 'merging', mode: 'ask' });
      assert.deepEqual(offered, ['Merge', 'Merge (no fast-forward)', 'Merge (fast-forward only)', 'Merge (squash)']);
      assert.equal(state().head.commit, tip);
      await controller.action({ kind: 'undo' });
      assert.equal(state().head.commit, before);

      // The changes are staged, the commit box is given the message git prepared, and Undo takes them out again.
      asked.length = 0;
      posted.length = 0;
      choose = way('Merge (squash)');
      await controller.action({ kind: 'merge', ref: 'merging', mode: 'ask' });
      assert.equal(state().head.commit, before);
      assert.deepEqual(state().workingTree.staged, [{ kind: 'added', path: 'merge-way.txt', originalPath: null }]);
      assert.ok(asked.some((question) => /the changes are staged, and nothing is committed yet/i.test(question.message)), JSON.stringify(asked));
      const message = posted.find((outgoing) => outgoing.type === 'commitMessage');
      assert.ok(message?.type === 'commitMessage' && message.message.includes('Add a file to merge'), JSON.stringify(message));
      asked.length = 0;
      await controller.action({ kind: 'undo' });
      assert.equal(asked[0].message, 'Undo "Merge merging into main (squash)"?');
      assert.match(asked[0].detail, /git reset --merge$/);
      assert.equal(state().head.commit, before);
      assert.equal(state().changes, 0);
      assert.ok(!fs.existsSync(path.join(app, 'merge-way.txt')));

      // With a commit of its own, main cannot be fast-forwarded: that is said before git runs.
      write('merge-own.txt', 'own\n');
      await git(app, ['add', '--all']);
      await git(app, ['commit', '-m', 'Add a file of main']);
      await vscode.commands.executeCommand('gitardo.refresh');
      const own = state().head.commit!;
      errors.length = 0;
      logged.length = 0;
      asked.length = 0;
      await controller.action({ kind: 'merge', ref: 'merging', mode: 'fastForwardOnly' });
      // The engine says it as a refusal, a warning; the extension as an error.
      const said = [...errors, ...asked.map((question) => question.message)];
      assert.ok(said.some((text) => /main cannot be fast-forwarded to merging/.test(text)), JSON.stringify(said));
      assert.ok(!logged.some((line) => line.startsWith('> git merge --ff-only')), JSON.stringify(logged));
      assert.equal(state().head.commit, own);

      await controller.action({ kind: 'merge', ref: 'merging', mode: 'noFastForward' });
      assert.deepEqual(state().rows[0].parents, [own, tip]);
      await controller.action({ kind: 'undo' });
      assert.equal(state().head.commit, own);
    } finally {
      choose = () => undefined;
      await git(app, ['reset', '--hard', before]);
      await git(app, ['branch', '-D', 'merging']);
      await vscode.commands.executeCommand('gitardo.refresh');
    }
  });

  await step('records the commit a cherry-pick was picked from when the settings ask', async () => {
    const before = state().head.commit!;
    await git(app, ['checkout', '-b', 'origins']);
    write('origin.txt', 'origin\n');
    await git(app, ['add', '--all']);
    await git(app, ['commit', '-m', 'Add a file to pick']);
    const picked = (await git(app, ['rev-parse', 'HEAD'])).stdout.trim();
    await git(app, ['checkout', 'main']);
    await vscode.commands.executeCommand('gitardo.refresh');
    await vscode.workspace.getConfiguration('gitardo').update('cherryPick.recordOrigin', true, vscode.ConfigurationTarget.Global);
    try {
      answer = run;
      await controller.action({ kind: 'cherryPick', hash: picked, isMerge: false });
      const message = (await git(app, ['log', '-1', '--format=%B'])).stdout.trim();
      assert.equal(message, `Add a file to pick\n\n(cherry picked from commit ${picked})`);
    } finally {
      await vscode.workspace.getConfiguration('gitardo').update('cherryPick.recordOrigin', undefined, vscode.ConfigurationTarget.Global);
      await git(app, ['reset', '--hard', before]);
      await git(app, ['branch', '-D', 'origins']);
      await vscode.commands.executeCommand('gitardo.refresh');
    }
  });

  await step('copies commits as a patch oldest first, and saves it to the file chosen', async () => {
    const hashOf = (subject: string) => state().rows.find((row) => row.subject === subject)!.hash;
    const hashes = [hashOf('Pick one'), hashOf('Pick two')];
    const made = (text: string) => [...text.matchAll(/^From ([0-9a-f]+) /gm)].map((match) => match[1]);
    asked.length = 0;
    await fromWebview({ type: 'formatPatch', hashes });
    assert.deepEqual(made(await vscode.env.clipboard.readText()), hashes);
    assert.deepEqual(
      asked.map((question) => question.message.toLowerCase()),
      ['gitardo: copied 2 commits as a patch.'],
    );

    const file = path.join(workspace, 'picks.patch');
    const offered: vscode.SaveDialogOptions[] = [];
    const showSaveDialog = ui.showSaveDialog;
    ui.showSaveDialog = ((options: vscode.SaveDialogOptions) => {
      offered.push(options);
      return Promise.resolve(vscode.Uri.file(file));
    }) as never;
    try {
      await fromWebview({ type: 'formatPatch', hashes: hashes.slice(1), save: true });
      assert.equal(offered[0].title, `Save ${hashes[1].slice(0, 7)} as a patch`);
      assert.ok(samePath(offered[0].defaultUri!.fsPath, path.join(app, `${hashes[1].slice(0, 7)}.patch`)));
      assert.deepEqual(made(fs.readFileSync(file, 'utf8')), hashes.slice(1));
      // Git makes no patch of a merge: nothing is written, and the user is told why.
      errors.length = 0;
      asked.length = 0;
      const merge = state().rows.find((row) => row.parents.length > 1 && !row.stash);
      if (merge) {
        await fromWebview({ type: 'formatPatch', hashes: [merge.hash], save: true });
        assert.equal(offered.length, 1);
        // The engine says it as a refusal, a warning; the extension as an error.
        const said = [...errors, ...asked.map((question) => question.message)];
        assert.equal(said.length, 1, JSON.stringify(said));
        assert.match(said[0], /^Gitardo: git makes no patch of a merge commit/i);
      }
    } finally {
      ui.showSaveDialog = showSaveDialog;
      fs.rmSync(file, { force: true });
    }
  });

  await step('applies a patch from the clipboard as commits, which undo takes back together, and refuses one that does not apply', async () => {
    const before = state().head.commit!;
    const subject = state().rows.find((row) => row.hash === before)!.subject;
    await git(app, ['checkout', '-b', 'patched']);
    write('patched.txt', 'one\n');
    await git(app, ['add', '--all']);
    await git(app, ['commit', '-m', 'Patch one']);
    write('patched.txt', 'one\ntwo\n');
    await git(app, ['commit', '-am', 'Patch two']);
    const made = (await git(app, ['rev-list', '-2', 'HEAD'])).stdout.trim().split('\n').reverse();
    await git(app, ['checkout', 'main']);
    await vscode.commands.executeCommand('gitardo.refresh');
    await fromWebview({ type: 'formatPatch', hashes: made });
    const titles: (string | undefined)[] = [];
    choose = (items, options) => {
      titles.push(options.title);
      return byLabel(/From the clipboard|As commits/)(items);
    };
    asked.length = 0;
    errors.length = 0;
    try {
      await vscode.commands.executeCommand('gitardo.applyPatch');
      assert.deepEqual(titles, ['Apply patch', 'Apply the patch in the clipboard']);
      assert.deepEqual(errors, []);
      assert.deepEqual((await git(app, ['log', '--format=%s', '-3', 'main'])).stdout.trim().split('\n'), ['Patch two', 'Patch one', subject]);
      // The patch reached git through a file of its own, which is gone.
      const command = [...logged].reverse().find((line) => line.startsWith('> git am --3way '))!;
      assert.match(command, /gitardo-[\w-]+\.patch"?$/);
      const temporary = command.slice('> git am --3way '.length).replace(/^"|"$/g, '').replace(/\\\\/g, '\\');
      assert.ok(!fs.existsSync(temporary), `${temporary} is removed`);

      // Undo takes back both commits.
      answer = run;
      asked.length = 0;
      await controller.action({ kind: 'undo' });
      assert.equal(asked[0].message, 'Undo "Apply the patch in the clipboard as 2 commits"?');
      assert.equal(state().head.commit, before);

      // A file the patch adds is there already: it is refused, and nothing changes.
      write('patched.txt', 'local\n');
      await vscode.commands.executeCommand('gitardo.refresh');
      asked.length = 0;
      answer = () => undefined;
      await vscode.commands.executeCommand('gitardo.applyPatch');
      assert.equal(asked.length, 1);
      // The engine gives git's reasons in the message itself, after the sentence.
      assert.ok(asked[0].message.replace(/^Gitardo: /, '').startsWith('The patch in the clipboard does not apply, so nothing was changed.'), asked[0].message);
      assert.match(`${asked[0].message}\n${asked[0].detail}`, /patched\.txt/);
      assert.equal(state().head.commit, before);
      assert.equal(state().operation, null);
      assert.equal(fs.readFileSync(path.join(app, 'patched.txt'), 'utf8'), 'local\n');
    } finally {
      choose = () => undefined;
      fs.rmSync(path.join(app, 'patched.txt'), { force: true });
      await git(app, ['branch', '-D', 'patched']);
      await vscode.commands.executeCommand('gitardo.refresh');
    }
  });

  await step('stops applying a patch from a file on a conflict, says so first, and aborts it', async () => {
    const before = state().head.commit!;
    await git(app, ['checkout', '-b', 'clashing-patch']);
    write('a.txt', 'patched\n');
    await git(app, ['commit', '-am', 'Patch a']);
    const text = (await git(app, ['format-patch', '--stdout', '-1', 'HEAD'])).stdout;
    await git(app, ['checkout', 'main']);
    write('a.txt', 'main side\n');
    await git(app, ['commit', '-am', 'Change a on main']);
    await vscode.commands.executeCommand('gitardo.refresh');
    const file = path.join(workspace, 'clash.patch');
    fs.writeFileSync(file, text);
    const showOpenDialog = ui.showOpenDialog;
    ui.showOpenDialog = (() => Promise.resolve([vscode.Uri.file(file)])) as never;
    choose = byLabel(/From a file|As commits/);
    answer = run;
    asked.length = 0;
    try {
      await vscode.commands.executeCommand('gitardo.applyPatch');
      assert.equal(asked[0].message, 'Apply clash.patch as 1 commit?');
      assert.match(asked[0].detail, /stop with conflicts in 1 file:\na\.txt/);
      assert.match(asked[0].detail, /git am --3way \S*clash\.patch"?$/);
      assert.equal(state().operation, 'patch');
      assert.deepEqual(
        state().workingTree.conflicted.map((change) => change.path),
        ['a.txt'],
      );
      const stopped = /^Apply clash\.patch as 1 commit stopped: 1 file has conflicts\. Resolve them, then continue or abort the am\./;
      assert.ok(asked.some((question) => stopped.test(question.message)));

      await controller.action({ kind: 'abort' });
      assert.equal(state().operation, null);
      assert.equal(state().changes, 0);
      assert.ok(fs.existsSync(file), 'the file chosen is left as it is');
    } finally {
      ui.showOpenDialog = showOpenDialog;
      choose = () => undefined;
      await git(app, ['am', '--abort']).catch(() => undefined);
      await git(app, ['reset', '--hard', before]);
      await git(app, ['branch', '-D', 'clashing-patch']);
      fs.rmSync(file, { force: true });
      await vscode.commands.executeCommand('gitardo.refresh');
    }
  });

  await step('follows a renamed file in its history', async () => {
    await git(app, ['mv', 'pick-one.txt', 'renamed.txt']);
    await git(app, ['commit', '-m', 'Rename a file']);
    await vscode.commands.executeCommand('gitardo.fileHistory', vscode.Uri.file(path.join(app, 'renamed.txt')));
    assert.deepEqual(state().filter, { kind: 'path', path: 'renamed.txt', renamedFrom: ['pick-one.txt'] });
    assert.deepEqual(subjects(), ['Rename a file', 'Pick one']);
    await controller.history.setFilter(null);
  });

  await step('bisects from the graph and reports the first bad commit', async () => {
    const rows = state().rows;
    asked.length = 0;
    answer = () => undefined;
    await controller.action({ kind: 'bisect', mark: 'bad', hash: rows[0].hash });
    await controller.action({ kind: 'bisect', mark: 'good', hash: rows.at(-1)!.hash });
    assert.ok(state().bisect?.remaining);
    for (let guard = 0; state().bisect!.remaining! > 1 && guard < 10; guard++) {
      // "Bad" from the commit that added b.txt onwards.
      const bad = fs.existsSync(path.join(app, 'b.txt'));
      await controller.action({ kind: 'bisect', mark: bad ? 'bad' : 'good' });
    }
    const found = state().rows.find((row) => row.hash === state().bisect!.bad);
    assert.equal(found?.subject, 'Add b');
    assert.ok(asked.some((question) => /first bad commit is \w{7} "Add b"/.test(question.message)));
    await vscode.commands.executeCommand('gitardo.bisectReset');
    assert.equal(state().bisect, null);
    assert.equal(checkedOutBranch(state()), 'main');
  });

  await step('stops on a conflicting merge, says so first, opens the merge editor and aborts cleanly', async () => {
    write('b.txt', 'bee\nmain\n');
    await controller.action({ kind: 'stageAll' });
    await controller.action({ kind: 'commit', message: 'Conflicting change', amend: false });
    asked.length = 0;
    errors.length = 0;
    answer = run;
    await controller.action({ kind: 'merge', ref: 'feature' });
    assert.match(asked[0].detail, /stop with conflicts in 1 file:\nb\.txt/);
    assert.equal(state().operation, 'merge');
    assert.deepEqual(state().workingTree.conflicted.map((file) => file.path), ['b.txt']);
    assert.ok(asked.some((question) => /Merge feature into main stopped: 1 file has conflicts/.test(question.message)));

    await fromWebview({ type: 'openDiff', source: { kind: 'conflict' }, file: { path: state().workingTree.conflicted[0].path, originalPath: null, kind: 'modified' } });
    const tab = await until('the merge editor', () => vscode.window.tabGroups.activeTabGroup.activeTab);
    // The stable API has no type for a merge editor: its tab is recognised by not being a plain text editor on the file.
    assert.ok(!(tab.input instanceof vscode.TabInputText), `a merge editor is open, not a text editor (${tab.label})`);
    assert.match(tab.label, /^Merging: b\.txt/);
    const side = async (ref: string) => (await vscode.workspace.openTextDocument(contentUri(state().root, ref, 'b.txt'))).getText();
    assert.equal(await side(CONFLICT.base), 'bee\n');
    assert.equal(await side(CONFLICT.ours), 'bee\nmain\n');
    assert.equal(await side(CONFLICT.theirs), 'bee\nfeature\n');
    // The merge editor starts its own resolution in the result, which counts as an unsaved change.
    await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
    assert.equal(vscode.window.tabGroups.all.flatMap((group) => group.tabs).length, 0);

    await controller.action({ kind: 'abort' });
    assert.equal(state().operation, null);
    assert.equal(state().changes, 0);
  });

  await step('switches a file in conflict between the merge editor and the Flow Merge, which joins each side to the result as git wrote it', async () => {
    interface Panel {
      initSent: boolean;
      currentModel: { result: string; regions: { kind: string; unresolved: boolean; resultOurs: { start: number; count: number }; resultTheirs: { start: number; count: number } }[] } | undefined;
    }
    const file = path.join(app, 'b.txt');
    const marked = 'bee\n<<<<<<< HEAD\nmain\n=======\nfeature\n>>>>>>> feature\n';
    const document = () => vscode.workspace.textDocuments.find((each) => samePath(each.uri.fsPath, file));
    const mergeTabs = () => vscode.window.tabGroups.all.flatMap((group) => group.tabs).filter((tab) => /^Merging: b\.txt/.test(tab.label));
    const panels = () => flowDiff.merge.all() as unknown as Panel[];
    const openMergeEditor = async () => {
      await fromWebview({ type: 'openDiff', source: { kind: 'conflict' }, file: { path: state().workingTree.conflicted[0].path, originalPath: null, kind: 'modified' } });
      await until('the merge editor', () => mergeTabs().length === 1);
      // As it opens, the merge editor writes its own start of the result in the file, without saving it.
      await until('the merge editor to start the result', () => document()?.isDirty && document()?.getText() === 'bee\n');
    };
    answer = run;
    await controller.action({ kind: 'merge', ref: 'feature' });
    assert.equal(fs.readFileSync(file, 'utf8'), marked);
    try {
      await openMergeEditor();
      // Nothing chosen in it yet: it closes without a question, and its start of the result is undone.
      asked.length = 0;
      await vscode.commands.executeCommand('gitardo.openInFlowMerge');
      const [panel] = await until('the Flow Merge', () => (panels().length ? panels() : undefined));
      assert.equal(mergeTabs().length, 0, 'the Flow Merge takes the place of the merge editor');
      assert.equal(asked.length, 0);
      assert.equal(document()?.isDirty ?? false, false);
      assert.equal(fs.readFileSync(file, 'utf8'), marked);
      // The page says it is ready once its module ran: the Monaco bundle loads under the content security policy.
      await until('the Flow Merge page to load', () => panel.initSent);
      const model = panel.currentModel;
      assert.equal(model?.result, marked);
      assert.deepEqual(
        model?.regions.map((region) => [region.kind, region.unresolved, region.resultOurs, region.resultTheirs]),
        [['conflict', true, { start: 3, count: 1 }, { start: 5, count: 1 }]],
      );

      // What becomes of the file shows in the result.
      fs.writeFileSync(file, 'bee\nmain and feature\n');
      await until('the result to follow the file', () => panel.currentModel?.result === 'bee\nmain and feature\n');
      assert.deepEqual(
        panel.currentModel?.regions.map((region) => [region.kind, region.unresolved]),
        [['conflict', false]],
      );
      fs.writeFileSync(file, marked);
      await until('the conflict to show again', () => panel.currentModel?.regions[0]?.unresolved === true);

      // A side chosen in the page, as its band button sends it: in the file, not saved until Ctrl+S.
      const send = (message: { type: 'edit'; text: string } | { type: 'saveFile' }) => (panel as unknown as { onMessage(message: unknown): void }).onMessage(message);
      send({ type: 'edit', text: 'bee\nmain\n' });
      await until('the choice in the file', () => document()?.isDirty && document()?.getText() === 'bee\nmain\n');
      assert.equal(fs.readFileSync(file, 'utf8'), marked, 'a choice is not saved');
      await until('the model of the choice', () => panel.currentModel?.result === 'bee\nmain\n');
      assert.deepEqual(
        panel.currentModel?.regions.map((region) => [region.kind, region.unresolved, (region as { state?: string }).state]),
        [['conflict', false, 'ours']],
      );
      send({ type: 'saveFile' });
      await until('the file saved', () => !document()?.isDirty);
      assert.equal(fs.readFileSync(file, 'utf8'), 'bee\nmain\n');
      fs.writeFileSync(file, marked);
      await until('the conflict to show again', () => panel.currentModel?.regions[0]?.unresolved === true);

      // And back: the merge editor again, in place of the Flow Merge.
      await vscode.commands.executeCommand('gitardo.flowMerge.openInMergeEditor');
      await until('the merge editor again', () => mergeTabs().length === 1);
      assert.equal(panels().length, 0);
      await until('the merge editor to start the result', () => document()?.isDirty && document()?.getText() === 'bee\n');
      await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
      await until('the file as it is saved', () => !document()?.isDirty);
      assert.equal(fs.readFileSync(file, 'utf8'), marked);

      // What was typed in the merge editor is not lost without asking.
      await openMergeEditor();
      // Typed in an editor beside it: a workspace edit of a file open in no text editor would be saved at once.
      const beside = await vscode.window.showTextDocument(vscode.Uri.file(file), { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true, preview: false });
      await beside.edit((builder) => builder.insert(new vscode.Position(1, 0), 'chosen\n'));
      assert.equal(document()?.getText(), 'bee\nchosen\n');
      asked.length = 0;
      answer = () => undefined;
      await vscode.commands.executeCommand('gitardo.openInFlowMerge', vscode.Uri.file(file));
      assert.match(asked[0]?.message ?? '', /The merge editor has changes to b\.txt that are not saved/);
      assert.equal(mergeTabs().length, 1, 'the merge editor stays when the question is not answered');
      assert.equal(panels().length, 0);
      answer = (_message, _detail, items) => (items.includes('Discard and Open') ? 'Discard and Open' : undefined);
      await vscode.commands.executeCommand('gitardo.openInFlowMerge', vscode.Uri.file(file));
      await until('the Flow Merge', () => panels().length === 1);
      assert.equal(mergeTabs().length, 0);
      assert.equal(document()?.isDirty ?? false, false);
      assert.equal(fs.readFileSync(file, 'utf8'), marked);

      // The command works on a file in conflict open in a text editor too, and reveals the panel it has already.
      await vscode.window.showTextDocument(vscode.Uri.file(file));
      await vscode.commands.executeCommand('gitardo.openInFlowMerge');
      assert.equal(panels().length, 1);

      // Closed with a choice not saved: the file shows in an editor, for the change not to be forgotten.
      const [last] = panels();
      await until('the Flow Merge page to load', () => last.initSent);
      await vscode.commands.executeCommand('workbench.action.closeOtherEditors');
      (last as unknown as { onMessage(message: unknown): void }).onMessage({ type: 'edit', text: 'bee\nfeature\n' });
      await until('the choice in the file', () => document()?.isDirty && document()?.getText() === 'bee\nfeature\n');
      (last as unknown as { panel: vscode.WebviewPanel }).panel.dispose();
      const shown = await until('the file in an editor', () => vscode.window.visibleTextEditors.find((editor) => samePath(editor.document.uri.fsPath, file)));
      assert.ok(shown.document.isDirty);
      await vscode.window.showTextDocument(shown.document);
      await vscode.commands.executeCommand('workbench.action.files.revert');
      assert.equal(fs.readFileSync(file, 'utf8'), marked);

      // Mark as Resolved: with the markers still in, only after a question; then saved, staged, and the page closed.
      await vscode.commands.executeCommand('gitardo.openInFlowMerge', vscode.Uri.file(file));
      const [resolving] = await until('the Flow Merge', () => (panels().length ? panels() : undefined));
      await until('the Flow Merge page to load', () => resolving.initSent);
      const sendTo = (message: unknown) => (resolving as unknown as { onMessage(message: unknown): void }).onMessage(message);
      asked.length = 0;
      answer = () => undefined;
      sendTo({ type: 'markResolved' });
      await until('the question about the markers', () => asked.length > 0);
      assert.match(asked[0].message, /b\.txt still has a conflict marked by git\. Mark it as resolved all the same/);
      assert.equal(panels().length, 1, 'not resolved without an answer');
      assert.deepEqual(state().workingTree.conflicted.map((file) => file.path), ['b.txt']);
      // An edit and the button right after it, as the page sends them: the edit is in the file before it is saved.
      answer = run;
      sendTo({ type: 'edit', text: 'bee\nmain\nfeature\n' });
      sendTo({ type: 'markResolved' });
      await until('the page closed', () => panels().length === 0);
      assert.equal(fs.readFileSync(file, 'utf8'), 'bee\nmain\nfeature\n');
      assert.equal(document()?.isDirty ?? false, false);
      await until('the file staged', () => state().workingTree.conflicted.length === 0);
      assert.equal(state().operation, 'merge');
    } finally {
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');
      answer = run;
      await controller.action({ kind: 'abort' });
    }
    assert.equal(state().operation, null);
    assert.equal(state().changes, 0);
    assert.equal(panels().length, 0);
  });

  await step('offers nothing about resolving conflicts until its setting is on, then resolves the ones that need no choice after showing what it would write, leaves the others, and takes it back', async () => {
    const lines = (changes: Record<number, string> = {}) => Array.from({ length: 12 }, (_, index) => `${changes[index] ?? `line ${index}`}\n`).join('');
    const read = (file: string) => fs.readFileSync(path.join(app, file), 'utf8');
    write('c.txt', lines());
    write('d.txt', 'one\ntwo\n');
    await git(app, ['add', '--', 'c.txt', 'd.txt']);
    await git(app, ['commit', '-m', 'Add c and d']);
    await git(app, ['checkout', '-b', 'beside']);
    // In c.txt each side changes a line next to one the other changes, and both change line 9; in d.txt only the first.
    write('c.txt', lines({ 2: 'two, theirs', 9: 'nine, theirs' }));
    write('d.txt', 'one\nTWO\n');
    await git(app, ['commit', '-am', 'Beside, theirs']);
    await git(app, ['checkout', 'main']);
    write('c.txt', lines({ 3: 'three, ours', 9: 'nine, ours' }));
    write('d.txt', 'ONE\ntwo\n');
    await git(app, ['commit', '-am', 'Beside, ours']);
    await controller.refresh();

    // Experimental, and off until it is switched on: the conflicts are not looked into, nothing is offered and nothing runs.
    const autoResolve = (value: boolean | undefined) =>
      vscode.workspace.getConfiguration('gitardo').update('conflicts.autoResolve', value, vscode.ConfigurationTarget.Global);
    assert.equal(vscode.workspace.getConfiguration('gitardo').get('conflicts.autoResolve'), false);
    asked.length = 0;
    answer = run;
    await controller.action({ kind: 'merge', ref: 'beside' });
    assert.equal(state().operation, 'merge');
    assert.deepEqual(state().conflictFiles, []);
    const plain = asked.find((question) => /stopped: 2 files have conflicts/.test(question.message));
    assert.match(plain?.message ?? '', /Resolve them, then continue or abort the merge\.$/);
    assert.deepEqual(plain?.items, ['Show Log']);
    const untouched = read('c.txt');
    asked.length = 0;
    await vscode.commands.executeCommand('gitardo.resolveConflicts');
    assert.deepEqual(
      asked.map((question) => question.message),
      ['Gitardo: resolving conflicts automatically is experimental, and it is switched off in the settings.'],
    );
    assert.deepEqual(asked[0].items, ['Open Settings']);
    assert.equal(read('c.txt'), untouched);

    await autoResolve(true);
    try {
      // The views follow the setting by themselves.
      await until('the conflicts looked into', () => state().conflictFiles.length === 2);
      await controller.action({ kind: 'abort' });
      assert.equal(state().operation, null);

      asked.length = 0;
      answer = run;
      await controller.action({ kind: 'merge', ref: 'beside' });
      assert.equal(state().operation, 'merge');
      const both = [
        { path: 'c.txt', conflicts: 2, resolvable: ['separate'] },
        { path: 'd.txt', conflicts: 1, resolvable: ['separate'] },
      ];
      assert.deepEqual(state().conflictFiles, both);
      const stopped = asked.find((question) => /stopped: 2 files have conflicts/.test(question.message));
      assert.match(stopped?.message ?? '', /Gitardo can resolve 2 of the 3 conflicts automatically\.$/);
      assert.deepEqual(stopped?.items, ['Resolve Automatically', 'Show Log']);

      // It asks first, saying the files it writes and the command it runs; each resolution can be looked at in the review.
      const marked = read('c.txt');
      asked.length = 0;
      posted.length = 0;
      addressed.length = 0;
      answer = (_text, _detail, items) => (items.includes('Review') ? 'Review' : undefined);
      await vscode.commands.executeCommand('gitardo.resolveConflicts');
      assert.equal(asked[0].message, 'Resolve 2 of 3 conflicts automatically?');
      assert.deepEqual(asked[0].items, ['Resolve', 'Review']);
      // The engine says a file's one conflict as "the conflict", and what it writes and runs in a sentence.
      assert.match(asked[0].detail, /^c\.txt: 1 of 2 conflicts \(changes to lines next to each other\)\nd\.txt: (1 of 1 conflict|the conflict) \(/);
      assert.match(asked[0].detail, /1 conflict will be left to resolve by hand/);
      assert.match(
        asked[0].detail,
        /(Gitardo will write in [^\n]+:\n\nc\.txt\nd\.txt\n\nand then run:\n\ngit add -- d\.txt|Writes c\.txt, d\.txt, then runs git add -- d\.txt\.)$/,
      );
      const tab = await until('the review', () => vscode.window.tabGroups.activeTabGroup.activeTab);
      assert.equal(tab.label, 'Conflicts · app');
      const shown = await until('the review sent', () => lastReview());
      assert.deepEqual(
        shown.files.map((file) => [file.path, file.conflicts.map((conflict) => conflict.proposal?.source ?? null)]),
        [
          ['c.txt', ['rules', null]],
          ['d.txt', ['rules']],
        ],
      );
      assert.equal(shown.files[1].conflicts[0].proposal?.text, 'ONE\nTWO\n');
      // Nothing is written until a resolution is accepted there.
      assert.equal(read('c.txt'), marked);
      assert.deepEqual(state().conflictFiles, both);
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');

      asked.length = 0;
      logged.length = 0;
      answer = (_text, _detail, items) => (items.includes('Resolve') ? 'Resolve' : undefined);
      await fromWebview({ type: 'action', action: { kind: 'resolveConflicts' } });
      assert.deepEqual(commandsLogged(), ['> write c.txt', '> write d.txt', '> git add -- d.txt']);
      assert.equal(read('d.txt'), 'ONE\nTWO\n');
      const left = '<<<<<<< HEAD\nnine, ours\n=======\nnine, theirs\n>>>>>>> beside\n';
      assert.equal(read('c.txt'), lines({ 2: 'two, theirs', 3: 'three, ours', 9: 'LEFT' }).replace('LEFT\n', left));
      assert.deepEqual(state().workingTree.conflicted.map((file) => file.path), ['c.txt']);
      assert.deepEqual(state().conflictFiles, [{ path: 'c.txt', conflicts: 1, resolvable: [] }]);
      const done = asked.find((question) => question.message.startsWith('Gitardo resolved conflicts'));
      assert.equal(done?.message, 'Gitardo resolved conflicts in 2 files. 1 file still has conflicts to resolve by hand.');
      assert.deepEqual(done?.items, ['Undo']);

      // Undo puts every conflict back, in the file that was staged too.
      asked.length = 0;
      answer = run;
      await controller.action({ kind: 'undo' });
      assert.equal(asked[0].message, 'Undo: restore the conflicts of 2 files?');
      assert.match(asked[0].detail, /git checkout --merge -- c\.txt d\.txt$/);
      assert.deepEqual(state().conflictFiles, both);
      assert.deepEqual(state().workingTree.staged, []);

      // A whole file for one side, which can be taken back the same way.
      asked.length = 0;
      await controller.action({ kind: 'takeSide', path: 'd.txt', side: 'theirs' });
      assert.equal(asked[0].message, 'Take the incoming side of d.txt?');
      assert.match(asked[0].detail, /The incoming side is the branch being merged\./);
      assert.match(asked[0].detail, /git checkout --theirs -- d\.txt\ngit add -- d\.txt$/);
      assert.equal(read('d.txt'), 'one\nTWO\n');
      assert.deepEqual(state().workingTree.conflicted.map((file) => file.path), ['c.txt']);
      await controller.action({ kind: 'undo' });
      assert.deepEqual(state().conflictFiles, both);

      // One file only, as from its row in the Changes view; with nothing left in conflict the merge is offered to continue.
      write('c.txt', lines({ 2: 'two, theirs', 3: 'three, ours', 9: 'nine, both' }));
      await controller.action({ kind: 'stage', paths: ['c.txt'] });
      asked.length = 0;
      answer = (_text, _detail, items) => (items.includes('Resolve') ? 'Resolve' : undefined);
      await controller.action({ kind: 'resolveConflicts', paths: ['d.txt'] });
      assert.equal(asked[0].message, 'Resolve the conflict automatically?');
      const whole = asked.find((question) => question.message.startsWith('Gitardo resolved conflicts'));
      assert.equal(whole?.message, 'Gitardo resolved conflicts in 1 file. No file is left in conflict: the merge can continue.');
      assert.deepEqual(whole?.items, ['Continue', 'Undo']);
      assert.deepEqual(state().workingTree.conflicted, []);

      // With nothing to resolve it says so and asks nothing: as an error without the engine, as its refusal with it.
      asked.length = 0;
      const before = errors.length;
      await controller.action({ kind: 'resolveConflicts' });
      const said = [...errors.slice(before), ...asked.map((question) => question.message)];
      assert.ok(
        said.some((text) => /There are no conflicts marked in the files to resolve\./.test(text)),
        JSON.stringify(said),
      );
      errors.length = before;
      assert.deepEqual(
        asked.filter((question) => question.items.length),
        [],
      );

      answer = run;
      await controller.action({ kind: 'continue' });
      assert.equal(state().operation, null);
      assert.equal(state().rows[0].parents.length, 2);
      assert.equal(read('d.txt'), 'ONE\nTWO\n');
    } finally {
      await autoResolve(undefined);
    }
  });

  await step('shows what the rules and the AI propose for each conflict, writes only what is accepted, and takes it back', async () => {
    const lines = (changes: Record<number, string> = {}) => Array.from({ length: 12 }, (_, index) => `${changes[index] ?? `line ${index}`}\n`).join('');
    const read = (file: string) => fs.readFileSync(path.join(app, file), 'utf8');
    write('e.txt', lines());
    await git(app, ['add', '--', 'e.txt']);
    await git(app, ['commit', '-m', 'Add e']);
    await git(app, ['checkout', '-b', 'aside']);
    write('e.txt', lines({ 2: 'two, theirs', 9: 'nine, theirs' }));
    await git(app, ['commit', '-am', 'Aside, theirs']);
    await git(app, ['checkout', 'main']);
    write('e.txt', lines({ 3: 'three, ours', 9: 'nine, ours' }));
    await git(app, ['commit', '-am', 'Aside, ours']);

    // No language model can be signed in to here: one that answers as a model would takes its place.
    const lm = vscode.lm as unknown as Record<string, unknown>;
    const select = lm.selectChatModels;
    const prompts: string[] = [];
    const reply = ['<resolution>\n', 'nine, ours', ' and theirs\n', '</resolution>\n<reason>Both sides renamed line nine.</reason>\n', '<confidence>high</confidence>'];
    // Two models: the one chosen in the settings is asked, with how much it thinks.
    const requests: { configuration?: Record<string, unknown> }[] = [];
    const first = {
      id: 'test-first',
      name: 'First model',
      vendor: 'test',
      family: 'first',
      sendRequest: async () => {
        throw new Error('The first model was asked.');
      },
    };
    const model = {
      id: 'test-chosen',
      name: 'Chosen model',
      vendor: 'test',
      family: 'chosen',
      sendRequest: async (messages: vscode.LanguageModelChatMessage[], options: { configuration?: Record<string, unknown> }) => {
        requests.push(options);
        prompts.push(messages.map((each) => each.content.map((part) => (part instanceof vscode.LanguageModelTextPart ? part.value : '')).join('')).join('\n'));
        return {
          text: (async function* () {
            for (const chunk of reply) {
              await sleep(150);
              yield chunk;
            }
          })(),
        };
      },
    };
    let models: unknown[] = [first, model];
    lm.selectChatModels = async () => models;
    const settings = () => vscode.workspace.getConfiguration('gitardo');
    const set = (key: string, value: boolean | string | undefined) => settings().update(key, value, vscode.ConfigurationTarget.Global);
    await set('conflicts.autoResolve', true);
    await set('conflicts.ai', true);
    try {
      await until('the setting of the AI in the state', () => state().resolveWithAI);

      // The model is chosen among those VS Code has, then how much it thinks.
      const offered: string[][] = [];
      choose = (items, options) => {
        offered.push(items.map((item) => item.label));
        return options.title === 'The model that resolves conflicts' ? byLabel('Chosen model')(items) : byLabel('High')(items);
      };
      await vscode.commands.executeCommand('gitardo.chooseConflictModel');
      // Area 6 of stage 4: the engine's `ai.choose-model` names the choices as the desktop does (G3).
      assert.deepEqual(
        offered,
        internals.engine?.running
          ? [
              ['The first model', 'First model', 'Chosen model'],
              ['As the model is set up', 'Low', 'Medium', 'High', 'Extra high', 'Max'],
            ]
          : [
              ['$(check) The first model of VS Code', 'First model', 'Chosen model'],
              ['$(check) As the model is set up in VS Code', 'Low', 'Medium', 'High', 'Extra high', 'Max'],
            ],
      );
      assert.equal(settings().get('conflicts.aiModel'), 'test-chosen');
      assert.equal(settings().get('conflicts.aiEffort'), 'high');
      // With every plan shown first, the picks are all that is asked: nothing runs that was not chosen in them (G6).
      if (internals.engine?.running) {
        await set('commandPreview', 'all');
        try {
          asked.length = 0;
          await vscode.commands.executeCommand('gitardo.chooseConflictModel');
          assert.ok(!asked.some((question) => /^Resolve conflicts with/.test(question.message)), JSON.stringify(asked));
          assert.equal(settings().get('conflicts.aiModel'), 'test-chosen');
        } finally {
          await set('commandPreview', undefined);
        }
      }
      asked.length = 0;
      answer = run;
      await controller.action({ kind: 'merge', ref: 'aside' });
      const stopped = asked.find((question) => /stopped: 1 file has conflicts/.test(question.message));
      assert.deepEqual(stopped?.items, ['Resolve Automatically', 'Resolve with AI', 'Show Log']);
      const marked = read('e.txt');

      // The rules propose for the first conflict; the model is asked about the second, and its answer shows as it comes.
      posted.length = 0;
      addressed.length = 0;
      await fromWebview({ type: 'reviewConflicts', ai: true });
      const answered = await until('the answer of the model', () => {
        const review = lastReview();
        return review?.files[0]?.conflicts[1]?.proposal?.source === 'ai' && review;
      });
      const thinking = posted.flatMap((message) => (message.type === 'review' && message.review ? message.review.files[0].conflicts[1].asking ?? [] : []));
      assert.ok(thinking.some((asking) => asking.state === 'waiting' || asking.state === 'thinking'));
      assert.ok(thinking.some((asking) => asking.state === 'thinking' && asking.text === 'nine, ours'));
      assert.equal(prompts.length, 1);
      assert.match(prompts[0], /^Resolve one merge conflict in the file e\.txt\./);
      assert.match(prompts[0], /"Incoming" is the branch being merged, in the commit "Aside, theirs"\./);
      assert.match(prompts[0], /<current>\nnine, ours\n<\/current>\n<incoming>\nnine, theirs\n<\/incoming>/);
      const [rules, ai] = answered.files[0].conflicts;
      assert.equal(rules.proposal?.source, 'rules');
      assert.deepEqual(ai.proposal, {
        source: 'ai',
        from: 14,
        to: 19,
        text: 'nine, ours and theirs\n',
        reason: 'Both sides renamed line nine.',
        confidence: 'high',
        newLines: 1,
        model: 'Chosen model',
      });
      assert.equal(answered.model, 'Chosen model · high effort');
      assert.deepEqual(requests[0].configuration, { reasoningEffort: 'high' });
      assert.equal(answered.asking, false);
      assert.equal(read('e.txt'), marked);

      // The same answer shows in the Flow Merge, beside the proposal of the rules.
      interface Panel {
        initSent: boolean;
        currentModel: { proposals?: { proposal: { source: string } }[] } | undefined;
      }
      const panels = () => flowDiff.merge.all() as unknown as Panel[];
      await vscode.commands.executeCommand('gitardo.openInFlowMerge', vscode.Uri.file(path.join(app, 'e.txt')));
      const [panel] = await until('the Flow Merge', () => (panels().length ? panels() : undefined));
      assert.deepEqual(
        panel.currentModel?.proposals?.map((each) => each.proposal.source),
        ['rules', 'ai'],
      );
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');

      // Only what is accepted is written: the file left with no conflict is staged, and the review says what it did.
      await fromWebview({ type: 'reviewConflicts' });
      const tab = view('conflicts');
      logged.length = 0;
      posted.length = 0;
      addressed.length = 0;
      await internals.onMessage(tab, { type: 'writeResolutions', accepted: [{ path: 'e.txt', key: rules.key }, { path: 'e.txt', key: ai.key }] });
      assert.deepEqual(commandsLogged(), ['> write e.txt', '> git add -- e.txt']);
      assert.equal(read('e.txt'), lines({ 2: 'two, theirs', 3: 'three, ours', 9: 'nine, ours and theirs' }));
      assert.deepEqual(state().workingTree.conflicted, []);
      const after = await until('the report', () => lastReview()?.report && lastReview());
      assert.deepEqual(after.report, { resolved: 2, files: 1, staged: 1, fromAI: 1 });
      assert.equal(after.conflicted, 0);

      // Undo puts the conflicts back, with the sides named as git names them then.
      answer = run;
      await controller.action({ kind: 'undo' });
      const unnamed = (text: string) => text.replace(/^(<{7}|>{7}) .*$/gm, '$1');
      assert.equal(unnamed(read('e.txt')), unnamed(marked));
      assert.equal(state().workingTree.conflicted.length, 1);

      // Without the model chosen, or without any, asking says why it could not.
      models = [first];
      posted.length = 0;
      addressed.length = 0;
      await internals.onMessage(tab, { type: 'askAI', conflicts: [{ path: 'e.txt', key: ai.key }] });
      const gone = await until('the failure without the model chosen', () => {
        const asking = lastReview()?.files[0]?.conflicts[1]?.asking;
        return asking?.state === 'failed' && asking;
      });
      assert.match(gone.state === 'failed' ? gone.error : '', /The model chosen to resolve conflicts, test-chosen, is not available in VS Code/);
      models = [];
      posted.length = 0;
      addressed.length = 0;
      await internals.onMessage(tab, { type: 'askAI', conflicts: [{ path: 'e.txt', key: ai.key }] });
      const failed = await until('the failure', () => {
        const asking = lastReview()?.files[0]?.conflicts[1]?.asking;
        return asking?.state === 'failed' && asking;
      });
      assert.match(failed.state === 'failed' ? failed.error : '', /No language model is available in VS Code/);
    } finally {
      lm.selectChatModels = select;
      choose = () => undefined;
      await set('conflicts.aiModel', undefined);
      await set('conflicts.aiEffort', undefined);
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');
      await set('conflicts.autoResolve', undefined);
      await set('conflicts.ai', undefined);
      answer = run;
      if (state().operation) await controller.action({ kind: 'abort' });
    }
    assert.equal(state().operation, null);
    assert.deepEqual(
      vscode.workspace.textDocuments.filter((document) => document.isDirty).map((document) => document.uri.toString()),
      [],
    );
  });

  await step('names who changed each line of a file, what is typed as not committed, and stops when asked', async () => {
    const before = errors.length;
    const uri = vscode.Uri.file(path.join(app, 'a.txt'));
    const editor = await vscode.window.showTextDocument(uri);
    const count = editor.document.getText().split('\n').filter(Boolean).length;
    assert.equal(fileBlame.linesOf(uri), undefined);
    await vscode.commands.executeCommand('gitardo.toggleFileBlame');
    const lines = await until('the blame', () => fileBlame.linesOf(uri));
    assert.equal(lines.length, count);
    assert.ok(lines.every((line) => line?.committed && line.author === 'Gitardo Test' && line.summary), JSON.stringify(lines));

    // A line that is typed is not committed, at once and when git has been asked.
    await editor.edit((builder) => builder.insert(new vscode.Position(0, 0), 'typed\n'));
    const typed = () => fileBlame.linesOf(uri)?.map((line) => line?.committed);
    const after = [false, ...Array<boolean>(count).fill(true)].join();
    // Whole lines leave the ones after them as they were, with no need to ask.
    assert.equal(typed()?.join(), after);
    await until('the blame of what is not saved', () => typed()?.join() === after);
    await sleep(1000);
    assert.equal(typed()?.join(), after);

    // The button of the editor names the file.
    await vscode.commands.executeCommand('gitardo.hideFileBlame', uri);
    assert.equal(fileBlame.linesOf(uri), undefined);
    await vscode.commands.executeCommand('workbench.action.files.revert');
    // A file that is in no commit has no one to name, and is told so rather than shown empty.
    write('fresh.txt', 'x\n');
    try {
      const fresh = vscode.Uri.file(path.join(app, 'fresh.txt'));
      await vscode.window.showTextDocument(fresh);
      await vscode.commands.executeCommand('gitardo.toggleFileBlame');
      assert.equal(fileBlame.linesOf(fresh), undefined);
    } finally {
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');
      fs.rmSync(path.join(app, 'fresh.txt'));
    }
    assert.equal(errors.length, before);
  });

  await step('opens the commit of a line and the file as it was before it, blames every file when asked, and re-reads when whitespace is ignored', async () => {
    const before = errors.length;
    const setting = (name: string) => vscode.workspace.getConfiguration('gitardo').get(name);
    const tabs = () => vscode.window.tabGroups.all.flatMap((group) => group.tabs);
    const uri = vscode.Uri.file(path.join(app, 'a.txt'));
    const show = async (index: number) => {
      const editor = await vscode.window.showTextDocument(uri);
      editor.selection = new vscode.Selection(index, 0, index, 0);
    };
    await show(0);
    await vscode.commands.executeCommand('gitardo.showFileBlame', uri);
    const lines = await until('the blame', () => fileBlame.linesOf(uri));
    const index = lines.findIndex((line) => line?.previous);
    const changed = lines[index];
    assert.ok(changed?.previous, 'a line changed by a commit that came after the one that added the file');

    // The commit of the line, in a tab of its own.
    await show(index);
    await vscode.commands.executeCommand('gitardo.openLineCommit');
    await until('the tab of the commit', () =>
      tabs().some((tab) => tab.input instanceof vscode.TabInputWebview && tab.label.startsWith(changed.hash.slice(0, 7))),
    );

    // The file as it was before that commit, named after the commit it is of, with the blame it had then.
    await show(index);
    await vscode.commands.executeCommand('gitardo.blamePrevious');
    const past = await until('the file as it was', () => {
      const active = vscode.window.activeTextEditor;
      return active?.document.uri.scheme === 'gitardo' ? active.document : undefined;
    });
    assert.ok(past.uri.path.endsWith(`a (${changed.previous.hash.slice(0, 7)}).txt`), past.uri.path);
    assert.equal(past.getText(), (await git(app, ['show', `${changed.previous.hash}:a.txt`])).stdout);
    const earlier = await until('its blame', () => fileBlame.linesOf(past.uri));
    assert.ok(earlier.length > 0 && earlier.every((line) => line?.committed && line.hash !== changed.hash), JSON.stringify(earlier));

    // Every file that comes in sight, but for the ones it is hidden in.
    const name = (await git(app, ['ls-files'])).stdout.split('\n').find((tracked) => tracked && tracked !== 'a.txt');
    assert.ok(name, 'another file in a commit');
    const other = vscode.Uri.file(path.join(app, name));
    await vscode.commands.executeCommand('gitardo.toggleBlameEveryFile');
    try {
      assert.equal(setting('fileBlame.everyFile'), true);
      await vscode.window.showTextDocument(other);
      await until('the blame of a file it was not asked for', () => fileBlame.linesOf(other));
      await vscode.commands.executeCommand('gitardo.hideFileBlame', other);
      await show(index);
      await vscode.window.showTextDocument(other);
      await sleep(800);
      assert.equal(fileBlame.linesOf(other), undefined);
      // Shown again when asked, as a file that only has it because every file does.
      await vscode.commands.executeCommand('gitardo.showFileBlame', other);
      await until('the blame it was asked for', () => fileBlame.linesOf(other));
    } finally {
      await vscode.commands.executeCommand('gitardo.toggleBlameEveryFile');
    }
    assert.equal(setting('fileBlame.everyFile'), false);
    // The ones that were asked for keep their blame, which a slow runner takes a while to read again.
    await until('the blame of the files it was asked for', () => fileBlame.linesOf(uri) && fileBlame.linesOf(other));

    // Whitespace: git is asked again, and the file still has its blame.
    await vscode.commands.executeCommand('gitardo.toggleBlameWhitespace');
    try {
      assert.equal(setting('blame.ignoreWhitespaceAndMoves'), true);
      await sleep(1500);
      assert.equal(fileBlame.linesOf(uri)?.length, lines.length);
    } finally {
      await vscode.commands.executeCommand('gitardo.toggleBlameWhitespace');
    }
    assert.equal(setting('blame.ignoreWhitespaceAndMoves'), false);
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
    assert.equal(errors.length, before);
  });

  await step('blames the line under the cursor without errors, and can be switched off', async () => {
    const before = errors.length;
    const editor = await vscode.window.showTextDocument(vscode.Uri.file(path.join(app, 'a.txt')));
    editor.selection = new vscode.Selection(1, 0, 1, 0);
    // The annotation is a decoration, which the API gives no way to read back: this only shows that making it does not fail.
    await sleep(1500);
    await vscode.commands.executeCommand('gitardo.toggleBlame');
    assert.equal(vscode.workspace.getConfiguration('gitardo').get('blame.enabled'), false);
    await vscode.commands.executeCommand('gitardo.toggleBlame');
    assert.equal(vscode.workspace.getConfiguration('gitardo').get('blame.enabled'), true);
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
    assert.equal(errors.length, before);
  });

  await step('asks the blame of the line under the cursor once for each line and version of the text, not as the cursor moves along it', async () => {
    const before = errors.length;
    const client = internals.engine?.running as EngineClient | undefined;
    if (!client) return;
    // The engine the extension asks is this one: what it is asked is counted on it.
    const asked: { line: number; contents: unknown }[] = [];
    const blameLine = client.blameLine;
    client.blameLine = (repository, file, line, contents, signal) => {
      asked.push({ line, contents });
      return blameLine.call(client, repository, file, line, contents, signal);
    };
    try {
      const editor = await vscode.window.showTextDocument(vscode.Uri.file(path.join(app, 'a.txt')));
      assert.ok(editor.document.lineCount > 2, 'a file of a few lines');
      const along = async (line: number) => {
        for (let character = 0; character <= Math.min(editor.document.lineAt(line).text.length, 6); character++) {
          editor.selection = new vscode.Selection(line, character, line, character);
          await sleep(300);
        }
        await sleep(600);
      };
      // Typed, the line is a new version of the text: asked once, with the text, since it is not saved.
      await editor.edit((builder) => builder.insert(new vscode.Position(1, 0), 'x'));
      editor.selection = new vscode.Selection(1, 0, 1, 0);
      await until('the blame of the line typed', () => asked.some((each) => each.line === 2 && each.contents));
      await sleep(600);
      const once = asked.length;
      await along(1);
      assert.equal(asked.length, once, 'no blame asked while the cursor moves along the line');
      // Another line is asked, and the first one, come back to in the same text, is not.
      await along(2);
      assert.equal(asked.length, once + 1, JSON.stringify(asked.slice(once)));
      await along(1);
      assert.equal(asked.length, once + 1, JSON.stringify(asked.slice(once)));
    } finally {
      client.blameLine = blameLine;
      await vscode.commands.executeCommand('workbench.action.files.revert');
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');
    }
    assert.equal(errors.length, before);
  });

  await step('sends the commits to the graph once, not again each time a file changes', async () => {
    const graph = view('graph');
    const sent: Extract<HostMessage, { type: 'state' }>[] = [];
    const postMessage = graph.postMessage.bind(graph);
    graph.postMessage = (outgoing: HostMessage) => {
      if (outgoing.type === 'state') sent.push(outgoing);
      return postMessage(outgoing);
    };
    const latest = (what: string, wanted: (sent: SentState) => boolean) =>
      until(what, () => [...sent].reverse().find((outgoing) => outgoing.state && wanted(outgoing.state)));
    try {
      const commits = state().rows.length;
      write('first.txt', '1\n');
      let outgoing = await latest('the state with one change', (told) => told.changes === 1);
      // The row of the uncommitted changes is new, and moves the lines of the graph: every commit is sent.
      assert.equal(outgoing.kept, undefined);
      assert.equal(outgoing.state!.rows?.length, commits);
      assert.equal(outgoing.state!.draft?.subject, 'Uncommitted changes (1)');

      write('second.txt', '2\n');
      outgoing = await latest('the state with two changes', (told) => told.changes === 2);
      // Only the row of the uncommitted changes says another count: the commits are not sent, nor laid out again.
      assert.ok(!outgoing.parts?.includes('rows'));
      assert.ok(outgoing.state!.rows === undefined, 'no commits sent');
      assert.equal(outgoing.state!.draft?.subject, 'Uncommitted changes (2)');
      // What the controller itself knows is whole.
      assert.equal(state().rows.length, commits + 1);

      await git(app, ['add', '.']);
      await git(app, ['commit', '-m', 'Two files']);
      outgoing = await latest('the state with the new commit', (told) => told.changes === 0);
      assert.equal(outgoing.kept, undefined);
      const committed = outgoing.state!;
      assert.equal(committed.rows?.[0].subject, 'Two files');
      assert.equal(committed.rows?.length, commits + 1);
      assert.ok(committed.draft === null, 'no row of the uncommitted changes');

      // Refresh reads every commit again as git prints them (D54): the same commits are not sent again.
      const all = state().rows.map((row) => row.hash);
      const from = sent.length;
      await vscode.commands.executeCommand('gitardo.refresh');
      await until('the read of Refresh', () => !internals.loading);
      assert.deepEqual(
        state().rows.map((row) => row.hash),
        all,
      );
      assert.equal(state().hasMore, false);
      assert.ok(!sent.slice(from).some((told) => told.parts?.includes('rows')), 'no commits sent again');
    } finally {
      graph.postMessage = postMessage;
    }
  });

  await step('tells the Changes view which files are not saved in an editor, and saves them', async () => {
    const exclude = path.join(app, '.git', 'info', 'exclude');
    const excluded = fs.readFileSync(exclude, 'utf8');
    fs.writeFileSync(exclude, `${excluded}\n*.log\n`);
    write('notes.log', 'ignored\n');
    const from = posted.length;
    const changes = view('changes');
    const told = (what: string, paths: string[]) =>
      until(what, () =>
        posted
          .slice(from)
          .some(
            (message, index) => message.type === 'unsaved' && JSON.stringify(message.paths) === JSON.stringify(paths) && addressed[from + index] === changes,
          ),
      );
    const edit = async (file: string) => {
      const editor = await vscode.window.showTextDocument(vscode.Uri.file(path.join(app, file)));
      await editor.edit((builder) => builder.insert(new vscode.Position(0, 0), 'edited\n'));
    };
    try {
      // A file git sees no change in, one that git ignores, and another.
      await edit('first.txt');
      await told('first.txt not saved', ['first.txt']);
      await edit('notes.log');
      await edit('second.txt');
      await told('both not saved, and not the ignored one', ['first.txt', 'second.txt']);

      await fromWebview({ type: 'save', paths: ['first.txt'] });
      await told('first.txt saved', ['second.txt']);
      assert.equal(fs.readFileSync(path.join(app, 'first.txt'), 'utf8'), 'edited\n1\n');
      await until('first.txt changed on disk', () => state().workingTree.unstaged.some((file) => file.path === 'first.txt'));

      for (const file of ['second.txt', 'notes.log']) {
        await vscode.window.showTextDocument(vscode.Uri.file(path.join(app, file)));
        await vscode.commands.executeCommand('workbench.action.files.revert');
      }
      await told('nothing left unsaved', []);
    } finally {
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');
      await git(app, ['checkout', '--', 'first.txt']);
      fs.rmSync(path.join(app, 'notes.log'), { force: true });
      fs.writeFileSync(exclude, excluded);
      // The reads that the files put back ask for, also once VS Code's Git extension has looked at them, happen here and
      // not in the next step: it goes on when the views have been told nothing for two seconds.
      for (let seen = -1; seen !== posted.length; ) {
        seen = posted.length;
        await sleep(2000);
      }
    }
  });

  await step('tells the views the graph is loading while a filter, a scope or the next page is read', async () => {
    const graph = view('graph');
    /** What the views were told from then on: the loading, and which filter each state was for. */
    const told = (from: number) =>
      posted.slice(from).flatMap((message) => {
        if (message.type === 'loading') return [`loading ${message.what}`];
        if (message.type === 'state') return [`state ${message.state?.filter?.kind ?? 'none'}`];
        return [];
      });
    const sequence = async (message: WebviewMessage) => {
      const from = posted.length;
      await internals.onMessage(graph, message);
      return told(from);
    };
    assert.deepEqual(await sequence({ type: 'setFilter', filter: { kind: 'path', path: 'b.txt' } }), ['loading filter', 'state path', 'loading null']);
    // The history of the file is read to its end: the list asks for no page past it, and the views wait for nothing (Q781).
    assert.deepEqual(await sequence({ type: 'loadMore' }), []);
    assert.deepEqual(await sequence({ type: 'setFilter', filter: null }), ['loading filter', 'state none', 'loading null']);
    assert.deepEqual(await sequence({ type: 'setScope', scope: { kind: 'kinds', kinds: ['branch'] } }), ['loading filter', 'state none', 'loading null']);
    await controller.history.setScope(null);

    // Another repository: the views wait for its commits as for those of a filter.
    const home = state().root;
    const other = state().repos.find((repo) => repo.root !== home);
    assert.ok(other, 'the workspace has another repository');
    const switched = await sequence({ type: 'selectRepo', root: other.root });
    assert.deepEqual(switched.filter((line) => line.startsWith('loading')), ['loading filter', 'loading null']);
    assert.ok(switched.indexOf('state none') < switched.indexOf('loading null'), switched.join(', '));
    assert.ok(switched.includes('state none'), switched.join(', '));
    await controller.repositories.selectRepo(home);

    // Reads that overlap are one wait, over when the last one is.
    const from = posted.length;
    await Promise.all([
      internals.onMessage(graph, { type: 'setFilter', filter: { kind: 'path', path: 'b.txt' } }),
      internals.onMessage(graph, { type: 'setFilter', filter: null }),
    ]);
    const loading = told(from).filter((line) => line.startsWith('loading'));
    assert.deepEqual(loading, ['loading filter', 'loading null']);

    // In a repository that keeps changing, as a large one does while it builds, the reads follow one another: the wait
    // is over with the first that started after it, not with the last of them.
    let changing = true;
    const stop = setTimeout(() => (changing = false), 10_000);
    const changes = (async () => {
      while (changing) {
        void controller.refresh();
        await sleep(5);
      }
    })();
    try {
      const searched = await sequence({ type: 'setFilter', filter: { kind: 'path', path: 'b.txt' } });
      assert.ok(changing, 'the wait went on for as long as the repository changed');
      const over = searched.indexOf('loading null');
      assert.ok(over > searched.indexOf('state path') && searched.indexOf('state path') >= 0, searched.join(', '));
      assert.ok(
        controller.activity.running.some((task) => task.detail?.startsWith('Read ')),
        'the status bar says the repository is read again and again',
      );
    } finally {
      changing = false;
      clearTimeout(stop);
      await changes;
      await controller.history.setFilter(null);
    }
    await until('the status bar shows nothing running', () => !controller.activity.running.length);
  });

  await step('reads nothing again for what git writes for itself, such as the files of fsmonitor in a worktree', async () => {
    const home = path.join(workspace, 'app.worktrees').replace(/\\/g, '/');
    const dir = `${home}/monitored`;
    /** What each read that was asked for since the start was asked for. */
    const causes: string[] = [];
    const schedule = internals.scheduleRefresh;
    internals.scheduleRefresh = (cause, delay) => {
      causes.push(cause);
      schedule.call(controller, cause, delay);
    };
    await git(app, ['config', 'core.fsmonitor', 'true']);
    await git(app, ['worktree', 'add', '-b', 'monitored', dir, 'main']);
    try {
      // Each read runs git status in the other worktree, which has the daemon of fsmonitor leave a file in its folder
      // of git: that is no change, and does not ask for another read, which would leave one again.
      await vscode.commands.executeCommand('gitardo.refresh');
      await vscode.commands.executeCommand('gitardo.refresh');
      const cookies = path.join(app, '.git', 'worktrees', 'monitored', 'fsmonitor--daemon');
      assert.ok(fs.existsSync(cookies), 'git status in the worktree started the daemon of fsmonitor');
      // Reads asked for by what VS Code's Git extension does in the new worktree are over in a while; these never were.
      const end = Date.now() + 20_000;
      for (let before = -1; before !== causes.length; await sleep(2000)) {
        assert.ok(Date.now() < end, `the reads went on: ${causes.slice(-3).join(', ')}`);
        before = causes.length;
      }
      assert.deepEqual(
        causes.filter((cause) => /fsmonitor--daemon|\.lock /.test(cause)),
        [],
      );
    } finally {
      internals.scheduleRefresh = schedule;
      await git(app, ['config', '--unset', 'core.fsmonitor']).catch(() => undefined);
      for (const cwd of [app, dir]) await git(cwd, ['fsmonitor--daemon', 'stop']).catch(() => undefined);
      await git(app, ['worktree', 'remove', '--force', dir]).catch(() => undefined);
      fs.rmSync(home, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
      await git(app, ['worktree', 'prune']);
      await git(app, ['branch', '-D', 'monitored']).catch(() => undefined);
      await vscode.commands.executeCommand('gitardo.refresh');
    }
  });

  await step('updates the submodules after a checkout, cloning from a folder on disk once allowed, unless the settings say not to', async () => {
    const home = checkedOutBranch(state())!;
    const config = vscode.workspace.getConfiguration('gitardo');
    const submodule = () => state().submodules.find((each) => each.folder === 'vendor/lib');
    // Two branches that record two commits of lib, made in a terminal without cloning it: here there is no submodule yet.
    const first = (await git(lib, ['rev-parse', 'HEAD'])).stdout.trim();
    fs.writeFileSync(path.join(lib, 'two.txt'), 'two\n');
    await git(lib, ['add', 'two.txt']);
    await git(lib, ['commit', '-m', 'Lib two']);
    const second = (await git(lib, ['rev-parse', 'HEAD'])).stdout.trim();
    await git(app, ['checkout', '-b', 'with-lib']);
    fs.writeFileSync(path.join(app, '.gitmodules'), `[submodule "vendor/lib"]\n\tpath = vendor/lib\n\turl = ${lib.replace(/\\/g, '/')}\n`);
    await git(app, ['add', '.gitmodules']);
    await git(app, ['update-index', '--add', '--cacheinfo', `160000,${first},vendor/lib`]);
    await git(app, ['commit', '-m', 'Add lib']);
    await git(app, ['checkout', '-b', 'with-lib-2']);
    await git(app, ['update-index', '--cacheinfo', `160000,${second},vendor/lib`]);
    await git(app, ['commit', '-m', 'Move lib']);
    await git(app, ['checkout', home]);
    await controller.refresh();
    try {
      // Git leaves the submodule out; the update clones it and checks out what the branch records, but git refuses to
      // clone from a folder on disk: that is asked.
      asked.length = 0;
      answer = run;
      logged.length = 0;
      const before = errors.length;
      await controller.action({ kind: 'checkout', target: 'branch', ref: 'with-lib' });
      // The refusal is logged, as every failure of git is.
      assert.ok(errors.slice(before).every((text) => text.includes("transport 'file' not allowed")), errors.slice(before).join('\n'));
      errors.length = before;
      assert.equal(checkedOutBranch(state()), 'with-lib');
      // The engine offers the rest of the checkout again under its own title, the extension the update alone (stage 4,
      // area 2).
      assert.equal(asked.length, 1, JSON.stringify(asked));
      assert.match(asked[0].message, /^(Update the submodules|Check out with-lib) from a folder on disk\?$/);
      assert.ok(logged.includes('> git checkout with-lib'), logged.join('\n'));
      assert.ok(logged.includes('> git -c protocol.file.allow=always submodule update --init --recursive'), logged.join('\n'));
      assert.deepEqual([submodule()?.checkout?.commit, submodule()?.checkout?.distance?.ahead, submodule()?.checkout?.distance?.behind], [first, 0, 0]);

      // Switched off, the submodule stays on what it had, one behind what the other branch records.
      await config.update('submodules.updateOnCheckout', false, vscode.ConfigurationTarget.Global);
      logged.length = 0;
      await controller.action({ kind: 'checkout', target: 'branch', ref: 'with-lib-2' });
      assert.ok(!logged.some((line) => line.includes('submodule update')), logged.join('\n'));
      assert.deepEqual([submodule()?.recorded, submodule()?.checkout?.commit, submodule()?.checkout?.distance?.behind], [second, first, 1]);
    } finally {
      answer = () => undefined;
      await config.update('submodules.updateOnCheckout', undefined, vscode.ConfigurationTarget.Global);
      await git(app, ['submodule', 'deinit', '--force', '--all']);
      await git(app, ['checkout', home]);
      fs.rmSync(path.join(app, 'vendor'), { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
      // VS Code's own git extension keeps the submodule open, and on Windows its clone with it: what is left of it stays
      // with the workspace, which is thrown away.
      try {
        fs.rmSync(path.join(app, '.git', 'modules'), { recursive: true, force: true });
      } catch {}
      await git(app, ['branch', '-D', 'with-lib', 'with-lib-2']);
      await git(lib, ['reset', '--hard', first]);
      await controller.refresh();
    }
    assert.deepEqual(state().submodules, []);
  });

  await step('changes the message of the last commit, of an older one after asking, and warns when it is pushed', async () => {
    const home = checkedOutBranch(state())!;
    const message = async (ref: string) => (await git(app, ['log', '-1', '--format=%B', ref])).stdout.trimEnd();
    await git(app, ['checkout', '-b', 'reword']);
    for (const [file, text] of [['one', 'Reword one\n\nIts body.'], ['two', 'Reword two\n\nKept under the new subject.'], ['three', 'Reword three']]) {
      fs.writeFileSync(path.join(app, `${file}.txt`), `${file}\n`);
      await git(app, ['add', `${file}.txt`]);
      await git(app, ['commit', '-m', text]);
    }
    await controller.refresh();
    try {
      // The last one is amended, without a question, and what is staged stays out of it.
      fs.writeFileSync(path.join(app, 'three.txt'), 'staged\n');
      await git(app, ['add', 'three.txt']);
      asked.length = 0;
      answer = run;
      await controller.action({ kind: 'reword', hash: state().head.commit!, message: 'Reword three, new\n\nWith a body now.' });
      assert.equal(asked.length, 0, JSON.stringify(asked));
      assert.equal(await message('HEAD'), 'Reword three, new\n\nWith a body now.');
      assert.deepEqual(state().workingTree.staged, [{ kind: 'modified', path: 'three.txt', originalPath: null }]);

      // An older one is asked first, and the changes not committed come back after; once done, what the rewrite changed
      // is offered (M36).
      const one = (await git(app, ['rev-parse', 'HEAD~2'])).stdout.trim();
      await controller.action({ kind: 'reword', hash: one, message: 'Reword one, new\n\nIts body.' });
      assert.deepEqual(
        asked.map((question) => question.message),
        [`Edit the message of ${one.slice(0, 7)}?`, `Gitardo: Edit the message of ${one.slice(0, 7)}: done.`],
      );
      assert.ok(asked[0].detail.includes('the 2 commits after it are made again'), asked[0].detail);
      assert.deepEqual(asked[1].items, ['See What Changed']);
      assert.equal(await message('HEAD~2'), 'Reword one, new\n\nIts body.');
      assert.equal(await message('HEAD'), 'Reword three, new\n\nWith a body now.');
      assert.equal(fs.readFileSync(path.join(app, 'three.txt'), 'utf8'), 'staged\n');
      assert.equal(state().operation, null);

      // From the interactive rebase a new subject keeps the body.
      const two = (await git(app, ['rev-parse', 'HEAD~1'])).stdout.trim();
      const steps: RebaseStep[] = [
        { action: 'reword', hash: two, message: 'Reword two, new', keepBody: true },
        { action: 'pick', hash: state().head.commit! },
      ];
      // The interactive rebase does not set the changes aside on its own.
      await git(app, ['stash', 'push', '--quiet']);
      await controller.action({ kind: 'rebaseInteractive', base: (await git(app, ['rev-parse', `${two}~1`])).stdout.trim(), steps });
      assert.equal(await message('HEAD~1'), 'Reword two, new\n\nKept under the new subject.');

      // Once pushed, the force push it takes is said, and asked.
      await git(app, ['push', '--quiet', 'origin', 'reword']);
      await controller.refresh();
      asked.length = 0;
      await controller.action({ kind: 'reword', hash: state().head.commit!, message: 'Reword three, again' });
      assert.equal(asked.length, 1);
      assert.ok(asked[0].detail.includes('is pushed already, on origin/reword'), asked[0].detail);
      assert.equal(await message('HEAD'), 'Reword three, again');
    } finally {
      answer = () => undefined;
      await git(app, ['checkout', '--force', home]);
      await git(app, ['stash', 'drop', '--quiet']).catch(() => undefined);
      await git(app, ['branch', '-D', 'reword']);
      await git(app, ['push', '--quiet', 'origin', '--delete', 'reword']).catch(() => undefined);
      await controller.refresh();
    }
  });

  await step('asks before applying a stash, saying what it would stop on, and before deleting a tag or a merged branch', async () => {
    const start = state().head.commit!;
    const apply = async () => {
      asked.length = 0;
      await controller.action({ kind: 'stashApply', ref: 'stash@{0}' });
      assert.deepEqual(
        asked.map((question) => question.message),
        ['Apply stash@{0}?'],
      );
      return asked[0].detail;
    };
    write('preview.txt', 'one\n');
    await git(app, ['add', 'preview.txt']);
    await git(app, ['commit', '-m', 'Add preview']);
    write('preview.txt', 'stashed\n');
    write('along.txt', 'along\n');
    await git(app, ['stash', 'push', '--include-untracked', '--quiet', '-m', 'preview']);
    await controller.refresh();
    try {
      // Against a commit that changed the same line, cancelled.
      write('preview.txt', 'committed\n');
      await git(app, ['commit', '-am', 'Change preview']);
      await controller.refresh();
      answer = () => undefined;
      let detail = await apply();
      assert.ok(detail.includes('This will stop with conflicts in 1 file:\npreview.txt'), detail);
      assert.equal(state().changes, 0);

      // With the file changed and not committed, git would refuse.
      await git(app, ['reset', '--hard', '--quiet', 'HEAD~1']);
      write('preview.txt', 'dirty\n');
      await controller.refresh();
      detail = await apply();
      assert.ok(detail.includes('Git will refuse it and change nothing, as 1 file it changes has changes that are not committed'), detail);

      // Its untracked file there again is left alone.
      await git(app, ['checkout', '--', 'preview.txt']);
      write('along.txt', 'here\n');
      await controller.refresh();
      detail = await apply();
      assert.ok(detail.includes('An untracked file it took along is there again') && detail.includes('along.txt'), detail);

      // Nothing in the way: no conflicts, and it runs.
      fs.rmSync(path.join(app, 'along.txt'));
      await controller.refresh();
      answer = run;
      detail = await apply();
      assert.ok(detail.includes('No conflicts expected.') && detail.includes('it stays in the list'), detail);
      assert.equal(fs.readFileSync(path.join(app, 'preview.txt'), 'utf8'), 'stashed\n');
      assert.equal(state().stashes.length, 1);

      // A tag, and a branch whose commits are merged.
      await git(app, ['tag', 'to-delete']);
      await git(app, ['branch', 'merged-here']);
      await controller.refresh();
      asked.length = 0;
      await controller.action({ kind: 'deleteTag', name: 'to-delete' });
      await controller.action({ kind: 'deleteBranch', name: 'merged-here' });
      assert.deepEqual(
        asked.map((question) => question.message),
        ['Delete tag to-delete?', 'Delete branch merged-here?'],
      );
      assert.ok(asked[0].detail.includes('A remote that has it keeps it.'), asked[0].detail);
      assert.ok(asked[1].detail.includes('none is lost') && asked[1].detail.endsWith('git branch -d merged-here'), asked[1].detail);
      assert.ok(!state().tags.some((tag) => tag.name === 'to-delete'));
      assert.ok(!state().branches.some((branch) => branch.name === 'merged-here'));
    } finally {
      answer = () => undefined;
      await git(app, ['reset', '--hard', '--quiet', start]);
      fs.rmSync(path.join(app, 'along.txt'), { force: true });
      await git(app, ['stash', 'drop', '--quiet']).catch(() => undefined);
      await git(app, ['tag', '-d', 'to-delete']).catch(() => undefined);
      await git(app, ['branch', '-D', 'merged-here']).catch(() => undefined);
      await controller.refresh();
    }
    assert.equal(state().changes, 0);
  });

  await step('stops an interactive rebase to edit a commit, and goes on once what was changed is committed', async () => {
    const home = checkedOutBranch(state())!;
    await git(app, ['checkout', '-b', 'edit-stop']);
    for (const name of ['one', 'two']) {
      write(`edit-${name}.txt`, `${name}\n`);
      await git(app, ['add', `edit-${name}.txt`]);
      await git(app, ['commit', '-m', `Edit ${name}`]);
    }
    await controller.refresh();
    try {
      const [one, two, base] = await Promise.all(['HEAD~1', 'HEAD', 'HEAD~2'].map(async (ref) => (await git(app, ['rev-parse', ref])).stdout.trim()));
      answer = run;
      await controller.action({
        kind: 'rebaseInteractive',
        base,
        steps: [
          { action: 'edit', hash: one },
          { action: 'pick', hash: two },
        ],
      });
      assert.equal(state().operation, 'rebase');
      assert.deepEqual(state().rebaseStop, { kind: 'edit', commit: one, subject: 'Edit one' });

      // What is changed and not committed is in the way, which is said rather than tried.
      write('edit-one.txt', 'one, changed\n');
      await controller.refresh();
      // The engine refuses, and the refusal is a warning: nothing failed, nothing ran.
      asked.length = 0;
      await controller.action({ kind: 'continue' });
      const refused = await until('the refusal', () => asked.find((said) => /amend the commit with them, commit them, or stash them/.test(said.message)));
      assert.ok(refused.message.startsWith('Gitardo: '), refused.message);
      assert.equal(state().operation, 'rebase');

      // Amended, it goes on, and the commit after it is made again on top.
      await git(app, ['commit', '--all', '--amend', '--no-edit']);
      await controller.refresh();
      await controller.action({ kind: 'continue' });
      assert.equal(state().operation, null);
      assert.equal(state().rebaseStop, null);
      assert.deepEqual(subjects().slice(0, 2), ['Edit two', 'Edit one']);
      assert.equal((await git(app, ['show', 'HEAD~1:edit-one.txt'])).stdout, 'one, changed\n');
    } finally {
      answer = () => undefined;
      await git(app, ['rebase', '--abort']).catch(() => undefined);
      await git(app, ['checkout', '--force', home]);
      await git(app, ['branch', '-D', 'edit-stop']);
      await controller.refresh();
    }
  });

  await step('splits the last commit and an older one, commits what is left on Continue, and undoes each split whole', async () => {
    const home = checkedOutBranch(state())!;
    const rev = async (ref: string) => (await git(app, ['rev-parse', ref])).stdout.trim();
    await git(app, ['checkout', '-b', 'split']);
    const commit = async (message: string, ...files: string[]) => {
      for (const file of files) write(file, `${file}\n`);
      await git(app, ['add', ...files]);
      await git(app, ['commit', '-m', message]);
      return rev('HEAD');
    };
    const start = await rev('HEAD');
    const older = await commit('Add split one and two\n\nIn one go.', 'split-one.txt', 'split-two.txt');
    const last = await commit('Add split three and four', 'split-three.txt', 'split-four.txt');
    await controller.refresh();
    const drafted = () => posted.flatMap((message) => (message.type === 'commitMessage' ? [message.message] : []));
    let failed: unknown;
    try {
      // The last one: a reset, a message that says what to do, its message in the box, and an undo of its own.
      asked.length = 0;
      posted.length = 0;
      answer = run;
      await controller.action({ kind: 'split', hash: last });
      assert.deepEqual(
        // What the engine says after a plan starts with the name of Gitardo.
        asked.map((question) => question.message.replace(/^Gitardo: /, '')),
        [
          `Split ${last.slice(0, 7)}?`,
          'The changes of "Add split three and four" are in the working tree, unstaged: commit them in as many commits as you like.',
        ],
      );
      assert.equal(state().head.commit, older);
      assert.equal(state().operation, null);
      assert.deepEqual(drafted(), ['Add split three and four']);
      await controller.action({ kind: 'undo' });
      assert.equal(state().head.commit, last);
      assert.equal(state().changes, 0);

      // An older one: a rebase that stops there, with a banner of its own.
      asked.length = 0;
      posted.length = 0;
      await controller.action({ kind: 'split', hash: older });
      assert.ok(asked[0].detail.includes('the commit after it is made again on top'), asked[0].detail);
      assert.equal(state().head.commit, start);
      assert.deepEqual(state().rebaseStop, { kind: 'split', commit: older, subject: 'Add split one and two' });
      assert.deepEqual(drafted(), ['Add split one and two\n\nIn one go.']);
      await git(app, ['add', 'split-one.txt']);
      await git(app, ['commit', '-m', 'Add split one']);
      await controller.refresh();

      // Continue with a file of it not committed: going back to committing changes nothing.
      // The engine asks it as a pick of two choices, which the extension shows as a quick pick with the question as title.
      const titles: string[] = [];
      choose = (items, options) => {
        titles.push(options.title ?? '');
        return byLabel('Go back to committing')(items);
      };
      await controller.action({ kind: 'continue' });
      assert.equal(titles.length, 1);
      assert.ok(/^"Add split one and two" is not all committed yet: 1 file of it is still changed\.$/.test(titles[0]), titles[0]);
      assert.equal(state().operation, 'rebase');
      // Or what is left is committed with the message the commit had, and the rebase goes on to the end.
      choose = byLabel('Commit what is left');
      answer = run;
      await controller.action({ kind: 'continue' });
      assert.equal(state().operation, null);
      assert.equal(state().rebaseStop, null);
      assert.deepEqual(subjects().slice(0, 3), ['Add split three and four', 'Add split one and two', 'Add split one']);
      assert.equal((await git(app, ['log', '-1', '--format=%B', 'HEAD~1'])).stdout.trim(), 'Add split one and two\n\nIn one go.');

      // One undo takes back the whole split.
      answer = run;
      await controller.action({ kind: 'undo' });
      assert.equal(state().head.commit, last);
    } catch (error) {
      failed = error;
      throw error;
    } finally {
      answer = () => undefined;
      choose = () => undefined;
      // What went wrong in the step is what is said, not that the clean-up could not end what it left.
      try {
        await git(app, ['rebase', '--abort']).catch(() => undefined);
        await git(app, ['checkout', '--force', home]);
        await git(app, ['branch', '-D', 'split']);
      } catch (error) {
        if (!failed) throw error;
      }
      for (const name of ['one', 'two', 'three', 'four']) fs.rmSync(path.join(app, `split-${name}.txt`), { force: true });
      await controller.refresh();
    }
  });

  await step('gives VS Code the bytes of an image at a commit, opens the diff of a PNG, and sends the page of a commit its two versions', async () => {
    const home = (await git(app, ['rev-parse', '--abbrev-ref', 'HEAD'])).stdout.trim();
    const png = (base64: string) => Buffer.from(base64, 'base64');
    const red = png('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==');
    const blue = png('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPj/HwADBwIAMCbHYQAAAABJRU5ErkJggg==');
    let failed: unknown;
    try {
      await git(app, ['checkout', '--quiet', '-b', 'images']);
      fs.writeFileSync(path.join(app, 'logo.png'), red);
      await git(app, ['add', '--', 'logo.png']);
      await git(app, ['commit', '--quiet', '-m', 'Add the logo']);
      fs.writeFileSync(path.join(app, 'logo.png'), blue);
      await git(app, ['commit', '--quiet', '--all', '-m', 'Make the logo blue']);
      const [hash, parent] = (await git(app, ['log', '-1', '--format=%H %P'])).stdout.trim().split(' ');
      await controller.refresh();

      // The scheme of Gitardo gives the bytes as they are, which VS Code's preview of images draws.
      assert.deepEqual(Buffer.from(await vscode.workspace.fs.readFile(contentUri(app, parent, 'logo.png'))), red);
      assert.deepEqual(Buffer.from(await vscode.workspace.fs.readFile(contentUri(app, hash, 'logo.png'))), blue);

      // The diff of the PNG changed in the working tree, from the Gitardo Changes view, and that of the commit.
      fs.writeFileSync(path.join(app, 'logo.png'), red);
      await until('the change read', () => state().workingTree.unstaged.some((file) => file.path === 'logo.png'));
      const file: FileChange = { kind: 'modified', path: 'logo.png', originalPath: null };
      await fromWebview({ type: 'openDiff', source: { kind: 'change' }, file });
      await until('the diff of the image', () => /logo\.png/.test(vscode.window.tabGroups.activeTabGroup.activeTab?.label ?? '') || undefined);
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');
      await fromWebview({ type: 'openDiff', source: { kind: 'commit', hash, parent }, file });
      await until('the diff of the commit', () => /logo\.png/.test(vscode.window.tabGroups.activeTabGroup.activeTab?.label ?? '') || undefined);
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');

      // The page of the commit gets its two versions, as `data:` URIs its policy of content lets it draw.
      posted.length = 0;
      await fromWebview({ type: 'readImages', source: { kind: 'commit', hash, parent }, file });
      const images = await until('the images', () => posted.find((outgoing) => outgoing.type === 'images'));
      assert.ok(images.type === 'images' && images.images);
      assert.deepEqual(images.images.before, { uri: `data:image/png;base64,${red.toString('base64')}`, weight: red.length });
      assert.deepEqual(images.images.after, { uri: `data:image/png;base64,${blue.toString('base64')}`, weight: blue.length });
      await controller.commitTabs.openCommit(hash);
      const commitView = () => [...internals.views.entries()].find(([, kind]) => kind === 'commit')?.[0] as vscode.Webview | undefined;
      const page = await until('the tab of the commit', commitView);
      assert.match(page.html, /img-src [^;]* data:;/);
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');
    } catch (error) {
      failed = error;
      throw error;
    } finally {
      try {
        await git(app, ['checkout', '--force', '--quiet', home]);
        await git(app, ['branch', '-D', 'images']);
      } catch (error) {
        if (!failed) throw error;
      }
      fs.rmSync(path.join(app, 'logo.png'), { force: true });
      await controller.refresh();
    }
  });

  // The tabs of the graph in the editor, each a group of views of its own on a repository (D89).
  const graphTabs = () => [...new Set(internals.groups.values())].filter((group) => group.panel);
  const tabShowing = (root: string, besides?: ViewGroup) => graphTabs().find((group) => group !== besides && !!group.state && samePath(group.state.root, root));
  const pageOf = (group: ViewGroup) => {
    const found = [...internals.groups].find(([, candidate]) => candidate === group);
    assert.ok(found, 'the tab has a page');
    return found[0] as vscode.Webview;
  };
  const sessionNames = () => internals.sessions.all().map((session) => path.basename(session.root)).sort();

  await step('opens a tab of the graph on another repository, which its own toolbar moves and its operations act on alone', async () => {
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
    if (!samePath(state().root, app)) await controller.repositories.selectRepo(app);
    const cached = async (folder: string) => (await git(folder, ['diff', '--cached', '--name-only'])).stdout.trim();
    try {
      // **Open in a new tab** in the menu of a row of the Repositories view.
      await internals.onMessage(view('repositories'), { type: 'openInTab', root: lib });
      const tab = await until('a tab of the graph on lib', () => tabShowing(lib));
      assert.equal(tab.panel!.title, 'Gitardo · workspace/lib');
      assert.deepEqual(
        tab.state!.rows.map((row) => row.subject),
        ['Add lib'],
      );
      assert.ok(samePath(state().root, app), 'the panel still shows app');
      assert.deepEqual(sessionNames(), ['app', 'lib']);
      const page = pageOf(tab);
      // The panel shows app: a state of lib is for the tab.
      await until('the page of the tab to be sent lib', () => posted.some((outgoing) => outgoing.type === 'state' && !!outgoing.state && samePath(outgoing.state.root, lib)));

      // What is done from the tab is done in lib.
      fs.writeFileSync(path.join(lib, 'tab.txt'), 'tab\n');
      write('panel.txt', 'panel\n');
      await until('the file in the tab', () => tab.state?.workingTree.untracked.includes('tab.txt'));
      await until('the file in the panel', () => state().workingTree.untracked.includes('panel.txt'));
      await internals.onMessage(page, { type: 'action', action: { kind: 'stage', paths: ['tab.txt'] } });
      assert.equal(await cached(lib), 'tab.txt');
      assert.equal(await cached(app), '');

      // One operation at a time in the window, whichever tab asks: the other is refused, naming where the first runs.
      errors.length = 0;
      await Promise.all([
        internals.onMessage(page, { type: 'action', action: { kind: 'unstage', paths: ['tab.txt'] } }),
        internals.onMessage(view('graph'), { type: 'action', action: { kind: 'stage', paths: ['panel.txt'] } }),
      ]);
      assert.ok(
        errors.some((text) => / is running in (workspace\/)?lib\. Try again once it ends\./.test(text)),
        JSON.stringify(errors),
      );
      errors.length = 0;
      assert.equal(await cached(lib), '');
      assert.equal(await cached(app), '');

      // The list of the repositories of the tab's toolbar moves the tab alone, and lib is let go with nothing on it.
      await internals.onMessage(page, { type: 'selectRepo', root: app });
      await until('the tab to show app', () => tab.state && samePath(tab.state.root, app));
      assert.equal(tab.panel!.title, 'Gitardo · workspace/app');
      assert.ok(samePath(state().root, app));
      assert.deepEqual(sessionNames(), ['app']);
      await internals.onMessage(page, { type: 'selectRepo', root: lib });
      await until('the tab to show lib again', () => tab.state && samePath(tab.state.root, lib));

      // Two tabs on one repository read it once, and each keeps what narrows its graph.
      await internals.onMessage(view('repositories'), { type: 'openInTab', root: lib });
      const second = await until('a second tab on lib', () => tabShowing(lib, tab));
      assert.equal(second.session, tab.session);
      await internals.onMessage(pageOf(second), { type: 'setFilter', filter: { kind: 'path', path: 'lib.txt' } });
      assert.deepEqual(second.filter, { kind: 'path', path: 'lib.txt' });
      assert.equal(tab.filter, null);
      assert.equal(internals.filter, null);

      // A session is let go with the last tab on it, not before.
      second.panel!.dispose();
      assert.deepEqual(sessionNames(), ['app', 'lib']);
      tab.panel!.dispose();
      assert.deepEqual(sessionNames(), ['app']);
      assert.equal(graphTabs().length, 0);
    } finally {
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');
      await git(lib, ['reset', '--quiet']).catch(() => undefined);
      await git(app, ['reset', '--quiet']).catch(() => undefined);
      fs.rmSync(path.join(lib, 'tab.txt'), { force: true });
      fs.rmSync(path.join(app, 'panel.txt'), { force: true });
      await controller.refresh();
    }
  });

  await step('shows in the panel and the side bar the repository of the tab of the graph that comes into focus, and moves no tab from them', async () => {
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
    if (!samePath(state().root, app)) await controller.repositories.selectRepo(app);
    // What the Changes view of the side bar is sent, as its page gets it.
    const changes = view('changes') as vscode.Webview & { postMessage(message: unknown): Thenable<boolean> };
    const toChanges: HostMessage[] = [];
    const postToChanges = changes.postMessage.bind(changes);
    changes.postMessage = (message: unknown) => {
      toChanges.push(message as HostMessage);
      return postToChanges(message);
    };
    /** Whether the state the Changes view was sent last names a repository. */
    const changesShow = (root: string) => () => {
      const named = toChanges.filter((message) => message.type === 'state' && message.state?.root).at(-1);
      return named?.type === 'state' && !!named.state?.root && samePath(named.state.root, root);
    };
    const chosen: string[] = [];
    const listening = controller.onDidChooseRepo((root) => chosen.push(root));
    try {
      await controller.repositories.openTab(lib);
      const onLib = await until('a tab on lib', () => tabShowing(lib));
      await controller.repositories.openTab(app);
      const onApp = await until('a tab on app', () => tabShowing(app, onLib));
      // Opened, a tab moves nothing: the panel shows what it showed.
      assert.ok(samePath(state().root, app));

      onLib.panel!.reveal();
      await until('the panel to show lib', () => samePath(state().root, lib));
      await until('the Changes view to show lib', changesShow(lib));
      assert.ok(chosen.some((root) => samePath(root, lib)), "VS Code's Repositories view is told to select lib");
      onApp.panel!.reveal();
      await until('the panel to show app again', () => samePath(state().root, app));
      await until('the Changes view to show app', changesShow(app));

      // Choosing in the panel moves the panel and the side bar, and neither tab.
      await internals.onMessage(view('graph'), { type: 'selectRepo', root: lib });
      await until('the panel to show lib', () => samePath(state().root, lib));
      assert.ok(onApp.state && samePath(onApp.state.root, app));
      assert.ok(onLib.state && samePath(onLib.state.root, lib));

      // A tab of a commit does not move them (Q1521).
      await internals.onMessage(view('graph'), { type: 'selectRepo', root: app });
      await until('the panel to show app', () => samePath(state().root, app));
      await controller.commitTabs.openCommit(state().head.commit!);
      await until('the tab of the commit', () => [...internals.views.values()].includes('commit'));
      await sleep(500);
      assert.ok(samePath(state().root, app));
    } finally {
      changes.postMessage = postToChanges;
      listening.dispose();
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');
      await until('the tabs of the graph to close', () => graphTabs().length === 0);
      if (!samePath(state().root, app)) await controller.repositories.selectRepo(app);
    }
  });

  await step('brings a tab of the graph back after a reload on what its page kept, and says so of a folder that is no longer there', async () => {
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
    if (!samePath(state().root, app)) await controller.repositories.selectRepo(app);
    // What VS Code gives the serializer of the tabs of the graph after a reload: a tab, and the state its page kept.
    const restore = (kept: unknown) => {
      const panel = vscode.window.createWebviewPanel('gitardo.graphEditor', 'Gitardo', vscode.ViewColumn.Active, { retainContextWhenHidden: true });
      const { root, ...shown } = keptTab(kept);
      return { panel, opened: controller.repositories.openTab(root, panel, shown) };
    };
    try {
      const filter = { kind: 'message', query: 'lib' } as const;
      const back = restore({ refsWidth: 240, tab: { root: lib, filter, scope: null } });
      await back.opened;
      const tab = await until('the tab back on lib', () => tabShowing(lib));
      assert.equal(tab.panel, back.panel);
      assert.equal(back.panel.title, 'Gitardo · workspace/lib');
      assert.deepEqual(tab.filter, filter);
      await until('the filter read', () => tab.state?.filter?.kind === 'message');
      assert.deepEqual(
        tab.state!.rows.map((row) => row.subject),
        ['Add lib'],
      );
      // The panel shows what it showed.
      assert.ok(samePath(state().root, app));
      assert.equal(internals.filter, null);
      back.panel.dispose();

      // A page kept before tabs kept what they show opens on the repository the panel shows.
      const older = restore({ refsWidth: 240 });
      await older.opened;
      await until('the older tab on app', () => tabShowing(app));
      older.panel.dispose();

      // A worktree removed while VS Code was closed: the tab says so, reads nothing, and Close closes it.
      const folder = path.join(workspace, 'app.worktrees', 'removed');
      const gone = restore({ tab: { root: folder, filter: null, scope: null } });
      await gone.opened;
      assert.ok(gone.panel.webview.html.includes('data-gone='), 'the page says the folder is no longer there');
      assert.equal(gone.panel.title, 'Gitardo · removed');
      assert.deepEqual(sessionNames(), ['app']);
      let closed = false;
      gone.panel.onDidDispose(() => (closed = true));
      await internals.onMessage(gone.panel.webview, { type: 'closeTab' });
      assert.ok(closed, 'Close closes the tab');
    } finally {
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');
      await until('the tabs of the graph to close', () => graphTabs().length === 0);
    }
  });

  await step('shows the files stored with Git LFS in the Large files tab, and stores a file of the Explorer with Git LFS', async () => {
    // Milestone M47. Without Git LFS the tab says it is not installed and lists the patterns all the same; with it, the
    // file committed shows Downloaded and Store with Git LFS… writes its pattern. Lock and Unlock need a server of locks,
    // which this suite does not start: the tests of the engine and of the extension check them (Q1198).
    const hasLfs = spawnSync('git', ['lfs', 'version']).status === 0;
    const head = (await git(app, ['rev-parse', 'HEAD'])).stdout.trim();
    type Large = Extract<HostMessage, { type: 'largeFiles' }>;
    const shown = (check: (info: NonNullable<Large['info']>) => boolean) =>
      until('what the Large files tab shows', () => posted.find((message): message is Large => message.type === 'largeFiles' && !!message.info && check(message.info)));
    try {
      write('.gitattributes', '*.psd filter=lfs diff=lfs merge=lfs -text\n');
      if (hasLfs) {
        await git(app, ['lfs', 'install', '--local']);
        write('logo.psd', 'the layers of the logo\n');
        await git(app, ['add', '--', '.gitattributes', 'logo.psd']);
        await git(app, ['commit', '--quiet', '-m', 'Store the drawings with Git LFS']);
      } else {
        // The patterns are read from the files .gitattributes git knows, as the engine lists them with git ls-files.
        await git(app, ['add', '--', '.gitattributes']);
      }
      await vscode.commands.executeCommand('gitardo.refresh');
      posted.length = 0;
      await vscode.commands.executeCommand('gitardo.largeFiles');
      const tab = (await until('the tab', () => [...internals.views].find(([, kind]) => kind === 'largeFiles')))[0];
      const first = await shown(() => true);
      assert.deepEqual(first.info!.status.patterns, [{ pattern: '*.psd', file: '.gitattributes', line: 1 }]);
      if (!hasLfs) {
        assert.equal(first.info!.status.version, null);
        assert.deepEqual(first.info!.files, []);
        return;
      }
      const read = await shown((info) => info.files.length > 0);
      assert.deepEqual(read.info!.files.map((file) => [file.path, file.isDownloaded]), [['logo.psd', true]]);

      // Store with Git LFS… on a file of the Explorer: all the files of its extension.
      write('data.bin', 'some data\n');
      choose = byLabel('All .bin files');
      answer = () => undefined;
      await vscode.commands.executeCommand('gitardo.storeWithLfs', vscode.Uri.file(path.join(app, 'data.bin')));
      assert.match(fs.readFileSync(path.join(app, '.gitattributes'), 'utf8'), /^\*\.bin filter=lfs diff=lfs merge=lfs -text$/m);
      posted.length = 0;
      await internals.onMessage(tab, { type: 'largeFiles', ask: { kind: 'refresh' } });
      await shown((info) => info.status.patterns.some((pattern) => pattern.pattern === '*.bin'));
    } finally {
      await vscode.commands.executeCommand('workbench.action.closeAllEditors');
      if (hasLfs) await git(app, ['lfs', 'uninstall', '--local']);
      await git(app, ['reset', '--quiet', '--hard', head]);
      for (const file of ['.gitattributes', 'data.bin']) fs.rmSync(path.join(app, file), { force: true });
      await vscode.commands.executeCommand('gitardo.refresh');
    }
  });

  await step('logged no unexpected errors', async () => {
    // The conflicting merge is the one failure the tests provoke on purpose.
    assert.deepEqual(
      errors.filter((text) => !/CONFLICT|Automatic merge failed/.test(text)),
      [],
    );
  });

  if (results.some((result) => !result.ok)) throw new Error('Some end-to-end checks failed.');
}
