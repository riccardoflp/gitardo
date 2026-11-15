/**
 * The `Target` of a feature of the engine from what a menu of the graph names: `HEAD`, a commit, or a branch, remote
 * branch or tag by its name, in full where another kind has the same name (`refArg` of `src/shared/refs.ts`).
 */
import type { Target } from './messages';

/** The names a target is looked for in. */
export interface NamedRefs {
  branches: ReadonlyArray<{ name: string }>;
  remoteBranches: ReadonlyArray<{ name: string }>;
  tags: ReadonlyArray<{ name: string }>;
}

/** What `at` names, for `feature/run`: a name not found is taken as a commit, as git would take it. */
export function targetOf(at: string, refs: NamedRefs): Target {
  if (at === 'HEAD') return { kind: 'head' };
  const full: Array<[string, 'branch' | 'remoteBranch' | 'tag']> = [
    ['refs/heads/', 'branch'],
    ['refs/remotes/', 'remoteBranch'],
    ['refs/tags/', 'tag'],
  ];
  for (const [prefix, kind] of full) if (at.startsWith(prefix)) return { kind, name: at.slice(prefix.length) };
  if (refs.branches.some((branch) => branch.name === at)) return { kind: 'branch', name: at };
  if (refs.remoteBranches.some((branch) => branch.name === at)) return { kind: 'remoteBranch', name: at };
  if (refs.tags.some((tag) => tag.name === at)) return { kind: 'tag', name: at };
  return { kind: 'commit', commit: at };
}
