/**
 * The reads of stage 5 of `docs/transition/06-roadmap.md`: each read of the engine asked through the client, and what
 * git itself says of the same repository. The two must say the same, but where `docs/transition/04-differences.md`
 * says why not, under **Reads (stage 5)**.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import type { Bytes, DiffLine } from '../src/engine/messages';
import { resolveConflicts } from '../src/shared/conflicts';
import { git } from './git';
import { built, cleanUpAfterEach, client, commitFile, folder, repository } from './engine/harness';

cleanUpAfterEach();

/** The text of bytes that travelled. */
const text = (bytes: Bytes | null) => (bytes ? Buffer.from(bytes.base64, 'base64').toString('utf8') : null);

/** Bytes as they travel. */
const bytesOf = (value: string): Bytes => ({ base64: Buffer.from(value, 'utf8').toString('base64') });

/** A line of a hunk of the engine as the extension keeps it: its sign, then its text. */
const signed = (line: DiffLine) => ({ context: ' ', added: '+', removed: '-', noNewline: '\\' })[line.kind] + line.text;

/** The full id of a revision. */
async function id(root: string, revision: string): Promise<string> {
  return (await git(root, ['rev-parse', revision])).stdout.trim();
}

/** A repository with `a.txt` changed twice on `main` and a branch `feature` at the first commit with a commit of its own. */
async function branched(): Promise<string> {
  const root = await repository();
  await git(root, ['branch', 'feature']);
  await commitFile(root, 'a.txt', 'one\ntwo\n', 'Add two');
  await commitFile(root, 'b.txt', 'b\n', 'Add b');
  await git(root, ['checkout', '--quiet', 'feature']);
  await commitFile(root, 'c.txt', 'c\n', 'Add c');
  await git(root, ['checkout', '--quiet', 'main']);
  return root;
}

/** A merge of `incoming` into `main` stopped on two conflicts in `c.txt`: one the sides differ in only by spaces, one real. */
async function conflicted(): Promise<string> {
  const root = await repository();
  const lines = ['one', 'two', 'three', 'four', 'five', 'six', 'seven', 'eight'];
  const file = (first: string, last: string) => [first, ...lines.slice(1, -1), last].join('\n') + '\n';
  await commitFile(root, 'c.txt', file('one', 'eight'), 'Add c');
  await git(root, ['branch', 'incoming']);
  await commitFile(root, 'c.txt', file('one  ,', 'eight here'), 'Change c here');
  await git(root, ['checkout', '--quiet', 'incoming']);
  await commitFile(root, 'c.txt', file('one ,', 'eight there'), 'Change c there');
  await git(root, ['checkout', '--quiet', 'main']);
  await git(root, ['merge', '--no-edit', 'incoming']).catch(() => undefined);
  return root;
}
/** What git shows of a file at a revision, or null when it has none there. */
async function show(root: string, revision: string, file: string): Promise<string | null> {
  return (await git(root, ['show', `${revision}:${file}`]).catch(() => null))?.stdout ?? null;
}

/** The commit git resolves a revision to, or null; what starts with a dash is not asked, as git would take it for an option. */
async function resolved(root: string, revision: string): Promise<string | null> {
  if (revision.startsWith('-')) return null;
  return (await git(root, ['rev-parse', '--verify', '--quiet', `${revision}^{commit}`]).catch(() => null))?.stdout.trim() || null;
}

/** The paths of a list of files, in one order. */
const paths = (files: Array<{ path: string }> = []) => files.map((file) => file.path).sort();

/** The lines of each file of a patch git writes, without its headers and the lines that say a file has no newline at its end. */
function patchLines(patch: string): string[][] {
  return patch
    .split(/^diff --git /m)
    .slice(1)
    .map((file) => {
      const lines = file.split('\n');
      const first = lines.findIndex((line) => line.startsWith('@@'));
      return first < 0 ? [] : lines.slice(first).filter((line) => /^[ +-]/.test(line));
    });
}

/** What `git blame --line-porcelain` says of each line: who made it, in which commit, and the version before. */
async function blamed(root: string, args: string[], input?: string) {
  const out = (await git(root, ['blame', '--line-porcelain', ...args], { input })).stdout.split('\n');
  const lines: Array<{ commit: string; author: string; summary: string; previous: { hash: string; file: string } | null } | 'notCommitted'> = [];
  let current: { commit: string; author: string; summary: string; previous: { hash: string; file: string } | null } | null = null;
  for (const line of out) {
    if (/^[0-9a-f]{40} /.test(line)) current = { commit: line.slice(0, 40), author: '', summary: '', previous: null };
    else if (current && line.startsWith('author ')) current.author = line.slice(7);
    else if (current && line.startsWith('summary ')) current.summary = line.slice(8);
    else if (current && line.startsWith('previous ')) {
      const [hash, ...file] = line.slice(9).split(' ');
      current.previous = { hash, file: file.join(' ') };
    } else if (current && line.startsWith('\t')) {
      lines.push(/^0+$/.test(current.commit) ? 'notCommitted' : current);
      current = null;
    }
  }
  return lines;
}

describe.skipIf(!built)('the reads of the engine', { timeout: 60_000 }, () => {
  it('search.read-revision and compare.resolve-revision give the commit git resolves a revision to', async () => {
    const root = await branched();
    const engine = client();
    for (const revision of ['main', 'feature~1', 'HEAD', 'no-such-branch', '-x']) {
      expect(await engine.searchRevision(root, revision), revision).toBe(await resolved(root, revision));
      expect(await engine.compareRevision(root, revision), revision).toBe(await resolved(root, revision));
    }
  });

  it('details.read-commit has what git says of a commit, of a stash with its untracked files and of the changes', async () => {
    const root = await branched();
    const engine = client();
    const head = await id(root, 'HEAD');
    const detail = await engine.details(root, { commit: head });
    const [parents, name, email, date, message] = (await git(root, ['log', '-1', '--format=%P%x01%an%x01%ae%x01%at%x01%B', head])).stdout.split('\x01');
    expect(detail).toMatchObject({ id: head, parents: parents.split(' '), author: { name, email, date: Number(date) }, message: message.trim() });
    expect(paths(detail?.files)).toEqual(
      (await git(root, ['diff-tree', '--no-commit-id', '--name-only', '-r', head])).stdout.split('\n').filter(Boolean).sort(),
    );

    fs.writeFileSync(path.join(root, 'a.txt'), 'edited\n');
    fs.writeFileSync(path.join(root, 'new.txt'), 'new\n');
    const changes = await engine.details(root, { uncommitted: true });
    expect(changes?.id).toBeNull();
    expect(paths(changes?.files)).toEqual(['a.txt', 'new.txt']);

    await git(root, ['stash', 'push', '--include-untracked', '-m', 'Both']);
    const stash = await id(root, 'stash@{0}');
    const stashed = await engine.details(root, { stash: 0 });
    expect(stashed?.id).toBe(stash);
    expect(paths(stashed?.files)).toEqual(['a.txt']);
    expect(paths(stashed?.untrackedFiles)).toEqual(['new.txt']);
    expect(await engine.details(root, { stash: 1 })).toBeNull();
  });

  it('commit-page.read-patch has the lines git shows of a commit, file by file', async () => {
    const root = await branched();
    const engine = client();
    const commit = await id(root, 'HEAD~1');
    const patch = await engine.commitPatch(root, commit, null);
    if (patch.kind !== 'files') throw new Error('No patch');
    const counts = (await git(root, ['diff', '--numstat', `${commit}~1`, commit])).stdout.split('\n').filter(Boolean);
    expect(patch.files.map((file) => [file.added, file.removed, file.path].join('\t'))).toEqual(counts);
    expect(patch.files.map((file) => file.hunks.flatMap((hunk) => hunk.lines.filter((line) => line.kind !== 'noNewline').map(signed)))).toEqual(
      patchLines((await git(root, ['diff', '--no-color', `${commit}~1`, commit])).stdout),
    );
  });

  it('changes.read-working-tree and changes.read-ignored read what git status and git check-ignore say', async () => {
    const root = await repository();
    await commitFile(root, '.gitignore', '*.log\n', 'Ignore the logs');
    fs.writeFileSync(path.join(root, 'kept.log'), 'tracked\n');
    await git(root, ['add', '--force', 'kept.log']);
    await git(root, ['commit', '--quiet', '-m', 'Keep a log']);
    fs.writeFileSync(path.join(root, 'a.txt'), 'edited\n');
    fs.writeFileSync(path.join(root, 'staged.txt'), 'staged\n');
    await git(root, ['add', 'staged.txt']);
    fs.writeFileSync(path.join(root, 'new.txt'), 'new\n');
    fs.writeFileSync(path.join(root, 'run.log'), 'log\n');
    const engine = client();
    const tree = await engine.workingTree(root);
    expect([paths(tree.staged), paths(tree.unstaged), [...tree.untracked].sort(), paths(tree.conflicted)]).toEqual([
      ['staged.txt'],
      ['a.txt'],
      ['new.txt'],
      [],
    ]);
    const asked = ['a.txt', 'run.log', 'kept.log', 'missing.log'];
    const ignored = (await git(root, ['check-ignore', '--', ...asked]).catch(() => ({ stdout: '' }))).stdout.split('\n').filter(Boolean);
    expect(new Set(await engine.ignored(root, asked))).toEqual(new Set(ignored));
  });

  it('diff.read-sides and file-view.read-revision give the bytes git and the disk give', async () => {
    const root = await branched();
    const engine = client();
    const first = await id(root, 'HEAD~2');
    fs.writeFileSync(path.join(root, 'a.txt'), 'on disk\n');
    const sides = await engine.diffSides(
      root,
      { version: { kind: 'commit', commit: first }, path: 'a.txt' },
      { version: { kind: 'workingTree', unsaved: null }, path: 'a.txt' },
    );
    expect(text(sides.left)).toBe(await show(root, first, 'a.txt'));
    expect(text(sides.right)).toBe(fs.readFileSync(path.join(root, 'a.txt'), 'utf8'));
    const typed = await engine.diffSides(
      root,
      { version: { kind: 'index' }, path: 'a.txt' },
      { version: { kind: 'workingTree', unsaved: bytesOf('typed\n') }, path: 'a.txt' },
    );
    expect([text(typed.left), text(typed.right)]).toEqual([await show(root, '', 'a.txt'), 'typed\n']);
    expect(text(await engine.fileRevision(root, 'a.txt', 'feature'))).toBe(await show(root, 'feature', 'a.txt'));
    // A file the commit does not have is null.
    expect(await engine.fileRevision(root, 'b.txt', 'feature')).toBeNull();
  });

  it('merge-view.read-sides and conflicts.read-files read the stages git has of a file in conflict', async () => {
    const root = await conflicted();
    const engine = client();
    const sides = await engine.mergeSides(root, 'c.txt', null);
    expect([text(sides.base), text(sides.current), text(sides.incoming)]).toEqual([
      await show(root, ':1', 'c.txt'),
      await show(root, ':2', 'c.txt'),
      await show(root, ':3', 'c.txt'),
    ]);
    expect(text(sides.result)).toBe(fs.readFileSync(path.join(root, 'c.txt'), 'utf8'));
    expect(sides.operation).toBe('merge');
    const files = await engine.conflictFiles(root);
    expect(files.map((file) => file.path)).toEqual((await git(root, ['diff', '--name-only', '--diff-filter=U'])).stdout.split('\n').filter(Boolean));
    const stages = files.map((file) => [file.base, file.current, file.incoming].flatMap((stage, index) => (stage ? [index + 1] : [])));
    const staged = (await git(root, ['ls-files', '-u', '--', 'c.txt'])).stdout.split('\n').filter(Boolean);
    expect(stages).toEqual([staged.map((line) => Number(line.split(/\s+/)[2]))]);
  });

  it('conflicts.side-names names the sides by the branch checked out and the one being merged', async () => {
    const root = await conflicted();
    const names = await client().sideNames(root);
    expect([names.current.name, names.incoming.name]).toEqual(['main', 'incoming']);
    expect([names.current.role, names.incoming.role]).toEqual(['branchCheckedOut', 'branchBeingMerged']);
  });

  it('conflicts.resolve-by-rules resolves the conflicts that resolveConflicts resolves, with the same lines', async () => {
    const root = await conflicted();
    const [proposals] = await client().conflictRules(root, null);
    const file = fs.readFileSync(path.join(root, 'c.txt'), 'utf8');
    const stage = async (number: number) => (await show(root, `:${number}`, 'c.txt')) ?? '';
    const resolved = resolveConflicts(file, await stage(1), { ours: await stage(2), theirs: await stage(3) });
    expect(proposals.leftAlone).toBeNull();
    expect(proposals.conflicts.map((conflict) => [conflict.line, conflict.proposal !== null])).toEqual(
      resolved.conflicts.map((conflict) => [conflict.line, conflict.resolution !== null]),
    );
    const lines = file.split(/(?<=\n)/);
    const written = (proposal: (typeof proposals.conflicts)[number]['proposal']) =>
      proposal ? [lines.slice(0, proposal.replaced.start).length, proposal.lines.map((line) => line.text + line.ending).join('')] : null;
    expect(proposals.conflicts.map((conflict) => written(conflict.proposal))).toEqual(
      resolved.conflicts.map((conflict) => (conflict.patch ? [conflict.patch.from, conflict.patch.text] : null)),
    );
  });

  it('compare.read-sides, compare.read-files and compare.read-patch read what git says of two commits', async () => {
    const root = await branched();
    const engine = client();
    const [main, feature] = [await id(root, 'main'), await id(root, 'feature')];
    const base = (await git(root, ['merge-base', feature, main])).stdout.trim();
    const request = { first: { kind: 'commit', commit: feature }, second: { kind: 'commit', commit: main }, mode: 'sinceDiverged', path: null } as const;
    const view = await engine.compareSides(root, request);
    expect(view.mergeBase).toBe(base);
    const names = async (args: string[]) => (await git(root, args)).stdout.split('\n').filter(Boolean).sort();
    expect(paths(view.files)).toEqual(await names(['diff', '--name-only', base, main]));
    const only = async (range: string) => (await git(root, ['rev-list', range])).stdout.split('\n').filter(Boolean);
    expect(view.commits.filter((commit) => commit.side === 'first').map((commit) => commit.id)).toEqual(await only(`${main}..${feature}`));
    expect(view.commits.filter((commit) => commit.side === 'second').map((commit) => commit.id)).toEqual(await only(`${feature}..${main}`));
    expect([view.onlyInFirst, view.onlyInSecond]).toEqual([(await only(`${main}..${feature}`)).length, (await only(`${feature}..${main}`)).length]);

    expect(paths(await engine.compareFiles(root, feature, main))).toEqual(await names(['diff', '--name-only', feature, main]));

    const patch = await engine.comparePatch(root, { ...request, mergeBase: view.mergeBase, files: view.files, form: 'lines' });
    if (patch.kind !== 'files') throw new Error('No patch');
    const counts = (await git(root, ['diff', '--numstat', base, main])).stdout.split('\n').filter(Boolean);
    expect(patch.files.map((file) => [file.added, file.removed, file.path].join('\t'))).toEqual(counts);
    const textPatch = await engine.comparePatch(root, { ...request, mergeBase: view.mergeBase, files: view.files, form: 'text' });
    expect(textPatch.kind === 'text' && patchLines(text(textPatch.text) ?? '')).toEqual(
      patchLines((await git(root, ['diff', '--no-color', base, main])).stdout),
    );
  });

  it('blame.read-line and blame.read-file blame the lines git blame blames', async () => {
    const root = await branched();
    await git(root, ['mv', 'a.txt', 'moved.txt']);
    await git(root, ['commit', '--quiet', '-m', 'Move a']);
    await commitFile(root, 'moved.txt', 'one\ntwo\nthree\n', 'Add three');
    const engine = client();
    const typed = 'one\ntyped\nthree\n';
    const ofEngine = (line: Awaited<ReturnType<typeof engine.blameLine>>) =>
      line &&
      (line.kind === 'committed'
        ? {
            commit: line.commit,
            author: line.author,
            summary: line.summary,
            previous: line.previous && { hash: line.previous.commit, file: line.previous.path },
          }
        : 'notCommitted');
    const typedBlame = await blamed(root, ['--contents', '-', '--', 'moved.txt'], typed);
    for (const line of [1, 2, 3]) {
      expect(ofEngine(await engine.blameLine(root, 'moved.txt', line, bytesOf(typed))), `line ${line}`).toEqual(typedBlame[line - 1]);
    }
    const third = await engine.blameLine(root, 'moved.txt', 3, null);
    expect(third?.kind === 'committed' && third.previous).toEqual({ commit: await id(root, 'HEAD~1'), path: 'moved.txt' });
    expect((await engine.blameFile(root, 'moved.txt', null, null))?.map(ofEngine)).toEqual(await blamed(root, ['--', 'moved.txt']));
    const before = await id(root, 'HEAD~1');
    expect((await engine.blameFile(root, 'moved.txt', before, null))?.map(ofEngine)).toEqual(await blamed(root, [before, '--', 'moved.txt']));
    expect(await engine.blameFile(root, 'never.txt', null, null)).toBeNull();
  });

  it('file-history.read-names and file-history.read-committed-lines follow a file as git does', async () => {
    const root = await repository();
    await commitFile(root, 'a.txt', 'one\ntwo\nthree\n', 'Grow a');
    await git(root, ['mv', 'a.txt', 'b.txt']);
    await git(root, ['commit', '--quiet', '-m', 'Rename a']);
    const engine = client();
    expect(await engine.fileNames(root, 'b.txt')).toEqual(['b.txt', 'a.txt']);
    // The lines `two` and `three` of the text now are the second and third of the last commit.
    const now = 'zero\none\ntwo\nthree\n';
    const lines = await engine.committedLines(root, { path: 'b.txt', renamedFrom: null, text: now, lines: { start: 3, end: 4 } });
    expect(lines).toEqual({ kind: 'committed', commit: await id(root, 'HEAD'), path: 'b.txt', lines: { start: 2, end: 3 } });
    const missing = await engine.committedLines(root, { path: 'never.txt', renamedFrom: null, text: now, lines: { start: 1, end: 1 } });
    expect(missing.kind).toBe('inNoCommit');
  });

  it('reflog.read, remote.read-remotes and identity.read read what git reflog, git remote and git config say', async () => {
    const root = await branched();
    await git(root, ['remote', 'add', 'origin', 'https://example.com/app.git']);
    await git(root, ['remote', 'add', 'fork', 'git@example.com:me/app.git']);
    const engine = client();
    const positions = await engine.reflog(root, 'HEAD', 5);
    const reflog = (await git(root, ['reflog', '--max-count=5', '--format=%H%x01%gs%x01%gd%x01%ct', 'HEAD'])).stdout.split('\n').filter(Boolean);
    expect(positions.map((position) => [position.commit, position.subject, position.selector, position.time].join('\x01'))).toEqual(reflog);
    const remotes = await engine.remotes(root);
    expect([...remotes].sort((one, other) => one.name.localeCompare(other.name))).toEqual([
      { name: 'fork', url: 'git@example.com:me/app.git' },
      { name: 'origin', url: 'https://example.com/app.git' },
    ]);
    const identity = await engine.identity(root);
    expect([...identity.localKeys].sort()).toEqual(['user.email', 'user.name']);
    expect(identity.identity).toEqual({ name: 'Gitardo Test', email: 'test@gitardo.invalid' });
  });

  it('submodule.read-list lists the submodules git records', async () => {
    const inside = await repository();
    const root = await repository();
    await git(root, ['-c', 'protocol.file.allow=always', 'submodule', 'add', '--quiet', inside, 'libs/inside']);
    await git(root, ['commit', '--quiet', '-m', 'Add the submodule']);
    const recorded = (await git(root, ['ls-tree', 'HEAD', 'libs/inside'])).stdout.split(/\s+/)[2];
    const [listed] = await client().submodules(root);
    expect(listed).toMatchObject({ name: 'libs/inside', folder: 'libs/inside', parent: null, url: inside, recorded, staged: null });
    expect(listed.checkout?.commit).toBe(recorded);
  });

  it('the reads of worktree-setup read what is ignored, what the patterns take and the commands a lockfile asks for', async () => {
    const root = await repository();
    await commitFile(root, '.gitignore', 'node_modules/\n.env\n*.local\n', 'Ignore');
    await commitFile(root, 'package.json', '{ "packageManager": "pnpm@9.1.0" }\n', 'Add the package');
    await commitFile(root, 'pnpm-lock.yaml', 'lockfileVersion: 9\n', 'Lock');
    fs.mkdirSync(path.join(root, 'node_modules', 'left-pad'), { recursive: true });
    fs.writeFileSync(path.join(root, 'node_modules', 'left-pad', 'index.js'), 'module.exports = 1;\n');
    fs.writeFileSync(path.join(root, '.env'), 'KEY=1\n');
    fs.writeFileSync(path.join(root, 'settings.local'), 'x\n');
    const engine = client();
    const entries = await engine.setupIgnored(root, root);
    expect(
      entries.map((entry) => ({
        path: entry.path,
        folder: entry.kind.kind === 'folder',
        bytes: entry.kind.kind === 'link' ? 0 : entry.kind.bytes,
        files: entry.kind.kind === 'folder' ? entry.kind.files : entry.kind.kind === 'file' ? 1 : 0,
      })),
    ).toEqual([
      { path: '.env', folder: false, bytes: 6, files: 1 },
      { path: 'node_modules', folder: true, bytes: 20, files: 1 },
      { path: 'settings.local', folder: false, bytes: 2, files: 1 },
    ]);
    const patterns = { link: ['node_modules'], copy: ['.env', '*.local'] };
    const [preview] = await engine.setupTakes(root, [{ folder: root, ...patterns, commit: null }]);
    expect([preview.linked, [...preview.copied].sort()]).toEqual([['node_modules'], ['.env', 'settings.local']]);
    const counts = (takes: Array<{ pattern: string; count: number }>) => Object.fromEntries(takes.map((take) => [take.pattern, take.count]));
    expect({ link: counts(preview.linkPatterns), copy: counts(preview.copyPatterns) }).toEqual({
      link: { node_modules: 1 },
      copy: { '.env': 1, '*.local': 1 },
    });
    expect(await engine.setupCommands(root)).toEqual([{ command: 'pnpm install --frozen-lockfile', file: 'pnpm-lock.yaml' }]);
  });

  it('github.read-pull-requests finds no repository on GitHub in a remote of another host, and reads nothing without a token', async () => {
    const root = await repository();
    const engine = client();
    await git(root, ['remote', 'add', 'origin', 'https://gitlab.com/owner/app.git']);
    expect(await engine.pullRequests(root, null)).toEqual({ kind: 'notOnGitHub' });
    await git(root, ['remote', 'add', 'hub', 'git@github.com:owner/app.git']);
    expect(await engine.pullRequests(root, 'hub')).toEqual({ kind: 'noToken' });
  });

  it('branch.read-log lists the commits git log lists, whether there are more, and those not pushed', async () => {
    const root = await repository();
    await commitFile(root, 'a.txt', 'two\n', 'Second');
    const engine = client();
    const compared = async (count: number, unpushed: string[] | null, upstream: string | null) => {
      const log = await engine.branchLog(root, count);
      const listed = (await git(root, ['log', `--max-count=${count + 1}`, '--format=%H%x01%P%x01%an%x01%ae%x01%at%x01%s', 'HEAD'])).stdout
        .split('\n')
        .filter(Boolean);
      expect(
        log.commits.map((commit) =>
          [commit.id, commit.parents.join(' '), commit.authorName, commit.authorEmail, commit.authorDate, commit.subject].join('\x01'),
        ),
      ).toEqual(listed.slice(0, count));
      expect([log.hasMore, log.unpushed, log.upstream]).toEqual([listed.length > count, unpushed, upstream]);
    };
    // With no remote, nothing is said of what is pushed.
    await compared(1, null, null);
    const remote = path.join(folder(), 'remote.git');
    await git(path.dirname(remote), ['init', '--quiet', '--bare', '--initial-branch=main', remote]);
    await git(root, ['remote', 'add', 'origin', remote]);
    await git(root, ['push', '--quiet', '--set-upstream', 'origin', 'main']);
    await commitFile(root, 'a.txt', 'three\n', 'Third');
    const third = await id(root, 'HEAD');
    await compared(5, [third], 'origin/main');
    await git(root, ['branch', '--unset-upstream']);
    await compared(5, [third], null);
  });

  it('repo.common-folder and commit.prepared-message read the folder git shares and the message it prepared', async () => {
    const root = await branched();
    const engine = client();
    const worktree = path.join(folder(), 'other');
    await git(root, ['worktree', 'add', '--quiet', worktree, 'feature']);
    for (const checkout of [root, worktree]) {
      const common = (await git(checkout, ['rev-parse', '--path-format=absolute', '--git-common-dir'])).stdout.trim();
      expect(path.resolve(await engine.commonFolder(checkout))).toBe(path.resolve(common));
    }
    expect(await engine.preparedMessage(root)).toBeNull();
    await git(root, ['merge', '--squash', 'feature']);
    // The message git prepared, without the newlines at its end.
    expect(await engine.preparedMessage(root)).toBe(fs.readFileSync(path.join(root, '.git', 'SQUASH_MSG'), 'utf8').trimEnd());
  });
});
