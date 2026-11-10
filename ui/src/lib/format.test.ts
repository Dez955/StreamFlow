import { describe, it, expect } from 'vitest';
import { formatBytes, formatSpeed, formatEta, formatPercent } from './format';

describe('formatBytes', () => {
  it('uses decimal units and one decimal below 100', () => {
    expect(formatBytes(1_600_000_000, 'en')).toBe('1.6 GB');
    expect(formatBytes(1500, 'en')).toBe('1.5 KB');
  });
  it('localises unit and decimal separator', () => {
    expect(formatBytes(1_600_000_000, 'uk')).toBe('1,6 ГБ');
  });
  it('drops decimals from 100 up and keeps small values in bytes', () => {
    expect(formatBytes(150_000_000, 'en')).toBe('150 MB');
    expect(formatBytes(999, 'en')).toBe('999 B');
  });
  it('shows a dash for unknown sizes, never 0 B', () => {
    expect(formatBytes(null, 'en')).toBe('—');
    expect(formatBytes(undefined, 'en')).toBe('—');
    expect(formatBytes(0, 'en')).toBe('0 B');
  });
});

describe('formatSpeed / formatEta', () => {
  it('formats speed and hides unknown values', () => {
    expect(formatSpeed(524288, 'en')).toBe('524 KB/s');
    expect(formatSpeed(null, 'en')).toBe('');
  });
  it('formats durations as m:ss or h:mm:ss', () => {
    expect(formatEta(6)).toBe('0:06');
    expect(formatEta(125)).toBe('2:05');
    expect(formatEta(3725)).toBe('1:02:05');
    expect(formatEta(null)).toBe('');
  });
});

describe('formatPercent', () => {
  const p = (downloaded: number, total: number | null) => ({ downloaded, total, speedBps: null, etaSecs: null });
  it('returns whole percent', () => {
    expect(formatPercent(p(50, 200))).toBe(25);
    expect(formatPercent(p(500, 200))).toBe(100);
  });
  it('is null when the total is unknown or zero', () => {
    expect(formatPercent(p(50, null))).toBeNull();
    expect(formatPercent(p(50, 0))).toBeNull();
  });
});
