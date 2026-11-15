/**
 * The engine in a window of VS Code: the binary found from `gitardo.engine.path` or the package, the settings read
 * from VS Code and sent again when they change, the questions of its features asked with VS Code's input boxes, picks
 * and dialogs, and the log in the **Gitardo** output channel. It is started the first time a repository is found, and
 * the output channel says when it cannot be; Gitardo then runs its TypeScript operations (D49).
 */
import * as vscode from 'vscode';
import { findBinary } from './binary';
import { EngineClient, type EngineOptions, type FeatureAsker } from './client';
import type { Answer, Choice, Plan, Question, Settings } from './messages';
import { engineSettings } from './settings';

/** The keys of every `gitardo.*` setting the extension contributes, without the prefix. */
function settingKeys(extension: vscode.Extension<unknown>): string[] {
  const json = extension.packageJSON as {
    contributes?: { configuration?: { properties?: Record<string, unknown> } };
  };
  return Object.keys(json.contributes?.configuration?.properties ?? {})
    .filter((key) => key.startsWith('gitardo.'))
    .map((key) => key.slice('gitardo.'.length));
}

/** The settings as VS Code resolves them now, by the desktop's names. */
export function readSettings(extension: vscode.Extension<unknown>): Settings {
  const ours = vscode.workspace.getConfiguration('gitardo');
  const values: Record<string, unknown> = {};
  const unset = new Set<string>();
  for (const key of settingKeys(extension)) {
    values[key] = ours.get(key);
    const set = ours.inspect(key);
    if (set?.workspaceFolderValue === undefined && set?.workspaceValue === undefined && set?.globalValue === undefined) unset.add(key);
  }
  const git = vscode.workspace.getConfiguration('git');
  const setValue = (key: string) => {
    const set = git.inspect(key);
    return set?.workspaceFolderValue ?? set?.workspaceValue ?? set?.globalValue;
  };
  const fallbacks = {
    branchSortOrder: setValue('branchSortOrder'),
    worktreeSymlinkFolders: setValue('worktreeSymlinkFolders'),
    worktreeIncludeFiles: setValue('worktreeIncludeFiles'),
  };
  return engineSettings(values, unset, fallbacks, -new Date().getTimezoneOffset());
}

/** A choice VS Code adds at the end of a pick of the engine: picking it runs it, and the feature is backed out of. */
export interface HostChoice {
  label: string;
  description?: string;
  run(): unknown;
}

/** What VS Code adds to the questions of a feature: choices of its own, by the key of the pick, and what it does once a plan is confirmed. */
export interface HostHooks {
  choices?: Record<string, HostChoice[]>;
  /** Run once the plan is confirmed, before its steps: what only VS Code can do first, such as closing a worktree in its Git. */
  confirmed?(plan: Plan): Promise<void>;
}

/**
 * Asks the questions of a feature of the engine as Gitardo's TypeScript operations ask theirs: text in an input box whose
 * text the engine checks as it is typed, a pick in a quick pick, a confirmation and the plan in a modal dialog, and so
 * a pick of a few choices whose detail has more than one line.
 */
export class VscodeAsker implements FeatureAsker {
  /** The title of the plan last confirmed, which says what stopped when a step of it fails. */
  confirmed: string | undefined;

  constructor(
    private readonly client: EngineClient,
    private readonly repository: string,
    private readonly hooks: HostHooks = {},
  ) {}

  async ask(question: Question): Promise<Answer | null> {
    const { kind } = question;
    const title = question.title;
    switch (kind.kind) {
      case 'text':
      case 'secret': {
        const validation = kind.kind === 'text' ? kind.validation : undefined;
        const text = await vscode.window.showInputBox({
          title,
          prompt: question.detail ?? undefined,
          placeHolder: kind.kind === 'text' ? (kind.placeholder ?? undefined) : undefined,
          value: question.default?.kind === 'text' ? question.default.text : undefined,
          password: kind.kind === 'secret',
          validateInput: validation ? (value) => this.client.validate(this.repository, validation, value).catch(() => null) : undefined,
        });
        return text === undefined ? null : { kind: 'text', text };
      }
      case 'pick':
      case 'search': {
        // A pick of a few choices that says more than the one line of a quick pick is asked as a dialog, with a button
        // for each choice, as **Resolve** and **Review** are (stage 4, area 4).
        if (kind.kind === 'pick' && question.detail?.includes('\n') && kind.choices.length <= 3) {
          const ordered = items(kind.choices, question);
          const chosen = await vscode.window.showInformationMessage(title, { modal: true, detail: question.detail }, ...ordered.map((item) => item.label));
          const button = ordered.find((item) => item.label === chosen);
          return button ? { kind: 'choice', identifier: button.identifier } : null;
        }
        const host = this.hooks.choices?.[question.key] ?? [];
        const own = host.map((choice, place) => ({ label: choice.label, description: choice.description, identifier: '', place }));
        const picked = await vscode.window.showQuickPick([...items(kind.choices, question).map((item) => ({ ...item, place: -1 })), ...own], {
          title,
          placeHolder: question.detail ?? undefined,
          matchOnDescription: kind.kind === 'search',
        });
        if (picked && picked.place >= 0) {
          await host[picked.place]?.run();
          return null;
        }
        return picked ? { kind: 'choice', identifier: picked.identifier } : null;
      }
      case 'pickMany': {
        const picked = await vscode.window.showQuickPick(items(kind.choices, question), {
          title,
          placeHolder: question.detail ?? undefined,
          canPickMany: true,
        });
        return picked ? { kind: 'choices', identifiers: picked.map((item) => item.identifier) } : null;
      }
      case 'confirm': {
        const chosen = await vscode.window.showWarningMessage(title, { modal: true, detail: question.detail ?? undefined }, kind.accept);
        return chosen === kind.accept ? { kind: 'confirmed' } : null;
      }
      case 'openFile':
      case 'saveFile': {
        const folder = kind.folder ? vscode.Uri.file(kind.folder) : undefined;
        const filters = Object.fromEntries(kind.filters.map((filter) => [filter.name, filter.extensions]));
        if (kind.kind === 'saveFile') {
          const where = await vscode.window.showSaveDialog({
            title,
            defaultUri: folder && kind.fileName ? vscode.Uri.joinPath(folder, kind.fileName) : folder,
            filters,
          });
          return where ? { kind: 'path', path: where.fsPath } : null;
        }
        const chosen = await vscode.window.showOpenDialog({ title, defaultUri: folder, filters, canSelectMany: false });
        return chosen?.[0] ? { kind: 'path', path: chosen[0].fsPath } : null;
      }
    }
  }

  async confirm(plan: Plan): Promise<boolean> {
    const ask = plan.danger.kind === 'destructive' ? vscode.window.showWarningMessage : vscode.window.showInformationMessage;
    const reason = plan.danger.kind === 'none' ? '' : `${plan.danger.reason}\n\n`;
    const detail = `${reason}Gitardo will run in ${this.repository}:\n\n${plan.steps.map((step) => step.line).join('\n')}`;
    if ((await ask(`${plan.title}?`, { modal: true, detail }, 'Run')) !== 'Run') return false;
    this.confirmed = plan.title;
    await this.hooks.confirmed?.(plan);
    return true;
  }
}

/** The choices of a question as items of a quick pick, the default first. */
function items(choices: Choice[], question: Question): Array<vscode.QuickPickItem & { identifier: string }> {
  const first = question.default?.kind === 'choice' ? question.default.identifier : undefined;
  // The choices a pick of many ticks first, such as the merged branches of a clean-up (stage 4, area 2).
  const ticked = question.default?.kind === 'choices' ? question.default.identifiers : [];
  const all = choices.map((choice) => ({
    label: choice.label,
    description: choice.description ?? undefined,
    identifier: choice.identifier,
    picked: ticked.includes(choice.identifier),
  }));
  return [...all.filter((item) => item.identifier === first), ...all.filter((item) => item.identifier !== first)];
}

/** What VS Code answers for the engine besides the questions of a feature: GitHub's sign-in and the language models (area 6). */
export type HostAnswers = Pick<EngineOptions, 'githubToken' | 'languageModel' | 'languageModels'>;

/** The engine of this window, started on the first call to `start`; none when its binary is missing. */
export class WindowEngine implements vscode.Disposable {
  private client?: EngineClient;
  private looked = false;
  private readonly subscriptions: vscode.Disposable[] = [];

  constructor(
    private readonly context: vscode.ExtensionContext,
    private readonly log: vscode.LogOutputChannel,
    private readonly answers: HostAnswers = {},
  ) {}

  /** Starts the engine the first time; the output channel says once when there is no binary for this system. */
  start(): EngineClient | undefined {
    if (this.looked) return this.client;
    this.looked = true;
    // Only the user's own settings may name another binary: a workspace's .vscode/settings.json comes with the repository.
    const setting = vscode.workspace.getConfiguration('gitardo').inspect<string>('engine.path')?.globalValue;
    const binary = findBinary(this.context.extensionPath, setting);
    if (!binary) {
      this.log.info(
        setting?.trim()
          ? `Gitardo's engine was not found at ${setting}, which gitardo.engine.path names; Gitardo works without it.`
          : `Gitardo's engine has no binary for ${process.platform}-${process.arch} in this package; Gitardo works without it.`,
      );
      return undefined;
    }
    const client = new EngineClient({
      binary,
      clientVersion: String((this.context.extension.packageJSON as { version?: string }).version ?? '0.0.0'),
      settings: () => readSettings(this.context.extension),
      logLevel: () => (this.log.logLevel <= vscode.LogLevel.Debug ? 'debug' : 'info'),
      log: this.log,
      ...this.answers,
    });
    this.client = client;
    // The features see the new values from their next run.
    this.subscriptions.push(
      vscode.workspace.onDidChangeConfiguration((event) => {
        if (event.affectsConfiguration('gitardo') || event.affectsConfiguration('git')) client.settingsChanged();
      }),
    );
    client.start().catch((error: unknown) => this.log.warn(error instanceof Error ? error.message : String(error)));
    return client;
  }

  /** The engine, when it runs or is starting: none when it has no binary, or stopped for good. */
  get running(): EngineClient | undefined {
    const client = this.start();
    return client && client.status !== 'given up' && client.status !== 'refused' ? client : undefined;
  }

  dispose(): void {
    for (const subscription of this.subscriptions) subscription.dispose();
    void this.client?.dispose();
  }
}
