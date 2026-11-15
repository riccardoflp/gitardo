import type { Confidence } from './resolution';

// The question about a conflict is asked by Gitardo's engine, which reads the answer too; what is left here is what the
// review shows of an answer, whole or while it is being written.

/** What the language model answered, read. */
export interface ConflictAnswer {
  text: string;
  reason: string;
  confidence: Confidence;
  /** The name of the model that answered. */
  model?: string;
}

/** What the model has written of its resolution so far, for the tab to show it coming. */
export function partialResolution(answer: string): string {
  const start = answer.indexOf('<resolution>');
  if (start < 0) return '';
  const rest = answer.slice(start + '<resolution>'.length).replace(/^\r?\n/, '');
  const end = rest.indexOf('</resolution>');
  return end < 0 ? rest.replace(/<\/?r?e?s?o?l?u?t?i?o?n?$/, '') : rest.slice(0, end);
}
