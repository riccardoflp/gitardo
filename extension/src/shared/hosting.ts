/**
 * The hosting service of the main remote, as Gitardo's engine finds it (`hosting.detect-remote`), and the words the
 * views say of its pull requests: **merge request** and `!12` on GitLab, **pull request** and `#12` elsewhere.
 */

/** A hosting service, as the engine names it. */
export type HostingService = 'gitHub' | 'gitLab' | 'bitbucket' | 'azureDevOps' | 'gitea' | 'forgejo';

/** The hosting service of the main remote, as the views know it. */
export interface Hosting {
  service: HostingService;
  /** The host name of its pages, which its token is kept for. */
  host: string;
  /** The name of the main remote. */
  remote: string;
  /** Whether a pull request can be checked out: the service keeps a reference of each (`hosting.checkout-pull-request`). */
  canCheckOut: boolean;
  /** Whether its comparisons show only what one side has since the two diverged. */
  comparesSinceDiverged: boolean;
}

const NAMES: Record<HostingService, string> = {
  gitHub: 'GitHub',
  gitLab: 'GitLab',
  bitbucket: 'Bitbucket',
  azureDevOps: 'Azure DevOps',
  gitea: 'Gitea',
  forgejo: 'Forgejo',
};

/** Its name, as the menus say it: **Open on Azure DevOps**. */
export function serviceName(service: HostingService = 'gitHub'): string {
  return NAMES[service];
}

/** What its pull requests are called: `merge request` on GitLab, `pull request` elsewhere. */
export function pullWords(service: HostingService = 'gitHub'): string {
  return service === 'gitLab' ? 'merge request' : 'pull request';
}

/** The number of a pull request as the service writes it: `!12` on GitLab, `#12` elsewhere. */
export function pullNumber(number: number, service: HostingService = 'gitHub'): string {
  return `${service === 'gitLab' ? '!' : '#'}${number}`;
}

/** Whether Gitardo reads the open pull requests of the service, to show them on the labels: GitHub's and GitLab's. */
export function readsPullRequests(service: HostingService): boolean {
  return service === 'gitHub' || service === 'gitLab';
}
