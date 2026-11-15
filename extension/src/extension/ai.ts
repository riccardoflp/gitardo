import * as vscode from 'vscode';
import type { LanguageModelInfo, LanguageModelRequest, LanguageModelResult } from '../engine/messages';

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
