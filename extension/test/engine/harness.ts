/**
 * What the tests of Gitardo's engine share: the debug binary, throwaway repositories, a client that logs to memory, an
 * asker that answers from the test, and, for the scenarios of the areas moved to the engine, a way to run the same
 * operation as a `GitOp` and as a feature and to compare what each left.
 */

import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import { afterEach } from 'vitest';
import { binaryName } from '../../src/engine/binary';
import { EngineClient, type EngineLog, type EngineOptions, type FeatureAsker } from '../../src/engine/client';
import type { Answer, Finished, Plan, Question, Settings, Target } from '../../src/engine/messages';
import { engineSettings } from '../../src/engine/settings';
import { git, writeFiles } from '../../src/git/exec';
import type { GitOp } from '../../src/git/ops';

export const binary = process.env.GITARDO_ENGINE ?? path.resolve(__dirname, '../../../rust/target/debug', binaryName());
export const built = fs.existsSync(binary);
if (!built) console.warn(`The engine's tests are skipped: ${binary} is not built. Run cargo build -p server in rust/.`);

const folders: string[] = [];
const clients: EngineClient[] = [];

/** Ends the engines started and deletes the folders made, after each test of the file that calls it. */
export function cleanUpAfterEach(): void {
  afterEach(async () => {
    await Promise.all(clients.splice(0).map((client) => client.dispose()));
    for (const folder of folders.splice(0)) fs.rmSync(folder, { recursive: true, force: true });
  });
}

/** A folder of its own for a test, by its real name, as git names it. */
export function folder(): string {
  const made = fs.realpathSync.native(fs.mkdtempSync(path.join(os.tmpdir(), 'gitardo-engine-')));
  folders.push(made);
  return made;
}

/** A new repository with a commit on `main`, or none. */
export async function repository(commit = true): Promise<string> {
  const root = path.join(folder(), 'repo');
  fs.mkdirSync(root);
  await git(root, ['init', '--initial-branch=main']);
  await git(root, ['config', 'user.name', 'Gitardo Test']);
  await git(root, ['config', 'user.email', 'test@gitardo.invalid']);
  await git(root, ['config', 'commit.gpgsign', 'false']);
  if (commit) await commitFile(root, 'a.txt', 'one\n', 'First commit');
  return root;
}

/** Writes a file and commits it. */
export async function commitFile(root: string, file: string, content: string, message: string): Promise<void> {
  fs.writeFileSync(path.join(root, file), content);
  await git(root, ['add', '--', file]);
  await git(root, ['commit', '--quiet', '-m', message]);
}

/** The lines the tests read of the log. */
export function memoryLog(): EngineLog & { lines: string[] } {
  const lines: string[] = [];
  const keep = (level: string) => (line: string) => lines.push(`${level} ${line}`);
  return { lines, info: keep('info'), warn: keep('warn'), error: keep('error'), debug: keep('debug') };
}

/**
 * A client of the debug binary, ended after the test; `host` answers the engine's questions to VS Code's sign-in and
 * language models in its place (area 6 of stage 4).
 */
export function client(
  log = memoryLog(),
  settings: Settings = engineSettings({}, new Set(), {}, 0),
  host: Pick<EngineOptions, 'githubToken' | 'languageModel' | 'languageModels'> = {},
): EngineClient {
  const started = new EngineClient({ binary, clientVersion: '0.0.0-test', settings: () => settings, logLevel: () => 'info', log, ...host });
  clients.push(started);
  return started;
}

/** An asker that answers by the key of each question, and keeps what it was asked and the plans it confirmed. */
export function answering(answers: Record<string, (question: Question) => Answer | null | Promise<Answer | null>>): FeatureAsker & {
  asked: Question[];
  confirmed: Plan[];
} {
  const asked: Question[] = [];
  const confirmed: Plan[] = [];
  return {
    asked,
    confirmed,
    async ask(question) {
      asked.push(question);
      const answer = answers[question.key];
      if (!answer) throw new Error(`No answer for ${question.key}`);
      return answer(question);
    },
    async confirm(plan) {
      confirmed.push(plan);
      return true;
    },
  };
}

export const text = (value: string) => (): Answer => ({ kind: 'text', text: value });
export const choice = (identifier: string) => (): Answer => ({ kind: 'choice', identifier });
export const confirmed = () => (): Answer => ({ kind: 'confirmed' });

/** Two copies of a repository made by `setup`, one for each path of a scenario. */
export async function twins(setup: (root: string) => Promise<void>): Promise<[string, string]> {
  const root = await repository();
  await setup(root);
  const copy = path.join(folder(), 'repo');
  fs.cpSync(root, copy, { recursive: true });
  return [root, copy];
}

/** A command as the engine writes the line of a step (`Step::log_line`), so that the two paths are compared by argument. */
export function stepLine(args: string[]): string {
  const quoted = (argument: string) =>
    argument === '' || /[\s"\\]/.test(argument) ? `"${argument.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"` : argument;
  return ['git', ...args.map(quoted)].join(' ');
}

/** Runs a `GitOp` as `Controller.run` does, and gives its commands as the engine would write their lines. */
export async function runGitOp(root: string, op: GitOp): Promise<string[]> {
  if (op.writes?.length) await writeFiles(root, op.writes);
  for (const [index, args] of op.commands.entries()) await git(root, args, { input: index === op.commands.length - 1 ? op.input : undefined });
  return op.commands.map(stepLine);
}

/** Runs a feature through the engine, its questions answered by key, and gives how it ended and the lines of its steps. */
export async function runFeature(
  root: string,
  feature: string,
  target: Target,
  answers: Record<string, (question: Question) => Answer | null> = {},
): Promise<{ finished: Finished; lines: string[] }> {
  const { finished } = await client().runFeature({ repository: root, feature, target }, answering(answers));
  return { finished, lines: finished.kind === 'ran' ? finished.plan.steps.map((step) => step.line) : [] };
}

/**
 * What a scenario compares of a repository once an operation ran: what `HEAD` and every reference point at, the
 * stashes with their messages, `git status`, and the contents of every file of the working tree but `.git`.
 */
export async function snapshot(root: string): Promise<Record<string, unknown>> {
  const out = async (args: string[]) => (await git(root, args)).stdout.trim();
  const contents: Record<string, string> = {};
  const walk = (folder: string) => {
    for (const entry of fs.readdirSync(folder, { withFileTypes: true })) {
      if (entry.name === '.git') continue;
      const full = path.join(folder, entry.name);
      if (entry.isDirectory()) walk(full);
      else contents[path.relative(root, full).split(path.sep).join('/')] = fs.readFileSync(full, 'utf8');
    }
  };
  walk(root);
  return {
    head: await out(['symbolic-ref', '-q', '--short', 'HEAD']).catch(() => 'detached'),
    // A commit made again has a new date, so commits are compared by their trees and messages.
    commits: await out(['log', '--format=%T %s', '-n', '5']).catch(() => ''),
    refs: await out(['for-each-ref', '--format=%(refname) %(tree) %(subject)', 'refs/heads', 'refs/tags']),
    stashes: await out(['stash', 'list', '--format=%gs %T']),
    index: await out(['ls-files', '--stage']),
    status: await out(['status', '--porcelain=v1', '--untracked-files=all']),
    contents,
  };
}
