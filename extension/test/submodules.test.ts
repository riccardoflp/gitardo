import { describe, expect, it } from 'vitest';
import type { Submodule } from '../src/shared/protocol';
import { submoduleDepth, submoduleFolder, submoduleStatus, submoduleStatusText } from '../src/shared/submodules';

// What the engine reads of submodules is checked against git in reads.test.ts; these are the words the views put on it.

describe('submodules', () => {
  const nested: Submodule = {
    folder: 'vendor/lib/inner',
    parent: 'vendor/lib',
    name: 'inner',
    url: null,
    branch: null,
    recorded: 'a'.repeat(40),
    staged: null,
    checkout: { commit: 'b'.repeat(40), branch: null, uncommitted: 0, distance: { ahead: 1, behind: 0 } },
  };
  const checkout = nested.checkout!;

  it('says where a submodule stands, and how deep it is', () => {
    expect(submoduleStatus(nested)).toBe('ahead');
    expect(submoduleStatusText(nested)).toBe('1 commit ahead of the recorded commit aaaaaaa');
    expect(submoduleStatus({ ...nested, checkout: { ...checkout, distance: { ahead: 2, behind: 3 } } })).toBe('diverged');
    expect(submoduleStatus({ ...nested, checkout: { ...checkout, distance: null } })).toBe('unknown');
    expect(submoduleStatus({ ...nested, checkout: null })).toBe('uninitialized');
    expect(submoduleStatus({ ...nested, recorded: null })).toBe('added');
    const outer = { ...nested, folder: 'vendor/lib', parent: null };
    expect(submoduleDepth(nested, [outer, nested])).toBe(1);
    expect(submoduleDepth(outer, [outer, nested])).toBe(0);
    expect(submoduleFolder(nested, { root: 'C:/repos/app' })).toBe('C:/repos/app/vendor/lib/inner');
  });
});
