import { unpushedCommits, type Ahead } from '../shared/graph';
import type { RepoState } from '../shared/protocol';
import { nameOnRemote, pullOf, pushedName, worktreeOf } from '../shared/refs';
import { stashName, upstreamBranch, upstreamName } from '../shared/state';
import type { Label } from './CommitList';

/**
 * The branches, tags, stashes and bisect marks of each commit that has some, with the open pull request of a branch. A
 * branch and its upstream share one label while they are at the same commit, unless the settings keep them apart.
 */
export function buildLabels(state: RepoState): Map<string, Label[]> {
  const labels = new Map<string, Label[]>();
  const add = (hash: string, label: Label) => {
    const list = labels.get(hash);
    if (list) list.push(label);
    else labels.set(hash, [label]);
  };
  if (state.head.commit && !state.head.branch) add(state.head.commit, { kind: 'head', name: 'HEAD', current: true });
  const pullFor = (name: string | null): Label['pull'] => {
    const request = pullOf(name, state);
    return request && name !== null ? { request, branch: name } : undefined;
  };
  const remoteTips = new Map(state.remoteBranches.map((remote) => [remote.name, remote.commit]));
  // An upstream of the same name at the same commit as its branch is said on the label of the branch, not on one of its own.
  const merged = new Set<string>();
  for (const branch of state.branches) {
    const worktree = worktreeOf(branch.name, state)?.folder;
    const upstream = branch.upstream;
    const same = upstream && !upstream.isGone && upstreamBranch(upstream) === branch.name && remoteTips.get(upstreamName(upstream)) === branch.commit;
    const remote = state.combineUpstream && same ? upstreamName(upstream) : undefined;
    if (remote) merged.add(remote);
    add(branch.commit, { kind: 'branch', name: branch.name, current: branch.isCheckedOut, worktree, remote, pull: pullFor(pushedName(branch, state)) });
  }
  // The pull request of a remote branch is said on the local branch that tracks it, when there is one.
  const tracked = new Set(state.branches.map((branch) => branch.upstream && upstreamName(branch.upstream)));
  for (const remote of state.remoteBranches) {
    if (merged.has(remote.name)) continue;
    const pull = state.github && !tracked.has(remote.name) ? pullFor(nameOnRemote(remote.name, state)) : undefined;
    add(remote.commit, { kind: 'remote', name: remote.name, pull });
  }
  for (const tag of state.tags) add(tag.commit, { kind: 'tag', name: tag.name });
  for (const stash of state.stashes) add(stash.commit, { kind: 'stash', name: stashName(stash) });
  if (state.bisect) {
    const { good, bad, skipped } = state.bisect;
    for (const hash of good) add(hash, { kind: 'good', name: 'good' });
    if (bad) add(bad, { kind: 'bad', name: 'bad' });
    for (const hash of skipped) add(hash, { kind: 'skipped', name: 'skipped' });
  }
  return labels;
}

/** The commits of each local branch that its upstream has not yet, by hash, with the upstream they would be pushed to. */
export function unpushedOf(state: RepoState): Map<string, string> {
  const ahead: Ahead[] = [];
  for (const branch of state.branches) {
    const upstream = branch.upstream;
    const tip = upstream && !upstream.isGone && upstream.ahead ? state.remoteBranches.find((ref) => ref.name === upstreamName(upstream)) : undefined;
    if (upstream && tip) ahead.push({ tip: branch.commit, upstream: upstreamName(upstream), upstreamTip: tip.commit, ahead: upstream.ahead });
  }
  return ahead.length ? unpushedCommits(state.rows, ahead) : new Map();
}
