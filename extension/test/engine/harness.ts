/**
 * What the tests of Gitardo's engine share: the debug binary, throwaway repositories, a client that logs to memory, an
 * asker that answers from the test, a way to run a feature with its questions answered, and the state of a repository
 * as the extension reads it through the engine.
 */

import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import { afterEach } from 'vitest';
import { binaryName } from '../../src/engine/binary';
import { EngineClient, type EngineLog, type EngineOptions, type FeatureAsker } from '../../src/engine/client';
import { graphState, readCommits } from '../../src/engine/graph';
import type { Answer, Finished, Plan, Question, Settings, Target } from '../../src/engine/messages';
import { Reads } from '../../src/engine/reads';
import { engineSettings } from '../../src/engine/settings';
import type { RepoState } from '../../src/shared/protocol';
import { git } from '../git';

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
  // No maintenance behind a commit: git starts it apart from the command, and it writes and removes files in the
  // objects while a test copies the repository or compares it with another.
  await git(root, ['config', 'maintenance.auto', 'false']);
  await git(root, ['config', 'gc.auto', '0']);
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

/** A command as the engine writes the line of a step (`Step::log_line`), for a test to say the line it expects. */
export function stepLine(args: string[]): string {
  const quoted = (argument: string) =>
    argument === '' || /[\s"\\]/.test(argument) ? `"${argument.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"` : argument;
  return ['git', ...args.map(quoted)].join(' ');
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
 * What a test compares of a repository before and after an operation: what `HEAD` and every reference point at, the
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

/** The state of a repository as the controller reads it through the engine, with the first `limit` commits of the graph. */
export async function engineState(root: string, limit = 10): Promise<RepoState> {
  const engine = client();
  const reads = new Reads(() => engine);
  const asked = { limit, filter: null, scope: null, stashes: true, firstParent: false };
  return graphState(engine, root, await reads.parts({ root }), asked, (graph) => readCommits(engine, root, graph, limit));
}
