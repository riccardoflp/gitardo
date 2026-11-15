/**
 * The resolutions accepted on the review of the conflicts as `review.write` of the engine takes them: the target
 * `resolutions` of the protocol (`docs/transition/02-protocol.md`, version 1.3).
 */
import type { Resolution } from '../shared/protocol';
import { CHOSEN, type Proposal, RESOLUTIONS } from '../shared/resolution';
import type { AcceptedProposal, FileResolutions } from './messages';

/** A file of the review with the proposals accepted in it, and what it had when they were made. */
export interface AcceptedInFile {
  path: string;
  text: string;
  conflicts: number;
  proposals: Proposal[];
}

/** The case of the rules, by the engine's name. */
const RULE_CASES: Record<Resolution, 'nextToEachOther' | 'sameChange' | 'spacesOrLineEndings' | 'oneSide'> = {
  separate: 'nextToEachOther',
  same: 'sameChange',
  whitespace: 'spacesOrLineEndings',
  oneSide: 'oneSide',
};

/** What the user chose, by the engine's name. */
const CHOICES = {
  current: 'currentOnly',
  incoming: 'incomingOnly',
  'current-first': 'bothCurrentFirst',
  'incoming-first': 'bothIncomingFirst',
  edited: 'written',
} as const;

/** The key of the words that say a reason, whatever their case. */
function keyOf<Key extends string>(words: Record<Key, string>, reason: string): Key | undefined {
  return (Object.keys(words) as Key[]).find((key) => words[key].toLowerCase() === reason.toLowerCase());
}

/** Who proposed a resolution, as the engine names it: the case of the rules and the choice are told by their reason. */
function sourceOf(proposal: Proposal): AcceptedProposal['source'] {
  if (proposal.source === 'ai') return { kind: 'ai', model: proposal.model ?? '', reason: proposal.reason, confidence: proposal.confidence ?? 'medium' };
  if (proposal.source === 'choice') return { kind: 'choice', choice: CHOICES[keyOf(CHOSEN, proposal.reason) ?? 'edited'] };
  return { kind: 'rules', case: RULE_CASES[keyOf(RESOLUTIONS, proposal.reason) ?? 'separate'] };
}

/** The files of the review with the proposals accepted, every file of it, as `review.write` takes them. */
export function engineResolutions(files: AcceptedInFile[]): FileResolutions[] {
  return files.map((file) => ({
    path: file.path,
    text: file.text,
    conflicts: file.conflicts,
    accepted: file.proposals.map((proposal) => ({
      source: sourceOf(proposal),
      from: proposal.from,
      to: proposal.to,
      text: proposal.text,
      newLines: proposal.newLines,
    })),
  }));
}
