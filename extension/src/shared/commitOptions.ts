/**
 * The options of the commit box of **Gitardo Changes** (`commit.options`): what each starts as, from git's configuration
 * and `gitardo.commit.signOff`, what the line under the box says of those changed, and the words of the signature of a
 * commit that the details beside the graph and the tab of a commit show (`details.signature`). The arguments they add
 * to `git commit` are the engine's (`commit.option-arguments`); what is here only says them.
 */
import type { Author, CommitSignature, Signing } from './state';

/** What is ticked in the menu of the commit box for the next commit, and the co-authors chosen, each `Name <email>`. */
export interface CommitOptions {
  sign: boolean;
  signOff: boolean;
  skipHooks: boolean;
  coAuthors: string[];
}

/** Where the options start for each commit: signed as git's configuration signs, `Signed-off-by` as the setting says. */
export function startingOptions(signing: Signing, signOff: boolean): CommitOptions {
  return { sign: signing.signsEveryCommit && canSign(signing), signOff, skipHooks: false, coAuthors: [] };
}

/** Whether git has a key to sign with: OpenPGP finds one from the email of the identity, SSH and X.509 need `user.signingKey` (D79). */
export function canSign(signing: Signing): boolean {
  return Boolean(signing.key?.trim()) || signing.kind === 'openpgp';
}

const KINDS: Record<Signing['kind'], string> = { openpgp: 'OpenPGP', x509: 'X.509', ssh: 'SSH' };

/** What **Sign this commit** signs with: `With the SSH key ~/.ssh/id_ed25519.pub`, or the key of the email when none is set. */
export function signsWith(signing: Signing): string {
  if (!canSign(signing)) return 'No key to sign with: set one with Switch identity…';
  const kind = KINDS[signing.kind];
  return signing.key ? `With the ${kind} key ${signing.key}` : `With the ${kind} key of your email`;
}

/** A co-author typed as `Name <email>`, or null without a name, or without an email in angle brackets. */
export function parseCoAuthor(typed: string): { name: string; email: string } | null {
  const match = /^([^<>]*)<([^<>]+)>$/.exec(typed.trim());
  if (!match) return null;
  const name = match[1].trim();
  const email = match[2].trim();
  return name && email.includes('@') ? { name, email } : null;
}

/** The line of the message that names a co-author, as the engine writes it. */
export const coAuthorTrailer = (coAuthor: string): string => {
  const parsed = parseCoAuthor(coAuthor);
  return parsed ? `Co-authored-by: ${parsed.name} <${parsed.email}>` : '';
};

/**
 * The line under the box, in the order of the menu, each part only while its option differs from where it starts:
 * `Not signed · Signed-off-by · With Ben, +1 · Hooks skipped`. Empty when nothing differs.
 */
export function optionsSaid(options: CommitOptions, start: CommitOptions): string {
  const said: string[] = [];
  if (options.sign !== start.sign) said.push(options.sign ? 'Signed' : 'Not signed');
  if (options.signOff !== start.signOff) said.push(options.signOff ? 'Signed-off-by' : 'No Signed-off-by');
  const names = options.coAuthors.map((coAuthor) => parseCoAuthor(coAuthor)?.name ?? coAuthor);
  if (names.length === 1) said.push(`With ${names[0]}`);
  else if (names.length > 1) said.push(`With ${names[0]}, +${names.length - 1}`);
  if (options.skipHooks) said.push('Hooks skipped');
  return said.join(' · ');
}

/** The tooltip of the line under the box: the arguments the options add to `git commit`, as the engine adds them. */
export function optionsArguments(options: CommitOptions, signing: Signing, message: string): string {
  const added: string[] = [];
  if (options.sign && !signing.signsEveryCommit) added.push('--gpg-sign');
  if (!options.sign && signing.signsEveryCommit) added.push('--no-gpg-sign');
  if (options.signOff) added.push('--signoff');
  if (options.skipHooks) added.push('--no-verify');
  const lines = message.split('\n').map((line) => line.trim());
  for (const coAuthor of options.coAuthors) {
    const trailer = coAuthorTrailer(coAuthor);
    if (trailer && !lines.includes(trailer)) added.push(`--trailer "${trailer}"`);
  }
  return added.length ? `git commit ${added.join(' ')}` : '';
}

/** What the line **Signature** says of what git said: `Good, Ada <ada@example.com>`, `Bad`; empty for a commit not signed. */
export function signatureWords(signature: CommitSignature): string {
  switch (signature.verdict) {
    case 'good':
      return signature.signer ? `Good, ${signature.signer}` : 'Good';
    case 'goodUnknownValidity':
      return 'Good, from a key whose validity is not known';
    case 'goodExpired':
      return 'Good, expired';
    case 'goodExpiredKey':
      return 'Good, expired key';
    case 'goodRevokedKey':
      return 'Good, revoked key';
    case 'bad':
      return 'Bad';
    case 'cannotCheck':
      return signature.key ? `Cannot be checked: no key ${signature.key} here` : 'Cannot be checked';
    case 'signedNotChecked':
      return signature.kind === 'ssh' ? 'Signed, not checked: no allowed signers are set' : 'Signed, not checked';
    case 'notSigned':
      return '';
  }
}

/** The color of the line: an addition for a good signature, a warning for one git doubts, a removal for a bad one. */
export function signatureTone(signature: CommitSignature): 'good' | 'doubted' | 'bad' | 'muted' {
  switch (signature.verdict) {
    case 'good':
      return 'good';
    case 'goodUnknownValidity':
    case 'goodExpired':
    case 'goodExpiredKey':
    case 'goodRevokedKey':
      return 'doubted';
    case 'bad':
      return 'bad';
    case 'cannotCheck':
    case 'signedNotChecked':
    case 'notSigned':
      return 'muted';
  }
}

/** The tooltip of the line: the kind of signature, the key, its fingerprint and the trust git gave it, each when git said it. */
export function signatureTooltip(signature: CommitSignature): string {
  const kind = signature.kind ? `${KINDS[signature.kind]} signature` : 'Signature';
  const lines = [kind];
  for (const [name, value] of [
    ['Key', signature.key],
    ['Fingerprint', signature.fingerprint],
    ['Trust', signature.trust],
  ] as const) {
    if (value) lines.push(`${name}: ${value}`);
  }
  return lines.join('\n');
}

/** An entry of the quick pick of the co-authors: someone of the history, or someone typed who is not. */
export interface CoAuthorEntry {
  /** `Name <email>`, as the engine is given it. */
  coAuthor: string;
  label: string;
  description?: string;
  /** Set on the one typed, `Add Dee <dee@example.com>`, which is listed first. */
  typed?: boolean;
}

/**
 * The entries of the quick pick of **Co-authors of this commit** (`commit.co-authors`): someone typed as `Name <email>`
 * whom no author has first, as **Add …**, then those chosen already that the history does not have, then the authors
 * of the history, the one with the most commits first, without the identity that commits.
 */
export function coAuthorEntries(authors: Author[], identityEmail: string | null, chosen: string[], typed: string): CoAuthorEntry[] {
  const same = (left: string, right: string) => left.toLowerCase() === right.toLowerCase();
  const listed = authors.filter((author) => !(identityEmail && same(author.email, identityEmail)));
  const known = (email: string) => listed.some((author) => same(author.email, email));
  const entries: CoAuthorEntry[] = [];
  const added = parseCoAuthor(typed);
  if (added && !known(added.email) && !chosen.some((coAuthor) => same(parseCoAuthor(coAuthor)?.email ?? '', added.email))) {
    const coAuthor = `${added.name} <${added.email}>`;
    entries.push({ coAuthor, label: `Add ${coAuthor}`, typed: true });
  }
  for (const coAuthor of chosen) {
    const parsed = parseCoAuthor(coAuthor);
    if (parsed && !known(parsed.email)) entries.push({ coAuthor, label: parsed.name, description: parsed.email });
  }
  for (const author of listed) {
    const commits = author.commits === 1 ? '1 commit' : `${author.commits} commits`;
    entries.push({ coAuthor: `${author.name} <${author.email}>`, label: author.name, description: `${author.email} · ${commits}` });
  }
  return entries;
}
