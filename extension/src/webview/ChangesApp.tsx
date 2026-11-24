import { useEffect, useLayoutEffect, useMemo, useRef, useState, type MouseEvent, type ReactNode } from 'react';
import { workingChanges, type WorkingChange } from '../shared/changes';
import { type CommitOptions, optionsArguments, optionsSaid, startingOptions } from '../shared/commitOptions';
import type { FileChange, FilesLayout, RepoState } from '../shared/protocol';
import { checkedOutBranch } from '../shared/state';
import { fileTree } from '../shared/tree';
import { BisectBanner } from './BisectBanner';
import { FILES_LIMIT, useCapped } from './Capped';
import { ActionButtons, Check, FileRow, type FileAction } from './FileRow';
import { FileTree, toggled } from './FileTree';
import { commitOptionsMenu } from './commitOptions';
import { useMenu } from './ContextMenu';
import { Icon } from './Icon';
import { OperationBanner, resolvableConflicts } from './OperationBanner';
import { RepoSelect } from './RepoSelect';
import { WorktreeBanner } from './WorktreeBanner';
import { act, changedFileMenu, hiddenFileMenu, hiddenGroupMenu, pathsOf, resolvedAsBeforeMenu } from './menus';
import { RESOLVED_AS_BEFORE, resolvedAsBefore } from '../shared/rerere';
import { type Loaded, Placeholder, loadedOf } from './Placeholder';
import { receive } from './received';
import { loadUiState, onMessage, post, saveUiState } from './vscode';

/** A diff as VS Code's list of changes opens it: what is not staged yet, or what is, when all of the file is. */
const SOURCE = { kind: 'change' } as const;

/**
 * A file of the list: a change git sees, or one only an editor has, not saved yet (`staged` null), which git sees
 * nothing of until it is saved.
 */
type Listed = WorkingChange | { file: FileChange; staged: null; untracked: false };

/** Where a file of the list is, for the tree. Stable, so that the tree is not made again at every render. */
const changePath = (change: Listed) => change.file.path;

/** The buttons of a file in conflict, by their icons, in the order its menu lists them. */
const MENU_ORDER = ['sparkle', 'wand', 'add', 'arrow-right', 'arrow-left'];
/** The buttons of a file in conflict that open the review of its conflicts, and change nothing until it is applied. */
const REVIEWS = ['sparkle', 'wand'];

const PICK_TITLES: Record<WorkingChange['staged'], string> = {
  all: 'Staged: click to unstage',
  some: 'Partly staged: click to stage all of it',
  none: 'Click to stage',
};

interface GroupProps<T> {
  title: string;
  items: T[];
  /** Before the title, such as the box that ticks every file. */
  lead?: ReactNode;
  headerActions?: FileAction[];
  /** `depth` is how far in the row is, when the files are under their directories. */
  row(item: T, depth?: number): ReactNode;
  /** Set to show the files under their directories, which open and close. */
  tree?: { path(item: T): string; folded: ReadonlySet<string>; onFold(path: string): void };
}

function Group<T>({ title, items, lead, headerActions, row, tree }: GroupProps<T>): ReactNode {
  const [listed, more] = useCapped(items, FILES_LIMIT, 'files');
  const nodes = useMemo(() => (tree ? fileTree(listed, tree.path) : []), [listed, tree?.path]);
  if (!items.length) return null;
  return (
    <section className="group" aria-label={title}>
      <header>
        {lead}
        <span className="grow">{title}</span>
        <ActionButtons actions={headerActions} />
        <span className="count">{items.length}</span>
      </header>
      {tree ? <FileTree nodes={nodes} folded={tree.folded} onFold={tree.onFold} row={row} /> : listed.map((item) => row(item))}
      {more}
    </section>
  );
}

/** What the row of a file hidden from git says under the pointer (`changes.skipped-list`). */
const HIDDEN_TITLE = 'Git does not look at the changes of this file: they stay out of every commit.';

/**
 * The files git is told not to look at, by hand, at the end of the view while there are any (`changes.skipped-list`):
 * **Hidden from git** and their count, which a click folds, with **…** for **Show all to git again**. Whether it is
 * folded is kept while the view is open, and not remembered (Q1178).
 */
function HiddenGroup({ files, busy }: { files: NonNullable<RepoState['skipped']>; busy: boolean }) {
  const [folded, setFolded] = useState(false);
  const openMenu = useMenu();
  if (!files.length) return null;
  return (
    <section className="group hidden-from-git" aria-label="Hidden from git">
      <header>
        <button className="grow fold-group" aria-expanded={!folded} onClick={() => setFolded(!folded)}>
          <Icon name={folded ? 'chevron-right' : 'chevron-down'} />
          Hidden from git
        </button>
        <button
          className="button icon"
          title="More for the files hidden from git"
          aria-label="More for the files hidden from git"
          onClick={(event) => openMenu(event, hiddenGroupMenu(busy))}
        >
          <Icon name="ellipsis" />
        </button>
        <span className="count">{files.length}</span>
      </header>
      {!folded &&
        files.map(({ path, skipWorktree }) => {
          const file: FileChange = { path, originalPath: null, kind: 'modified' };
          return (
            <FileRow
              key={path}
              file={file}
              mark="unchanged"
              title={`${path}\n${HIDDEN_TITLE}`}
              // Between the last commit and the disk, whatever git is told: the list does not know whether it differs (Q1451).
              onOpen={() => post({ type: 'openDiff', source: { kind: 'worktree' }, file })}
              extra={<span className="hint muted">{skipWorktree ? 'skip-worktree' : 'assumed unchanged'}</span>}
              actions={[{ icon: 'eye', title: 'Show its changes to git again', run: () => act({ kind: 'showToGit', path }) }]}
              onMenu={(event) => openMenu(event, hiddenFileMenu(path, busy))}
            />
          );
        })}
    </section>
  );
}

/**
 * The message, and the button that commits. It keeps the message to itself, so that typing it does not draw the
 * lists of files again at every key: they can be long.
 */
function CommitBox({ state, busy }: { state: RepoState; busy: boolean }) {
  const [message, setMessage] = useState(() => loadUiState({ message: '' }).message);
  const [amend, setAmend] = useState(false);
  const [drafting, setDrafting] = useState(false);
  // The repository the box is of, and what was typed in it, as the view last showed them.
  const shown = useRef(state.root);
  const typed = useRef(message);
  // The options ticked for the next commit, null while they are where they start; each repository keeps its own.
  const [changed, setChanged] = useState<CommitOptions | null>(null);
  const kept = useRef(new Map<string, CommitOptions>());
  const start = useMemo(() => startingOptions(state.signing, state.signOff), [state.signing, state.signOff]);
  const options = changed ?? start;
  const openMenu = useMenu();
  // Where the options start, for the co-authors that come back from the quick pick of the extension.
  const startRef = useRef(start);

  useEffect(
    () =>
      onMessage((incoming) => {
        if (incoming.type === 'committed') {
          setMessage('');
          setAmend(false);
          // A tick held for that commit alone: the next starts again as git's configuration and the settings say.
          setChanged(null);
        } else if (incoming.type === 'coAuthors') {
          setChanged((current) => ({ ...(current ?? startRef.current), coAuthors: incoming.coAuthors }));
        } else if (incoming.type === 'drafting') {
          setDrafting(incoming.active);
          if (incoming.message) setMessage(incoming.message);
        } else if (incoming.type === 'keptMessage') {
          if (incoming.root === shown.current) setMessage(incoming.message);
        } else if (incoming.type === 'commitMessage') {
          // What the user has written already is kept.
          setMessage((written) => (written.trim() ? written : incoming.message));
        }
      }),
    [],
  );

  useEffect(() => {
    saveUiState({ message });
  }, [message]);

  // Pointed at another repository, the box gives the extension the message it had in the one left, which keeps it,
  // and gets back the one it had in the repository shown, when it kept one (`repo.keep-open`).
  useLayoutEffect(() => {
    if (shown.current !== state.root) {
      if (state.keepOpen) post({ type: 'keepMessage', root: shown.current, message: typed.current, shown: state.root });
      if (changed) kept.current.set(shown.current, changed);
      else kept.current.delete(shown.current);
      setChanged(kept.current.get(state.root) ?? null);
      shown.current = state.root;
    }
    typed.current = message;
    startRef.current = start;
  });

  const { identity } = state;
  const { staged, unstaged, untracked } = state.workingTree;
  const changes = staged.length + unstaged.length + untracked.length;
  const canCommit = !busy && (amend || (message.trim() !== '' && changes > 0));
  const commit = () => canCommit && act({ kind: 'commit', message, amend, options, start });
  const said = optionsSaid(options, start);
  const showOptions = (event: MouseEvent) =>
    openMenu(
      event,
      commitOptionsMenu({
        options,
        signing: state.signing,
        identity,
        change: setChanged,
        pickCoAuthors: () => post({ type: 'pickCoAuthors', chosen: options.coAuthors }),
        switchIdentity: () => post({ type: 'command', command: 'switchIdentity' }),
      }),
    );

  return (
    <div className="commit-box">
      <textarea
        value={message}
        rows={3}
        placeholder={`Message (Ctrl+Enter to commit on ${checkedOutBranch(state) ?? 'detached HEAD'})`}
        aria-label="Commit message"
        onChange={(event) => setMessage(event.target.value)}
        onKeyDown={(event) => event.key === 'Enter' && (event.ctrlKey || event.metaKey) && commit()}
      />
      <div className="commit-actions">
        <button className="button primary" disabled={!canCommit} onClick={commit}>
          <Icon name="check" />
          {amend ? 'Amend' : 'Commit'}
        </button>
        <button className="button icon" title="Sign, sign off, add co-authors or skip the hooks, for the next commit" aria-label="Commit options" onClick={showOptions}>
          <Icon name="settings" />
        </button>
        <button
          className="button icon"
          disabled={drafting || changes === 0}
          title="Draft a message from the changes with the language model set up in VS Code"
          aria-label="Draft commit message"
          onClick={() => post({ type: 'command', command: 'draftMessage' })}
        >
          <Icon name={drafting ? 'loading' : 'sparkle'} spin={drafting} />
        </button>
        <label title="Replace the last commit instead of adding a new one. Leave the message empty to keep it.">
          <input type="checkbox" checked={amend} disabled={!state.head.commit} onChange={(event) => setAmend(event.target.checked)} />
          Amend last commit
        </label>
      </div>
      {said && (
        <button className="link commit-options-said" title={optionsArguments(options, state.signing, message)} onClick={showOptions}>
          <span className="grow">{said}</span>
        </button>
      )}
      <button
        className="link identity"
        title="Who commits in this repository are made as. Click to switch identity."
        onClick={() => post({ type: 'command', command: 'switchIdentity' })}
      >
        <Icon name="account" />
        <span className="grow">{identity.name || identity.email ? `${identity.name ?? ''} <${identity.email ?? ''}>` : 'No identity configured'}</span>
      </button>
    </div>
  );
}

export function ChangesApp() {
  const [state, setState] = useState<RepoState | null>(null);
  const [loaded, setLoaded] = useState<Loaded>(false);
  const [busy, setBusy] = useState(false);
  const [layout, setLayout] = useState(() => loadUiState({ changesFiles: 'list' as FilesLayout }).changesFiles);
  const [folded, setFolded] = useState<ReadonlySet<string>>(new Set());
  /** The files with edits not saved in an editor, by their paths. */
  const [unsaved, setUnsaved] = useState<string[]>([]);
  const openMenu = useMenu();

  useEffect(() => {
    const stop = onMessage((incoming) => {
      if (incoming.type === 'state') {
        setState(receive(incoming));
        setLoaded(loadedOf(incoming));
      } else if (incoming.type === 'busy') {
        setBusy(incoming.title !== null);
      } else if (incoming.type === 'unsaved') {
        setUnsaved(incoming.paths);
      }
    });
    post({ type: 'ready' });
    return stop;
  }, []);

  const working = state?.workingTree;
  const changes = useMemo(() => (working ? workingChanges(working) : []), [working]);
  const isUnsaved = useMemo(() => new Set(unsaved), [unsaved]);
  // The files not saved that git sees no change in join the list, in the order of the paths.
  const listed = useMemo(() => {
    const known = new Set([...(working?.conflicted ?? []), ...changes.map((change) => change.file)].map((file) => file.path));
    const others = unsaved
      .filter((path) => !known.has(path))
      .map((path): Listed => ({ file: { path, originalPath: null, kind: 'modified' }, staged: null, untracked: false }));
    if (!others.length) return changes;
    return [...changes, ...others].sort((one, other) => (one.file.path < other.file.path ? -1 : one.file.path > other.file.path ? 1 : 0));
  }, [unsaved, changes, working]);

  if (!state) return <Placeholder loaded={loaded} />;

  const { staged } = state.workingTree;
  // A file in conflict is listed as a change to it, marked `U`.
  const conflicts = state.workingTree.conflicted.map((file): FileChange => ({ path: file.path, originalPath: null, kind: 'modified' }));
  const switchLayout = () => {
    const wanted = layout === 'tree' ? 'list' : 'tree';
    setLayout(wanted);
    saveUiState({ changesFiles: wanted });
  };
  // Files changed only in their editors have nothing to stage: with no others, the box of the list cannot be ticked.
  const allStaged = changes.length > 0 && changes.every((change) => change.staged === 'all');
  const noneStaged = changes.every((change) => change.staged === 'none');
  // With files in conflict, `git add --all` would take them as resolved and `git reset` would forget their conflicts:
  // the other files are named one by one instead.
  const stageAll = () =>
    act(
      conflicts.length
        ? { kind: 'stage', paths: changes.filter((change) => change.staged !== 'all').map((change) => change.file.path) }
        : { kind: 'stageAll' },
    );
  const unstageAll = () =>
    act(
      conflicts.length
        ? { kind: 'unstage', paths: changes.filter((change) => change.staged !== 'none').flatMap((change) => pathsOf(change.file)) }
        : { kind: 'unstageAll' },
    );

  return (
    <div className="changes">
      <RepoSelect state={state} disabled={busy} />
      <WorktreeBanner state={state} busy={busy} />
      <OperationBanner state={state} busy={busy} />
      <BisectBanner state={state} busy={busy} />
      <CommitBox state={state} busy={busy} />

      <div className="groups">
        <Group
          title="Conflicts"
          items={conflicts}
          row={(file) => {
            // Git wrote a resolution it recorded: what it wrote is looked at, accepted or forgotten (`rerere.resolved-as-before`).
            if (resolvedAsBefore(state, file.path)) {
              const menu = resolvedAsBeforeMenu(file.path, false, busy);
              return (
                <FileRow
                  key={file.path}
                  file={file}
                  mark="conflict"
                  unsaved={isUnsaved.has(file.path)}
                  extra={<span className="muted">{RESOLVED_AS_BEFORE}</span>}
                  onOpen={() => post({ type: 'openDiff', source: { kind: 'resolvedAsBefore' }, file })}
                  actions={[
                    { icon: 'eye', title: 'Show what git wrote', run: () => post({ type: 'openDiff', source: { kind: 'resolvedAsBefore' }, file }) },
                    { icon: 'discard', title: 'Forget this resolution…', run: () => act({ kind: 'forgetResolution', path: file.path }) },
                    { icon: 'add', title: 'Mark as resolved', run: () => act({ kind: 'stage', paths: [file.path] }) },
                  ]}
                  onMenu={(event) => openMenu(event, menu)}
                />
              );
            }
            const [resolvable, marked] = resolvableConflicts(state, file.path);
            const some = resolvable === marked ? (marked === 1 ? 'its conflict' : `its ${marked} conflicts`) : `${resolvable} of its ${marked} conflicts`;
            const actions: FileAction[] = [
              ...(resolvable
                ? [{ icon: 'wand', title: `Resolve ${some} automatically, after looking at what is proposed`, run: () => post({ type: 'reviewConflicts', paths: [file.path] }) }]
                : []),
              ...(state.resolveWithAI && (!marked || resolvable < marked)
                ? [{ icon: 'sparkle', title: 'Resolve with AI, after looking at what is proposed', run: () => post({ type: 'reviewConflicts', paths: [file.path], ai: true }) }]
                : []),
              { icon: 'arrow-left', title: 'Take the current side for the whole file', run: () => act({ kind: 'takeSide', path: file.path, side: 'ours' }) },
              { icon: 'arrow-right', title: 'Take the incoming side for the whole file', run: () => act({ kind: 'takeSide', path: file.path, side: 'theirs' }) },
              { icon: 'add', title: 'Mark as resolved', run: () => act({ kind: 'stage', paths: [file.path] }) },
            ];
            return (
              <FileRow
                key={file.path}
                file={file}
                mark="conflict"
                unsaved={isUnsaved.has(file.path)}
                onOpen={() => post({ type: 'openDiff', source: { kind: 'conflict' }, file })}
                actions={actions}
                // The menu has what the buttons do, in the order of the desktop's (`changes.row-menu`); a review only opens a page.
                onMenu={(event) =>
                  openMenu(
                    event,
                    [...actions]
                      .sort((one, other) => MENU_ORDER.indexOf(one.icon) - MENU_ORDER.indexOf(other.icon))
                      .map((action) => ({ label: action.title, disabled: busy && !REVIEWS.includes(action.icon), run: action.run })),
                  )
                }
              />
            );
          }}
        />
        <Group
          title="Changes"
          items={listed}
          lead={
            <Check
              checked={allStaged}
              mixed={!allStaged && !noneStaged}
              disabled={!changes.length}
              label={allStaged ? 'Unstage all changes' : 'Stage all changes'}
              title={allStaged ? 'All staged: click to unstage all changes' : 'Click to stage all changes'}
              onChange={(checked) => (checked ? stageAll() : unstageAll())}
            />
          }
          tree={layout === 'tree' ? { path: changePath, folded, onFold: (path) => setFolded((current) => toggled(current, path)) } : undefined}
          headerActions={[
            layout === 'tree'
              ? { icon: 'list-flat', title: 'View as list', run: switchLayout }
              : { icon: 'list-tree', title: 'View as tree', run: switchLayout },
            {
              icon: 'diff-multiple',
              title: 'Open all changes in one editor',
              run: () => post({ type: 'openChanges', source: SOURCE, files: changes.map((change) => change.file), title: 'Changes' }),
            },
            ...(staged.length ? [{ icon: 'git-stash', title: 'Stash only the staged changes…', run: () => act({ kind: 'stashPush', staged: true }) }] : []),
            { icon: 'git-stash', title: 'Stash all changes…', run: () => act({ kind: 'stashPush' }) },
            ...(unsaved.length ? [{ icon: 'save-all', title: 'Save all', run: () => post({ type: 'save', paths: unsaved }) }] : []),
            {
              icon: 'discard',
              title: 'Discard all changes',
              // What is staged goes back to HEAD too. With nothing staged the index is HEAD, and the files go back to it.
              run: () =>
                act(staged.length ? { kind: 'discardChanges', paths: changes.flatMap((change) => pathsOf(change.file)) } : { kind: 'discardAll' }),
            },
          ]}
          row={({ file, staged: done, untracked: notTracked }, depth) => {
            const save = { icon: 'save', title: 'Save', run: () => post({ type: 'save', paths: [file.path] }) };
            // Only its editor has changed it: there is nothing for git to stage, stash or discard until it is saved.
            if (done === null) {
              return (
                <FileRow
                  key={file.path}
                  file={file}
                  mark="unchanged"
                  depth={depth}
                  unsaved
                  onOpen={() => post({ type: 'openFile', file })}
                  onPick={() => undefined}
                  pickDisabled
                  pickVerb="Stage"
                  pickTitle="Not saved: save it to stage it"
                  onMenu={(event) => openMenu(event, changedFileMenu({ file, staged: null, untracked: false, unsaved: true }, busy))}
                  actions={[
                    { icon: 'git-compare', title: 'Compare with the saved file', run: () => post({ type: 'compareWithSaved', path: file.path }) },
                    save,
                  ]}
                />
              );
            }
            // A file git resolved as before and staged keeps its words and its menu until the operation ends.
            const resolved = resolvedAsBefore(state, file.path);
            const fileMenu = changedFileMenu({ file, staged: done, untracked: notTracked, unsaved: isUnsaved.has(file.path) }, busy);
            return (
              <FileRow
                key={file.path}
                file={file}
                mark={notTracked ? 'untracked' : undefined}
                depth={depth}
                extra={resolved && <span className="muted">{RESOLVED_AS_BEFORE}</span>}
                unsaved={isUnsaved.has(file.path)}
                // The file, or its diff: as the settings say.
                onOpen={() => post({ type: 'openChange', file })}
                // A half ticked box is ticked by the click: all of the file is staged.
                onPick={(checked) => act(checked ? { kind: 'stage', paths: [file.path] } : { kind: 'unstage', paths: pathsOf(file) })}
                picked={done === 'all'}
                mixed={done === 'some'}
                pickVerb={done === 'all' ? 'Unstage' : 'Stage'}
                pickTitle={PICK_TITLES[done]}
                onMenu={(event) => openMenu(event, resolved ? [...resolvedAsBeforeMenu(file.path, true, busy), 'separator', ...fileMenu] : fileMenu)}
                actions={[
                  ...(file.kind === 'deleted' ? [] : [{ icon: 'go-to-file', title: 'Open File', run: () => post({ type: 'openFile', file }) }]),
                  { icon: 'git-compare', title: 'Open Changes', run: () => post({ type: 'openDiff', source: SOURCE, file }) },
                  ...(isUnsaved.has(file.path) ? [save] : []),
                  { icon: 'git-stash', title: 'Stash this file…', run: () => act({ kind: 'stashPush', paths: pathsOf(file) }) },
                  {
                    icon: 'discard',
                    title: 'Discard changes',
                    // With some of it staged, all of it goes back to HEAD.
                    run: () => act(done === 'none' ? { kind: 'discard', paths: [file.path] } : { kind: 'discardChanges', paths: pathsOf(file) }),
                  },
                ]}
              />
            );
          }}
        />
        {state.changes === 0 && !unsaved.length && <p className="muted none">No changes</p>}
        <HiddenGroup files={state.skipped ?? []} busy={busy} />
      </div>
    </div>
  );
}
