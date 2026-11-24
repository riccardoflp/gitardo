/**
 * Choosing a commit: a branch or a tag from a list, the working tree, a comparison opened before, or anything git can
 * turn into a commit, typed: a hash, `HEAD~3`, `v1.0^`.
 */
import * as vscode from 'vscode';
import { type Repository, reads } from '../engine/reads';
import { type ComparedRef, type RepoState, WORKTREE } from '../shared/protocol';
import { isHash } from '../shared/refs';
import { checkedOutBranch } from '../shared/state';

/** The working tree, as a side of a comparison. */
export const WORKTREE_SIDE: ComparedRef = { name: 'Working tree', hash: WORKTREE };

/** A comparison opened before, to open again: its sides by the names they had, and the file or folder it showed. */
export interface RecentComparison {
  from: ComparedRef;
  to: ComparedRef;
  path?: string;
}

/** What can be a side of a comparison: a commit, or the working tree. */
export const isSide = (hash: unknown): hash is string => typeof hash === 'string' && (hash === WORKTREE || isHash(hash));

/** Lets the user choose a branch or a tag, or type anything git can turn into a commit: a hash, `HEAD~3`, `v1.0^`. */
export async function pickRevision(
  state: RepoState,
  repo: Repository,
  title: string,
  placeHolder: string,
  options: {
    /** The side chosen already, which is not offered again. */
    other?: ComparedRef;
    /** Offers the working tree too. */
    worktree?: boolean;
    /** Comparisons to open again, offered first. */
    recent?: RecentComparison[];
  } = {},
): Promise<ComparedRef | RecentComparison | undefined> {
  const { other, worktree, recent = [] } = options;
  type Item = vscode.QuickPickItem & { ref?: ComparedRef; recent?: RecentComparison };
  const group = (label: string, refs: { name: string; commit: string }[], icon: string): Item[] =>
    refs.length
      ? [
          { label, kind: vscode.QuickPickItemKind.Separator },
          ...refs.map((ref) => ({ label: `$(${icon}) ${ref.name}`, description: ref.commit.slice(0, 7), ref: { name: ref.name, hash: ref.commit } })),
        ]
      : [];
  const items: Item[] = recent.length
    ? [
        { label: 'Opened before', kind: vscode.QuickPickItemKind.Separator },
        ...recent.map((entry) => ({ label: `$(history) ${entry.from.name} ↔ ${entry.to.name}`, description: entry.path, recent: entry })),
        { label: '', kind: vscode.QuickPickItemKind.Separator },
      ]
    : [];
  items.push({ label: '$(edit) Enter a hash or a revision…', description: 'such as 1a2b3c4 or HEAD~3', alwaysShow: true });
  const head = state.head.commit;
  if (head) {
    const name = checkedOutBranch(state) ?? 'HEAD';
    items.push({ label: `$(target) ${name}`, description: `${head.slice(0, 7)} · checked out`, ref: { name, hash: head } });
  }
  if (worktree && other?.hash !== WORKTREE) {
    items.push({ label: `$(files) ${WORKTREE_SIDE.name}`, description: 'with the changes not committed', ref: WORKTREE_SIDE });
  }
  items.push(
    ...group(
      'Branches',
      state.branches.filter((branch) => !branch.isCheckedOut),
      'git-branch',
    ),
    ...group('Remote branches', state.remoteBranches, 'cloud'),
    ...group('Tags', state.tags, 'tag'),
  );
  // Comparing something with itself shows nothing.
  const offered = items.filter((item) => !item.ref || item.ref.name !== other?.name || item.ref.hash !== other.hash);
  const choice = await vscode.window.showQuickPick(offered, { title, placeHolder, matchOnDescription: true });
  if (!choice) return;
  return choice.recent ?? choice.ref ?? askRevision(repo, title);
}

/** Lets the user type a hash, or anything else git can turn into a commit, and says which commit that is. */
export async function askRevision(repo: Repository, title: string): Promise<ComparedRef | undefined> {
  const revision = (
    await vscode.window.showInputBox({
      title,
      prompt: 'A commit hash, or anything else git understands: HEAD~3, v1.0^, main@{yesterday}',
      validateInput: async (value) =>
        !value.trim() || (await reads.resolve(repo, value.trim())) ? undefined : 'Git does not know a commit by this name in this repository.',
    })
  )?.trim();
  if (!revision) return;
  // The box can be accepted before the last check has come back.
  const hash = await reads.resolve(repo, revision);
  if (!hash) {
    void vscode.window.showWarningMessage(`Gitardo: "${revision}" is not a commit of this repository.`);
    return;
  }
  return { name: isHash(revision) ? hash.slice(0, 7) : revision, hash };
}
