/** Where a repository is on GitHub, from the URL of a remote, and the addresses of its pages. */
export interface GitHubRepo {
  owner: string;
  repo: string;
}

/** Recognizes the HTTPS, SSH and scp-like forms of a github.com remote URL. */
export function parseGitHubRemote(url: string): GitHubRepo | null {
  const match = /^(?:https?:\/\/(?:[^@/]+@)?github\.com\/|(?:ssh:\/\/)?git@github\.com[:/])([^/\s]+)\/([^/\s]+?)(?:\.git)?\/?$/.exec(url.trim());
  return match ? { owner: match[1], repo: match[2] } : null;
}

export function compareUrl({ owner, repo }: GitHubRepo, branch: string): string {
  const encoded = branch.split('/').map(encodeURIComponent).join('/');
  return `https://github.com/${owner}/${repo}/compare/${encoded}?expand=1`;
}

/** The page of GitHub that compares two branches, tags or commits: from where they diverged with `...`, directly with `..`. */
export function compareRangeUrl({ owner, repo }: GitHubRepo, from: string, to: string, dots: '...' | '..'): string {
  const encode = (ref: string) => ref.split('/').map(encodeURIComponent).join('/');
  return `https://github.com/${owner}/${repo}/compare/${encode(from)}${dots}${encode(to)}`;
}
