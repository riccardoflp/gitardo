import * as vscode from 'vscode';

const SCOPES = ['repo'];

/** The user's GitHub session from VS Code. Only asks them to sign in when `prompt` is set. */
export async function githubSession(prompt: boolean): Promise<vscode.AuthenticationSession | undefined> {
  return vscode.authentication.getSession('github', SCOPES, prompt ? { createIfNone: true } : { silent: true });
}

/**
 * The token of the user's GitHub session, for the engine (`host/githubToken`); asks them to sign in when `ask` is set.
 * Null when there is none, or they backed out of signing in (area 6 of stage 4).
 */
export async function githubToken(ask: boolean): Promise<string | null> {
  try {
    return (await githubSession(ask))?.accessToken ?? null;
  } catch {
    // VS Code fails the sign-in the user closed.
    return null;
  }
}
