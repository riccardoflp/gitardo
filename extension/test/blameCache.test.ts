import { describe, expect, it } from 'vitest';
import { HeadsSeen, LineBlames } from '../src/shared/blameCache';

/** A reader that counts what it is asked, and answers with the version and the line. */
const counting = () => {
  const asked: string[] = [];
  const read = (version: number, line: number) => () => {
    asked.push(`${version}:${line}`);
    return Promise.resolve(`blame of ${line} in ${version}`);
  };
  return { asked, read };
};

describe('the blame of the line under the cursor, kept', () => {
  it('asks once for a line of a version, however often the cursor comes back to it', async () => {
    const kept = new LineBlames<string>();
    const { asked, read } = counting();
    expect(await kept.get('a.txt', 1, 4, read(1, 4))).toBe('blame of 4 in 1');
    expect(await kept.get('a.txt', 1, 4, read(1, 4))).toBe('blame of 4 in 1');
    await kept.get('a.txt', 1, 5, read(1, 5));
    await kept.get('a.txt', 1, 4, read(1, 4));
    expect(asked).toEqual(['1:4', '1:5']);
  });

  it('asks once while the first answer is still coming', async () => {
    const kept = new LineBlames<string>();
    const { asked, read } = counting();
    const first = kept.get('a.txt', 1, 0, read(1, 0));
    const second = kept.get('a.txt', 1, 0, read(1, 0));
    expect(second).toBe(first);
    await Promise.all([first, second]);
    expect(asked).toEqual(['1:0']);
  });

  it('asks again for a new version of the text, and forgets the lines of the one before', async () => {
    const kept = new LineBlames<string>();
    const { asked, read } = counting();
    await kept.get('a.txt', 1, 2, read(1, 2));
    await kept.get('a.txt', 1, 3, read(1, 3));
    expect(await kept.get('a.txt', 2, 2, read(2, 2))).toBe('blame of 2 in 2');
    expect(kept.has('a.txt', 1, 3)).toBe(false);
    expect(kept.has('a.txt', 2, 2)).toBe(true);
    expect(asked).toEqual(['1:2', '1:3', '2:2']);
  });

  it('keeps each document apart', async () => {
    const kept = new LineBlames<string>();
    const { asked, read } = counting();
    await kept.get('a.txt', 1, 0, read(1, 0));
    await kept.get('b.txt', 1, 0, read(1, 0));
    await kept.get('a.txt', 1, 0, read(1, 0));
    expect(asked).toEqual(['1:0', '1:0']);
  });

  it('asks again after a read that failed, and keeps an answer of no blame', async () => {
    const kept = new LineBlames<string | null>();
    let fails = true;
    let reads = 0;
    const read = () => {
      reads++;
      return fails ? Promise.reject(new Error('the engine stopped')) : Promise.resolve(null);
    };
    await expect(kept.get('a.txt', 1, 0, read)).rejects.toThrow('the engine stopped');
    expect(kept.has('a.txt', 1, 0)).toBe(false);
    fails = false;
    expect(await kept.get('a.txt', 1, 0, read)).toBeNull();
    expect(await kept.get('a.txt', 1, 0, read)).toBeNull();
    expect(reads).toBe(2);
  });

  it('asks again for a document forgotten, or for every one', async () => {
    const kept = new LineBlames<string>();
    const { asked, read } = counting();
    await kept.get('a.txt', 1, 0, read(1, 0));
    await kept.get('b.txt', 1, 0, read(1, 0));
    kept.forget('a.txt');
    expect(kept.has('a.txt', 1, 0)).toBe(false);
    expect(kept.has('b.txt', 1, 0)).toBe(true);
    kept.forget();
    expect(kept.has('b.txt', 1, 0)).toBe(false);
    await kept.get('b.txt', 1, 0, read(1, 0));
    expect(asked).toEqual(['1:0', '1:0', '1:0']);
  });
});

describe('where HEAD was last seen', () => {
  it('says HEAD moved the first time and when it is at another commit, not when a read left it where it was', () => {
    const heads = new HeadsSeen();
    expect(heads.moved('/work/app', 'a'.repeat(40))).toBe(true);
    expect(heads.moved('/work/app', 'a'.repeat(40))).toBe(false);
    expect(heads.moved('/work/app', 'b'.repeat(40))).toBe(true);
    expect(heads.moved('/work/app', 'b'.repeat(40))).toBe(false);
  });

  it('keeps each repository apart, whatever the slashes and case of its folder', () => {
    const heads = new HeadsSeen();
    expect(heads.moved('C:\\work\\app', 'a'.repeat(40))).toBe(true);
    expect(heads.moved('c:/work/app', 'a'.repeat(40))).toBe(false);
    expect(heads.moved('c:/work/lib', 'a'.repeat(40))).toBe(true);
    expect(heads.moved('c:/work/app', 'a'.repeat(40))).toBe(false);
  });

  it('tells a first commit from a repository that had none', () => {
    const heads = new HeadsSeen();
    expect(heads.moved('/work/new', null)).toBe(true);
    expect(heads.moved('/work/new', null)).toBe(false);
    expect(heads.moved('/work/new', 'c'.repeat(40))).toBe(true);
  });
});
