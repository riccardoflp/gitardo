import * as vscode from 'vscode';
import type { Repo } from '../git/repo';
import { buildPrompt, cleanMessage } from '../shared/commitPrompt';
import { buildConflictPrompt, fitResolution, parseConflictAnswer, type ConflictAnswer, type ConflictQuestion } from '../shared/conflictPrompt';
import type { LanguageModelInfo, LanguageModelRequest, LanguageModelResult } from '../engine/messages';
import type { RepoState } from '../shared/protocol';

/**
 * Drafts a commit message with whichever language model the user has set up in VS Code.
 * It describes the staged changes, or all changes when nothing is staged, as the Commit button would commit them.
 */
export async function draftCommitMessage(repo: Repo, state: RepoState, token: vscode.CancellationToken): Promise<string> {
  const { staged, unstaged } = state.working;
  if (!staged.length && !unstaged.length) throw new Error('There are no changes to describe.');

  const [model] = await vscode.lm.selectChatModels();
  if (!model) {
    throw new Error('No language model is available in VS Code. Install an extension that provides one, such as GitHub Copilot.');
  }

  const useStaged = staged.length > 0;
  const diff = await repo.diffText(useStaged);
  const untracked = useStaged ? [] : unstaged.filter((file) => file.status === '?').map((file) => file.path);
  const subjects = (await repo.state(10)).rows.filter((row) => row.author).map((row) => row.subject);

  const messages = [vscode.LanguageModelChatMessage.User(buildPrompt(diff, untracked, subjects))];
  const options = { justification: 'Gitardo sends your uncommitted changes to the model to draft a commit message.' };
  const response = await model.sendRequest(messages, options, token);
  let text = '';
  for await (const chunk of response.text) text += chunk;
  return cleanMessage(text);
}

/** How much a model thinks before it answers, as VS Code names it in the configuration of a model. */
const EFFORTS = { low: 'low', medium: 'medium', high: 'high', xhigh: 'extra high', max: 'max' } as const;
type Effort = keyof typeof EFFORTS;

/** The model chosen in the settings to resolve conflicts, by its id, and how much it thinks: nothing for what VS Code does. */
function conflictSettings(): { id: string; effort: Effort | null } {
  const settings = vscode.workspace.getConfiguration('gitardo.conflicts');
  const effort = settings.get<string>('aiEffort') ?? '';
  return { id: settings.get<string>('aiModel') ?? '', effort: effort in EFFORTS ? (effort as Effort) : null };
}

/** The model the conflicts are asked of: the one chosen, or the first VS Code has. */
async function conflictModel(): Promise<vscode.LanguageModelChat> {
  const models = await vscode.lm.selectChatModels();
  if (!models.length) throw new Error('No language model is available in VS Code. Install an extension that provides one, such as GitHub Copilot.');
  const { id } = conflictSettings();
  const chosen = id ? models.find((model) => model.id === id) : models[0];
  if (!chosen) throw new Error(`The model chosen to resolve conflicts, ${id}, is not available in VS Code: choose another one.`);
  return chosen;
}

/** The model the conflicts are asked of and how much it thinks, as the review names it, or null when there is none. */
export async function conflictModelName(): Promise<string | null> {
  const { id, effort } = conflictSettings();
  const thinks = effort ? ` · ${EFFORTS[effort]} effort` : '';
  try {
    return `${(await conflictModel()).name}${thinks}`;
  } catch {
    return id ? `${id}, not available` : null;
  }
}

/** Asks which language model resolves conflicts, and how much it thinks, and keeps the answers in the settings. */
export async function chooseConflictModel(): Promise<void> {
  const models = await vscode.lm.selectChatModels();
  if (!models.length) {
    void vscode.window.showWarningMessage('Gitardo: no language model is available in VS Code. Install an extension that provides one, such as GitHub Copilot.');
    return;
  }
  const { id, effort } = conflictSettings();
  const mark = (current: boolean, label: string) => (current ? `$(check) ${label}` : label);
  const model = await vscode.window.showQuickPick(
    [
      { id: '', label: mark(!id, 'The first model of VS Code'), description: models[0].name },
      ...models.map((each) => ({ id: each.id, label: mark(each.id === id, each.name), description: `${each.vendor} · ${each.family}` })),
    ],
    { title: 'The model that resolves conflicts', placeHolder: 'Gitardo asks it about the conflicts the rules leave' },
  );
  if (!model) return;
  const thinks = await vscode.window.showQuickPick(
    [
      { effort: '', label: mark(!effort, 'As the model is set up in VS Code') },
      ...Object.entries(EFFORTS).map(([value, label]) => ({ effort: value, label: mark(value === effort, `${label[0].toUpperCase()}${label.slice(1)}`) })),
    ],
    { title: 'How much the model thinks before it answers', placeHolder: 'More is slower, and often better on hard conflicts. A model without such a setting ignores it.' },
  );
  if (!thinks) return;
  const settings = vscode.workspace.getConfiguration('gitardo.conflicts');
  await settings.update('aiModel', model.id || undefined, vscode.ConfigurationTarget.Global);
  await settings.update('aiEffort', thinks.effort || undefined, vscode.ConfigurationTarget.Global);
}

/**
 * Asks the language model chosen for conflicts how to resolve one of them. `onText` is called with the whole answer so
 * far, each time more of it comes.
 */
export async function askResolution(question: ConflictQuestion, token: vscode.CancellationToken, onText: (text: string) => void): Promise<ConflictAnswer> {
  const model = await conflictModel();
  const { effort } = conflictSettings();
  const messages = [vscode.LanguageModelChatMessage.User(buildConflictPrompt(question))];
  // The effort goes as VS Code sends it from the configuration of a model, which it takes the place of for this request.
  const options = {
    justification: 'Gitardo sends the conflicts of the merge, and the lines around them, to the model to propose how to resolve them.',
    ...(effort ? { configuration: { reasoningEffort: effort } } : {}),
  } as vscode.LanguageModelChatRequestOptions;
  const response = await model.sendRequest(messages, options, token);
  let text = '';
  for await (const chunk of response.text) {
    text += chunk;
    onText(text);
  }
  return { ...fitResolution(parseConflictAnswer(text), question), model: model.name };
}

// Area 6 of stage 4: what the engine asks of VS Code's language models, `host/languageModel` and
// `host/languageModels`. The prompts and the reading of the answers are the engine's; only the sending is here.

/** What VS Code shows the user when it asks whether Gitardo may use a model, by what the question is for. */
const JUSTIFICATIONS: Record<LanguageModelRequest['purpose'], string> = {
  commitMessage: 'Gitardo sends your uncommitted changes to the model to draft a commit message.',
  conflict: 'Gitardo sends the conflicts of the merge, and the lines around them, to the model to propose how to resolve them.',
};

/** The language models of VS Code, in its order, as the engine lists them. */
export async function languageModels(): Promise<LanguageModelInfo[]> {
  return (await vscode.lm.selectChatModels()).map((model) => ({ id: model.id, name: model.name }));
}

/**
 * Asks a language model of VS Code a question of the engine: the model of `request.model`, or the first VS Code has,
 * with the effort as the configuration of the request. `onText` is given the whole answer so far as it comes. What
 * keeps it from asking is said in the words the extension said it with before the engine asked.
 */
export async function answerLanguageModel(
  request: LanguageModelRequest,
  onText: ((text: string) => void) | undefined,
  signal: AbortSignal,
): Promise<LanguageModelResult> {
  const models = await vscode.lm.selectChatModels();
  if (!models.length) return { refused: 'No language model is available in VS Code. Install an extension that provides one, such as GitHub Copilot.' };
  const model = request.model ? models.find((each) => each.id === request.model) : models[0];
  if (!model) return { refused: `The model chosen to resolve conflicts, ${request.model}, is not available in VS Code: choose another one.` };
  const messages = request.messages.map((message) =>
    message.role === 'user' ? vscode.LanguageModelChatMessage.User(message.text) : vscode.LanguageModelChatMessage.Assistant(message.text),
  );
  const options = {
    justification: JUSTIFICATIONS[request.purpose],
    ...(request.effort ? { configuration: { reasoningEffort: request.effort } } : {}),
  } as vscode.LanguageModelChatRequestOptions;
  const source = new vscode.CancellationTokenSource();
  const cancel = () => source.cancel();
  signal.addEventListener('abort', cancel, { once: true });
  try {
    const response = await model.sendRequest(messages, options, source.token);
    let text = '';
    for await (const chunk of response.text) {
      text += chunk;
      onText?.(text);
    }
    return { text, model: model.name };
  } finally {
    signal.removeEventListener('abort', cancel);
    source.dispose();
  }
}
