/**
 * Git LFS in the extension (milestone M47), through the engine: a version of a file stored with Git LFS opens as its
 * content or as the text that says it is not downloaded, never as its pointer, and **Download** runs `lfs.download`.
 * What needs no Git LFS, a pointer whose content is not in the store and the patterns in the state, runs everywhere;
 * what needs the program is skipped where it is missing and says so (Q1198), and runs on the runners of the pipeline,
 * which have it.
 */
import { spawnSync } from 'node:child_process';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { Reads } from '../src/engine/reads';
import { notDownloadedText } from '../src/shared/lfs';
import { git } from './git';
import { built, cleanUpAfterEach, client, engineState, folder, repository, runFeature } from './engine/harness';

cleanUpAfterEach();

/** Whether the program Git LFS is installed here. */
const hasLfs = spawnSync('git', ['lfs', 'version']).status === 0;
if (!hasLfs) console.log('engineLfs: the tests that need Git LFS are skipped: Git LFS is missing');

/** The pointer Git LFS writes for a content of 2,000 bytes whose id is 64 times `a`. */
const POINTER = `version https://git-lfs.github.com/spec/v1\noid sha256:${'a'.repeat(64)}\nsize 2000\n`;

/** A repository whose `.gitattributes` stores `*.psd` with Git LFS, with the pointer of `art/logo.psd` committed. */
async function withPointer(): Promise<string> {
  const root = await repository();
  fs.mkdirSync(path.join(root, 'art'));
  fs.writeFileSync(path.join(root, '.gitattributes'), '*.psd filter=lfs diff=lfs merge=lfs -text\n');
  fs.writeFileSync(path.join(root, 'art', 'logo.psd'), POINTER);
  fs.writeFileSync(path.join(root, 'notes.txt'), POINTER);
  await git(root, ['add', '--', '.gitattributes', 'art/logo.psd', 'notes.txt']);
  await git(root, ['commit', '--quiet', '-m', 'Add the pointers']);
  return root;
}

/**
 * A clone of a repository that stores `art/logo.psd` with Git LFS, made without downloading: its file is a pointer and
 * the store of the clone is empty. The origin is a folder, which Git LFS reads and writes without a server.
 */
async function cloneWithoutDownloading(): Promise<{ clone: string; content: string }> {
  const work = await repository();
  const origin = path.join(path.dirname(work), 'origin.git');
  await git(path.dirname(work), ['init', '--quiet', '--bare', '--initial-branch=main', origin]);
  await git(work, ['lfs', 'install', '--local']);
  await git(work, ['remote', 'add', 'origin', origin]);
  fs.mkdirSync(path.join(work, 'art'));
  fs.writeFileSync(path.join(work, '.gitattributes'), '*.psd filter=lfs diff=lfs merge=lfs -text\n');
  const content = 'the layers of the logo\n'.repeat(100);
  fs.writeFileSync(path.join(work, 'art', 'logo.psd'), content);
  await git(work, ['add', '--', '.gitattributes', 'art/logo.psd']);
  await git(work, ['commit', '--quiet', '-m', 'Add the logo']);
  await git(work, ['push', '--quiet', 'origin', 'main']);
  const clone = path.join(folder(), 'clone');
  await git(path.dirname(clone), [
    '-c',
    'filter.lfs.smudge=git-lfs smudge --skip -- %f',
    '-c',
    'filter.lfs.process=git-lfs filter-process --skip',
    'clone',
    '--quiet',
    origin,
    clone,
  ]);
  await git(clone, ['lfs', 'install', '--local', '--skip-smudge']);
  await git(clone, ['config', 'user.name', 'Gitardo Test']);
  await git(clone, ['config', 'user.email', 'test@gitardo.invalid']);
  return { clone, content };
}

describe.skipIf(!built)('Git LFS, through the engine', { timeout: 120_000 }, () => {
  it('lfs.show-in-pages: a pointer whose content is not downloaded opens as the text that says so, of a commit and of the index', async () => {
    const root = await withPointer();
    const reads = new Reads(() => client());
    const head = (await git(root, ['rev-parse', 'HEAD'])).stdout.trim();
    for (const revision of ['HEAD', '']) {
      const shown = await reads.fileShownAt(root, revision, 'art/logo.psd');
      expect(shown.stored).toEqual({ size: 2000, oid: 'a'.repeat(64), isDownloaded: false });
      expect(shown.commit).toBe(revision ? head : null);
      const text = Buffer.from(shown.bytes!).toString('utf8');
      expect(text).toBe(notDownloadedText('art/logo.psd', { size: 2000, oid: 'a'.repeat(64) }));
      expect(text).not.toContain('version https://git-lfs.github.com');
    }
  });

  it('lfs.pointer: a text that only looks like a pointer, at a path not stored with Git LFS, opens as it is', async () => {
    const root = await withPointer();
    const shown = await new Reads(() => client()).fileShownAt(root, 'HEAD', 'notes.txt');
    expect(shown.stored).toBeNull();
    expect(Buffer.from(shown.bytes!).toString('utf8')).toBe(POINTER);
  });

  it('lfs.read-status: the patterns come with the state of the repository', async () => {
    const state = await engineState(await withPointer());
    expect(state.lfs?.patterns).toEqual([{ pattern: '*.psd', file: '.gitattributes', line: 1 }]);
    expect(!!state.lfs?.version).toBe(hasLfs);
  });

  it.skipIf(hasLfs)('lfs.download: without Git LFS, Download refuses with its message and runs nothing', async () => {
    const root = await withPointer();
    const { finished } = await runFeature(root, 'lfs.download', { kind: 'lfsFiles', paths: [{ path: 'art/logo.psd', isFolder: false }], commit: null });
    expect(finished).toMatchObject({ kind: 'refused', message: 'Git LFS is not installed: install it from git-lfs.com, then try again.' });
  });

  it.skipIf(!hasLfs)('lfs.download: in a clone made without downloading, Download pulls the file and its content shows after', async () => {
    const { clone, content } = await cloneWithoutDownloading();
    const reads = new Reads(() => client());
    const before = await reads.fileShownAt(clone, 'HEAD', 'art/logo.psd');
    expect(before.stored).toMatchObject({ size: content.length, isDownloaded: false });
    const { finished, lines } = await runFeature(clone, 'lfs.download', {
      kind: 'lfsFiles',
      paths: [{ path: 'art/logo.psd', isFolder: false }],
      commit: before.commit,
    });
    expect(finished.kind, JSON.stringify(finished)).toBe('ran');
    expect(lines).toEqual(['git lfs pull --include=art/logo.psd']);
    const after = await reads.fileShownAt(clone, 'HEAD', 'art/logo.psd');
    expect(after.stored?.isDownloaded).toBe(true);
    expect(Buffer.from(after.bytes!).toString('utf8')).toBe(content);
    expect(fs.readFileSync(path.join(clone, 'art', 'logo.psd'), 'utf8')).toBe(content);
  });
});
