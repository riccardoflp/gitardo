import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { contributedKeys } from '../src/engine/settings';
import { GraphLayout, fitGraph, laneCount, oneLane } from '../src/shared/graph';
import { WORKTREE } from '../src/shared/protocol';

/** A history of three branches, `main`, `feature` and `fix`, both merged into `main`, newest first. */
const HISTORY: [hash: string, ...parents: string[]][] = [
  ['m2', 'm1', 'f2'],
  ['f2', 'f1'],
  ['m1', 'c', 'x2'],
  ['x2', 'x1'],
  ['c', 'b'],
  ['x1', 'b'],
  ['f1', 'b'],
  ['b', 'a'],
  ['a'],
];

describe('oneLane (graph.one-lane)', () => {
  it('puts the dot of every row of a history with three branches and two merges in the first lane', () => {
    const layout = new GraphLayout();
    const withLanes = HISTORY.map(([hash, ...parents]) => layout.next(hash, parents));
    expect(Math.max(...withLanes.map(laneCount))).toBeGreaterThan(1);
    const rows = oneLane(HISTORY.map(([hash]) => ({ hash })));
    expect(rows.map((row) => row.graph.col)).toEqual(HISTORY.map(() => 0));
    expect(rows.map((row) => row.graph.color)).toEqual(HISTORY.map(() => 0));
    expect(Math.max(...rows.map((row) => laneCount(row.graph)))).toBe(1);
    expect(rows[0].graph.top).toEqual([]);
    expect(rows[4].graph.top).toEqual([{ from: 0, to: 0, color: 0 }]);
    expect(rows[4].graph.bottom).toEqual([{ from: 0, to: 0, color: 0 }]);
    expect(rows[rows.length - 1].graph.bottom).toEqual([]);
  });

  it('dashes the line under the uncommitted changes and a stash, as before', () => {
    const rows = oneLane([{ hash: WORKTREE }, { hash: 'c' }, { hash: 's', stash: 'stash@{0}' }, { hash: 'b' }]);
    expect(rows[0].graph.bottom).toEqual([{ from: 0, to: 0, color: 0, dashed: true }]);
    expect(rows[1].graph.top).toEqual([{ from: 0, to: 0, color: 0, dashed: true }]);
    expect(rows[1].graph.bottom).toEqual([{ from: 0, to: 0, color: 0 }]);
    expect(rows[3].graph.top).toEqual([{ from: 0, to: 0, color: 0, dashed: true }]);
  });

  it('makes the column of the graph as wide as one lane, even dragged wider', () => {
    expect(fitGraph(1, 300, 1200).width).toBe(fitGraph(1, null, 1200).width);
  });
});

describe('contributes.configuration (settings.sections)', () => {
  const json = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8')) as {
    contributes: { configuration: { title: string; order: number; properties?: Record<string, unknown> }[] };
  };
  const categories = json.contributes.configuration;

  it('has the sections of the settings page that have settings of the extension, with the same names in the same order', () => {
    const sections = [
      'General',
      'Appearance',
      'Timeline',
      'Branches',
      'Pull and commits',
      'Worktrees',
      'Files and blame',
      'AI and conflicts',
      'Terminal and tools',
    ];
    const titles = categories.map((category) => category.title);
    expect(titles).toEqual(sections.filter((section) => titles.includes(section)));
    expect(titles).toEqual(['General', 'Timeline', 'Branches', 'Pull and commits', 'Worktrees', 'Files and blame', 'AI and conflicts']);
    expect(categories.map((category) => category.order)).toEqual([...categories.map((category) => category.order)].sort((one, other) => one - other));
  });

  it('puts graph.oneLane in Timeline, off by default, and gives the engine the keys of every category', () => {
    const timeline = categories.find((category) => category.title === 'Timeline');
    expect(timeline?.properties?.['gitardo.graph.oneLane']).toMatchObject({ type: 'boolean', default: false });
    const keys = contributedKeys(categories);
    expect(keys).toContain('graph.oneLane');
    expect(keys).toContain('commandPreview');
    expect(keys).toContain('conflicts.ai');
    expect(new Set(keys).size).toBe(keys.length);
    expect(contributedKeys({ properties: { 'gitardo.readOnStartup': {} } })).toEqual(['readOnStartup']);
  });
});
