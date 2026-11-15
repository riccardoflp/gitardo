import { createHash } from 'node:crypto';
import type { ConflictAnswer } from '../shared/conflictPrompt';
import { choiceText, proposalLines, repeatedKeys } from '../shared/proposalLines';
import {
  CHOSEN,
  RESOLUTIONS,
  newLineCount,
  type Asking,
  type Choice,
  type ConflictChanges,
  type Proposal,
  type ProposalWarning,
  type ReviewConflict,
  type ReviewFile,
} from '../shared/resolution';
import { applyReplacements, type Conflict } from './conflicts';
import type { ConflictPlan } from './repo';

/** How many lines of the file the tab shows before a conflict and after it. */
const AROUND = 3;

/** What a language model answered for a conflict, wherever the conflict is in its file. */
export type Answer = ConflictAnswer;

/**
 * What is known of the questions to the language model, and of what the user chose in the review, by file and by the
 * fingerprint of the conflict.
 */
export interface Asked {
  answer(path: string, fingerprint: string): Answer | undefined;
  asking(path: string, fingerprint: string): Asking | undefined;
  choice?(path: string, fingerprint: string): Choice | undefined;
}

const linesOf = (text: string) => text.split(/(?<=\n)/);

/**
 * What a conflict is as git marked it: its markers and what is between them. Two conflicts with the same fingerprint
 * are resolved the same way, wherever they are in the file, whatever was edited around them, and however git named
 * the sides on the markers, which changes when Undo puts the conflicts back.
 */
export function fingerprint(text: string, conflict: Pick<Conflict, 'from' | 'to'>): string {
  const marked = linesOf(text)
    .slice(conflict.from, conflict.to)
    .map((line) => line.replace(/\r?\n$|\r$/, '').replace(/^(<{7}|\|{7}|>{7}) .*$/, '$1'));
  return createHash('sha1').update(marked.join('\n')).digest('hex').slice(0, 16);
}

/**
 * Lines that take the place of a conflict in a file, as they are written there: with the line endings of the file, and
 * without a line break after them where the last marker had none, at the end of the file.
 */
function inPlaceOf(text: string, conflict: Conflict, written: string): string {
  const marked = linesOf(text).slice(conflict.from, conflict.to);
  const ending = /\r\n$/.test(marked[0] ?? '') ? '\r\n' : '\n';
  const last = marked[marked.length - 1] ?? '';
  const lines = written ? written.replace(/\r?\n$/, '').split(/\r?\n/) : [];
  const ends = /\n$/.test(last) || conflict.to < linesOf(text).length;
  return lines.length ? lines.join(ending) + (ends ? ending : '') : '';
}

/**
 * An answer of the language model as the lines it replaces in a file: the markers of the conflict and what is between
 * them, with the line endings of the file.
 */
export function answerProposal(text: string, conflict: Conflict, answer: Answer): Proposal {
  const resolved = inPlaceOf(text, conflict, answer.text);
  return {
    source: 'ai',
    from: conflict.from,
    to: conflict.to,
    text: resolved,
    reason: answer.reason,
    confidence: answer.confidence,
    newLines: newLineCount(resolved, conflict.ours, conflict.theirs, conflict.stretch?.base ?? null),
    ...(answer.model ? { model: answer.model } : {}),
  };
}

/** What the user chose in the review, as the lines it replaces in the file: the markers and what is between them. */
export function choiceProposal(text: string, conflict: Conflict, choice: Choice): Proposal {
  const written = choice.kind === 'edited' ? choice.text : choiceText(choice.kind, conflict.ours, conflict.theirs);
  const resolved = inPlaceOf(text, conflict, written);
  return {
    source: 'choice',
    from: conflict.from,
    to: conflict.to,
    text: resolved,
    reason: CHOSEN[choice.kind],
    confidence: null,
    newLines: choice.kind === 'edited' ? newLineCount(resolved, conflict.ours, conflict.theirs, conflict.stretch?.base ?? null) : 0,
  };
}

/** How many lines around a conflict the language model is shown. */
const ASKED_AROUND = 40;

/** What the language model is shown of a conflict: what is in it and the lines around it. */
export function questionParts(text: string, conflict: Conflict): { before: string; after: string; ours: string; theirs: string; base: string | null } {
  const lines = linesOf(text);
  return {
    before: lines.slice(Math.max(0, conflict.from - ASKED_AROUND), conflict.from).join(''),
    after: lines.slice(conflict.to, conflict.to + ASKED_AROUND).join(''),
    ours: conflict.ours,
    theirs: conflict.theirs,
    base: conflict.stretch?.base ?? null,
  };
}

/**
 * What each side of a conflict changed of the base, with the lines of the file around it. The stretch can take some
 * lines of the file before the markers and after them, which both sides have: they are found again in the file, for
 * the lines around to start where the stretch ends.
 */
export function conflictChanges(lines: string[], conflict: Conflict): ConflictChanges | null {
  const stretch = conflict.stretch;
  if (!stretch) return null;
  const [side, own] = [linesOf(stretch.ours), linesOf(conflict.ours)].map((list) => list.filter(Boolean));
  const same = (one: string[], other: string[]) => one.length === other.length && one.every((line, index) => line === other[index]);
  let [above, below] = [0, 0];
  for (let taken = 0; taken <= side.length - own.length; taken++) {
    const after = side.length - own.length - taken;
    const fits =
      same(side.slice(0, taken), lines.slice(conflict.from - taken, conflict.from)) &&
      same(side.slice(taken, taken + own.length), own) &&
      same(side.slice(taken + own.length), lines.slice(conflict.to, conflict.to + after));
    if (fits) {
      [above, below] = [taken, after];
      break;
    }
  }
  const [start, end] = [conflict.from - above, conflict.to + below];
  const first = Math.max(0, start - AROUND);
  return {
    base: stretch.base,
    ours: stretch.ours,
    theirs: stretch.theirs,
    before: lines.slice(first, start).join(''),
    beforeLine: first + 1,
    after: lines.slice(end, end + AROUND).join(''),
  };
}

/** What the rules put in place of a conflict, when they resolve it. */
export function rulesProposal(conflict: Conflict): Proposal | null {
  if (!conflict.patch || !conflict.resolution) return null;
  const reason = RESOLUTIONS[conflict.resolution];
  return { source: 'rules', ...conflict.patch, reason: `${reason[0].toUpperCase()}${reason.slice(1)}`, confidence: null, newLines: 0 };
}

/**
 * The files in conflict as the tab of the review shows them: each conflict with what is around it, and what is
 * proposed for it. The rules are taken first, where `rules`; the language model answers the others.
 */
export function reviewFiles(plans: ConflictPlan[], rules: boolean, asked: Asked): ReviewFile[] {
  return plans.map((plan) => {
    const lines = linesOf(plan.text);
    const parses = jsonCheck(plan);
    const conflicts = plan.conflicts.map((conflict, index): ReviewConflict => {
      const print = fingerprint(plan.text, conflict);
      const answer = asked.answer(plan.path, print);
      const choice = asked.choice?.(plan.path, print);
      // What the user chose comes before what was proposed to them.
      const proposal =
        (choice ? choiceProposal(plan.text, conflict, choice) : null) ??
        (rules ? rulesProposal(conflict) : null) ??
        (answer ? answerProposal(plan.text, conflict, answer) : null);
      // The rules can take some lines around the markers too: what is shown around is what the proposal leaves.
      const [from, to] = proposal ? [proposal.from, proposal.to] : [conflict.from, conflict.to];
      const region = {
        ours: lines.slice(from, conflict.from).join('') + conflict.ours + lines.slice(conflict.to, to).join(''),
        theirs: lines.slice(from, conflict.from).join('') + conflict.theirs + lines.slice(conflict.to, to).join(''),
      };
      const warnings: ProposalWarning[] = [];
      if (proposal) {
        warnings.push(...repeatedKeys(proposal.text, proposalLines(region.ours, region.theirs, proposal.text, conflict.stretch?.base ?? null).origins));
        const error = parses?.(proposal);
        if (error) warnings.push({ kind: 'invalid', format: 'JSON', error });
      }
      return {
        key: `${index}:${print}`,
        line: conflict.line,
        ours: conflict.ours,
        theirs: conflict.theirs,
        base: conflict.stretch?.base ?? null,
        before: lines.slice(Math.max(0, from - AROUND), from).join(''),
        after: lines.slice(to, to + AROUND).join(''),
        region,
        changes: conflictChanges(lines, conflict),
        proposal,
        warnings,
        asking: proposal ? null : (asked.asking(plan.path, print) ?? null),
      };
    });
    return { path: plan.path, conflicts };
  });
}

/** Why a text is not JSON, or null when it is. */
function jsonError(text: string): string | null {
  try {
    JSON.parse(text);
    return null;
  } catch (error) {
    return error instanceof Error ? error.message : String(error);
  }
}

/**
 * For a JSON file, what tells whether a proposal leaves it JSON: the file with that proposal in place of its conflict,
 * and the other conflicts resolved for the side that makes the file JSON. Null for any other file, and for a file
 * that neither side makes JSON, such as one with comments, where nothing can be told.
 */
function jsonCheck(plan: ConflictPlan): ((proposal: Proposal) => string | null) | null {
  if (!/\.json$/i.test(plan.path)) return null;
  const taking = (side: 'ours' | 'theirs', except?: Proposal) =>
    plan.conflicts
      .filter((conflict) => !except || conflict.to <= except.from || conflict.from >= except.to)
      .map((conflict) => ({ from: conflict.from, to: conflict.to, text: conflict[side] }));
  const side = (['ours', 'theirs'] as const).find((each) => jsonError(applyReplacements(plan.text, taking(each))) === null);
  if (!side) return null;
  return (proposal) => {
    try {
      return jsonError(applyReplacements(plan.text, [...taking(side, proposal), proposal]));
    } catch {
      // A proposal whose lines cannot be put in the file with the others is one this cannot tell anything about.
      return null;
    }
  };
}
