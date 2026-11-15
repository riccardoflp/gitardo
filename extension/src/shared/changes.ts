import type { FileChange, WorkingTree } from './protocol';

/** Every file that is not committed, each once, the files in conflict first: as many as the row of the changes says. */
export function changedPaths(working: WorkingTree): string[] {
  return [...new Set([...working.conflicted.map((file) => file.path), ...working.staged.map((file) => file.path), ...working.unstaged.map((file) => file.path), ...working.untracked])];
}

/**
 * A changed file as the Changes view lists it: what it is next to HEAD, how much of that is staged, and whether git
 * does not track it, which the view marks `?`.
 */
export interface WorkingChange {
  file: FileChange;
  staged: 'all' | 'some' | 'none';
  untracked: boolean;
}

/** A file with changes both in the index and in the working tree, as the working tree is next to HEAD. */
function againstHead(staged: FileChange, unstaged: FileChange): FileChange {
  // Taken out of the index, and still on disk: HEAD has it, and so does the working tree.
  if (staged.kind === 'deleted') return { path: staged.path, originalPath: null, kind: 'modified' };
  // Deleted from disk since it was staged. A file added to the index only was never in HEAD: it stays an addition.
  if (unstaged.kind === 'deleted' && staged.kind !== 'added') return { ...staged, kind: 'deleted' };
  return staged;
}

/**
 * The staged, the unstaged and the untracked files in one list, each once, in the order of their paths. Files in
 * conflict are left out.
 */
export function workingChanges(working: WorkingTree): WorkingChange[] {
  // An untracked file is a file the working tree added. One the index deleted can be untracked too, left on disk.
  const unstaged = new Map<string, WorkingChange>([
    ...working.unstaged.map((file): [string, WorkingChange] => [file.path, { file, staged: 'none', untracked: false }]),
    ...working.untracked.map((path): [string, WorkingChange] => [path, { file: { path, originalPath: null, kind: 'added' }, staged: 'none', untracked: true }]),
  ]);
  const changes: WorkingChange[] = [];
  for (const file of working.staged) {
    const rest = unstaged.get(file.path);
    if (!rest) {
      changes.push({ file, staged: 'all', untracked: false });
      continue;
    }
    unstaged.delete(file.path);
    changes.push({ file: againstHead(file, rest.file), staged: 'some', untracked: false });
  }
  changes.push(...unstaged.values());
  return changes.sort((one, other) => (one.file.path < other.file.path ? -1 : one.file.path > other.file.path ? 1 : 0));
}
