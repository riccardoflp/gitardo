/** The options of the commit box of **Gitardo Changes**, its menu, and the words of the line **Signature** (milestone M34). */
import { describe, expect, it } from 'vitest';
import { optionAnswers } from '../src/engine/actions';
import {
  coAuthorEntries,
  optionsArguments,
  optionsSaid,
  signatureTone,
  signatureTooltip,
  signatureWords,
  signsWith,
  startingOptions,
} from '../src/shared/commitOptions';
import type { CommitSignature, Signing } from '../src/shared/state';
import { commitOptionsMenu } from '../src/webview/commitOptions';
import type { MenuItem } from '../src/webview/ContextMenu';

const signing = (signsEveryCommit: boolean, kind: Signing['kind'] = 'openpgp', key: string | null = null): Signing => ({ signsEveryCommit, kind, key });
const signature = (verdict: CommitSignature['verdict'], fields: Partial<CommitSignature> = {}): CommitSignature => ({
  verdict,
  kind: 'ssh',
  signer: '',
  key: '',
  fingerprint: '',
  trust: '',
  ...fields,
});

describe('details.signature in the extension', () => {
  it('a good signature says its signer, in the color of an addition, with the fingerprint in its tooltip', () => {
    const good = signature('good', { signer: 'Ada <ada@example.com>', fingerprint: 'SHA256:abc' });
    expect(signatureWords(good)).toBe('Good, Ada <ada@example.com>');
    expect(signatureTone(good)).toBe('good');
    expect(signatureTooltip(good)).toBe('SSH signature\nFingerprint: SHA256:abc');
  });

  it('a commit not signed has no words, so no line; a bad one says Bad, in the color of a removal', () => {
    expect(signatureWords(signature('notSigned', { kind: null }))).toBe('');
    expect(signatureWords(signature('bad'))).toBe('Bad');
    expect(signatureTone(signature('bad'))).toBe('bad');
    expect(signatureWords(signature('signedNotChecked'))).toBe('Signed, not checked: no allowed signers are set');
    expect(signatureTone(signature('goodExpiredKey'))).toBe('doubted');
  });
});

describe('commit.options in the extension', () => {
  it('Sign this commit starts ticked when commit.gpgSign is on, and unticked says Not signed under the box', () => {
    const start = startingOptions(signing(true), false);
    expect(start.sign).toBe(true);
    const unticked = { ...start, sign: false };
    expect(optionsSaid(unticked, start)).toBe('Not signed');
    expect(optionsArguments(unticked, signing(true), 'Fix')).toBe('git commit --no-gpg-sign');
    expect(optionAnswers(unticked, start)).toEqual({ sign: { kind: 'flag', value: false } });
  });

  it('with gpg.format ssh and no user.signingKey, Sign this commit cannot be ticked and says so', () => {
    const ssh = signing(true, 'ssh');
    const entries = commitOptionsMenu({
      options: startingOptions(ssh, false),
      signing: ssh,
      identity: { name: 'Ada', email: 'ada@example.com' },
      change: () => {},
      pickCoAuthors: () => {},
      switchIdentity: () => {},
    }).filter((entry): entry is MenuItem => typeof entry === 'object' && entry !== null);
    expect(entries[0]).toMatchObject({ label: 'Sign this commit', disabled: true, checked: false });
    expect(signsWith(ssh)).toBe('No key to sign with: set one with Switch identity…');
    expect(entries[1].label).toBe('Switch identity…');
  });

  it('gitardo.commit.signOff starts Add Signed-off-by ticked, and Skip the hooks never starts ticked', () => {
    const start = startingOptions(signing(false), true);
    expect(start).toEqual({ sign: false, signOff: true, skipHooks: false, coAuthors: [] });
    expect(optionsSaid(start, start)).toBe('');
    expect(optionAnswers(start, start)).toEqual({ 'sign-off': { kind: 'flag', value: true } });
  });

  it('two co-authors are listed in the menu and under the box, each taken away by a click', () => {
    const start = startingOptions(signing(false), false);
    const options = { ...start, skipHooks: true, coAuthors: ['Ben <ben@example.com>', 'Cy <cy@example.com>'] };
    expect(optionsSaid(options, start)).toBe('With Ben, +1 · Hooks skipped');
    let changed = options;
    const entries = commitOptionsMenu({
      options,
      signing: signing(false),
      identity: { name: 'Ada', email: 'ada@example.com' },
      change: (next) => (changed = next),
      pickCoAuthors: () => {},
      switchIdentity: () => {},
    }).filter((entry): entry is MenuItem => typeof entry === 'object' && entry !== null);
    const ben = entries.find((entry) => entry.label === 'Ben <ben@example.com>');
    expect(ben?.title).toBe('Remove from the co-authors');
    ben?.run();
    expect(changed.coAuthors).toEqual(['Cy <cy@example.com>']);
    expect(optionAnswers(options, start)['co-authors']).toEqual({ kind: 'choices', identifiers: options.coAuthors });
  });
});

describe('commit.co-authors in the extension', () => {
  const authors = [
    { name: 'Ben', email: 'ben@example.com', commits: 3 },
    { name: 'Ada', email: 'ada@example.com', commits: 5 },
  ];

  it('lists the authors of the history without the identity that commits, and offers someone typed first', () => {
    expect(coAuthorEntries(authors, 'Ada@Example.com', [], '').map((entry) => entry.label)).toEqual(['Ben']);
    const typed = coAuthorEntries(authors, 'ada@example.com', [], 'Dee <dee@example.com>');
    expect(typed[0]).toMatchObject({ label: 'Add Dee <dee@example.com>', coAuthor: 'Dee <dee@example.com>', typed: true });
    expect(coAuthorEntries(authors, 'ada@example.com', [], 'Dee').some((entry) => entry.typed)).toBe(false);
  });
});
