/**
 * Ask Gitardo in VS Code's Chat (D76): the participant `@gitardo`, which asks the engine's `read/help.ask` of the model
 * chosen in the Chat and writes the answer as it comes, with a button for each feature it names that has a command;
 * and **Gitardo: Ask How To…**, which opens the Chat with `@gitardo ` typed. What they do is in `src/engine/help.ts`;
 * here is only what VS Code hands them.
 */
import * as vscode from 'vscode';
import { ErrorCode, EngineError } from '../engine/connection';
import { type ChatTurn, PARTICIPANT, answerInChat, askHowTo, earlierExchanges } from '../engine/help';
import { reads } from '../engine/reads';
import { useChatModel } from './ai';

/** The title of each command the extension contributes, as `package.json` gives it. */
function commandTitles(extension: vscode.Extension<unknown>): Record<string, string> {
  const json = extension.packageJSON as { contributes?: { commands?: Array<{ command: string; title: string }> } };
  return Object.fromEntries((json.contributes?.commands ?? []).map((each) => [each.command, each.title]));
}

/** The turns of the Chat's history with `@gitardo`, as questions and the Markdown of their answers. */
function turnsOf(history: vscode.ChatContext['history']): ChatTurn[] {
  return history.flatMap((turn): ChatTurn[] => {
    if (turn instanceof vscode.ChatRequestTurn) return [{ question: turn.prompt }];
    const markdown = turn.response.flatMap((part) => (part instanceof vscode.ChatResponseMarkdownPart ? [part.value.value] : []));
    return [{ answer: markdown.join('') }];
  });
}

/** The participant `@gitardo` and the command **Gitardo: Ask How To…**. */
export function registerAskGitardo(context: vscode.ExtensionContext): vscode.Disposable[] {
  const titles = commandTitles(context.extension);
  const handler: vscode.ChatRequestHandler = async (request, chatContext, stream, token) => {
    const aborted = new AbortController();
    const listener = token.onCancellationRequested(() => aborted.abort());
    const chosen = useChatModel(request.model);
    try {
      const client = reads.available ? reads.client() : undefined;
      const outcome = await answerInChat(
        { ask: client && ((params, onText, signal) => client.askHelp(params, onText, signal)), titles },
        { question: request.prompt, earlier: earlierExchanges(turnsOf(chatContext.history)), model: request.model.id },
        { markdown: (text) => stream.markdown(text), button: (button) => stream.button({ command: button.command, title: button.title }) },
        aborted.signal,
        (error) => error instanceof EngineError && error.code === ErrorCode.cancelled,
      );
      return outcome.error ? { errorDetails: { message: outcome.error } } : {};
    } finally {
      chosen.dispose();
      listener.dispose();
    }
  };
  const participant = vscode.chat.createChatParticipant(PARTICIPANT, handler);
  participant.iconPath = vscode.Uri.joinPath(context.extensionUri, 'media', 'icon.png');
  const command = vscode.commands.registerCommand('gitardo.askHowTo', () =>
    askHowTo({
      models: async () => (await vscode.lm.selectChatModels()).length,
      execute: (id, ...args) => vscode.commands.executeCommand(id, ...args),
      warn: (message) => void vscode.window.showWarningMessage(message),
    }),
  );
  return [participant, command];
}
