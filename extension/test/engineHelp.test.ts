/**
 * Ask Gitardo through the engine (milestone M31, `read/help.ask`): a fake language model stands for the one chosen in
 * VS Code's Chat, and the participant's answer is what it writes in the Chat as the engine asks the two requests.
 */
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { type ChatButton, answerInChat } from '../src/engine/help';
import type { LanguageModelInfo, LanguageModelRequest, LanguageModelResult } from '../src/engine/messages';
import { built, cleanUpAfterEach, client, memoryLog } from './engine/harness';

cleanUpAfterEach();

type Manifest = { contributes: { commands: Array<{ command: string; title: string }> } };
const manifest = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8')) as Manifest;
const titles = Object.fromEntries(manifest.contributes.commands.map((each) => [each.command, each.title]));

/** Two models, the Chat's the second: the first request chooses `stash.push`, the second answers with it, in pieces. */
function chatModels() {
  const models: LanguageModelInfo[] = [
    { id: 'first', name: 'First model' },
    { id: 'chat-model', name: 'Chat model' },
  ];
  const asked: LanguageModelRequest[] = [];
  return {
    asked,
    languageModels: async () => models,
    languageModel: async (request: LanguageModelRequest, onText: ((text: string) => void) | undefined): Promise<LanguageModelResult> => {
      asked.push(request);
      const model = models.find((each) => each.id === request.model);
      if (!model) return { refused: 'Not the model of the Chat.' };
      const text = asked.length === 1 ? 'stash.push' : 'Run [[stash.push]] from the Command Palette.';
      onText?.(text.slice(0, 9));
      onText?.(text);
      return { text, model: model.name };
    },
  };
}

describe.skipIf(!built)('Ask Gitardo through the engine', { timeout: 60_000 }, () => {
  it('asks the model chosen in the Chat, not the first, and writes its answer as it comes with the button Stash Changes (help.ask, help.page)', async () => {
    const model = chatModels();
    const engine = client(memoryLog(), undefined, model);
    const written: string[] = [];
    const buttons: ChatButton[] = [];
    const outcome = await answerInChat(
      { titles, ask: (params, onText, signal) => engine.askHelp(params, onText, signal) },
      { question: 'how do I put my changes aside', earlier: [], model: 'chat-model' },
      { markdown: (text) => written.push(text), button: (button) => buttons.push(button) },
      new AbortController().signal,
      () => false,
    );
    expect(outcome).toEqual({});
    expect(model.asked.map((request) => [request.purpose, request.model, request.effort])).toEqual([
      ['help', 'chat-model', null],
      ['help', 'chat-model', null],
    ]);
    // Only the second request's text is written, as it comes, each feature as the bold title of its card.
    expect(written).toEqual(['Run', ' **Stash all changes** from the Command Palette.']);
    expect(buttons).toEqual([{ command: 'gitardo.stash', title: 'Stash Changes' }]);
  });
});
