import { useEffect, useState } from 'react';
import { signatureTone, signatureTooltip, signatureWords } from '../shared/commitOptions';
import type { CommitSignature } from '../shared/state';
import { Icon } from './Icon';
import { onMessage, post } from './vscode';

/** What is known of the signature of a commit shown: being read, or what git said of it. */
type Read = { hash: string; signature: CommitSignature | null | 'checking' };

/**
 * The signature of a commit, asked once its details are shown, so that they never wait for it (`details.signature`):
 * none asked for the uncommitted changes. A commit selected after it asks again, and its read cancels the one before.
 */
export function useSignature(hash: string, shown: boolean): CommitSignature | null | 'checking' {
  const [read, setRead] = useState<Read | null>(null);
  useEffect(() => {
    if (!shown) return;
    setRead({ hash, signature: 'checking' });
    const stop = onMessage((message) => {
      if (message.type === 'signature' && message.hash === hash) setRead({ hash, signature: message.signature });
    });
    post({ type: 'getSignature', hash });
    return stop;
  }, [hash, shown]);
  return read?.hash === hash ? read.signature : null;
}

/**
 * The line **Signature** of the details and of the tab of a commit: what git said, in the color of what it said, with
 * the kind, the key, its fingerprint and its trust in its tooltip. A commit not signed, or whose signature could not be
 * read, has no line.
 */
export function SignatureLine({ signature }: { signature: CommitSignature | null | 'checking' }) {
  if (signature === null) return null;
  if (signature === 'checking') {
    return (
      <>
        <dt>Signature</dt>
        <dd className="muted">Checking the signature…</dd>
      </>
    );
  }
  const words = signatureWords(signature);
  if (!words) return null;
  return (
    <>
      <dt>Signature</dt>
      <dd className={`signature ${signatureTone(signature)}`} title={signatureTooltip(signature)}>
        <Icon name="shield" /> {words}
      </dd>
    </>
  );
}
