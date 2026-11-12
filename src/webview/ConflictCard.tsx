import { Fragment, useEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from 'react';
import type { Hunk } from '../shared/protocol';
import { linesOf, proposalLines, summarize, type LineOrigin, type ProposalShape, type ProposalSummary } from '../shared/proposalLines';
import type { Asking, Choice, ConflictRef, DeletedConflict, Proposal, ProposalWarning, ReviewConflict, ReviewFile, SideChoice } from '../shared/resolution';
import { ChangeLines, Changes, Code, fileRows, numbering, PART, plural } from './conflictCode';
import { Columns, ORIGIN_CLASS, SideLabel, type Side } from './ConflictColumns';
import { FileIcon } from './FileIcon';
import { Icon } from './Icon';
import { sideChanges } from './sideChanges';
import { useColors } from './syntax';
import { post } from './vscode';

/** How a side is named in a sentence: its branch or commit when it is known, else what it is. */
const named = (side: Side) => side.name ?? `the ${side.role.toLowerCase()} side`;

/** A side's name in its color, as the sentences of a card say it. */
function Name({ side }: { side: Side }) {
  return <span className={`named ${side.role === 'Current' ? 'ours' : 'theirs'}`}>{named(side)}</span>;
}

/** What a proposal does with the two sides, in a sentence. */
function shapeSentence(summary: ProposalSummary, sides: [Side, Side]): ReactNode {
  const [current, incoming] = sides;
  const taken = summary.ours + summary.theirs + summary.both + summary.base;
  switch (summary.shape) {
    case 'current':
      return (
        <>
          Takes only the current side, <Name side={current} />
        </>
      );
    case 'incoming':
      return (
        <>
          Takes only the incoming side, <Name side={incoming} />
        </>
      );
    case 'current-first':
      return (
        <>
          Keeps both sides: <Name side={current} /> first, then <Name side={incoming} />
        </>
      );
    case 'incoming-first':
      return (
        <>
          Keeps both sides: <Name side={incoming} /> first, then <Name side={current} />
        </>
      );
    case 'nothing':
      return 'Leaves out the lines of both sides';
    case 'mixed':
      if (!summary.fresh) return 'Takes lines of both sides';
      if (!taken) return `Writes ${plural(summary.fresh, 'line')} of its own in place of both sides`;
      return `Takes lines of both sides, and writes ${plural(summary.fresh, 'line')} of its own`;
  }
}

/**
 * How many lines of a proposal come from where, as a bar and in words: the current side, both, the base, the incoming
 * side and none of them, in the colors the lines have.
 */
function Composition({ summary, sides, source }: { summary: ProposalSummary; sides: [Side, Side]; source: Proposal['source'] }) {
  const parts: [keyof ProposalSummary, number, string, string][] = [
    ['ours', summary.ours, 'ours', `from ${named(sides[0])}`],
    ['both', summary.both, 'both', 'on both sides'],
    ['base', summary.base, 'base', 'from the base'],
    ['theirs', summary.theirs, 'theirs', `from ${named(sides[1])}`],
    ['fresh', summary.fresh, 'new', 'new'],
  ];
  const total = parts.reduce((sum, [, count]) => sum + count, 0);
  // What the language model writes is counted even when it is nothing: that is what is worth knowing of it.
  const said = parts.filter(([key, count]) => count > 0 || (key === 'fresh' && source === 'ai'));
  if (!total) return null;
  return (
    <div className="composition" title="Where the lines of the proposal come from, empty lines aside">
      <div className="bar" aria-hidden="true">
        {parts
          .filter(([, count]) => count > 0)
          .map(([key, count, className]) => (
            <span key={key} className={className} style={{ flexGrow: count } as CSSProperties} />
          ))}
      </div>
      <span className="counts">
        {said.map(([key, count, className, words], index) => (
          <Fragment key={key}>
            {index > 0 && ' · '}
            <span className={className}>
              {count} {words}
            </span>
          </Fragment>
        ))}
      </span>
    </div>
  );
}

/** What a warning says on the card, short, and in its tooltip. */
function warningWords(warning: ProposalWarning, path: string, sides: [Side, Side]): [string, string] {
  if (warning.kind === 'invalid') return [`Not valid ${warning.format}`, `With this, ${path} is no longer valid ${warning.format}: ${warning.error}`];
  return [
    `${warning.key} is set twice`,
    `${named(sides[0])} and ${named(sides[1])} each set ${warning.key}, and the proposal keeps both lines: keep only one of them`,
  ];
}

/** The warnings of a proposal, each in a few words, with the whole of it in its tooltip. */
function Warnings({ warnings, path, sides }: { warnings: ProposalWarning[]; path: string; sides: [Side, Side] }) {
  if (!warnings.length) return null;
  const said = warnings.map((warning) => warningWords(warning, path, sides));
  const short = said.length === 1 ? said[0][0] : `${said.length} warnings`;
  return (
    <span className="warnings" role="note" title={said.map(([, long]) => long).join('\n')}>
      <Icon name="warning" />
      {short}
    </span>
  );
}

/** Says where a proposal comes from, and how sure the language model is of it. */
function Source({ proposal }: { proposal: Proposal }) {
  if (proposal.source === 'rules') {
    return (
      <span className="source rules" title="Every line of it is a line of one of the two sides">
        <Icon name="wand" />
        Rules
      </span>
    );
  }
  if (proposal.source === 'choice') {
    return (
      <span className="source choice" title="Chosen by you in place of what was proposed">
        <Icon name="person" />
        Your choice
      </span>
    );
  }
  return (
    <span className={`source ai ${proposal.confidence ?? ''}`} title="Written by the language model: look at it before you accept it">
      <Icon name="sparkle" />
      AI{proposal.model ? ` · ${proposal.model}` : ''}
      {proposal.confidence ? ` · ${proposal.confidence} confidence` : ''}
    </span>
  );
}

/** What the page shows while the language model is asked about a conflict. */
function AskingState({ asking, onRetry }: { asking: Asking; onRetry(): void }) {
  if (asking.state === 'waiting') {
    return (
      <div className="asking waiting" role="status">
        <div className="asking-label">
          <Icon name="sparkle" />
          Waiting for its turn…
        </div>
        <div className="skeleton" aria-hidden="true">
          <span />
          <span />
          <span />
        </div>
      </div>
    );
  }
  if (asking.state === 'thinking') {
    return (
      <div className="asking thinking" role="status">
        <div className="asking-label">
          <span className="orbit" aria-hidden="true">
            <Icon name="sparkle" />
          </span>
          {asking.text ? 'The AI is writing a resolution…' : 'The AI is reading the conflict…'}
        </div>
        {asking.text ? (
          <Code text={asking.text} className="streaming" caret />
        ) : (
          <div className="skeleton" aria-hidden="true">
            <span />
            <span />
            <span />
          </div>
        )}
      </div>
    );
  }
  return (
    <div className="asking failed" role="alert">
      <Icon name="error" />
      <span className="grow">The AI could not propose a resolution: {asking.error}</span>
      <button className="button" onClick={onRetry}>
        <Icon name="refresh" />
        Ask again
      </button>
    </div>
  );
}

/** The choices of the two sides a card offers, with what each button says and does. */
const CHOICES: { choice: SideChoice; label: string; title: (sides: [Side, Side]) => string }[] = [
  { choice: 'current', label: 'Current only', title: ([current]) => `Put only the lines of ${named(current)} in place of the conflict, and accept it` },
  { choice: 'incoming', label: 'Incoming only', title: ([, incoming]) => `Put only the lines of ${named(incoming)} in place of the conflict, and accept it` },
  {
    choice: 'current-first',
    label: 'Both, current first',
    title: ([current, incoming]) => `Put the lines of ${named(current)} and then those of ${named(incoming)} in place of the conflict, and accept it`,
  },
  {
    choice: 'incoming-first',
    label: 'Both, incoming first',
    title: ([current, incoming]) => `Put the lines of ${named(incoming)} and then those of ${named(current)} in place of the conflict, and accept it`,
  },
];

/** Lines written by hand in place of a conflict: a box with the lines to start from, and the ways to keep or drop them. */
function Editor({ text, onSave, onCancel }: { text: string; onSave(text: string): void; onCancel(): void }) {
  const [value, setValue] = useState(text);
  const box = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    box.current?.focus();
  }, []);
  return (
    <div className="editing">
      <textarea
        ref={box}
        className="code-edit"
        value={value}
        spellCheck={false}
        rows={Math.min(30, Math.max(3, linesOf(value).length + 1))}
        aria-label="The lines that take the place of the conflict"
        onChange={(event) => setValue(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === 'Escape') onCancel();
          if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) onSave(value);
        }}
      />
      <div className="editing-actions">
        <span className="muted grow">Ctrl+Enter to use these lines, Escape to leave them.</span>
        <button className="button" onClick={onCancel}>
          Cancel
        </button>
        <button className="button primary" title="Put these lines in place of the conflict, and accept them" onClick={() => onSave(value)}>
          <Icon name="check" />
          Use these lines
        </button>
      </div>
    </div>
  );
}

/** What a line of a proposal says when the pointer is on it: where it comes from, and what is wrong with it. */
function lineTitle(origin: LineOrigin | undefined, warned: string | undefined, sides: [Side, Side]): string | undefined {
  const from: Partial<Record<LineOrigin, string>> = {
    ours: `From ${named(sides[0])}`,
    theirs: `From ${named(sides[1])}`,
    both: 'On both sides',
    base: 'As it was before either side changed it',
    new: 'New: neither side has this line',
  };
  const said = [origin && from[origin], warned].filter(Boolean);
  return said.length ? said.join('. ') : undefined;
}

/** One conflict: what is proposed in its place and where its lines come from, the two sides, and what can be done with it. */
export function Conflict(props: {
  file: ReviewFile;
  conflict: ReviewConflict;
  sides: [Side, Side];
  ai: boolean;
  accepted: boolean;
  busy: boolean;
  /** Whether it is shown in three columns rather than as what is proposed over the two sides. */
  columns: boolean;
  /** Whether only its first line is shown, as for a conflict whose proposal is accepted. */
  folded: boolean;
  onAccept(accepted: boolean): void;
  /** Puts what the user chose in place of what is proposed, accepted, or takes the choice back with null. */
  onChoose(choice: Choice | null): void;
  onFold(folded: boolean): void;
  onWidths(weights: number[]): void;
}) {
  const { file, conflict, sides, ai, accepted, busy, columns, folded, onAccept, onChoose, onFold, onWidths } = props;
  const { proposal, asking, changes, warnings } = conflict;
  const ref: ConflictRef = { path: file.path, key: conflict.key };
  const parts = [
    conflict.before,
    conflict.ours,
    conflict.theirs,
    proposal?.text ?? '',
    conflict.after,
    conflict.base ?? '',
    conflict.region.ours,
    conflict.region.theirs,
    changes?.base ?? '',
    changes?.ours ?? '',
    changes?.theirs ?? '',
    changes?.before ?? '',
    changes?.after ?? '',
  ];
  const hunks = useMemo<Hunk[]>(() => parts.map((text) => ({ header: '', lines: linesOf(text).map((line) => ` ${line}`) })), parts);
  const colors = useColors(file.path, hunks, !folded);
  const [showBase, setShowBase] = useState(false);
  // The sides are what to look at when nothing is proposed; with a proposal they are there to check it, on demand.
  const [showSides, setShowSides] = useState<boolean | null>(null);
  const sidesShown = showSides ?? !proposal;
  const [editing, setEditing] = useState(false);
  const changed = useMemo(
    () => (changes && !folded && !columns && sidesShown ? { ours: sideChanges(changes.base, changes.ours), theirs: sideChanges(changes.base, changes.theirs) } : null),
    [changes?.base, changes?.ours, changes?.theirs, folded, columns, sidesShown],
  );
  const lines = useMemo(
    () => (proposal ? proposalLines(conflict.region.ours, conflict.region.theirs, proposal.text, conflict.base) : null),
    [conflict.region.ours, conflict.region.theirs, proposal?.text, conflict.base],
  );
  const summary = useMemo(
    () => (proposal && lines ? summarize(conflict.region.ours, conflict.region.theirs, proposal.text, lines.origins) : null),
    [lines],
  );
  // Each line of the proposal that a warning is about, with what the warning says of it.
  const warned = useMemo(() => {
    const found = new Map<number, string>();
    for (const warning of warnings) {
      if (warning.kind !== 'repeated') continue;
      for (const line of warning.lines) found.set(line, warningWords(warning, file.path, sides)[1]);
    }
    return found;
  }, [warnings]);
  const kind = (_line: string, index: number) => `${ORIGIN_CLASS[lines?.origins[index] ?? 'blank']}${warned.has(index) ? ' warned' : ''}`;
  const title = (_line: string, index: number) => lineTitle(lines?.origins[index], warned.get(index), sides);
  const numbers = numbering(conflict);
  const open = (editor: 'flowMerge' | 'mergeEditor' | 'text') => post({ type: 'openConflict', path: file.path, line: conflict.line, editor });
  const thinking = asking?.state === 'waiting' || asking?.state === 'thinking';

  const accept = proposal && (
    <button
      className={`button${accepted ? ' accept on' : ' primary accept'}`}
      disabled={busy}
      title={accepted ? 'Take back the acceptance: the conflict is left as it is' : 'Put this in place of the conflict when the accepted resolutions are written'}
      onClick={() => onAccept(!accepted)}
    >
      <Icon name={accepted ? 'pass-filled' : 'check'} />
      {accepted ? 'Accepted' : 'Accept'}
    </button>
  );
  const className = `conflict${accepted ? ' accepted' : ''}${proposal ? ` proposed ${proposal.source}` : ''}${folded ? ' folded' : ''}`;
  const head = (
    <>
      <button className="button icon" title={folded ? 'Show this conflict' : 'Show only the first line of this conflict'} aria-expanded={!folded} onClick={() => onFold(!folded)}>
        <Icon name={folded ? 'chevron-right' : 'chevron-down'} />
      </button>
      <button className="link" title="Open the file at this line" onClick={() => open('text')}>
        Line {conflict.line}
      </button>
    </>
  );

  if (folded) {
    const first = linesOf(proposal?.text ?? conflict.ours).find((line) => line.trim()) ?? '';
    return (
      <article className={className} aria-label={`Conflict at line ${conflict.line}`}>
        <header>
          {head}
          <code className="grow first-line" title={proposal?.text}>
            {first.trim()}
          </code>
          <Warnings warnings={warnings} path={file.path} sides={sides} />
          {proposal && <Source proposal={proposal} />}
          {accept}
        </header>
      </article>
    );
  }

  const nothing = asking ? (
    <AskingState asking={asking} onRetry={() => post({ type: 'askAI', conflicts: [ref] })} />
  ) : (
    <p className="muted nothing">Nothing is proposed: the two sides changed the same lines in different ways. Choose a side, or write the lines.</p>
  );
  const shape: ProposalShape | null = summary?.shape ?? null;
  const editFrom = proposal?.text ?? `${conflict.ours}${conflict.ours && !/\n$/.test(conflict.ours) ? '\n' : ''}${conflict.theirs}`;
  const editor = editing && (
    <Editor
      text={editFrom}
      onCancel={() => setEditing(false)}
      onSave={(text) => {
        setEditing(false);
        onChoose({ kind: 'edited', text });
      }}
    />
  );

  return (
    <article className={className} aria-label={`Conflict at line ${conflict.line}`} style={{ '--number-width': numbers.width } as CSSProperties}>
      <header>
        {head}
        <span className="grow summary">{summary ? shapeSentence(summary, sides) : <span className="muted">Nothing proposed yet</span>}</span>
        <Warnings warnings={warnings} path={file.path} sides={sides} />
        {conflict.base !== null && (
          <button className={`button icon${showBase ? ' toggled' : ''}`} title={showBase ? 'Hide the base' : 'Show what the lines were before either side changed them'} onClick={() => setShowBase(!showBase)}>
            <Icon name="history" />
          </button>
        )}
      </header>
      {summary && proposal && <Composition summary={summary} sides={sides} source={proposal.source} />}
      {showBase && conflict.base !== null && (
        <div className="side base">
          <div className="side-label">Base · before either side changed it</div>
          <Code text={conflict.base} first={changes ? changes.beforeLine + linesOf(changes.before).length : conflict.line} colors={colors?.[PART.base]} />
        </div>
      )}

      {columns ? (
        editor || <Columns conflict={conflict} sides={sides} accepted={accepted} colors={colors} lines={lines} kind={kind} title={title} middle={nothing} onWidths={onWidths} />
      ) : (
        <>
          {editor ||
            (proposal ? (
              <div className={`proposal ${proposal.source}`} key={`${proposal.source}:${proposal.text}`}>
                <Code text={conflict.before} first={numbers.before} colors={colors?.[PART.before]} className="around" />
                <Code text={proposal.text} first={numbers.start} colors={colors?.[PART.proposal]} className="resolved" kind={kind} title={title} />
                <Code text={conflict.after} first={numbers.after(proposal.text)} colors={colors?.[PART.after]} className="around" />
              </div>
            ) : (
              nothing
            ))}
          {sidesShown && (
            <div className="sides">
              <div className="side ours">
                <SideLabel side={sides[0]} />
                {changes && changed?.ours ? (
                  <Changes changes={changes} lines={changed.ours} side="ours" colors={colors} />
                ) : (
                  <Code text={conflict.ours} first={conflict.line} colors={colors?.[PART.ours]} />
                )}
              </div>
              <div className="side theirs">
                <SideLabel side={sides[1]} />
                {changes && changed?.theirs ? (
                  <Changes changes={changes} lines={changed.theirs} side="theirs" colors={colors} />
                ) : (
                  <Code text={conflict.theirs} first={conflict.line} colors={colors?.[PART.theirs]} />
                )}
              </div>
            </div>
          )}
        </>
      )}

      {!editing && (
        <div className="choices">
          <span className="muted">{proposal ? 'Instead:' : 'Choose:'}</span>
          {CHOICES.filter((each) => each.choice !== shape).map((each) => (
            <button key={each.choice} className="button" disabled={busy || thinking} title={each.title(sides)} onClick={() => onChoose({ kind: each.choice })}>
              {each.label}
            </button>
          ))}
          <button className="button" disabled={busy || thinking} title="Write the lines that take the place of the conflict, starting from what is proposed" onClick={() => setEditing(true)}>
            <Icon name="edit" />
            Edit
          </button>
          {proposal?.source === 'choice' && (
            <button className="button" disabled={busy} title="Take back your choice: what was proposed before it comes back" onClick={() => onChoose(null)}>
              <Icon name="discard" />
              Drop your choice
            </button>
          )}
          <span className="grow" />
          {!columns && proposal && (
            <button
              className={`button${sidesShown ? ' toggled' : ''}`}
              aria-expanded={sidesShown}
              title={sidesShown ? 'Hide what each side changed' : 'Show what each side changed of the base, to compare it with what is proposed'}
              onClick={() => setShowSides(!sidesShown)}
            >
              <Icon name={sidesShown ? 'chevron-up' : 'chevron-down'} />
              {sidesShown ? 'Hide the sides' : 'Compare with the sides'}
            </button>
          )}
        </div>
      )}

      <footer>
        {proposal && <Source proposal={proposal} />}
        <span className="grow reason" title={proposal?.reason}>
          {proposal?.reason}
          {proposal && proposal.newLines > 0 && (
            <span className="new-lines">
              <Icon name="warning" />
              {plural(proposal.newLines, 'line is', 'lines are')} new: neither side has {proposal.newLines === 1 ? 'it' : 'them'}.
            </span>
          )}
        </span>
        <button className="button" title="Resolve it in the Flow Merge, choosing for each change" onClick={() => open('flowMerge')}>
          <Icon name="git-merge" />
          Resolve by hand
        </button>
        <button className="button icon" title="Open it in the merge editor of VS Code" onClick={() => open('mergeEditor')}>
          <Icon name="split-horizontal" />
        </button>
        {ai && (!proposal || proposal.source === 'ai') && !thinking && (
          <button
            className="button"
            disabled={busy}
            title={proposal ? 'Ask the language model again: its answer takes the place of this one' : 'Ask the language model how to resolve it'}
            onClick={() => post({ type: 'askAI', conflicts: [ref] })}
          >
            <Icon name="sparkle" />
            {proposal ? 'Ask again' : 'Ask the AI'}
          </button>
        )}
        {accept}
      </footer>
    </article>
  );
}

/**
 * A file one side deleted and the other changed. There are no lines to choose between: the file is kept with the
 * changes of the side that has it, or deleted as the other side did.
 */
export function DeletedFile({ file, sides, busy }: { file: DeletedConflict; sides: [Side, Side]; busy: boolean }) {
  const [deleter, keeper] = file.deletedBy === 'ours' ? [sides[0], sides[1]] : [sides[1], sides[0]];
  const kept = file.deletedBy === 'ours' ? 'theirs' : 'ours';
  const hunks = useMemo<Hunk[]>(() => [file.base, file.changed].map((text) => ({ header: '', lines: linesOf(text).map((line) => ` ${line}`) })), [file.base, file.changed]);
  const colors = useColors(file.path, hunks, true);
  const lines = useMemo(() => sideChanges(file.base, file.changed), [file.base, file.changed]);
  const rows = useMemo(() => lines && fileRows(lines, colors), [lines, colors]);
  const sideName = (side: Side) => `${side.role.toLowerCase()} side${side.name ? ` (${side.name})` : ''}`;
  const take = (side: 'ours' | 'theirs') => post({ type: 'action', action: { kind: 'takeSide', path: file.path, side } });
  return (
    <section className="conflict-file deleted" aria-label={file.path}>
      <h2>
        <FileIcon path={file.path} />
        <span className="grow path" title={file.path}>
          {file.path}
        </span>
        <span className="muted">
          Deleted by the {sideName(deleter)}, changed by the {sideName(keeper)}
        </span>
      </h2>
      <article className="conflict">
        <div className="side">
          <SideLabel side={keeper} />
          {rows?.length ? (
            <ChangeLines rows={rows} titles={{ removed: 'Taken out of the file by this side', added: 'Written by this side', around: 'Line of the file', skipped: 'Lines left out here' }} />
          ) : (
            <p className="muted nothing">{rows ? 'This side changed nothing of the lines of the file.' : 'The changes are too many to show here.'}</p>
          )}
        </div>
        <footer>
          <button
            className="button primary"
            disabled={busy}
            title={`Keep ${file.path} as the ${keeper.role.toLowerCase()} side has it, and mark it as resolved`}
            onClick={() => take(kept)}
          >
            <Icon name="check" />
            Keep it, with these changes
          </button>
          <button
            className="button"
            disabled={busy}
            title={`Delete ${file.path}, as the ${deleter.role.toLowerCase()} side did: the changes above are left out`}
            onClick={() => take(file.deletedBy)}
          >
            <Icon name="trash" />
            Delete it
          </button>
          <button
            className="button icon"
            title="Show what it was beside what it is now, in the diff editor"
            onClick={() => post({ type: 'openConflict', path: file.path, line: 1, editor: 'mergeEditor' })}
          >
            <Icon name="diff" />
          </button>
        </footer>
      </article>
    </section>
  );
}
