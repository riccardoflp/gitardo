/**
 * The files in conflict as the review and the Flow Merge work on them: each file as it is in the working tree, with the
 * markers of git, and its sides as the engine reads them from the index (`merge-view.read-sides`). What can be resolved
 * without a choice is worked out from that text by `resolveConflicts` of `src/shared/conflicts.ts`.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { resolveConflicts, type ConflictPlan } from '../shared/conflicts';
import type { DeletedConflict, RecordedConflict } from '../shared/resolution';
import { MARKED } from '../shared/rerere';
import type { Reads, Repository } from './reads';

/** Larger files in conflict are left to the user. */
const CONFLICT_BYTES = 2 * 1024 * 1024;
/** No more files in conflict than this are read to see what can be resolved. */
const CONFLICT_FILES = 200;

/**
 * What can be resolved without a choice in each of these files in conflict. Left out are the files without the markers
 * of git: binary ones, the ones a side deleted, and the ones that are not text that can be written as read.
 */
export async function conflictPlans(reads: Reads, repo: Repository, files: string[]): Promise<ConflictPlan[]> {
  const plans: ConflictPlan[] = [];
  for (const file of files.slice(0, CONFLICT_FILES)) {
    try {
      const full = path.join(repo.root, file);
      const stat = await fs.promises.lstat(full);
      if (!stat.isFile() || stat.size > CONFLICT_BYTES) continue;
      const bytes = await fs.promises.readFile(full);
      const text = bytes.toString('utf8');
      if (!text.includes('<<<<<<<') || text.includes('\0') || !Buffer.from(text, 'utf8').equals(bytes)) continue;
      // The file before either side changed it: there is none when both added it.
      const { base, ours, theirs } = await reads.conflictVersions(repo, file);
      const { text: resolved, conflicts } = resolveConflicts(text, base, { ours, theirs });
      if (conflicts.length) plans.push({ path: file, text, resolved, conflicts });
    } catch {
      // A file that cannot be read is one to resolve by hand.
    }
  }
  return plans;
}

/**
 * Of these files in conflict, the ones that one side deleted and the other changed, with what they were and what the
 * side that kept them made of them. Left out are those that are not text, or too large to show.
 */
export async function deletedConflicts(reads: Reads, repo: Repository, files: string[]): Promise<DeletedConflict[]> {
  const found: DeletedConflict[] = [];
  for (const file of files.slice(0, CONFLICT_FILES)) {
    const stages = await reads.conflictStages(repo, file);
    if (!stages.includes(1) || stages.includes(2) === stages.includes(3)) continue;
    const deletedBy = stages.includes(2) ? 'theirs' : 'ours';
    const versions = await reads.conflictVersions(repo, file);
    const changed = deletedBy === 'theirs' ? versions.ours : versions.theirs;
    if ([versions.base, changed].some((text) => text.includes('\0') || text.length > CONFLICT_BYTES)) continue;
    found.push({ path: file, deletedBy, base: versions.base, changed });
  }
  return found;
}

/**
 * Of these files git resolved as before, what git marked when it stopped and what it wrote from the resolution it
 * recorded (`review.recorded`). Left out are those that are not text, or too large to show.
 */
export async function recordedConflicts(reads: Reads, repo: Repository, files: string[]): Promise<RecordedConflict[]> {
  const found: RecordedConflict[] = [];
  for (const file of files.slice(0, CONFLICT_FILES)) {
    try {
      const marked = Buffer.from((await reads.fileBytesAt(repo.root, MARKED, file)) ?? new Uint8Array()).toString('utf8');
      const written = await fs.promises.readFile(path.join(repo.root, file), 'utf8');
      if ([marked, written].some((text) => text.includes('\0') || text.length > CONFLICT_BYTES)) continue;
      found.push({ path: file, marked, written });
    } catch {
      // A file that cannot be read is one to look at in an editor.
    }
  }
  return found;
}
