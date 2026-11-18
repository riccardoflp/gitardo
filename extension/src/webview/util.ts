const LANE_COLORS = ['#4c9df3', '#b57edc', '#3fb68b', '#e5a13a', '#e06c75', '#56b6c2', '#c678dd', '#98c379'];

export const laneColor = (index: number) => LANE_COLORS[index % LANE_COLORS.length];

export const shortHash = (hash: string) => hash.slice(0, 7);

export function relativeTime(seconds: number, now = Date.now()): string {
  const elapsed = Math.max(0, now / 1000 - seconds);
  if (elapsed < 60) return 'now';
  if (elapsed < 3600) return `${Math.floor(elapsed / 60)}m ago`;
  if (elapsed < 86400) return `${Math.floor(elapsed / 3600)}h ago`;
  if (elapsed < 86400 * 7) return `${Math.floor(elapsed / 86400)}d ago`;
  return new Date(seconds * 1000).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
}

export const fullDate = (seconds: number) => new Date(seconds * 1000).toLocaleString();

/**
 * How far back a time is, as the separators of the graph name it: 0 is today, and each further back is more. Its name
 * is asked apart, of the few rows that start a time: writing the name of a month for each commit of a long history
 * took seconds.
 */
function period(seconds: number, now: Date): number {
  const day = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const at = new Date(seconds * 1000);
  const days = Math.round((day.getTime() - new Date(at.getFullYear(), at.getMonth(), at.getDate()).getTime()) / 86400000);
  // The weeks start on Monday.
  const week = (now.getDay() + 6) % 7;
  if (days <= 0) return 0;
  if (days === 1) return 1;
  if (days <= week) return 2;
  if (days <= week + 7) return 3;
  return 4 + (now.getFullYear() - at.getFullYear()) * 12 + now.getMonth() - at.getMonth();
}

/** What a time that far back is called: the days and the weeks by what they are from today, then each month by its name. */
function periodName(rank: number, seconds: number): string {
  const names = ['Today', 'Yesterday', 'This week', 'Last week'];
  return names[rank] ?? new Date(seconds * 1000).toLocaleDateString(undefined, { month: 'long', year: 'numeric' });
}

/**
 * The rows that start a time further back than all those above them, by their index, with its name: Yesterday, This
 * week, Last week, then each month. The rows of the first time there is are not marked, being at the top, nor are the
 * uncommitted changes, which have no time. A commit older than the ones under it, as a rebase leaves them, starts
 * nothing: a time is never named twice.
 */
export function datePeriods(rows: { date: number }[], now = Date.now()): Map<number, string> {
  const today = new Date(now);
  const starts = new Map<number, string>();
  let reached = -1;
  rows.forEach((row, index) => {
    if (!row.date) return;
    const rank = period(row.date, today);
    if (rank <= reached) return;
    if (reached >= 0) starts.set(index, periodName(rank, row.date));
    reached = rank;
  });
  return starts;
}
