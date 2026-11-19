import type { MouseEvent, ReactNode } from 'react';
import { CHANGE_LETTERS, type FileChange, type WorkingTree } from '../shared/state';
import { FileIcon } from './FileIcon';
import { Icon } from './Icon';

export interface FileAction {
  icon: string;
  title: string;
  run(): void;
}

/**
 * What the letter of a file says instead of its kind of change, for a file of the working tree: git does not track it
 * (`?`), it is in conflict (`U`), or git sees no change in it and it is listed for another reason (no letter).
 */
export type FileMark = 'untracked' | 'conflict' | 'unchanged';

/** The marks of the files of the working tree that have one, by path. */
export function workingMarks(working: WorkingTree): Map<string, FileMark> {
  const marks = new Map<string, FileMark>(working.untracked.map((path) => [path, 'untracked']));
  for (const file of working.conflicted) marks.set(file.path, 'conflict');
  return marks;
}

interface Props {
  file: FileChange;
  mark?: FileMark;
  onOpen(): void;
  actions?: FileAction[];
  /** Said after the name, before the buttons: such as how many lines changed. */
  extra?: ReactNode;
  /** How far in the row is, in a tree. There its directory is left out: the rows above say it. */
  depth?: number;
  /** Set when files can be picked to act on several at once: a box to tick before the name. */
  onPick?(picked: boolean): void;
  picked?: boolean;
  /** The box is half ticked: ticking it picks all of the file. */
  mixed?: boolean;
  /** What ticking the box does to the file, said before its name to a screen reader: `Select` when not given. */
  pickVerb?: string;
  /** What the box does, under the pointer. */
  pickTitle?: string;
  /** The box is there, but cannot be ticked. */
  pickDisabled?: boolean;
  /** An editor has edits of the file that are not saved: a dot after its name says so, as on the tab of the editor. */
  unsaved?: boolean;
  /** Opens the context menu of the file. */
  onMenu?(event: MouseEvent): void;
  /** What the row says under the pointer, in place of its path. */
  title?: string;
}

const STATUS_TITLES: Record<string, string> = {
  A: 'Added',
  M: 'Modified',
  D: 'Deleted',
  R: 'Renamed',
  C: 'Copied',
  T: 'Type changed',
  U: 'Conflict',
  '?': 'Untracked',
};

/** The room a level of a tree takes, in pixels: what puts a file under the name of its directory, past the arrow. */
export const INDENT = 18;

export function ActionButtons({ actions }: { actions?: FileAction[] }) {
  return actions?.map((action) => (
    <button key={action.title} className="button icon" title={action.title} aria-label={action.title} onClick={action.run}>
      <Icon name={action.icon} />
    </button>
  ));
}

/** The letter git has for what happened to a file, in the color VS Code gives it. */
export function Status({ file, mark }: { file: FileChange; mark?: FileMark }) {
  // A file git sees no change in, listed for another reason: the room of the letter stays, for the names to line up.
  if (mark === 'unchanged') return <span className="status" />;
  const letter = mark === 'untracked' ? '?' : mark === 'conflict' ? 'U' : CHANGE_LETTERS[file.kind];
  return (
    <span className={`status status-${letter === '?' ? 'untracked' : letter}`} title={STATUS_TITLES[letter]}>
      {letter}
    </span>
  );
}

export const fileTitle = (file: FileChange) => (file.originalPath ? `${file.originalPath} → ${file.path}` : file.path);

interface CheckProps {
  checked: boolean;
  /** Half ticked: neither all nor nothing. A click ticks it. */
  mixed?: boolean;
  label: string;
  title?: string;
  disabled?: boolean;
  onChange(checked: boolean): void;
}

/** A box to tick, which can also be half ticked. */
export function Check({ checked, mixed = false, label, title, disabled, onChange }: CheckProps) {
  return (
    <input
      // Half ticked is not an attribute: it is set on the element, at every render so that it follows.
      ref={(box) => {
        if (box) box.indeterminate = mixed;
      }}
      type="checkbox"
      className="pick"
      checked={checked}
      aria-label={label}
      title={title}
      disabled={disabled}
      onChange={(event) => {
        onChange(event.target.checked);
        // React puts back `checked` until the new state comes, but not the half tick that the click took away.
        event.target.indeterminate = mixed;
      }}
    />
  );
}

export function FileRow({
  file,
  mark,
  onOpen,
  actions,
  extra,
  depth,
  onPick,
  picked,
  mixed,
  pickVerb = 'Select',
  pickTitle,
  pickDisabled,
  unsaved,
  onMenu,
  title,
}: Props) {
  const slash = file.path.lastIndexOf('/');
  return (
    <div className="file" style={depth === undefined ? undefined : { paddingLeft: 12 + depth * INDENT }} onContextMenu={onMenu}>
      {onPick && (
        <Check checked={!!picked} mixed={mixed} label={`${pickVerb} ${file.path}`} title={pickTitle} disabled={pickDisabled} onChange={onPick} />
      )}
      <button className="file-open" title={title ?? fileTitle(file)} onClick={onOpen}>
        <Status file={file} mark={mark} />
        <FileIcon path={file.path} />
        <span className="name">{file.path.slice(slash + 1)}</span>
        {unsaved && (
          <span className="unsaved" title="Not saved" aria-label="Not saved">
            <Icon name="circle-filled" />
          </span>
        )}
        {depth === undefined && slash > 0 && (
          <span className="dir">
            {/* Read from the left as it is written, whichever end the line is cut at. */}
            <bdi>{file.path.slice(0, slash)}</bdi>
          </span>
        )}
      </button>
      {extra}
      <ActionButtons actions={actions} />
    </div>
  );
}
