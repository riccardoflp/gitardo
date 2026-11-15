/**
 * Git for the tests, which make their throwaway repositories with it and look at what the engine left: the extension
 * itself runs no git (stage 10 of `docs/transition/06-roadmap.md`).
 */
import { spawn } from 'node:child_process';

export interface GitResult {
  stdout: string;
  stderr: string;
  code: number;
}

export class GitError extends Error {
  constructor(
    readonly args: string[],
    readonly result: GitResult,
  ) {
    super(result.stderr.trim() || result.stdout.trim() || `git ${args[0]} exited with code ${result.code}`);
  }
}

/** Runs `git <args>` in a folder, with `input` on its standard input; it fails with a `GitError` when git does. */
export function git(cwd: string, args: string[], options: { input?: string } = {}): Promise<GitResult> {
  return new Promise((resolve, reject) => {
    const child = spawn('git', args, { cwd, env: { ...process.env, GIT_TERMINAL_PROMPT: '0' }, windowsHide: true });
    const out: Buffer[] = [];
    const err: Buffer[] = [];
    child.stdout.on('data', (chunk: Buffer) => out.push(chunk));
    child.stderr.on('data', (chunk: Buffer) => err.push(chunk));
    child.on('error', reject);
    child.stdin.on('error', () => {});
    child.stdin.end(options.input ?? '');
    child.on('close', (code) => {
      const result = { stdout: Buffer.concat(out).toString('utf8'), stderr: Buffer.concat(err).toString('utf8'), code: code ?? -1 };
      if (result.code === 0) resolve(result);
      else reject(new GitError(args, result));
    });
  });
}
