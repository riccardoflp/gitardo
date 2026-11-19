/**
 * What a rewrite changed, commit by commit (`range-diff.compare`, `range-diff.page`, milestone M36): the shapes of
 * `read/range-diff.read-pairs` and of the follow-up that opens its tab (protocol 1.14,
 * `rust/crates/protocol/src/range_diff.rs`), and what the views decide from the state: which branches the menus offer
 * it on, and which upstream a fetch found force pushed.
 */
import type { Branch } from './state';

/** One line of commits: from the commit it starts after to the one it ends at. */
export interface RangeDiffLine {
  from: string;
  to: string;
}

/** The two lines compared, and the arguments of `git range-diff` that compare them. */
export interface RangeDiffRanges {
  arguments: string[];
  before: RangeDiffLine;
  now: RangeDiffLine;
}

/** What the tab of what a rewrite changed shows, as the follow-up `openRangeDiff` of the engine opens it. */
export interface RangeDiffTarget {
  /** **What the rebase of feature changed**. */
  title: string;
  /** The branch, which names the tab: `feature · what changed`. */
  branch: string;
  ranges: RangeDiffRanges;
}

export type RangeDiffPairKind = 'same' | 'changed' | 'added' | 'dropped';

/** One pair of commits: the same commit made again, changed, added or dropped. */
export interface RangeDiffPair {
  kind: RangeDiffPairKind;
  /** The commit before, null for one added. */
  before: string | null;
  /** The commit now, null for one dropped. */
  now: string | null;
  /** The subject now, or before for one dropped. */
  subject: string;
  messageChanged: boolean;
  changesChanged: boolean;
  /** Its diff of diffs, each line with its outer mark; empty unless it changed. */
  diff: string[];
}

/** The answer of `read/range-diff.read-pairs`. */
export interface RangeDiffPairs {
  pairs: RangeDiffPair[];
  beforeCount: number;
  nowCount: number;
  /** `2 the same · 1 changed · 1 dropped`. */
  counts: string;
}

/** The feature of the engine that opens what a rewrite changed. */
export const COMPARE_REWRITE = 'range-diff.compare';

/** The button that opens what a rewrite changed, on a message of VS Code. */
export const SEE_WHAT_CHANGED = 'See What Changed';

/** The words of a button of the engine as VS Code's buttons have them, each word capitalized: **Compare First**. */
export const buttonWords = (label: string) => label.replace(/(^|\s)(\p{Ll})/gu, (_whole, space: string, letter: string) => space + letter.toUpperCase());

/** The name of the tab: `feature · what changed`. */
export const rangeDiffTabTitle = (branch: string) => `${branch} · what changed`;

/** What changed in a changed pair, in words: `message`, `changes` or `message, changes`. */
export function changedParts(pair: RangeDiffPair): string {
  return [pair.messageChanged && 'message', pair.changesChanged && 'changes'].filter(Boolean).join(', ');
}

/** How a line of a diff of diffs is shown: lost before, gained now, a heading of a part, or unchanged. */
export type DiffTone = 'lost' | 'gained' | 'heading' | 'context';

/** The tone of a line of a diff of diffs, by its outer mark; a heading `## Commit message ##` keeps its own. */
export function diffTone(line: string): DiffTone {
  if (/^.? ## .* ##$/.test(line) || /^.?@@ /.test(line)) return 'heading';
  if (line.startsWith('-')) return 'lost';
  if (line.startsWith('+')) return 'gained';
  return 'context';
}

/** The name of a branch's upstream without `refs/remotes/`: `origin/feature`; null for none, gone, or a local one. */
export function upstreamOf(branch: Branch): string | null {
  const upstream = branch.upstream;
  if (!upstream || upstream.isGone) return null;
  const prefix = 'refs/remotes/';
  return upstream.reference.startsWith(prefix) ? upstream.reference.slice(prefix.length) : null;
}

/** Whether a branch and its upstream have each commits the other has not, as of the last fetch. */
export function hasDiverged(branch: Branch): boolean {
  return upstreamOf(branch) !== null && branch.upstream!.ahead > 0 && branch.upstream!.behind > 0;
}

/**
 * The upstream a fetch found force pushed: a local branch has more commits its upstream has not than before, which
 * only an upstream that lost commits gives, the branch having stayed where it was. Null when none was.
 */
export function forcePushed(before: { branches: Branch[] }, after: { branches: Branch[] }): string | null {
  for (const branch of after.branches) {
    const upstream = upstreamOf(branch);
    const was = before.branches.find((each) => each.name === branch.name);
    if (!upstream || !was || upstreamOf(was) !== upstream || was.commit !== branch.commit) continue;
    if (branch.upstream!.ahead > was.upstream!.ahead) return upstream;
  }
  return null;
}
