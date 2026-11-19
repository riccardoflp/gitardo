import { useEffect, useState } from 'react';
import type { RangeDiffRead } from '../shared/protocol';
import { changedParts, diffTone, type RangeDiffPair, type RangeDiffPairKind } from '../shared/rangeDiff';
import { Icon } from './Icon';
import { Progress } from './Progress';
import { shortHash } from './util';
import { onMessage, post } from './vscode';

/** The icon and the words of each kind of pair. */
const KINDS: Record<RangeDiffPairKind, { icon: string; words: string }> = {
  same: { icon: 'check', words: 'The same' },
  changed: { icon: 'diff-modified', words: 'Changed' },
  added: { icon: 'diff-added', words: 'Added' },
  dropped: { icon: 'diff-removed', words: 'Dropped' },
};

/** A short hash that opens the tab of its commit, or a dash where the pair has none. */
function Hash({ hash }: { hash: string | null }) {
  if (!hash) return <span className="muted mono">—</span>;
  return (
    <button className="link mono" title="Open this commit in a tab of its own" onClick={() => post({ type: 'openCommit', hash })}>
      {shortHash(hash)}
    </button>
  );
}

/** One pair: its kind, the hash before and the hash now, its subject, what changed, and its diff of diffs when open. */
function PairRow({ pair, open, onToggle }: { pair: RangeDiffPair; open: boolean; onToggle: () => void }) {
  const kind = KINDS[pair.kind];
  const changed = pair.kind === 'changed';
  return (
    <li className={`range-diff-pair ${pair.kind}`}>
      <div className="range-diff-row">
        {changed ? (
          <button className="link" aria-expanded={open} aria-label={open ? 'Hide what changed' : 'Show what changed'} onClick={onToggle}>
            <Icon name={open ? 'chevron-down' : 'chevron-right'} />
          </button>
        ) : (
          <span className="range-diff-spacer" />
        )}
        <span title={kind.words}>
          <Icon name={kind.icon} />
        </span>
        <Hash hash={pair.before} />
        <Icon name="arrow-right" />
        <Hash hash={pair.now} />
        <span className="range-diff-subject" onClick={changed ? onToggle : undefined}>
          {pair.subject}
        </span>
        {changed && <span className="muted">{changedParts(pair)}</span>}
      </div>
      {changed && open && (
        <pre className="range-diff-lines mono">
          {pair.diff.map((line, index) => (
            <div key={index} className={`range-diff-line ${diffTone(line)}`}>
              {line || ' '}
            </div>
          ))}
        </pre>
      )}
    </li>
  );
}

/**
 * The tab of what a rewrite changed, commit by commit (`range-diff.page`): its title, the two lines and the counts, and
 * one row for each pair, a changed one opening on its diff of diffs. The extension reads the pairs and sends them.
 */
export function RangeDiffApp() {
  const [read, setRead] = useState<RangeDiffRead | null>(null);
  const [opened, setOpened] = useState<ReadonlySet<number>>(new Set());

  useEffect(() => {
    const stop = onMessage((message) => {
      if (message.type === 'rangeDiff') {
        setRead(message.read);
        setOpened(new Set());
      }
    });
    post({ type: 'ready' });
    return stop;
  }, []);

  if (!read || read.kind === 'comparing') {
    return (
      <main className="range-diff">
        <Progress label="Comparing" page />
        <p className="muted">Comparing…</p>
      </main>
    );
  }
  const { title, ranges } = read.target;
  const toggle = (index: number) =>
    setOpened((now) => {
      const next = new Set(now);
      if (!next.delete(index)) next.add(index);
      return next;
    });
  return (
    <main className="range-diff">
      <h1>{title}</h1>
      {read.kind === 'failed' ? (
        <p className="range-diff-error">{read.message}</p>
      ) : (
        <>
          <p className="range-diff-lines-compared">
            <span>
              <strong>Before</strong> {read.pairs.beforeCount} {read.pairs.beforeCount === 1 ? 'commit' : 'commits'} after{' '}
              <span className="mono">{shortHash(ranges.before.from)}</span>
            </span>
            <span>
              <strong>Now</strong> {read.pairs.nowCount} {read.pairs.nowCount === 1 ? 'commit' : 'commits'} after{' '}
              <span className="mono">{shortHash(ranges.now.from)}</span>
            </span>
            <span className="muted">{read.pairs.counts}</span>
          </p>
          <ul className="range-diff-pairs">
            {read.pairs.pairs.map((pair, index) => (
              <PairRow key={index} pair={pair} open={opened.has(index)} onToggle={() => toggle(index)} />
            ))}
          </ul>
        </>
      )}
    </main>
  );
}
