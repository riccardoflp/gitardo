import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from 'react';
import { createPortal } from 'react-dom';
import type { RefKind, RepoState, Scope } from '../shared/protocol';
import { byMatch } from '../shared/refs';
import { scopeChoices, scopeKinds, scopeLabel, scopeRefName, scopeRefs, toggleListedKind, toggleScopeRef, type ScopeChoice } from '../shared/scope';
import { Icon } from './Icon';
import { post } from './vscode';

const same = (one: Scope | null, other: Scope | null) => JSON.stringify(one) === JSON.stringify(other);

const GROUPS: { kind: ScopeChoice['kind']; title: string }[] = [
  { kind: 'branch', title: 'Local' },
  { kind: 'remote', title: 'Remotes' },
  { kind: 'tag', title: 'Tags' },
];

const QUICK: { scope: Scope | null; label: string; title: string }[] = [
  { scope: null, label: 'All', title: 'Every branch, remote branch and tag' },
  { scope: { kind: 'current' }, label: 'Current', title: 'Only what is checked out, whichever branch that is as you switch' },
  {
    scope: { kind: 'current', upstream: true },
    label: 'Upstream',
    title: 'What is checked out and its upstream, whichever branch that is as you switch: what is ahead of it and what is behind',
  },
  { scope: { kind: 'kinds', kinds: ['branch'] }, label: 'Local', title: 'Only the local branches, and what is checked out' },
  { scope: { kind: 'kinds', kinds: ['remote'] }, label: 'Remotes', title: 'Only the remote branches, and what is checked out' },
];

/** The kinds of references the filter of the references lists, as check boxes at the bottom of its list. */
const KINDS: { kind: RefKind; label: string }[] = [
  { kind: 'branch', label: 'Show local branches' },
  { kind: 'remote', label: 'Show remote branches' },
  { kind: 'tag', label: 'Show tags' },
];

interface PickerProps {
  state: RepoState;
  anchor: HTMLElement;
  /** What is chosen now: the branches shown in the graph, or what the references list. */
  choice: Scope | null;
  onChoose(next: Scope | null): void;
  /** What the list is for, said to a screen reader, and above the list when `heading` is set. */
  title: string;
  heading?: boolean;
  /** With the kinds of references listed as check boxes at the bottom, as the filter of the references has them. */
  kinds?: boolean;
  onClose(): void;
}

/**
 * The list of the branches, remote branches and tags to choose from, with the presets above them: the same for the
 * branches shown in the graph and for what the references list (`refs.list-filter`), each with its own choice.
 */
function Picker({ state, anchor, choice, onChoose, title, heading, kinds, onClose }: PickerProps) {
  const element = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const [text, setText] = useState('');
  const [place, setPlace] = useState<CSSProperties>({ left: 0, top: 0 });
  // What was last chosen here, until the graph shows it: a second tick does not wait for the first to come back.
  const [wanted, setWanted] = useState<{ scope: Scope | null } | null>(null);
  const scope = wanted && !same(wanted.scope, choice) ? wanted.scope : choice;
  const latest = useRef(scope);
  latest.current = scope;
  const setScope = (next: Scope | null) => {
    latest.current = next;
    setWanted({ scope: next });
    onChoose(next);
  };
  // The kinds the choice lists: none while it is of references ticked one by one or what is checked out.
  const kindsListed = scopeKinds(scope);

  // Under the button and inside the view: as tall as the room there is, which in the panel is little.
  // Placed before it is painted, and never hidden, or the box could not take the focus.
  useLayoutEffect(() => {
    const button = anchor.getBoundingClientRect();
    const box = element.current!.getBoundingClientRect();
    const top = button.bottom + 2;
    setPlace({
      left: Math.max(4, Math.min(button.left, window.innerWidth - box.width - 4)),
      top,
      maxHeight: Math.max(120, window.innerHeight - top - 4),
    });
    input.current!.focus();
  }, [anchor]);

  useEffect(() => {
    const outside = (event: Event) => {
      const target = event.target as Node;
      // A click on the button is the button's to answer: it closes what it opened.
      if (!element.current?.contains(target) && !anchor.contains(target)) onClose();
    };
    // The first Escape empties the box, wherever in the list the focus is; the second closes the list.
    const escape = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      if (text) setText('');
      else onClose();
    };
    document.addEventListener('mousedown', outside);
    document.addEventListener('keydown', escape);
    window.addEventListener('blur', onClose);
    window.addEventListener('resize', onClose);
    return () => {
      document.removeEventListener('mousedown', outside);
      document.removeEventListener('keydown', escape);
      window.removeEventListener('blur', onClose);
      window.removeEventListener('resize', onClose);
    };
  }, [anchor, onClose, text]);

  const chosen = new Set(scope ? scopeRefs(scope, state) : []);
  const needle = text.trim().toLowerCase();
  // The names that match best come first in each group.
  const choices = byMatch(scopeChoices(state), (choice) => choice.name, needle);

  return createPortal(
    <div className={`popover scope-picker${kinds ? ' list-filter-picker' : ''}`} role="dialog" aria-label={title} ref={element} style={place}>
      {heading && <div className="title">{title}</div>}
      <div className="quick" role="radiogroup" aria-label="Which branches">
        {QUICK.map((quick) => (
          <button
            key={quick.label}
            role="radio"
            aria-checked={same(scope, quick.scope)}
            title={quick.title}
            onClick={() => setScope(quick.scope)}
          >
            {quick.label}
          </button>
        ))}
      </div>
      <div className="refs-filter">
        <Icon name="filter" />
        <input
          ref={input}
          type="text"
          value={text}
          aria-label="Filter branches and tags"
          placeholder="Filter branches and tags"
          onChange={(event) => setText(event.target.value)}
        />
      </div>
      <div className="choices">
        {GROUPS.map(({ kind, title }) => {
          const group = choices.filter((choice) => choice.kind === kind);
          if (!group.length) return null;
          return (
            <div key={kind} role="group" aria-label={title}>
              <div className="heading">{title}</div>
              {group.map((choice) => (
                <label key={choice.ref} className={choice.current ? 'current' : undefined} title={choice.name}>
                  <input type="checkbox" checked={chosen.has(choice.ref)} onChange={() => setScope(toggleScopeRef(latest.current, choice.ref, state))} />
                  <span>{choice.name}</span>
                </label>
              ))}
            </div>
          );
        })}
        {!choices.length && <div className="none">{needle ? 'No branch or tag has that in its name' : 'No branches or tags yet'}</div>}
      </div>
      {kinds && (
        <div className="kinds" role="group" aria-label="Kinds of references listed">
          {KINDS.map(({ kind, label }) => (
            <label key={kind} title={kindsListed ? undefined : 'Chosen in the filter'}>
              <input
                type="checkbox"
                checked={kindsListed?.includes(kind) ?? false}
                disabled={!kindsListed}
                onChange={() => setScope(toggleListedKind(latest.current, kind))}
              />
              <span>{label}</span>
            </label>
          ))}
        </div>
      )}
    </div>,
    document.body,
  );
}

/** Says which branches the graph shows, and opens the list to choose them from. */
export function ScopeButton({ state, compact }: { state: RepoState; compact?: boolean }) {
  const [open, setOpen] = useState(false);
  const button = useRef<HTMLButtonElement>(null);
  const label = scopeLabel(state.scope);
  const refs = state.scope?.kind === 'refs' ? state.scope.refs : [];
  const classes = ['button', 'scope'];
  if (compact) classes.push('icon');
  if (state.scope) classes.push('on');
  return (
    <>
      <button
        ref={button}
        className={classes.join(' ')}
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-label={`Branches shown: ${label}`}
        title={`Branches shown in the graph: ${label}`}
        onClick={() => setOpen(!open)}
      >
        <Icon name="list-filter" />
        {/* The count apart from the name, which is cut when it is long. */}
        {!compact && <span className="label">{refs.length ? scopeRefName(refs[0]) : label}</span>}
        {!compact && refs.length > 1 && <span className="label more">+{refs.length - 1}</span>}
        {!compact && <Icon name="chevron-down" />}
      </button>
      {open && button.current && (
        <Picker
          state={state}
          anchor={button.current}
          choice={state.scope}
          onChoose={(scope) => post({ type: 'setScope', scope })}
          title="Branches shown in the graph"
          onClose={() => setOpen(false)}
        />
      )}
    </>
  );
}

/**
 * The funnel at the top of the references: says what they list, and opens the list to choose it from, apart from the
 * branches shown in the graph (`refs.list-filter`). It is pressed in while they list less than everything.
 */
export function ListFilterButton({ state }: { state: RepoState }) {
  const [open, setOpen] = useState(false);
  const button = useRef<HTMLButtonElement>(null);
  const label = scopeLabel(state.listed);
  return (
    <>
      <button
        ref={button}
        className={`list-filter${state.listed ? ' on' : ''}`}
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-pressed={state.listed !== null}
        aria-label={`Branches listed: ${label}`}
        title={`Branches listed here: ${label}`}
        onClick={() => setOpen(!open)}
      >
        <Icon name={state.listed ? 'filter-filled' : 'filter'} />
      </button>
      {open && button.current && (
        <Picker
          state={state}
          anchor={button.current}
          choice={state.listed}
          onChoose={(listed) => post({ type: 'setListed', listed })}
          title="Branches listed"
          heading
          kinds
          onClose={() => setOpen(false)}
        />
      )}
    </>
  );
}
