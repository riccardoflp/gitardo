/**
 * The labels of a commit on its row, and the list of all of them that opens under the pointer when the row does not
 * show them all (`graph.label-overflow`), with what a row of the list can do.
 */
import { useLayoutEffect, useRef, useState, type CSSProperties, type DragEvent, type MouseEvent } from 'react';
import type { PullRequest, Row } from '../shared/protocol';
import { serviceName } from '../shared/hosting';
import { pullBadgeText, pullTitle } from '../shared/refs';
import { Icon } from './Icon';
import { endDrag, startDrag, type DragSource } from './drag';
import { laneColor } from './util';
import { post } from './vscode';

export interface Label {
  /** `good`, `bad` and `skipped` are the marks of a bisect in progress. */
  kind: 'head' | 'branch' | 'remote' | 'tag' | 'stash' | 'good' | 'bad' | 'skipped';
  name: string;
  current?: boolean;
  /** For a branch: the folder of the other worktree that has it checked out. */
  worktree?: string;
  /** For a branch: its upstream, when it has the same name there and is at the same commit, so the one label stands for both. */
  remote?: string;
  /** For a branch: its open pull request, and the name of the branch on its hosting service. */
  pull?: { request: PullRequest; branch: string };
  /** For a tag: the mark of the remote checked, which lacks it or has another tag of its name (`tag.remote-marks`). */
  mark?: { icon: string; title: string };
}

/** A plain click selects one commit; with Ctrl or Cmd it adds or removes one; with Shift it selects up to the clicked one. */
export type SelectMode = 'single' | 'toggle' | 'range';

/** How long the pointer rests on a label of a row that does not show all of them before their list opens (Q642). */
const LIST_AFTER = 300;
/** How long the list stays once the pointer left it and the labels, for the pointer to go from one to the other. */
const LIST_CLOSE_AFTER = 120;

export const LABEL_ICONS = {
  head: 'target',
  branch: 'git-branch',
  remote: 'cloud',
  tag: 'tag',
  stash: 'git-stash',
  good: 'check',
  bad: 'close',
  skipped: 'debug-step-over',
} as const;

/** The icons of a label: of a branch, also the worktree that has it checked out and the upstream it is the same as. */
export function LabelIcons({ label }: { label: Label }) {
  if (!label.worktree && !label.remote && !label.mark) return <Icon name={LABEL_ICONS[label.kind]} />;
  return (
    <span className="icons">
      {label.worktree && <Icon name="worktree" />}
      <Icon name={LABEL_ICONS[label.kind]} />
      {label.remote && <Icon name={LABEL_ICONS.remote} />}
      {label.mark && <Icon name={label.mark.icon} />}
    </span>
  );
}

/** The name of a label, with the worktree and the upstream its icons stand for. */
export function labelTitle(label: Label): string {
  const lines = [label.name];
  if (label.worktree) lines.push(`In the worktree ${label.worktree}`);
  if (label.remote) lines.push(`Up to date with ${label.remote}`);
  if (label.mark) lines.push(label.mark.title);
  return lines.join('\n');
}

/** The open pull request of a branch, on its label: its number, in the color of its checks. A click opens it. */
function PullBadge({ request, branch }: { request: PullRequest; branch: string }) {
  return (
    <span
      className={`pull ${request.checks ?? ''}`}
      role="link"
      title={`${pullTitle(request)}\nClick to open it on ${serviceName(request.service)}`}
      onClick={(event) => {
        // Not the selection of the commit as well.
        event.stopPropagation();
        post({ type: 'pullRequest', branch });
      }}
      onDoubleClick={(event) => event.stopPropagation()}
    >
      <Icon name={request.draft ? 'git-pull-request-draft' : 'git-pull-request'} />
      {pullBadgeText(request)}
    </span>
  );
}

/**
 * What a row of the list can do, the same object for the whole life of the list, so that a row draws again only when
 * what it shows changed. Each calls what the list was last given.
 */
export interface RowActions {
  select(hash: string, mode: SelectMode): void;
  open(hash: string): void;
  rowMenu(event: MouseEvent, row: Row): void;
  labelMenu(event: MouseEvent, label: Label, row: Row): void;
  labelOpen(label: Label): (() => void) | undefined;
  /** Answers from the label and the row alone, since a row asks it only when it draws again. */
  dragSource(label: Label | null, row: Row): DragSource | null;
  canDrop(row: Row): boolean;
  drop(event: MouseEvent, row: Row): void;
  trace(hash: string | null): void;
  fold(merge: string): void;
  unfold(merge: string): void;
  /** The row a drag is over, which shows where it would be dropped. */
  setDropTarget(update: (current: string | null) => string | null): void;
  /** The pointer came to a label of a row that does not show all of them, or to their list: it opens after a while. */
  restOnLabels(hash: string, label: HTMLElement | null): void;
  /** The pointer left a label of such a row, or their list: it closes unless the pointer comes back. */
  leaveLabels(label: HTMLElement | null): void;
  /** Closes the list of the labels of a row, after a label of it was opened or its menu. */
  closeLabels(): void;
}

/** What a click with the keys held does to the selection. */
const selectMode = (event: MouseEvent): SelectMode => (event.shiftKey ? 'range' : event.ctrlKey || event.metaKey ? 'toggle' : 'single');

interface LabelChipProps {
  label: Label;
  row: Row;
  actions: RowActions;
  /** The width it is given when it is shortened beside `+N`. */
  width?: number;
  /** Whether the row does not show all its labels, and the pointer resting on this one opens their list. */
  opensList: boolean;
  /** In the list of all the labels of the commit, where a click selects the commit and a double click or a menu closes the list. */
  inList?: boolean;
}

/** A label of a commit, on its row or in the list of all of them, with what the pointer does on it. */
export function LabelChip({ label, row, actions, width, opensList, inList }: LabelChipProps) {
  return (
    <span
      role="listitem"
      className={`label ${label.kind}${label.current ? ' current' : ''}`}
      title={labelTitle(label)}
      style={width === undefined ? undefined : { maxWidth: width }}
      onContextMenu={(event) => {
        actions.labelMenu(event, label, row);
        if (inList) actions.closeLabels();
      }}
      onMouseEnter={(event) => {
        actions.trace(row.hash);
        if (opensList) actions.restOnLabels(row.hash, event.currentTarget);
      }}
      onMouseLeave={(event) => {
        actions.trace(null);
        if (opensList) actions.leaveLabels(event.currentTarget);
      }}
      onClick={inList ? (event) => actions.select(row.hash, selectMode(event)) : undefined}
      onDoubleClick={(event) => {
        const open = actions.labelOpen(label);
        if (!open && !inList) return;
        // Not the commit's own double click as well.
        event.stopPropagation();
        if (open) open();
        else actions.open(row.hash);
        if (inList) actions.closeLabels();
      }}
      {...dragProps(actions, label, row)}
    >
      <LabelIcons label={label} />
      <span className="text">{label.name}</span>
      {label.pull && <PullBadge {...label.pull} />}
    </span>
  );
}

interface LabelListProps {
  row: Row;
  labels: Label[];
  /** The label the pointer rested on, which the list never covers. */
  anchor: DOMRect;
  actions: RowActions;
}

/**
 * The list of all the labels of a commit whose row does not show them all, under the label the pointer rested on, or
 * above it when there is no room under it and more above; taller than the room, it scrolls (`graph.label-overflow`).
 */
export function LabelList({ row, labels, anchor, actions }: LabelListProps) {
  const element = useRef<HTMLDivElement>(null);
  const [place, setPlace] = useState<CSSProperties>({ left: anchor.left, top: anchor.bottom + 2, visibility: 'hidden' });
  useLayoutEffect(() => {
    const list = element.current!;
    const below = window.innerHeight - anchor.bottom - 6;
    const above = anchor.top - 6;
    const height = list.scrollHeight;
    const under = height <= below || below >= above;
    const room = under ? below : above;
    setPlace({
      left: Math.max(0, Math.min(anchor.left, window.innerWidth - list.offsetWidth - 4)),
      top: under ? anchor.bottom + 2 : anchor.top - 2 - Math.min(height, room),
      maxHeight: room,
    });
  }, [anchor]);
  return (
    <div
      ref={element}
      className="label-list"
      role="list"
      aria-label="Labels"
      style={{ ...place, '--lane': laneColor(row.graph.color) } as CSSProperties}
      onMouseEnter={() => actions.restOnLabels(row.hash, null)}
      onMouseLeave={() => actions.leaveLabels(null)}
    >
      {labels.map((label) => (
        <div key={`${label.kind}:${label.name}`} className="message">
          <LabelChip label={label} row={row} actions={actions} opensList={false} inList />
        </div>
      ))}
    </div>
  );
}

/** What makes a row or one of its labels draggable, and what it carries. */
export function dragProps(actions: RowActions, label: Label | null, row: Row) {
  const source = actions.dragSource(label, row);
  return {
    draggable: source !== null,
    onDragStart: (event: DragEvent) => source && startDrag(event, source),
    onDragEnd: () => {
      endDrag();
      actions.setDropTarget(() => null);
    },
  };
}

/** What the list of the labels of a row is: where it opens, and when it closes, from where the pointer rests (`graph.label-overflow`). */
export function labelListHover(show: (list: { hash: string; anchor: DOMRect } | null) => void) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let open: string | null = null;
  // The label the pointer is on, and the one the list was closed on with `Escape` or a press: that one opens nothing until the pointer leaves it.
  let over: HTMLElement | null = null;
  let blocked: HTMLElement | null = null;
  const set = (list: { hash: string; anchor: DOMRect } | null) => {
    open = list?.hash ?? null;
    show(list);
  };
  return {
    rest(hash: string, label: HTMLElement | null) {
      clearTimeout(timer);
      // On the list itself, or on another label of the row it is of, it stays.
      if (!label || open === hash) return;
      over = label;
      if (open) set(null);
      if (label !== blocked) timer = setTimeout(() => set({ hash, anchor: label.getBoundingClientRect() }), LIST_AFTER);
    },
    leave(label: HTMLElement | null) {
      clearTimeout(timer);
      if (label) {
        over = null;
        if (label === blocked) blocked = null;
      }
      if (open) timer = setTimeout(() => set(null), LIST_CLOSE_AFTER);
    },
    close(blocking: boolean) {
      clearTimeout(timer);
      if (blocking) blocked = over;
      if (open) set(null);
    },
    dispose() {
      clearTimeout(timer);
    },
  };
}
