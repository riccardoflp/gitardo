/**
 * The scenarios of area 6 moved to Gitardo's engine in stage 4 of `docs/transition/06-roadmap.md`: AI and GitHub. A
 * fake language model stands for VS Code's; each question is asked as the engine asks it with `host/languageModel`, and
 * what it sends to the model, and what it makes of the answer, must be what the extension sent and made of it, but
 * where `docs/transition/04-differences.md` says why not (G1 to G9).
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import type { LanguageModelInfo, LanguageModelRequest, LanguageModelResult } from '../src/engine/messages';
import { conflictPlans } from '../src/engine/conflictPlans';
import { Reads } from '../src/engine/reads';
import { engineSettings, extensionKey } from '../src/engine/settings';
import { git } from './git';
import { answering, built, choice, cleanUpAfterEach, client, commitFile, memoryLog, repository } from './engine/harness';

cleanUpAfterEach();

/** A language model that answers every question with the same text, in pieces, and keeps what it was asked. */
function fakeModel(answer: string, models: LanguageModelInfo[] = [{ id: 'fast', name: 'Fast model' }]) {
  const asked: LanguageModelRequest[] = [];
  const shown: string[] = [];
  return {
    asked,
    shown,
    languageModels: async () => models,
    languageModel: async (request: LanguageModelRequest, onText: ((text: string) => void) | undefined): Promise<LanguageModelResult> => {
      asked.push(request);
      if (!models.length) return { refused: 'No language model is available in VS Code.' };
      const model = request.model ? models.find((each) => each.id === request.model) : models[0];
      if (!model) return { refused: `The model chosen to resolve conflicts, ${request.model}, is not available in VS Code: choose another one.` };
      const half = Math.floor(answer.length / 2);
      onText?.(answer.slice(0, half));
      shown.push(answer.slice(0, half));
      onText?.(answer);
      shown.push(answer);
      return { text: answer, model: model.name };
    },
  };
}

/** The settings sent to the engine, with these settings of the extension. */
const settingsOf = (values: Record<string, unknown> = {}) => engineSettings(values, new Set(), {}, 0);

/** The subjects of the last commits, as the extension takes them for a draft. */
async function subjects(root: string): Promise<string[]> {
  return (await git(root, ['log', '-n', '10', '--format=%s'])).stdout.trim().split('\n');
}

/** The conflicts of `e.txt` as the review reads them through the engine. */
async function planOf(root: string) {
  const [plan] = await conflictPlans(new Reads(() => client()), { root }, ['e.txt']);
  return plan;
}

/** A repository where `b.txt` and `c.txt` are committed, `b.txt` changed and staged, `c.txt` changed only. */
async function changed(root: string): Promise<void> {
  await commitFile(root, 'b.txt', 'two\n', 'Add b');
  await commitFile(root, 'c.txt', 'three\n', 'Add c');
  fs.writeFileSync(path.join(root, 'b.txt'), 'two, changed\n');
  await git(root, ['add', '--', 'b.txt']);
  fs.writeFileSync(path.join(root, 'c.txt'), 'three, changed\n');
}

/** A repository whose merge of `aside` stopped on a conflict in `e.txt`, between lines the model is shown around it. */
async function conflicted(root: string): Promise<void> {
  const file = (middle: string) => `function start() {\n  before();\n  ${middle}\n  after();\n}\n`;
  await commitFile(root, 'e.txt', file('base();'), 'Add e');
  await git(root, ['checkout', '--quiet', '-b', 'aside']);
  await commitFile(root, 'e.txt', file('theirs();'), 'Call theirs');
  await git(root, ['checkout', '--quiet', 'main']);
  await commitFile(root, 'e.txt', file('ours();'), 'Call ours');
  await git(root, ['merge', '--no-edit', 'aside']).catch(() => undefined);
}

/** The parts of a question between their tags: what the file and the conflict are, whatever the words around them. */
function parts(prompt: string): Record<string, string> {
  const found: Record<string, string> = {};
  for (const tag of ['before', 'base', 'current', 'incoming', 'after']) {
    const match = new RegExp(`<${tag}>\\n([\\s\\S]*?)</${tag}>`).exec(prompt);
    if (match) found[tag] = match[1];
  }
  return found;
}

describe.skipIf(!built)('AI and GitHub through the engine', { timeout: 60_000 }, () => {
  it('drafts a commit message from the staged diff and the last subjects, and cleans the answer (commit.draft-message)', async () => {
    const root = await repository();
    await changed(root);
    const model = fakeModel('```text\nChange b\n```');
    const engine = client(memoryLog(), settingsOf({ 'conflicts.aiModel': 'fast', 'conflicts.aiEffort': 'high' }), model);
    const wanted = await subjects(root);
    const drafted = await engine.draftMessage(root, wanted);
    expect(model.asked).toHaveLength(1);
    expect(model.asked[0].messages).toHaveLength(1);
    const [asked] = model.asked[0].messages;
    expect(asked.role).toBe('user');
    expect(asked.text).toContain('+two, changed');
    expect(asked.text).not.toContain('three, changed');
    for (const subject of wanted) expect(asked.text).toContain(subject);
    // The draft is asked of VS Code's first model, as the extension asked it: the model chosen is the conflicts' (G4).
    expect(model.asked[0]).toMatchObject({ purpose: 'commitMessage', model: null, effort: null });
    expect(drafted).toEqual({ kind: 'drafted', message: 'Change b' });
  });

  it('drafts from every change with the untracked files when nothing is staged, and asks nothing with no change', async () => {
    const root = await repository();
    await changed(root);
    await git(root, ['reset', '--quiet']);
    fs.writeFileSync(path.join(root, 'new.txt'), 'new\n');
    const model = fakeModel('Change b and c');
    const engine = client(memoryLog(), settingsOf(), model);
    const wanted = await subjects(root);
    await engine.draftMessage(root, wanted);
    const { text } = model.asked[0].messages[0];
    expect(text).toContain('+two, changed');
    expect(text).toContain('+three, changed');
    expect(text).toContain('new.txt');

    const clean = await repository();
    const nothing = fakeModel('Unused');
    expect(await client(memoryLog(), settingsOf(), nothing).draftMessage(clean, [])).toEqual({
      kind: 'failed',
      message: 'There are no changes to describe.',
    });
    expect(nothing.asked).toEqual([]);
  });

  it('says what VS Code says when it has no model, for the draft and for the choice of a model', async () => {
    const root = await repository();
    await changed(root);
    const none = fakeModel('Unused', []);
    const engine = client(memoryLog(), settingsOf(), none);
    expect(await engine.draftMessage(root, [])).toEqual({ kind: 'failed', message: 'No language model is available in VS Code.' });
    const { finished } = await engine.runFeature({ repository: root, feature: 'ai.choose-model', target: { kind: 'head' } }, answering({}));
    expect(finished).toMatchObject({
      kind: 'refused',
      message: 'No language model is available in VS Code. Install an extension that provides one, such as GitHub Copilot.',
    });
  });

  it('asks about a conflict with the parts of the file around it and makes a proposal of the answer (ai.ask-conflict)', async () => {
    const root = await repository();
    await conflicted(root);
    const plan = await planOf(root);
    const conflict = plan.conflicts[0];
    // The model wrote the lines before the conflict again, as small models do: both take them off.
    const answer = '<resolution>\nfunction start() {\n  before();\n  ours();\n  theirs();\n</resolution>\n<reason>Both calls are kept.</reason>\n<confidence>high</confidence>';
    const model = fakeModel(answer, [
      { id: 'fast', name: 'Fast model' },
      { id: 'chosen', name: 'Chosen model' },
    ]);
    const engine = client(memoryLog(), settingsOf({ 'conflicts.aiModel': 'chosen', 'conflicts.aiEffort': 'high' }), model);
    const shown: string[] = [];
    const lines = plan.text.split(/(?<=\n)/).map((line) => line.replace(/\r?\n$/, ''));
    const sideLines = (text: string) => (text ? text.replace(/\r?\n$/, '').split(/\r?\n/) : []);
    const asked = {
      start: conflict.from,
      end: conflict.to,
      current: sideLines(conflict.ours),
      incoming: sideLines(conflict.theirs),
      base: conflict.stretch ? sideLines(conflict.stretch.base) : null,
    };
    const engineAnswer = await engine.askConflict(root, { path: plan.path, lines, conflict: asked }, (text) => shown.push(text));

    const enginePrompt = model.asked[0].messages[0].text;
    // The parts of the file sent: what is before the conflict, the base and each side, and what is after.
    expect(parts(enginePrompt)).toEqual({
      before: 'function start() {\n  before();\n',
      base: '  base();\n',
      current: '  ours();\n',
      incoming: '  theirs();\n',
      after: '  after();\n}\n',
    });
    expect(enginePrompt).toContain('Resolve one merge conflict in the file e.txt.');
    expect(model.asked[0]).toMatchObject({ purpose: 'conflict', model: 'chosen', effort: 'high' });
    // What the model wrote so far is shown where the review asked, as it comes.
    expect(shown).toEqual(model.shown);

    expect(engineAnswer).toEqual({ kind: 'answered', model: 'Chosen model', lines: ['  ours();', '  theirs();'], reason: 'Both calls are kept.', confidence: 'high' });
  });

  it('fails a conflict asked of a model VS Code does not have in the words the extension said it with', async () => {
    const root = await repository();
    await conflicted(root);
    const plan = await planOf(root);
    const conflict = plan.conflicts[0];
    const engine = client(memoryLog(), settingsOf({ 'conflicts.aiModel': 'gone' }), fakeModel('Unused'));
    const answer = await engine.askConflict(
      root,
      { path: plan.path, lines: plan.text.split('\n'), conflict: { start: conflict.from, end: conflict.to, current: ['  ours();'], incoming: ['  theirs();'], base: null } },
      () => {},
    );
    expect(answer).toEqual({
      kind: 'failed',
      message: 'The model chosen to resolve conflicts, gone, is not available in VS Code: choose another one.',
      stopsTheRest: false,
    });
  });

  it('stops the model when the review stops asking: the engine cancels its question to the extension', async () => {
    const root = await repository();
    await conflicted(root);
    const plan = await planOf(root);
    const conflict = plan.conflicts[0];
    let stopped: (() => void) | undefined;
    const asked = new Promise<void>((resolve) => {
      stopped = resolve;
    });
    let modelSignal: AbortSignal | undefined;
    const engine = client(memoryLog(), settingsOf(), {
      languageModel: (_request, _onText, signal) => {
        modelSignal = signal;
        stopped?.();
        return new Promise((resolve) => signal.addEventListener('abort', () => resolve({ refused: 'Cancelled.' })));
      },
    });
    const aborted = new AbortController();
    const answer = engine.askConflict(
      root,
      { path: plan.path, lines: plan.text.split('\n'), conflict: { start: conflict.from, end: conflict.to, current: ['  ours();'], incoming: ['  theirs();'], base: null } },
      () => {},
      aborted.signal,
    );
    await asked;
    aborted.abort();
    await expect(answer).rejects.toMatchObject({ code: 7 });
    await expect.poll(() => modelSignal?.aborted).toBe(true);
  });

  it('chooses the model among VS Code’s and leaves the two settings for the extension to write (ai.choose-model)', async () => {
    const root = await repository();
    const model = fakeModel('Unused', [
      { id: 'fast', name: 'Fast model' },
      { id: 'chosen', name: 'Chosen model' },
    ]);
    const engine = client(memoryLog(), settingsOf(), model);
    const asker = answering({ model: choice('model:chosen'), effort: choice('high') });
    const { finished, followUps } = await engine.runFeature({ repository: root, feature: 'ai.choose-model', target: { kind: 'head' } }, asker);
    expect(finished.kind).toBe('ran');
    const picks = asker.asked.find((question) => question.key === 'model');
    expect(picks?.kind.kind === 'pick' && picks.kind.choices.map((each) => each.label)).toEqual(['The first model', 'Fast model', 'Chosen model']);
    expect(followUps).toEqual([
      { kind: 'writeSetting', key: 'ai.model', value: 'chosen', target: 'user' },
      { kind: 'writeSetting', key: 'ai.effort', value: 'high', target: 'user' },
    ]);
    expect(followUps.map((followUp) => extensionKey(String((followUp as { key?: unknown }).key)))).toEqual(['conflicts.aiModel', 'conflicts.aiEffort']);
  });

  it('loads the pull requests with VS Code’s token, signing in when asked, and backs out with the user (github.load-pull-requests)', async () => {
    const root = await repository();
    await git(root, ['remote', 'add', 'origin', 'https://github.com/owner/app.git']);
    const asks: boolean[] = [];
    const engine = client(memoryLog(), settingsOf(), {
      githubToken: async (ask) => {
        asks.push(ask);
        return null;
      },
    });
    const { finished } = await engine.runFeature({ repository: root, feature: 'github.load-pull-requests', target: { kind: 'head' } }, answering({}));
    // No input box for a token: the user backed out of VS Code's sign-in (G4).
    expect(finished).toEqual({ kind: 'backedOut' });
    expect(asks).toEqual([true]);
  });
});
