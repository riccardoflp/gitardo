/** Two spellings of the same path compare equal, whatever the slashes and the case. */
export const samePath = (a: string, b: string) => a.replace(/\\/g, '/').toLowerCase() === b.replace(/\\/g, '/').toLowerCase();
