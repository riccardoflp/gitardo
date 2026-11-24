/**
 * What VS Code does around an operation the engine runs: VS Code's Git letting go of what it is about to move, what the
 * plan leaves for the host before and after the repository is read again, the commands and the message that follow a
 * new worktree, and what is said when the engine refuses an operation, when one fails or stops on conflicts, and when a
 * bisect found its commit.
 */
import * as vscode from 'vscode';
import type { EngineRun } from '../engine/actions';
import { EngineError, ErrorCode } from '../engine/connection';
import type { Finished, FollowUp, OfferedRun, Step, Target, WriteSettingFollowUp } from '../engine/messages';
import { type Repository, reads } from '../engine/reads';
import { settingLine, settingWritten } from '../engine/settings';
import { samePath } from '../shared/paths';
import type { Action, RepoState } from '../shared/protocol';
import { COMPARE_REWRITE, buttonWords, type RangeDiffTarget } from '../shared/rangeDiff';
import { stopWords } from '../shared/rerere';
import { operationName } from '../shared/state';
import { submoduleFolder } from '../shared/submodules';
import { followMarks } from '../shared/tagMarks';
import { LAST_INPUTS, SETUP_OFFERED, SET_UP, closeInVSCodeGit, openIn, openWorktree } from './actions';
import type { ControllerHost } from './controllerHost';
import { type ViewGroup, errorText } from './repositorySession';
import { runSetupCommands } from './worktreeCommands';
import { SETUP_PAGE } from './worktreeSetupTab';

/** Carries out, in VS Code, what comes before and after each operation the engine runs. */
export class RunFollowUps {
  constructor(
    private readonly host: Pick<
      ControllerHost,
      | 'action'
      | 'groupShowing'
      | 'heldFor'
      | 'log'
      | 'markTags'
      | 'memento'
      | 'openCommit'
      | 'openRangeDiff'
      | 'openWorktreeSetup'
      | 'post'
      | 'resolveWithAI'
      | 'reveal'
      | 'reviewConflicts'
      | 'run'
      | 'selectRepo'
      | 'sessions'
      | 'show'
      | 'tagMarks'
    >,
  ) {}

  // Area 5: worktrees and what new worktrees get, submodules, run by the engine.

  /**
   * Has VS Code's Git extension close the worktree or the submodules, and those inside them, that the engine is about to
   * move, empty or delete, which Windows refuses while the extension watches them.
   */
  async letGoInVSCodeGit(action: Action, state: RepoState): Promise<void> {
    if (action.kind === 'worktreeRemove' || action.kind === 'worktreeRename') {
      // Git never moves nor removes the main worktree, which the engine refuses: VS Code's Git keeps it open.
      const main = state.worktrees.some((worktree) => worktree.isMain && samePath(worktree.folder, action.path));
      if (!main) await closeInVSCodeGit([action.path], this.host.log);
    }
    if (action.kind === 'submoduleDeinit' || action.kind === 'submoduleRemove') {
      const path = action.path;
      const inside = state.submodules.filter((each) => each.checkout?.commit && (each.folder === path || each.folder.startsWith(`${path}/`)));
      await closeInVSCodeGit(
        inside.map((each) => submoduleFolder(each, state)),
        this.host.log,
      );
    }
  }

  /**
   * Carries out what follows a new worktree made by the engine as the extension did: the commands of
   * gitardo.worktrees.setupCommands, handed over by the engine, in a terminal of this window, and only once they all
   * succeeded the message that it was created, with its buttons to open it, or the worktree opened for **Open in
   * worktree** where gitardo.worktrees.openIn says; when a command fails, Gitardo says so and offers to open it. Gives the
   * follow-ups left to carry out.
   */
  worktreeMade(feature: string, finished: Extract<Finished, { kind: 'ran' }>, followUps: FollowUp[], repo: Repository): FollowUp[] {
    const creates = ['worktree.create', 'worktree.create-from-branch', 'worktree.create-at-commit'].includes(feature);
    if (!creates && feature !== 'worktree.open-in-worktree') return followUps;
    // `git worktree add [<options>] <folder> <commit-ish>`.
    const argumentsOf = (step: Step) => (step.kind === 'git' && Array.isArray(step.arguments) ? (step.arguments as string[]) : []);
    const added = finished.plan.steps.map(argumentsOf).find((args) => args[0] === 'worktree' && args[1] === 'add') ?? [];
    const dir = added[added.length - 2];
    if (!dir) return followUps;
    const agents = this.host.heldFor(repo.root)?.agentsWindow ?? false;
    const says = followUps.flatMap((followUp) => (followUp.kind === 'say' ? [String(followUp.text)] : []));
    const offersSetup =
      followUps.some((followUp) => followUp.kind === 'offerRun' && followUp.feature === 'worktree-setup.page') && !this.host.memento.get(SETUP_OFFERED);
    const opens = [...(agents ? ['Open in Agents Window'] : []), 'Open in New Window'];
    const open = (choice: string | undefined) => {
      if (choice) void openWorktree(dir, choice === 'Open in Agents Window' ? 'agents' : 'window');
    };
    const done = () => {
      if (!creates) {
        void openWorktree(dir, openIn({ agentsWindow: agents }));
        for (const said of says) void vscode.window.showInformationMessage(`Gitardo: ${said}`);
        return;
      }
      if (offersSetup) void this.host.memento.update(SETUP_OFFERED, true);
      const [created, ...others] = says;
      for (const said of others) void vscode.window.showInformationMessage(`Gitardo: ${said}`);
      void vscode.window
        // Without the name of Gitardo before it, as the message of the extension had it.
        .showInformationMessage(created ?? `Worktree created in ${dir}`, ...opens, ...(offersSetup ? [SET_UP] : []))
        .then((choice) => (choice === SET_UP ? void vscode.commands.executeCommand('gitardo.worktreeSetup') : open(choice)));
    };
    const commands = finished.waiting.filter((step) => step.kind === 'task').map((step) => String(step.command));
    if (!commands.length) done();
    else {
      // Not waited for: the commands can take minutes, and other operations go on meanwhile.
      void runSetupCommands(repo.root, dir, commands, this.host.log).then((stopped) => {
        if (!stopped) return done();
        void vscode.window.showWarningMessage(`Gitardo: ${stopped}`, ...opens).then(open);
      });
    }
    return followUps.filter((followUp) => followUp.kind !== 'say' && followUp.kind !== 'openWindow');
  }

  /** Carries out what a plan of the engine leaves for the host before the repository is read again. */
  async beforeRead(followUps: FollowUp[], run: { feature: string; target: Target; repo: Repository; title?: string }): Promise<void> {
    // A setting VS Code could not write: what is said next would say it was saved.
    let unsaved = false;
    for (const [index, followUp] of followUps.entries()) {
      switch (followUp.kind) {
        case 'say':
          if (!unsaved) this.say(String(followUp.text), followUps.slice(index + 1), run);
          break;
        // **See What Changed** after a rewrite that said nothing of its own (M36): said as the desktop says it.
        case 'offerRun':
          if (followUp.feature === COMPARE_REWRITE && !followUps.slice(0, index).some((before) => before.kind === 'say')) {
            this.say(`${run.title ?? 'The operation'}: done.`, followUps.slice(index), run);
          }
          break;
        case 'rememberInputs': {
          const last = this.host.memento.get<Record<string, string>>(LAST_INPUTS, {});
          await this.host.memento.update(LAST_INPUTS, { ...last, ...(followUp.values as Record<string, string>) });
          break;
        }
        case 'emptyCommitMessage':
          this.host.post({ type: 'committed' });
          break;
        case 'copyToClipboard':
          await vscode.env.clipboard.writeText(String(followUp.text));
          break;
        case 'openUrl':
          void vscode.env.openExternal(vscode.Uri.parse(String(followUp.url)));
          break;
        case 'openWindow':
          void vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(String(followUp.folder)), { forceNewWindow: true });
          break;
        case 'showFolder':
          void vscode.commands.executeCommand('revealFileInOS', vscode.Uri.file(String(followUp.folder)));
          break;
        // Branches, tags and remotes (stage 4, area 2): a fetch, a pull, a push or a remote changed.
        case 'readPullRequests':
          void this.host.sessions.find(run.repo.root)?.refreshPulls(true);
          break;
        // The model that resolves conflicts (stage 4, area 6), and what new worktrees get (M48): settings of VS Code.
        case 'writeSetting':
          if (unsaved) break;
          try {
            await this.writeSetting(followUp as unknown as WriteSettingFollowUp, run.repo.root);
          } catch (error) {
            unsaved = true;
            void vscode.window.showErrorMessage(`Gitardo: the settings were not saved. ${errorText(error)}`);
          }
          break;
        // The commit box of the Changes view, after a squash merge and a split (area 3).
        case 'preparedMessage': {
          const prepared = (await reads.squashMessage(run.repo).catch(() => null))?.trim();
          if (prepared) this.host.post({ type: 'commitMessage', message: prepared });
          break;
        }
        case 'fillCommitMessage':
          if (String(followUp.message)) this.host.post({ type: 'commitMessage', message: String(followUp.message) });
          break;
        // A tag made, pushed or deleted from a remote changes its mark without asking the remote again (`tag.remote-marks`).
        case 'tagsOnRemote': {
          const marks = this.host.tagMarks.get(run.repo.root);
          const changed = { remote: (followUp.remote as string | null) ?? null, pushed: followUp.pushed as string[], notPushed: followUp.notPushed as string[] };
          if (marks) this.host.markTags(run.repo.root, followMarks(marks, changed, this.host.heldFor(run.repo.root)?.tags ?? []));
          break;
        }
        default:
          break;
      }
    }
  }

  /**
   * A message of a plan of the engine, with a button for each feature offered right after it, run on the same target.
   * Resolving automatically keeps the **Undo** it had beside them (stage 4, area 4).
   */
  private say(text: string, rest: FollowUp[], run: { feature: string; target: Target; repo: Repository }): void {
    const offers: Array<EngineRun & { label: string }> = [];
    for (const next of rest) {
      if (next.kind !== 'offer') break;
      offers.push({ feature: String(next.feature), label: String(next.label), target: run.target });
    }
    // **See What Changed** after a rewrite, until what is said next (M36), on its own target.
    const until = rest.findIndex((next) => next.kind === 'say');
    for (const next of until < 0 ? rest : rest.slice(0, until)) {
      if (next.kind !== 'offerRun' || next.feature !== COMPARE_REWRITE) continue;
      const offered = next as OfferedRun;
      offers.push({ feature: offered.feature, label: buttonWords(offered.label), target: offered.target, answers: offered.answers });
    }
    const undo = run.feature === 'conflicts.resolve-automatically' ? ['Undo'] : [];
    const words = text.startsWith('Gitardo ') ? text : `Gitardo: ${text}`;
    void vscode.window.showInformationMessage(words, ...offers.map((offer) => offer.label), ...undo).then(async (choice) => {
      const offer = offers.find((each) => each.label === choice);
      // **Choose What New Worktrees Get…** opens the tab, for the repository the settings were saved for.
      if (offer?.feature === SETUP_PAGE) {
        await this.host.selectRepo(run.repo.root);
        return this.host.openWorktreeSetup();
      }
      if (offer) void this.host.run([{ feature: offer.feature, target: offer.target, answers: offer.answers }], run.repo);
      if (choice === 'Undo') void this.host.action({ kind: 'undo' }, this.host.groupShowing(run.repo.root));
    });
  }

  /**
   * Writes a setting the engine left, by the extension's name, in the place it names, of the workspace folder of the
   * repository in `root`, of the workspace or of the user; null removes it from there. It is logged as a step is.
   */
  private async writeSetting(followUp: WriteSettingFollowUp, root: string): Promise<void> {
    const value = followUp.value === null ? undefined : followUp.value;
    const { section, key } = settingWritten(followUp.key);
    const places = { workspaceFolder: vscode.ConfigurationTarget.WorkspaceFolder, workspace: vscode.ConfigurationTarget.Workspace, user: vscode.ConfigurationTarget.Global };
    this.host.log.info(`> ${settingLine(followUp)}`);
    await vscode.workspace.getConfiguration(section, vscode.Uri.file(root)).update(key, value, places[followUp.target ?? 'user']);
  }

  /** Carries out what a plan of the engine leaves for the host once the repository was read again: what to select. */
  afterRead(followUps: FollowUp[], repo: Repository, on: ViewGroup): void {
    // What to select is selected in the group the operation was asked in, when it still shows that repository.
    const state = on.state && samePath(on.state.root, repo.root) ? on.state : null;
    for (const followUp of followUps) {
      // The tab of what a rewrite changed (M36), in the repository the operation ran in.
      if (followUp.kind === 'openRangeDiff') void this.host.openRangeDiff(followUp as RangeDiffTarget, repo);
      if (followUp.kind === 'selectHead' && state?.head.commit) this.host.show(state.head.commit, on);
      if (followUp.kind === 'selectCommit' && state) this.host.show(String(followUp.commit), on);
      if (followUp.kind === 'selectBeforeHead' && state?.head.commit) {
        const count = Number(followUp.count);
        void reads.resolve(repo, `HEAD~${count}`).then((hash) => hash && this.host.show(hash, on));
      }
      if (followUp.kind === 'openCommitPage') void this.host.openCommit(String(followUp.commit), repo);
      // Conflicts (stage 4, area 4): **Review** of resolving automatically.
      if (followUp.kind === 'openReview') void this.host.reviewConflicts(Array.isArray(followUp.paths) ? (followUp.paths as string[]) : undefined, false, on);
    }
  }

  /** Says why the engine refused, with the button of the feature it offers in its place, run on the same target. */
  offerAfterRefusal(message: string, offer: { feature: string; label: string } | null, target: Target, repo: Repository): void {
    const items = offer ? [offer.label] : [];
    void vscode.window.showWarningMessage(`Gitardo: ${message}`, ...items).then((choice) => {
      if (offer && choice === offer.label) void this.host.run([{ feature: offer.feature, target }], repo);
    });
  }

  /**
   * Says that a step of the engine failed: that a merge, a rebase, a cherry-pick, a revert or a patch stopped when files
   * are left in conflict, with the title of the plan confirmed, and with the engine's words otherwise, with a button for
   * each operation the plan offers once git refused it, such as **Delete anyway**.
   */
  reportEngineFailure(failure: unknown, title: string | undefined, repo: Repository): void {
    const state = this.host.heldFor(repo.root);
    const operation = state?.operation;
    if (failure instanceof EngineError && failure.code === ErrorCode.stepFailed && state?.workingTree.conflicted.length) {
      this.reportFailure({ title: title ?? `The ${operation ? operationName(operation) : 'operation'}` }, errorText(failure), repo);
      return;
    }
    const data = failure instanceof EngineError ? (failure.data as { followUps?: FollowUp[] } | undefined) : undefined;
    const offers = (data?.followUps ?? []).filter((followUp): followUp is OfferedRun => followUp.kind === 'offerRun');
    void vscode.window.showErrorMessage(`Gitardo: ${errorText(failure)}`, ...offers.map((offer) => offer.label), 'Show Log').then((choice) => {
      const offer = offers.find((each) => each.label === choice);
      if (offer) void this.host.run([{ feature: offer.feature, target: offer.target, answers: offer.answers }], repo);
      else if (choice) this.host.log.show();
    });
  }

  /** Says that an operation failed or stopped on conflicts, with what can be done next. */
  reportFailure(op: { title: string }, error: string, repo: Repository): void {
    const showLog = (choice?: string) => choice && this.host.log.show();
    const state = this.host.heldFor(repo.root);
    const on = this.host.groupShowing(repo.root);
    const conflicts = state?.workingTree.conflicted.length ?? 0;
    // Git may have resolved, and staged, every file in conflict from the resolutions it recorded: it stopped all the same.
    const before = state && stopWords(state);
    if (state && (conflicts || before)) {
      const { operation } = state;
      const files = conflicts === 1 ? '1 file has conflicts' : `${conflicts} files have conflicts`;
      // Without an operation to continue, as after a stash that did not apply cleanly, staging the files is all there is to do.
      const then = operation ? `continue or abort the ${operationName(operation)}` : 'stage the files';
      const marked = state.conflictFiles.reduce((sum, file) => sum + file.conflicts, 0);
      const resolvable = state.conflictFiles.reduce((sum, file) => sum + file.resolvable.length, 0);
      // The ones that need no choice between the two sides are a click away.
      const which = marked === 1 ? 'the conflict' : resolvable === marked ? `all ${marked} conflicts` : `${resolvable} of the ${marked} conflicts`;
      const offer = resolvable ? ` Gitardo can resolve ${which} automatically.` : '';
      // The language model is offered for what the rules leave.
      const ai = this.host.resolveWithAI() && (!marked || resolvable < marked);
      const items = [...(resolvable ? ['Resolve Automatically'] : []), ...(ai ? ['Resolve with AI'] : []), 'Show Log'];
      const said = conflicts ? `${op.title} stopped: ${files}. Resolve them, then ${then}.` : `${op.title} stopped.`;
      void vscode.window.showWarningMessage(`${said}${before ? ` ${before}` : ''}${offer}`, ...items).then((choice) => {
        if (choice === 'Resolve Automatically') void this.host.action({ kind: 'resolveConflicts' }, on);
        if (choice === 'Resolve with AI') void this.host.reviewConflicts(undefined, true, on);
        if (choice === 'Show Log') this.host.log.show();
      });
      return;
    }
    // Git prefixes the line that matters; hints and progress come after it.
    const lines = error.split('\n').filter(Boolean);
    const summary = lines.find((line) => /^(error|fatal):/.test(line)) ?? lines[0] ?? 'unknown error';
    void vscode.window.showErrorMessage(`${op.title} failed. ${summary}`, 'Show Log').then(showLog);
  }

  /** Says so when the last answer left a single suspect: that is the commit the bisect was looking for. */
  reportBisect(on: ViewGroup): void {
    const bisect = on.state?.bisect;
    const bad = bisect?.bad;
    if (!bad || bisect.remaining !== 1) return;
    const subject = on.state?.rows.find((row) => row.hash === bad)?.subject;
    void vscode.window
      .showInformationMessage(`Bisect: the first bad commit is ${bad.slice(0, 7)}${subject ? ` "${subject}"` : ''}.`, 'Show in the graph', 'End bisect')
      .then((choice) => {
        if (choice === 'Show in the graph') void this.host.reveal(bad, on);
        if (choice === 'End bisect') void this.host.action({ kind: 'bisectReset' }, on);
      });
  }
}
