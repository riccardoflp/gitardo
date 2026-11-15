import type { PullRequest } from '../shared/protocol';
import type { GitHubRepo } from '../shared/github';

export { compareRangeUrl, compareUrl, parseGitHubRemote, type GitHubRepo } from '../shared/github';

export const PULLS_QUERY = `query($owner: String!, $repo: String!) {
  repository(owner: $owner, name: $repo) {
    pullRequests(first: 100, states: OPEN, orderBy: { field: UPDATED_AT, direction: DESC }) {
      nodes {
        number
        title
        url
        isDraft
        headRefName
        headRepositoryOwner { login }
        commits(last: 1) { nodes { commit { statusCheckRollup { state } } } }
      }
    }
  }
}`;

interface PullNode {
  number: number;
  title: string;
  url: string;
  isDraft: boolean;
  headRefName: string;
  headRepositoryOwner: { login: string } | null;
  commits: { nodes: { commit: { statusCheckRollup: { state: string } | null } }[] };
}

export interface PullsResponse {
  data?: { repository: { pullRequests: { nodes: PullNode[] } } | null };
  errors?: { message: string }[];
}

const CHECK_STATES: Record<string, PullRequest['checks']> = {
  SUCCESS: 'success',
  FAILURE: 'failure',
  ERROR: 'failure',
  PENDING: 'pending',
  EXPECTED: 'pending',
};

/**
 * Open pull requests by branch name. Pull requests from forks are left out:
 * their branch names say nothing about the branches of this repository.
 */
export function mapPulls(response: PullsResponse, owner: string): Record<string, PullRequest> {
  if (response.errors?.length) throw new Error(response.errors[0].message);
  const pulls: Record<string, PullRequest> = {};
  for (const node of response.data?.repository?.pullRequests.nodes ?? []) {
    if (node.headRepositoryOwner?.login.toLowerCase() !== owner.toLowerCase()) continue;
    // The list is newest first, so the first pull request seen for a branch wins.
    if (Object.hasOwn(pulls, node.headRefName)) continue;
    const rollup = node.commits.nodes[0]?.commit.statusCheckRollup?.state;
    pulls[node.headRefName] = {
      number: node.number,
      title: node.title,
      url: node.url,
      draft: node.isDraft,
      checks: rollup ? (CHECK_STATES[rollup] ?? null) : null,
    };
  }
  return pulls;
}
