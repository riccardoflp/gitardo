/**
 * Ask Gitardo in VS Code's Chat (`help.ask`, `help.page`, `help.show-me`, D76): what the participant `@gitardo` writes
 * as the answer comes, the buttons under it, the earlier questions of the Chat it sends again, and what **Gitardo: Ask
 * How To…** does. The questions and the reading of the answer are the engine's (`read/help.ask`); only VS Code's side
 * is here, without VS Code, so that the tests run it.
 */
import type { FeatureTitle, HelpAnswerResult, HelpAskParams, HelpExchange, LanguageModelRequest } from './messages';
import { NO_ENGINE } from '../shared/protocol';

/** The identifier of the participant, as `chatParticipants` in `package.json` names it. */
export const PARTICIPANT = 'gitardo.ask';

/** What **Gitardo: Ask How To…** types in the box of the Chat. */
export const CHAT_QUERY = '@gitardo ';

/** What **Gitardo: Ask How To…** says when VS Code has no language model. */
export const NO_MODEL = 'Gitardo: No language model is available in VS Code. Install an extension that provides one, such as GitHub Copilot.';

/**
 * The command of the extension that does each feature, by the identifier of its card: under an answer, each feature it
 * names that is here has a button with the title of its command. A feature that is not, as `squash.commits`, done on
 * the commits chosen in the graph, has none.
 */
export const FEATURE_COMMANDS: Readonly<Record<string, string>> = {
  'stash.push': 'gitardo.stash',
  'sync.fetch': 'gitardo.fetch',
  'sync.pull': 'gitardo.pull',
  'sync.pull-with': 'gitardo.pull',
  'sync.push': 'gitardo.push',
  'sync.push-force-with-lease': 'gitardo.pushForceWithLease',
  'sync.push-force': 'gitardo.pushForce',
  'sync.sync': 'gitardo.sync',
  'undo.last': 'gitardo.undo',
  'log.show': 'gitardo.showLog',
  'changes.pane': 'gitardo.openChanges',
  'file-history.file': 'gitardo.fileHistory',
  'file-history.lines': 'gitardo.lineHistory',
  'blame.toggle-current-line': 'gitardo.toggleBlame',
  'blame.toggle-file': 'gitardo.toggleFileBlame',
  'blame.open-line-commit': 'gitardo.openLineCommit',
  'blame.reveal-line-commit': 'gitardo.revealLineCommit',
  'blame.previous': 'gitardo.blamePrevious',
  'blame.ignore-whitespace': 'gitardo.toggleBlameWhitespace',
  'blame.every-file': 'gitardo.toggleBlameEveryFile',
  'reflog.browse': 'gitardo.reflog',
  'branch.clean-up': 'gitardo.cleanupBranches',
  'branch.create': 'gitardo.createBranch',
  'identity.switch': 'gitardo.switchIdentity',
  'repo.select': 'gitardo.selectRepository',
  'remote.manage': 'gitardo.remotes',
  'bisect.end': 'gitardo.bisectReset',
  'compare.choose': 'gitardo.compare',
  'compare.with': 'gitardo.compareWith',
  'search.go-to-commit': 'gitardo.goToCommit',
  'commit-page.open': 'gitardo.openCommit',
  'conflicts.resolve-automatically': 'gitardo.resolveConflicts',
  'ai.resolve-conflicts': 'gitardo.resolveConflictsWithAI',
  'worktree.create': 'gitardo.createWorktree',
  'worktree.switch': 'gitardo.switchWorktree',
  'worktree-setup.page': 'gitardo.worktreeSetup',
  'lfs.page': 'gitardo.largeFiles',
  'submodule.add': 'gitardo.addSubmodule',
  'patch.apply': 'gitardo.applyPatch',
};

/** A button of the Chat: the command it runs and its words. */
export interface ChatButton {
  command: string;
  title: string;
}

/** What the participant writes in the Chat: `stream.markdown` and `stream.button` of VS Code. */
export interface ChatWriter {
  markdown(text: string): void;
  button(button: ChatButton): void;
}

/** A turn of the history of the Chat with `@gitardo`: a question, or the Markdown of an answer. */
export type ChatTurn = { question: string } | { answer: string };

/**
 * The buttons under an answer: one for each feature named that has a command, in the order named, each command once,
 * with the title `package.json` gives it.
 */
export function helpButtons(named: readonly string[], titles: Readonly<Record<string, string>>): ChatButton[] {
  const buttons: ChatButton[] = [];
  for (const feature of named) {
    const command = FEATURE_COMMANDS[feature];
    const title = command && titles[command];
    if (title && !buttons.some((button) => button.command === command)) buttons.push({ command, title });
  }
  return buttons;
}

/** The earlier questions of the Chat and their answers, oldest first: each question with the answer after it, if any. */
export function earlierExchanges(turns: readonly ChatTurn[]): HelpExchange[] {
  const exchanges: HelpExchange[] = [];
  for (const turn of turns) {
    if ('question' in turn) exchanges.push({ question: turn.question, answer: '' });
    else if (exchanges.length && !exchanges[exchanges.length - 1].answer) exchanges[exchanges.length - 1].answer = turn.answer;
  }
  return exchanges;
}

/**
 * The model's answer so far as the Chat writes it, as the engine reads the whole answer (`help.read-answer`): a code
 * fence opening it taken off, each `[[id]]` of a feature given written as its title in bold, any other taken out, and
 * what may still change held back: a `[[` or a `[` not closed yet, a line that may be the fence closing it, and the
 * spaces at the end.
 */
export function shownSoFar(text: string, features: readonly FeatureTitle[]): string {
  let rest = text.replace(/^\s*```[^\n]*\n/, '');
  if (/^\s*`{1,3}[^\n]*$/.test(rest)) return '';
  const last = rest.lastIndexOf('\n');
  if (/^\s*`{1,3}/.test(rest.slice(last + 1))) rest = rest.slice(0, last + 1);
  let shown = '';
  for (;;) {
    const start = rest.indexOf('[[');
    if (start < 0) break;
    shown += rest.slice(0, start);
    const end = rest.indexOf(']]', start + 2);
    if (end < 0) return shown.trimEnd();
    const feature = features.find((each) => each.id === rest.slice(start + 2, end).trim());
    if (feature) shown += `**${feature.title}**`;
    else if (shown.endsWith(' ') && /^[ .,;:)]/.test(rest.slice(end + 2))) shown = shown.slice(0, -1);
    rest = rest.slice(end + 2);
  }
  shown += rest.endsWith('[') ? rest.slice(0, -1) : rest;
  return shown.trimEnd();
}

/** What a question needs of the engine and of the window. */
export interface HelpAsking {
  /** `read/help.ask` of the engine; none when it has no binary for this system (D49). */
  ask?: (params: HelpAskParams, onText: (text: string, request: LanguageModelRequest) => void, signal: AbortSignal) => Promise<HelpAnswerResult>;
  /** The title of each command of `package.json`. */
  titles: Readonly<Record<string, string>>;
}

/** What answering a question came to, as the Chat takes it: nothing more, or an error under the answer. */
export interface HelpOutcome {
  error?: string;
}

/**
 * Answers a question of the Chat: asks it of the model chosen there, writes the answer as it comes, then the rest of
 * the answer the engine read and a button for each feature named that has a command. Without the engine it says so,
 * with **Show the log**; cancelled, it keeps what came.
 */
export async function answerInChat(
  asking: HelpAsking,
  params: HelpAskParams,
  writer: ChatWriter,
  signal: AbortSignal,
  isCancelled: (error: unknown) => boolean,
): Promise<HelpOutcome> {
  if (!asking.ask) {
    writer.markdown(NO_ENGINE);
    writer.button({ command: 'gitardo.showLog', title: 'Show the log' });
    return {};
  }
  let written = '';
  const write = (shown: string) => {
    if (shown.length <= written.length || !shown.startsWith(written)) return;
    writer.markdown(shown.slice(written.length));
    written = shown;
  };
  try {
    const answer = await asking.ask(params, (text, request) => write(shownSoFar(text, request.features ?? [])), signal);
    if (answer.kind === 'failed') return { error: answer.message };
    write(answer.markdown);
    for (const button of helpButtons(answer.named, asking.titles)) writer.button(button);
    return {};
  } catch (error) {
    if (isCancelled(error)) return {};
    return { error: error instanceof Error ? error.message : String(error) };
  }
}

/** What **Gitardo: Ask How To…** needs of VS Code. */
export interface AskHowToHost {
  /** How many language models VS Code has. */
  models(): Promise<number>;
  /** Runs a command of VS Code. */
  execute(command: string, ...args: unknown[]): Thenable<unknown> | Promise<unknown>;
  /** Shows a warning. */
  warn(message: string): void;
}

/** **Gitardo: Ask How To…**: opens the Chat with `@gitardo ` typed and not sent, or warns that VS Code has no model. */
export async function askHowTo(host: AskHowToHost): Promise<void> {
  if (!(await host.models())) {
    host.warn(NO_MODEL);
    return;
  }
  await host.execute('workbench.action.chat.open', { query: CHAT_QUERY, isPartialQuery: true });
}
