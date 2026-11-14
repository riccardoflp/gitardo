import { useEffect, useMemo, useRef, useState, type CSSProperties } from 'react';
import type { Choice, ConflictRef, ConflictReview, DeletedConflict, ReviewConflict, ReviewFile } from '../shared/resolution';
import { Conflict, DeletedFile } from './ConflictCard';
import { widths, type Side } from './ConflictColumns';
import { plural } from './conflictCode';
import { FileIcon } from './FileIcon';
import { Icon } from './Icon';
import { loadUiState, onMessage, post, saveUiState } from './vscode';

/**
 * What the page remembers while it is hidden: the conflicts whose proposal was accepted, by file and key, whether
 * they are shown in columns, and how wide the columns were made.
 */
interface Remembered {
  accepted: string[];
  columns?: boolean;
  widths?: number[];
  /** The file chosen in the list, or none for all of them, and which of their conflicts are shown. */
  file?: string | null;
  filter?: Filter;
}

/** Which conflicts are shown: all of them, those not accepted yet, those with nothing proposed, or those from the AI. */
type Filter = 'all' | 'open' | 'nothing' | 'ai';

const FILTERS: { filter: Filter; label: string; title: string }[] = [
  { filter: 'all', label: 'All', title: 'Show every conflict' },
  { filter: 'open', label: 'Not accepted', title: 'Show the conflicts whose resolution is not accepted yet' },
  { filter: 'nothing', label: 'Nothing proposed', title: 'Show the conflicts with nothing proposed, to resolve by hand or to ask the AI about' },
  { filter: 'ai', label: 'From the AI', title: 'Show the conflicts the AI proposed a resolution for, to look at before you accept them' },
];

/** Whether a conflict is one the filter shows. */
function shows(filter: Filter, conflict: ReviewConflict, accepted: boolean): boolean {
  if (filter === 'open') return !accepted;
  if (filter === 'nothing') return !conflict.proposal;
  if (filter === 'ai') return conflict.proposal?.source === 'ai';
  return true;
}

const idOf = (path: string, key: string) => `${path}\0${key}`;

/** What the last resolutions written did, with the way back and the way on. */
function Report({ review }: { review: ConflictReview }) {
  const report = review.report;
  if (!report) return null;
  const left = review.files.reduce((sum, file) => sum + file.conflicts.length, 0);
  const done = !review.conflicted;
  return (
    <div className="report" role="status">
      <Icon name="pass" />
      <div className="grow">
        <strong>
          Wrote the {report.resolved === 1 ? 'resolution of 1 conflict' : `resolutions of ${report.resolved} conflicts`} in {plural(report.files, 'file')}
          {report.fromAI ? `, ${report.fromAI === report.resolved ? 'all' : report.fromAI} from the AI` : ''}.
        </strong>{' '}
        {report.staged ? `${plural(report.staged, 'file was', 'files were')} left with no conflict, and staged. ` : ''}
        {done
          ? `No file is left in conflict${review.operation ? `: the ${review.operation} can continue` : ''}.`
          : left
            ? `${plural(left, 'conflict is', 'conflicts are')} left to resolve.`
            : `${plural(review.conflicted, 'file is', 'files are')} still in conflict.`}
      </div>
      <button className="button" title="Put back the conflicts of the files written, as git left them" onClick={() => post({ type: 'action', action: { kind: 'undo' } })}>
        <Icon name="discard" />
        Undo
      </button>
      {done && review.operation && (
        <button className="button primary" title={`Commit what is staged and go on with the ${review.operation}`} onClick={() => post({ type: 'action', action: { kind: 'continue' } })}>
          <Icon name="debug-continue" />
          Continue the {review.operation}
        </button>
      )}
    </div>
  );
}

/** The bar that says how far the language model has gone, while it is asked. */
function Progress({ review }: { review: ConflictReview }) {
  const all = review.files.flatMap((file) => file.conflicts);
  const asked = all.filter((conflict) => conflict.asking && conflict.asking.state !== 'failed').length;
  if (!review.asking) return null;
  const answered = all.filter((conflict) => conflict.proposal?.source === 'ai').length;
  const total = asked + answered;
  return (
    <div className="ai-progress" role="status">
      <span className="orbit" aria-hidden="true">
        <Icon name="sparkle" />
      </span>
      <span className="grow">
        Asking the AI: {answered} of {plural(total, 'conflict')} answered
      </span>
      <button className="button" onClick={() => post({ type: 'stopAsking' })}>
        <Icon name="debug-stop" />
        Stop
      </button>
      <div className="bar" style={{ '--done': total ? answered / total : 0 } as CSSProperties} aria-hidden="true" />
    </div>
  );
}

/** Where a file of the review stands: how many of its conflicts are accepted, and whether some have nothing proposed. */
function FileState({ file, accepted }: { file: ReviewFile; accepted: (conflict: ReviewConflict) => boolean }) {
  const total = file.conflicts.length;
  const done = file.conflicts.filter(accepted).length;
  const nothing = file.conflicts.filter((conflict) => !conflict.proposal).length;
  const [icon, state, title] =
    done === total
      ? ['pass-filled', 'done', 'Every conflict of it is accepted']
      : nothing
        ? ['circle-filled', 'nothing', `${plural(nothing, 'conflict has', 'conflicts have')} nothing proposed`]
        : ['circle-outline', 'open', 'Something is proposed for each conflict: not all of it is accepted yet'];
  return (
    <span className={`file-state ${state}`} title={title}>
      {done}/{total}
      <Icon name={icon} />
    </span>
  );
}

/**
 * The files of the review, beside their conflicts: which ones are shown, and where each file stands. A file chosen
 * here is shown alone; the first entry shows them all again.
 */
function FileList(props: {
  review: ConflictReview;
  filter: Filter;
  selected: string | null;
  /** How many conflicts each filter shows, in the order of `FILTERS`. */
  counts: number[];
  accepted: (file: ReviewFile, conflict: ReviewConflict) => boolean;
  /** Whether a file has a conflict the filter shows. */
  listed: (file: ReviewFile) => boolean;
  /** The files a side deleted that the filter shows. */
  deleted: DeletedConflict[];
  onFilter(filter: Filter): void;
  onSelect(path: string | null): void;
}) {
  const { review, filter, selected, counts, accepted, listed, deleted, onFilter, onSelect } = props;
  const files = review.files.filter(listed);
  const total = counts[FILTERS.findIndex((each) => each.filter === filter)];
  return (
    <nav className="conflicts-nav" aria-label="Files in conflict">
      <div className="filters" role="radiogroup" aria-label="Which conflicts are shown">
        {FILTERS.map((each, index) => (
          <button key={each.filter} className={`filter${filter === each.filter ? ' on' : ''}`} role="radio" aria-checked={filter === each.filter} title={each.title} onClick={() => onFilter(each.filter)}>
            <span className="grow">{each.label}</span>
            <span className="count">{counts[index]}</span>
          </button>
        ))}
      </div>
      <div className="file-entries">
        <button className={`file-entry${selected === null ? ' on' : ''}`} aria-current={selected === null} title="Show the conflicts of every file" onClick={() => onSelect(null)}>
          <Icon name="files" />
          <span className="grow name">All files</span>
          <span className="file-state">{plural(total, 'conflict')}</span>
        </button>
        {files.map((file) => {
          const slash = file.path.lastIndexOf('/');
          return (
            <button key={file.path} className={`file-entry${selected === file.path ? ' on' : ''}`} aria-current={selected === file.path} title={file.path} onClick={() => onSelect(file.path)}>
              <FileIcon path={file.path} />
              <span className="grow name">
                {file.path.slice(slash + 1)}
                {slash > 0 && <span className="folder"> {file.path.slice(0, slash)}</span>}
              </span>
              <FileState file={file} accepted={(conflict) => accepted(file, conflict)} />
            </button>
          );
        })}
        {!files.length && !deleted.length && <p className="muted none">No file has a conflict to show here.</p>}
      </div>
      {deleted.length > 0 && (
        <div className="file-entries others">
          <div className="others-label" title="Files one side deleted and the other changed: keep them with those changes, or delete them">
            Deleted on one side
          </div>
          {deleted.map((file) => {
            const slash = file.path.lastIndexOf('/');
            return (
              <button key={file.path} className={`file-entry${selected === file.path ? ' on' : ''}`} aria-current={selected === file.path} title={file.path} onClick={() => onSelect(file.path)}>
                <FileIcon path={file.path} />
                <span className="grow name">
                  {file.path.slice(slash + 1)}
                  {slash > 0 && <span className="folder"> {file.path.slice(0, slash)}</span>}
                </span>
                <Icon name="trash" />
              </button>
            );
          })}
        </div>
      )}
      {review.others.length > 0 && (
        <div className="file-entries others">
          <div className="others-label" title="Binary files, very large files, and files both sides added are resolved in an editor, or by taking one side">
            Also in conflict
          </div>
          {review.others.map((file) => (
            <button key={file} className="file-entry" title={`${file}: open it in the merge editor`} onClick={() => post({ type: 'openConflict', path: file, line: 1, editor: 'mergeEditor' })}>
              <FileIcon path={file} />
              <span className="grow name">{file.slice(file.lastIndexOf('/') + 1)}</span>
              <Icon name="go-to-file" />
            </button>
          ))}
        </div>
      )}
    </nav>
  );
}

/** The tab where what is proposed for the conflicts is looked at, accepted and written. */
export function ConflictsApp() {
  // Undefined while it is read.
  const [review, setReview] = useState<ConflictReview | null>();
  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(false);
  const [accepted, setAccepted] = useState(() => new Set(loadUiState<Remembered>({ accepted: [] }).accepted));
  const [columns, setColumns] = useState(() => loadUiState<Remembered>({ accepted: [] }).columns === true);
  const [kept, setKept] = useState(() => loadUiState<Remembered>({ accepted: [] }).widths);
  widths.now ??= kept;
  /**
   * The conflicts and files folded or unfolded by hand, by id and by path. The others are folded once accepted: a
   * conflict when its proposal is, a file when all of its conflicts are.
   */
  const [folds, setFolds] = useState(() => new Map<string, boolean>());
  const [selected, setSelected] = useState(() => loadUiState<Remembered>({ accepted: [] }).file ?? null);
  const [filter, setFilter] = useState<Filter>(() => loadUiState<Remembered>({ accepted: [] }).filter ?? 'all');
  /**
   * The conflicts accepted or taken back since the filter was chosen: they stay in view, folded, until it changes,
   * rather than leaving under the pointer the moment they are accepted.
   */
  const [lingering, setLingering] = useState(() => new Set<string>());
  const list = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const stop = onMessage((message) => {
      if (message.type === 'review') {
        setReview(message.review);
        setError(message.error);
      }
      if (message.type === 'busy') setBusy(message.title !== null);
    });
    post({ type: 'ready' });
    return stop;
  }, []);

  // What was accepted and is no longer there, written or edited, is forgotten.
  const present = useMemo(() => new Set(review?.files.flatMap((file) => file.conflicts.filter((each) => each.proposal).map((each) => idOf(file.path, each.key)))), [review]);
  const chosen = useMemo(() => [...accepted].filter((id) => present.has(id)), [accepted, present]);
  useEffect(() => saveUiState({ accepted: chosen, columns, widths: kept, file: selected, filter }), [chosen, columns, kept, selected, filter]);
  // Another file, or other conflicts, are read from their top.
  useEffect(() => {
    if (list.current) list.current.scrollTop = 0;
  }, [selected, filter]);

  if (review === undefined) return <div className="empty">Reading the files in conflict…</div>;
  if (review === null) return <div className="empty">Gitardo could not read the files in conflict. {error}</div>;

  const conflicts = review.files.flatMap((file) => file.conflicts.map((conflict) => ({ file, conflict })));
  const byRules = conflicts.filter(({ conflict }) => conflict.proposal?.source === 'rules').length;
  const byAI = conflicts.filter(({ conflict }) => conflict.proposal?.source === 'ai').length;
  const byYou = conflicts.filter(({ conflict }) => conflict.proposal?.source === 'choice').length;
  const unanswered = conflicts.filter(({ conflict }) => !conflict.proposal);
  const askable = unanswered.filter(({ conflict }) => conflict.asking?.state !== 'waiting' && conflict.asking?.state !== 'thinking').length;
  const refs = (ids: string[]): ConflictRef[] => ids.map((id) => ({ path: id.slice(0, id.indexOf('\0')), key: id.slice(id.indexOf('\0') + 1) }));
  /** Accepts conflicts or takes them back; `open` keeps them in view, as after a choice whose result is to be seen. */
  const accept = (ids: string[], on: boolean, open = false) => {
    const next = new Set(accepted);
    const unfolded = new Map(folds);
    for (const id of ids) {
      if (on) next.add(id);
      else next.delete(id);
      // What is accepted or taken back folds or unfolds again as it would by itself, and so does its file.
      for (const each of [id, id.slice(0, id.indexOf('\0'))]) {
        if (open) unfolded.set(each, false);
        else unfolded.delete(each);
      }
    }
    setAccepted(next);
    setFolds(unfolded);
    if (filter !== 'all') setLingering(new Set([...lingering, ...ids]));
  };
  const fold = (id: string, folded: boolean) => setFolds(new Map(folds).set(id, folded));
  const choose = (next: Filter) => {
    setFilter(next);
    setLingering(new Set());
  };
  const isAccepted = (file: ReviewFile, conflict: ReviewConflict) => accepted.has(idOf(file.path, conflict.key)) && !!conflict.proposal;
  const shown = (file: ReviewFile, conflict: ReviewConflict) =>
    shows(filter, conflict, isAccepted(file, conflict)) || lingering.has(idOf(file.path, conflict.key));
  const listed = (file: ReviewFile) => file.conflicts.some((conflict) => shown(file, conflict));
  const counts = FILTERS.map((each) => conflicts.filter(({ file, conflict }) => shows(each.filter, conflict, isAccepted(file, conflict))).length);
  // A file no longer in conflict, or no longer shown, gives way to all of them.
  const current = review.files.find((file) => file.path === selected && listed(file));
  // The files a side deleted have nothing proposed, and are shown with the conflicts that have none.
  const deleted = filter === 'ai' ? [] : review.deleted;
  const currentDeleted = current ? undefined : deleted.find((file) => file.path === selected);
  const files = currentDeleted ? [] : (current ? [current] : review.files).filter(listed);
  const deletedShown = currentDeleted ? [currentDeleted] : current ? [] : deleted;
  // A proposal with a warning is accepted only one by one, after it was looked at: never with the others.
  const unwarned = (file: ReviewFile) => file.conflicts.filter((conflict) => conflict.proposal && !conflict.warnings.length).map((conflict) => idOf(file.path, conflict.key));
  const allProposed = review.files.flatMap(unwarned);
  const warnedLeft = conflicts.filter(({ file, conflict }) => conflict.warnings.length && conflict.proposal && !accepted.has(idOf(file.path, conflict.key))).length;
  const pick = (file: ReviewFile, conflict: ReviewConflict, choice: Choice | null) => {
    post({ type: 'chooseResolution', conflict: { path: file.path, key: conflict.key }, choice });
    // What is chosen is accepted with the choice, and stays in view for its result to be seen; what was proposed before
    // it comes back as it was, not accepted.
    accept([idOf(file.path, conflict.key)], choice !== null, true);
  };
  const sides: [Side, Side] = [
    { role: 'Current', label: review.oursLabel, name: review.oursName, subject: review.oursSubject },
    { role: 'Incoming', label: review.theirsLabel, name: review.theirsName, subject: review.theirsSubject },
  ];
  const summary = [
    byRules && `${byRules} proposed by the rules`,
    byAI && `${byAI} by the AI`,
    byYou && `${byYou} chosen by you`,
    unanswered.length && `${unanswered.length} with nothing proposed`,
  ].filter(Boolean);
  const said = [
    conflicts.length && `${plural(conflicts.length, 'conflict')} in ${plural(review.files.length, 'file')}${summary.length ? `: ${summary.join(', ')}` : ''}`,
    review.deleted.length && `${plural(review.deleted.length, 'file')} deleted on one side and changed on the other`,
  ].filter(Boolean);

  return (
    <main className={`conflicts-page${columns ? ' wide' : ''}`}>
      <header className="conflicts-head">
        <h1>
          Conflicts
          {review.operation && <span className="muted"> of the {review.operation}</span>}
        </h1>
        <p className="muted">
          {said.length ? `${said.join('; ')}. Nothing is written until you write what you accepted.` : 'No conflict with markers to look at here.'}
        </p>
        {!review.rules && !review.ai && <p className="note">Neither the rules nor the AI are switched on in the settings: nothing is proposed.</p>}
        {warnedLeft > 0 && (
          <p className="note warned-note">
            <Icon name="warning" />
            {plural(warnedLeft, 'proposal has a warning', 'proposals have a warning')}: look at {warnedLeft === 1 ? 'it' : 'them'} and accept one by one.
          </p>
        )}
        <div className="actions">
          <button
            className="button"
            disabled={!allProposed.length || allProposed.every((id) => accepted.has(id))}
            title="Accept every resolution proposed, from the rules, from the AI and chosen by you, but those with a warning, which are accepted one by one"
            onClick={() => accept(allProposed, true)}
          >
            <Icon name="check-all" />
            Accept all proposed
          </button>
          {chosen.length > 0 && (
            <button className="button" title="Take back every acceptance" onClick={() => accept(chosen, false)}>
              <Icon name="clear-all" />
              Clear
            </button>
          )}
          {review.ai && (
            <button
              className="button ask-all"
              disabled={!askable || review.asking}
              title="Ask the language model about each conflict that has nothing proposed"
              onClick={() => post({ type: 'askAI' })}
            >
              <Icon name="sparkle" />
              Ask the AI about the rest{askable ? ` (${askable})` : ''}
            </button>
          )}
          {review.ai && (
            <button
              className="button model"
              title="Choose the language model that resolves the conflicts, and how much it thinks"
              onClick={() => post({ type: 'chooseConflictModel' })}
            >
              <Icon name="hubot" />
              <span className="name">{review.model ?? 'No language model'}</span>
            </button>
          )}
          <span className="grow" />
          <div className="segmented" role="group" aria-label="How each conflict is shown">
            <button
              className={`button${columns ? '' : ' on'}`}
              aria-pressed={!columns}
              title="Show the two sides of each conflict over what is proposed, with the lines around it"
              onClick={() => setColumns(false)}
            >
              <Icon name="list-flat" />
              Cards
            </button>
            <button
              className={`button${columns ? ' on' : ''}`}
              aria-pressed={columns}
              title="Show each conflict in three columns, as the Flow Merge does: the current side, what is proposed and the incoming side, joined by bands"
              onClick={() => setColumns(true)}
            >
              <Icon name="editor-layout" />
              Columns
            </button>
          </div>
          <button
            className="button"
            disabled={!chosen.length}
            title="Show each file beside what the accepted resolutions make of it, in the diff editor"
            onClick={() => post({ type: 'previewResolutions', accepted: refs(chosen) })}
          >
            <Icon name="diff-multiple" />
            Show the result
          </button>
          <button
            className="button primary"
            disabled={!chosen.length || busy}
            title="Write the accepted resolutions in the files, and stage the files left with no conflict"
            onClick={() => post({ type: 'writeResolutions', accepted: refs(chosen) })}
          >
            <Icon name="save-all" />
            {chosen.length ? `Write ${plural(chosen.length, 'resolution')}` : 'Write the accepted'}
          </button>
        </div>
        <Progress review={review} />
      </header>

      <Report review={review} />

      <div className="conflicts-body">
        {(review.files.length > 0 || review.deleted.length > 0 || review.others.length > 0) && (
          <FileList
            review={review}
            filter={filter}
            selected={current?.path ?? currentDeleted?.path ?? null}
            counts={counts}
            accepted={isAccepted}
            listed={listed}
            deleted={deleted}
            onFilter={choose}
            onSelect={setSelected}
          />
        )}
        <div className="conflicts-list" ref={list}>
          {files.map((file) => {
            const ids = file.conflicts.filter((conflict) => conflict.proposal).map((conflict) => idOf(file.path, conflict.key));
            const all = ids.length > 0 && ids.every((id) => accepted.has(id));
            const acceptable = unwarned(file);
            const settled = all && ids.length === file.conflicts.length;
            // A file chosen alone is not folded unless asked: it is there to be looked at.
            const folded = folds.get(file.path) ?? (settled && !current);
            return (
              <section key={file.path} className={`conflict-file${folded ? ' folded' : ''}`} aria-label={file.path}>
                <h2>
                  <button
                    className="button icon"
                    title={folded ? 'Show the conflicts of this file' : 'Hide the conflicts of this file'}
                    aria-expanded={!folded}
                    onClick={() => fold(file.path, !folded)}
                  >
                    <Icon name={folded ? 'chevron-right' : 'chevron-down'} />
                  </button>
                  <FileIcon path={file.path} />
                  <span className="grow path" title={file.path}>
                    {file.path}
                  </span>
                  <span className="muted">
                    {settled
                      ? `${file.conflicts.length === 1 ? 'Its conflict is' : `All ${file.conflicts.length} conflicts are`} accepted`
                      : `${ids.length} of ${plural(file.conflicts.length, 'conflict')} proposed`}
                  </span>
                  <button
                    className="button"
                    disabled={!acceptable.length || acceptable.every((id) => accepted.has(id))}
                    title="Accept every resolution proposed in this file, but those with a warning, which are accepted one by one"
                    onClick={() => accept(acceptable, true)}
                  >
                    <Icon name="check-all" />
                    Accept the file
                  </button>
                </h2>
                {!folded &&
                  file.conflicts.filter((conflict) => shown(file, conflict)).map((conflict) => {
                    const id = idOf(file.path, conflict.key);
                    const on = accepted.has(id);
                    return (
                      <Conflict
                        key={conflict.key}
                        file={file}
                        conflict={conflict}
                        sides={sides}
                        ai={review.ai}
                        busy={busy}
                        columns={columns}
                        accepted={on}
                        folded={folds.get(id) ?? (on && !!conflict.proposal)}
                        onAccept={(yes) => accept([id], yes)}
                        onChoose={(choice) => pick(file, conflict, choice)}
                        onFold={(yes) => fold(id, yes)}
                        onWidths={setKept}
                      />
                    );
                  })}
              </section>
            );
          })}

          {deletedShown.map((file) => (
            <DeletedFile key={file.path} file={file} sides={sides} busy={busy} />
          ))}

          {review.files.length > 0 && !files.length && !deletedShown.length && (
            <div className="empty-filter">
              <p className="muted">No conflict is left to show here.</p>
              <button className="button" onClick={() => choose('all')}>
                Show every conflict
              </button>
            </div>
          )}

          {review.others.length > 0 && !current && filter === 'all' && (
            <section className="conflict-file others" aria-label="Other files in conflict">
              <h2>
                <Icon name="file-binary" />
                <span className="grow">Also in conflict, with no markers to look at here</span>
              </h2>
              <p className="muted">Binary files, very large files, and files both sides added are resolved in an editor, or by taking one side.</p>
              <ul>
                {review.others.map((file) => (
                  <li key={file}>
                    <FileIcon path={file} />
                    <button className="link" onClick={() => post({ type: 'openConflict', path: file, line: 1, editor: 'mergeEditor' })}>
                      {file}
                    </button>
                  </li>
                ))}
              </ul>
            </section>
          )}

          {!review.files.length && !review.deleted.length && !review.others.length && !review.report && (
            <div className="empty-review">
              <Icon name="pass" />
              <p>No file is in conflict{review.operation ? `: the ${review.operation} can continue` : ''}.</p>
              {review.operation && (
                <button className="button primary" onClick={() => post({ type: 'action', action: { kind: 'continue' } })}>
                  <Icon name="debug-continue" />
                  Continue the {review.operation}
                </button>
              )}
            </div>
          )}
          <p className="muted footnote">
            {review.rules ? 'The rules resolve a conflict only where no choice is needed, and every line they write is a line of one of the two sides. ' : ''}
            {review.ai ? 'The AI proposes the rest from the lines around each conflict: look at what it writes before you accept it.' : ''}
          </p>
        </div>
      </div>
    </main>
  );
}
