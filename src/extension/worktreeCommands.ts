import * as path from 'node:path';
import * as vscode from 'vscode';
import type { OpLog } from '../git/ops';

/*
 * The commands of gitardo.worktrees.setupCommands, run in a new worktree once Gitardo has made it and its links and
 * copies. Each is a task of VS Code of its own, run by the shell the terminal of VS Code runs tasks with, whichever it
 * is: the exit code of each comes from its process, and the next one starts only when it is 0. Chaining them in one
 * command line would need a syntax that PowerShell 5.1, cmd, bash and zsh do not share.
 */

/** Waits for the task to end, and gives the exit code of its process: undefined when it had none, as when it is stopped. */
function runTask(task: vscode.Task): Promise<number | undefined> {
  return new Promise((resolve, reject) => {
    // Kept from before the execution is known, for a command that ends before executeTask says it started.
    const codes = new Map<vscode.TaskExecution, number | undefined>();
    const ended = new Set<vscode.TaskExecution>();
    let execution: vscode.TaskExecution | undefined;
    const settle = () => {
      if (!execution || !ended.has(execution)) return;
      for (const listener of listeners) listener.dispose();
      resolve(codes.get(execution));
    };
    const listeners = [
      vscode.tasks.onDidEndTaskProcess((event) => void codes.set(event.execution, event.exitCode)),
      vscode.tasks.onDidEndTask((event) => {
        ended.add(event.execution);
        settle();
      }),
    ];
    vscode.tasks.executeTask(task).then(
      (started) => {
        execution = started;
        settle();
      },
      (error: unknown) => {
        for (const listener of listeners) listener.dispose();
        reject(error);
      },
    );
  });
}

/**
 * Runs the commands one after the other in the worktree in `dir`, made from the repository in `root`, each a task named
 * after the worktree, in a terminal of this window. Each goes to the log, as `run <command>`, before it starts. Returns
 * what stopped them, said for the user, or undefined when they all succeeded. Nothing runs in a workspace that is not
 * trusted, whose settings may come from a repository that was only cloned.
 */
export async function runSetupCommands(root: string, dir: string, commands: string[], log: OpLog): Promise<string | undefined> {
  const name = path.basename(dir);
  if (!vscode.workspace.isTrusted) {
    log.warn('The commands of gitardo.worktrees.setupCommands did not run: this workspace is not trusted.');
    return `The commands for new worktrees did not run in ${name}: this workspace is not trusted.`;
  }
  const scope = vscode.workspace.getWorkspaceFolder(vscode.Uri.file(root)) ?? vscode.TaskScope.Workspace;
  for (const [index, command] of commands.entries()) {
    log.info(`> run ${command}`);
    // The same definition for each command of a worktree, so that they follow one another in its terminal.
    const task = new vscode.Task({ type: 'gitardo', worktree: dir }, scope, name, 'Gitardo', new vscode.ShellExecution(command, { cwd: dir }), []);
    task.presentationOptions = {
      reveal: vscode.TaskRevealKind.Always,
      panel: vscode.TaskPanelKind.Dedicated,
      echo: true,
      focus: false,
      clear: false,
      showReuseMessage: false,
    };
    let code: number | undefined;
    try {
      code = await runTask(task);
    } catch (error) {
      log.warn(`${command}: ${error instanceof Error ? error.message : String(error)}`);
      code = undefined;
    }
    if (code === 0) continue;
    const how = code === undefined ? 'did not finish' : `failed with exit code ${code}`;
    log.warn(`${command} ${how}.`);
    const rest = commands.length - index - 1;
    const after = rest ? `, so the ${rest === 1 ? 'command after it' : `${rest} commands after it`} did not run` : '';
    return `${command} ${how} in ${name}${after}.`;
  }
  return undefined;
}
