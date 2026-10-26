import { describe, expect, it } from 'vitest';
import { granted, octal, specialBits, symbolic, withGrant, type Perm, type Who } from './fileMode';

describe('fileMode', () => {
  it('reads each r/w/x bit for each class', () => {
    const mode = 0o754;
    const has = (who: Who, perm: Perm): boolean => granted(mode, who, perm);
    expect([has('owner', 'read'), has('owner', 'write'), has('owner', 'execute')]).toEqual([
      true,
      true,
      true
    ]);
    expect([has('group', 'read'), has('group', 'write'), has('group', 'execute')]).toEqual([
      true,
      false,
      true
    ]);
    expect([has('others', 'read'), has('others', 'write'), has('others', 'execute')]).toEqual([
      true,
      false,
      false
    ]);
  });

  it('sets and clears one bit without touching the rest', () => {
    expect(withGrant(0o644, 'others', 'write', true)).toBe(0o646);
    expect(withGrant(0o646, 'others', 'write', false)).toBe(0o644);
    // The special bits survive a checkbox toggle.
    expect(withGrant(0o4755, 'others', 'write', true)).toBe(0o4757);
    expect(withGrant(0o4757, 'others', 'write', false)).toBe(0o4755);
    // A cleared bit cannot take a neighbour with it.
    expect(withGrant(0o777, 'group', 'execute', false)).toBe(0o767);
  });

  it('formats octal and symbolic forms', () => {
    expect(octal(0o644)).toBe('644');
    expect(octal(0o755)).toBe('755');
    expect(octal(0o600)).toBe('600');
    expect(symbolic(0o755)).toBe('rwxr-xr-x');
    expect(symbolic(0o644)).toBe('rw-r--r--');
    expect(symbolic(0o000)).toBe('---------');
    expect(symbolic(0o777)).toBe('rwxrwxrwx');
  });

  it('keeps the setuid/setgid/sticky bits apart', () => {
    expect(specialBits(0o4755)).toBe(0o4000);
    expect(specialBits(0o2755)).toBe(0o2000);
    expect(specialBits(0o1755)).toBe(0o1000);
    expect(specialBits(0o755)).toBe(0);
  });
});
