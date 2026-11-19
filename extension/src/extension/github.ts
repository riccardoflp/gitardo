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

/** The key of the secret storage that keeps the token of a GitLab, by its host name (`hosting.token`). */
export function hostingTokenKey(host: string): string {
  return `hosting.token:${host.toLowerCase()}`;
}

/**
 * The answer to `host/githubToken`, which names the host whose token the engine wants: VS Code's sign-in for github.com;
 * for a GitLab, the token kept in the secret storage of the extension for its host name, or, when `ask` is set and none
 * is kept, one asked in an input box that hides it, then kept (`hosting.token`, Q1159). Null when there is none, or the
 * user backed out.
 */
export function tokenOfHost(secrets: vscode.SecretStorage): (ask: boolean, host: string) => Promise<string | null> {
  return async (ask, host) => {
    if (host === 'github.com') return githubToken(ask);
    const kept = await secrets.get(hostingTokenKey(host));
    if (kept || !ask) return kept ?? null;
    const typed = await vscode.window.showInputBox({
      title: 'GitLab token',
      prompt: `A personal access token of ${host} with the scope read_api`,
      password: true,
      ignoreFocusOut: true,
      validateInput: (value) => (value.trim() ? null : 'Enter a token.'),
    });
    const token = typed?.trim();
    if (!token) return null;
    await secrets.store(hostingTokenKey(host), token);
    return token;
  };
}
