import { describe, it, expect } from 'vitest';
import { normalizeUrl } from './url';

describe('normalizeUrl', () => {
  it('trims whitespace and newlines', () => {
    expect(normalizeUrl('  https://a.b/c\n')).toBe('https://a.b/c');
  });
  it('rejects text, empty input and internal spaces', () => {
    expect(normalizeUrl('привіт')).toBeNull();
    expect(normalizeUrl('   ')).toBeNull();
    expect(normalizeUrl('https://a.b/c d')).toBeNull();
    expect(normalizeUrl('https://')).toBeNull();
  });
});
