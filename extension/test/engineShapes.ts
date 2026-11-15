/**
 * The extension's types of the state of a repository and of the details of a commit, checked against the JSON the
 * engine writes (stage 6 of `docs/transition/06-roadmap.md`). The JSON is the engine's golden files of
 * `rust/crates/protocol/tests/shapes.rs`, a state and the details of a commit with every optional part filled and every
 * list holding one entry, so that each field has a value whose kind TypeScript can read. `npm run typecheck` checks
 * this file; nothing runs it.
 *
 * Each side must be assignable to the other: a field added, removed or renamed on either side, or a value of another
 * kind (a string for a number, an object for a list), is a type error here. What it cannot see: which strings an
 * enumeration has (`cherryPick`, `typeChanged`), since TypeScript reads every string of a JSON file as `string`, and
 * which fields may be null, since none is null in these files. The tests of `engine.test.ts` and `engineReads.test.ts`
 * compare those values with the engine's.
 */

import type engineDetail from '../../rust/crates/protocol/tests/golden/commit-detail-filled.json';
import type engineState from '../../rust/crates/protocol/tests/golden/state-filled.json';
import type { CommitDetail, RepositoryState } from '../src/shared/state';

/** A type as JSON has it, with every field filled: a string of an enumeration is a string, and null is left out. */
type Filled<T> = T extends string
  ? string
  : T extends number
    ? number
    : T extends boolean
      ? boolean
      : T extends null
        ? never
        : T extends readonly (infer Element)[]
          ? Filled<Element>[]
          : T extends object
            ? { -readonly [Key in keyof T]-?: Filled<T[Key]> }
            : T;

type EngineState = typeof engineState;
type EngineDetail = typeof engineDetail;

/** The extension's state is what the engine writes, and the engine's what the extension reads. */
export const stateFromEngine = (state: EngineState): Filled<RepositoryState> => state;
export const stateToEngine = (state: Filled<RepositoryState>): EngineState => state;

/** The same for the details of a commit. */
export const detailFromEngine = (detail: EngineDetail): Filled<CommitDetail> => detail;
export const detailToEngine = (detail: Filled<CommitDetail>): EngineDetail => detail;
