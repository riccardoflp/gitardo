/**
 * Ask Gitardo in VS Code's Chat (`help.ask`, `help.page`, `help.show-me`): what `@gitardo` writes as the answer comes,
 * the buttons under it and the commands they run, the earlier questions of the Chat, and **Gitardo: Ask How To…**.
 */
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import {
  CHAT_QUERY,
  FEATURE_COMMANDS,
  NO_MODEL,
  PARTICIPANT,
  type ChatButton,
  answerInChat,
  askHowTo,
  earlierExchanges,
  helpButtons,
  shownSoFar,
} from '../src/engine/help';
import type { HelpAnswerResult, LanguageModelRequest } from '../src/engine/messages';
import { NO_ENGINE } from '../src/shared/protocol';

interface Manifest {
  contributes: {
    commands: Array<{ command: string; title: string; category?: string }>;
    chatParticipants: Array<{ id: string; name: string; description: string }>;
  };
}

const manifest = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8')) as Manifest;
const titles = Object.fromEntries(manifest.contributes.commands.map((each) => [each.command, each.title]));

/** The identifiers of the features of the extension's guide, as `cargo xtask guide` wrote it. */
const guide = readFileSync(new URL('../../rust/crates/model/src/help/guide.txt', import.meta.url), 'utf8');
const guideFeatures = new Set([...guide.matchAll(/^@card extension (\S+)$/gm)].map((match) => match[1]));

/** What a question wrote in the Chat. */
function chat() {
  const written: string[] = [];
  const buttons: ChatButton[] = [];
  return { written, buttons, writer: { markdown: (text: string) => written.push(text), button: (button: ChatButton) => buttons.push(button) } };
}

/** A question to the model as the engine sends it, with the titles of the features its answer may name. */
const asked = (features = [{ id: 'stash.push', title: 'Stash all changes' }]): LanguageModelRequest => ({
  purpose: 'help',
  model: 'chat-model',
  effort: null,
  messages: [],
  read: 1,
  features,
});

describe('the table of the features and their commands (help.show-me)', () => {
  it('names only commands package.json contributes, and only features of the guide of the extension', () => {
    for (const [feature, command] of Object.entries(FEATURE_COMMANDS)) {
      expect(titles[command], `${feature} runs ${command}`).toBeDefined();
      expect(guideFeatures.has(feature), `${feature} is in the guide of the extension`).toBe(true);
    }
  });

  it('gives an answer naming stash.push the button of Gitardo: Stash Changes, and squash.commits, which has no command, none', () => {
    expect(helpButtons(['stash.push', 'squash.commits'], titles)).toEqual([{ command: 'gitardo.stash', title: 'Stash Changes' }]);
    expect(helpButtons(['squash.commits'], titles)).toEqual([]);
    // Two features of one command have one button.
    expect(helpButtons(['sync.pull', 'sync.pull-with'], titles)).toEqual([{ command: 'gitardo.pull', title: 'Pull' }]);
  });

  it('contributes the participant @gitardo and the command Gitardo: Ask How To… (help.page)', () => {
    const participant = { id: PARTICIPANT, name: 'gitardo', description: 'Ask how to do something with Gitardo' };
    expect(manifest.contributes.chatParticipants).toEqual([expect.objectContaining(participant)]);
    expect(manifest.contributes.commands).toContainEqual({ command: 'gitardo.askHowTo', title: 'Ask How To…', category: 'Gitardo' });
  });
});

describe('the answer as it comes (help.read-answer)', () => {
  const features = [{ id: 'squash.commits', title: 'Squash commits into one' }];

  it('writes a feature as the bold title of its card, and takes out one it was not given', () => {
    expect(shownSoFar('Select the commits, then [[squash.commits]].', features)).toBe('Select the commits, then **Squash commits into one**.');
    expect(shownSoFar('Use [[branch.made-up]] to do it.', features)).toBe('Use to do it.');
  });

  it('holds back a feature cut in half, a fence and the spaces at the end', () => {
    expect(shownSoFar('Select the commits, then [[squash.co', features)).toBe('Select the commits, then');
    expect(shownSoFar('Select the commits, then [', features)).toBe('Select the commits, then');
    expect(shownSoFar('```markdown\nSelect the commits', features)).toBe('Select the commits');
    expect(shownSoFar('Select the commits.\n```', features)).toBe('Select the commits.');
    expect(shownSoFar('``', features)).toBe('');
  });
});

describe('the earlier questions of the Chat (help.ask)', () => {
  it('pairs each question with the answer after it, oldest first', () => {
    expect(earlierExchanges([{ question: 'one' }, { answer: 'First.' }, { question: 'two' }, { question: 'three' }, { answer: 'Third.' }])).toEqual([
      { question: 'one', answer: 'First.' },
      { question: 'two', answer: '' },
      { question: 'three', answer: 'Third.' },
    ]);
  });
});

describe('a question of the Chat (help.ask, help.page)', () => {
  const never = () => false;

  it('writes the answer as it comes, the rest of the answer read, then its buttons', async () => {
    const { written, buttons, writer } = chat();
    const question = { question: 'how do I put my changes aside', earlier: [], model: 'chat-model' };
    const outcome = await answerInChat(
      {
        titles,
        ask: async (params, onText) => {
          expect(params).toEqual(question);
          onText('Run [[stash', asked());
          onText('Run [[stash.push]] from', asked());
          onText('Run [[stash.push]] from the Command Palette.', asked());
          return { kind: 'answered', markdown: 'Run **Stash all changes** from the Command Palette.', named: ['stash.push'], model: 'Chat model' };
        },
      },
      question,
      writer,
      new AbortController().signal,
      never,
    );
    expect(outcome).toEqual({});
    expect(written).toEqual(['Run', ' **Stash all changes** from', ' the Command Palette.']);
    expect(buttons).toEqual([{ command: 'gitardo.stash', title: 'Stash Changes' }]);
  });

  it('says what the model refused as the error of the answer, with no button', async () => {
    const { buttons, writer } = chat();
    const failed: HelpAnswerResult = { kind: 'failed', message: 'The model refused.' };
    const question = { question: 'q', earlier: [], model: null };
    const outcome = await answerInChat({ titles, ask: async () => failed }, question, writer, new AbortController().signal, never);
    expect(outcome).toEqual({ error: 'The model refused.' });
    expect(buttons).toEqual([]);
  });

  it('keeps what came when it is cancelled', async () => {
    const { written, writer } = chat();
    const cancelled = new Error('cancelled');
    const ask = async (_params: unknown, onText: (text: string, request: LanguageModelRequest) => void) => {
      onText('Open the graph.', asked([]));
      throw cancelled;
    };
    const question = { question: 'q', earlier: [], model: null };
    const outcome = await answerInChat({ titles, ask }, question, writer, new AbortController().signal, (error) => error === cancelled);
    expect(outcome).toEqual({});
    expect(written).toEqual(['Open the graph.']);
  });

  it('says that the engine is not available, with Show the log, when it has no binary (D49)', async () => {
    const { written, buttons, writer } = chat();
    await answerInChat({ titles }, { question: 'q', earlier: [], model: null }, writer, new AbortController().signal, never);
    expect(written).toEqual([NO_ENGINE]);
    expect(buttons).toEqual([{ command: 'gitardo.showLog', title: 'Show the log' }]);
  });
});

describe('Gitardo: Ask How To… (help.page)', () => {
  it('opens the Chat with @gitardo in its box, not sent', async () => {
    const executed: unknown[][] = [];
    const warned: string[] = [];
    await askHowTo({ models: async () => 1, execute: async (...args) => executed.push(args), warn: (message) => warned.push(message) });
    expect(executed).toEqual([['workbench.action.chat.open', { query: CHAT_QUERY, isPartialQuery: true }]]);
    expect(CHAT_QUERY).toBe('@gitardo ');
    expect(warned).toEqual([]);
  });

  it('warns when VS Code has no language model, and opens nothing', async () => {
    const executed: unknown[][] = [];
    const warned: string[] = [];
    await askHowTo({ models: async () => 0, execute: async (...args) => executed.push(args), warn: (message) => warned.push(message) });
    expect(warned).toEqual(['Gitardo: No language model is available in VS Code. Install an extension that provides one, such as GitHub Copilot.']);
    expect(warned).toEqual([NO_MODEL]);
    expect(executed).toEqual([]);
  });
});
