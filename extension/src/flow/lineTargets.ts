/**
 * The sides and the rows selected of a Flow Diff as the engine takes them in a change (`diff.stage-lines`,
 * `diff.unstage-lines`). No `vscode` import: unit-tested with plain node.
 */
import type { DiffChangeTarget, DiffSideTarget } from '../engine/messages';
import { type RowSpan, sideLinesOfRows } from './diff/lineMap';
import type { AlignedDiffModel } from './diff/model';
import type { DiffDescriptor, Version } from './sides';

/** A side of a Flow Diff as the engine names it. */
export function sideTarget(version: Version): DiffSideTarget {
  switch (version.kind) {
    case 'worktree':
      return { kind: 'workingTree' };
    case 'index':
      return { kind: 'index' };
    case 'commit':
      return version.ref === 'HEAD' ? { kind: 'head' } : { kind: 'commit', commit: version.ref };
    case 'none':
      return { kind: 'empty' };
  }
}

/**
 * The rows selected of a Flow Diff as the engine takes them for `diff.stage-lines` and `diff.unstage-lines`: a change
 * whose lines on each side are those in the rows, with their text (`diff.select-lines`).
 */
export function linesTarget(descriptor: DiffDescriptor, model: AlignedDiffModel, rows: RowSpan): DiffChangeTarget {
  const left = sideLinesOfRows(model, rows, 'left');
  const right = sideLinesOfRows(model, rows, 'right');
  return {
    kind: 'diffChange',
    path: descriptor.path,
    left: sideTarget(descriptor.left),
    right: sideTarget(descriptor.right),
    leftLines: { start: left.start, end: left.end },
    rightLines: { start: right.start, end: right.end },
    leftText: left.text,
    rightText: right.text,
  };
}
