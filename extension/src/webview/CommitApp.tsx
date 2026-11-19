import { useEffect, useRef, useState } from 'react';
import { said, subjectAndBody } from '../shared/message';
import type { CommitDetail, DiffSource, FilePatch, FilesLayout, RepoState } from '../shared/protocol';
import { buildLabels } from './labels';
import { BackToTop } from './BackToTop';
import { LabelIcons, labelTitle } from './CommitList';
import { FileChanges } from './FileChanges';
import { FileRow } from './FileRow';
import { Icon } from './Icon';
import { NoteBlock } from './NoteBlock';
import { fileMenu } from './menus';
import { fullDate, shortHash } from './util';
import { SignatureLine, useSignature } from './Signature';
import { onMessage, post } from './vscode';

interface Props {
  /** The full hash of the commit this tab is of. */
  hash: string;
  /** The repository it is in, which may not be the one the other views show. */
  root: string;
  layout: FilesLayout;
}

/** One commit in an editor tab: its whole message, the files it changed and what it changed in each. */
export function CommitApp({ hash, root, layout: remembered }: Props) {
  // Undefined while they are asked for; null when git has none to give.
  const [detail, setDetail] = useState<CommitDetail | null>();
  const [patch, setPatch] = useState<FilePatch[] | null>();
  const [state, setState] = useState<RepoState | null>(null);
  const [layout, setLayout] = useState(remembered);
  // The changes of the files are in the page only once asked for.
  const [diffs, setDiffs] = useState(false);
  const page = useRef<HTMLElement>(null);
  // Where the reference of notes was at the last state, which tells when to read the note again.
  const notesTip = useRef<string | null | undefined>(undefined);

  useEffect(() => {
    const stop = onMessage((message) => {
      if (message.type === 'state') {
        setState(message.state);
        // The note of the commit is read again when the reference of notes moved, as after Edit note….
        const tip = message.state?.root === root ? (message.state.notes?.tip ?? null) : undefined;
        if (tip !== undefined && notesTip.current !== undefined && tip !== notesTip.current) post({ type: 'getDetail', hash });
        if (tip !== undefined) notesTip.current = tip;
      } else if (message.type === 'detail' && message.hash === hash) setDetail(message.detail);
      else if (message.type === 'patch' && message.hash === hash) setPatch(message.patch);
    });
    post({ type: 'ready' });
    post({ type: 'getDetail', hash });
    post({ type: 'getPatch', hash });
    return stop;
  }, [hash, root]);
  // Asked once the details are drawn, so that they never wait for it (`details.signature`).
  const signature = useSignature(hash, Boolean(detail));

  if (detail === undefined) return <div className="empty">Loading the commit…</div>;
  if (!detail) return <div className="empty">This commit is not in the repository any more.</div>;

  const id = detail.id ?? hash;
  const { author, committer } = detail;
  const source: DiffSource = { kind: 'commit', hash: id, parent: detail.parents[0] ?? null };
  const { subject, body } = subjectAndBody(detail.message);
  const title = `${shortHash(id)} ${subject}`;
  const message = said(subject, body);
  // A stash keeps the untracked files it took in its third parent.
  const untracked = detail.parents[2] && detail.untrackedFiles.length ? detail.parents[2] : null;
  // The labels are those of the repository the other views show, which may be another by now.
  const labels = state?.root === root ? (buildLabels(state).get(id) ?? []) : [];
  const choose = (wanted: FilesLayout) => {
    setLayout(wanted);
    post({ type: 'setFilesLayout', layout: wanted });
  };

  return (
    <>
      <main ref={page} className="commit-page">
        <header className="commit-head">
          <h1>{message.title}</h1>
          {labels.length > 0 && (
            <div className="message commit-labels">
              {labels.map((label) => (
                <span key={`${label.kind}:${label.name}`} className={`label ${label.kind}${label.current ? ' current' : ''}`} title={labelTitle(label)}>
                  <LabelIcons label={label} />
                  {label.name}
                </span>
              ))}
            </div>
          )}
          {message.text && <pre className="body">{message.text}</pre>}
          {detail.note && (
            <NoteBlock hash={id} editable={state?.root === root}>
              <pre className="body">{detail.note}</pre>
            </NoteBlock>
          )}
          <dl>
            {author && (
              <>
                <dt>Author</dt>
                <dd>
                  {author.name} <span className="muted">&lt;{author.email}&gt;</span>
                </dd>
                <dt>Date</dt>
                <dd>{fullDate(author.date)}</dd>
              </>
            )}
            {committer && (committer.name !== author?.name || committer.date !== author?.date) && (
              <>
                <dt>Committer</dt>
                <dd>
                  {committer.name} <span className="muted">&lt;{committer.email}&gt;</span>, {fullDate(committer.date)}
                </dd>
              </>
            )}
            <dt>Commit</dt>
            <dd>
              <button className="link mono" title="Copy the hash" onClick={() => post({ type: 'copy', text: id })}>
                {id} <Icon name="copy" />
              </button>
            </dd>
            <SignatureLine signature={signature} />
            {detail.parents.length > 0 && (
              <>
                <dt>{detail.parents.length > 1 ? 'Parents' : 'Parent'}</dt>
                <dd>
                  {detail.parents.map((parent) => (
                    <button key={parent} className="link mono" title="Open this commit in a tab of its own" onClick={() => post({ type: 'openCommit', hash: parent })}>
                      {shortHash(parent)}
                    </button>
                  ))}
                </dd>
              </>
            )}
          </dl>
          <div className="commit-actions">
            <button className="button" onClick={() => post({ type: 'reveal', hash: id })}>
              <Icon name="git-commit" />
              Show in the graph
            </button>
            {detail.files.length > 0 && (
              <button className="button" onClick={() => post({ type: 'openChanges', source, files: detail.files, title })}>
                <Icon name="diff-multiple" />
                Open in the diff editor
              </button>
            )}
          </div>
        </header>

        <FileChanges
          files={detail.files}
          patch={patch}
          source={source}
          title={title}
          layout={layout}
          onLayout={choose}
          note={detail.parents.length > 1 && <span className="muted"> against the first parent</span>}
          fileMenu={(file) => fileMenu(file, source)}
          shown={diffs}
          onShown={setDiffs}
        >
          {untracked && (
            <>
              <h2>
                {detail.untrackedFiles.length} untracked {detail.untrackedFiles.length === 1 ? 'file' : 'files'}
              </h2>
              <div className="files">
                {detail.untrackedFiles.map((file) => (
                  <FileRow
                    key={file.path}
                    file={file}
                    onOpen={() => post({ type: 'openDiff', source: { kind: 'commit', hash: untracked, parent: null }, file })}
                  />
                ))}
              </div>
            </>
          )}
        </FileChanges>
      </main>
      <BackToTop page={page} />
    </>
  );
}
