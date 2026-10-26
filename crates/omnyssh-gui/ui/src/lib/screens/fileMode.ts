// Unix permission bits for the properties dialog (tech-gui.md §3.2): three r/w/x
// triples the checkboxes drive, and the setuid/setgid/sticky bits they leave alone.
// Pure so the arithmetic is unit-tested without a webview.

export type Who = 'owner' | 'group' | 'others';
export type Perm = 'read' | 'write' | 'execute';

const SHIFT: Record<Who, number> = { owner: 6, group: 3, others: 0 };
const MASK: Record<Perm, number> = { read: 4, write: 2, execute: 1 };

/** Whether `mode` grants `perm` to `who`. */
export function granted(mode: number, who: Who, perm: Perm): boolean {
  return (mode & (MASK[perm] << SHIFT[who])) !== 0;
}

/** `mode` with one r/w/x bit set or cleared; every other bit stays as it was. */
export function withGrant(mode: number, who: Who, perm: Perm, on: boolean): number {
  const bit = MASK[perm] << SHIFT[who];
  return on ? mode | bit : mode & ~bit;
}

/** The permission triples as octal, `644` or `755`. */
export function octal(mode: number): string {
  return (mode & 0o777).toString(8).padStart(3, '0');
}

/** The `ls -l` form, `rwxr-xr-x`. */
export function symbolic(mode: number): string {
  const chars = ['r', 'w', 'x'] as const;
  const triplet = (shift: number): string =>
    chars.map((ch, i) => (mode & ((4 >> i) << shift) ? ch : '-')).join('');
  return triplet(6) + triplet(3) + triplet(0);
}

/** The special bits the dialog keeps but does not show as checkboxes. */
export function specialBits(mode: number): number {
  return mode & 0o7000;
}
