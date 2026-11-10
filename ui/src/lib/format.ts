import { getLanguage, type Lang } from './i18n';
import type { Progress } from './types';

const UNITS: Record<Lang, string[]> = {
  en: ['B', 'KB', 'MB', 'GB', 'TB'],
  uk: ['Б', 'КБ', 'МБ', 'ГБ', 'ТБ'],
};

/** Decimal units. Unknown sizes show a dash, never "0 B". */
export function formatBytes(n?: number | null, lang: Lang = getLanguage()): string {
  if (n == null || !Number.isFinite(n)) return '—';
  const units = UNITS[lang];
  let value = n;
  let i = 0;
  while (value >= 1000 && i < units.length - 1) {
    value /= 1000;
    i++;
  }
  if (i === 0) return `${Math.round(value)} ${units[0]}`;
  let text = value >= 100 ? String(Math.round(value)) : (Math.round(value * 10) / 10).toFixed(1).replace(/\.0$/, '');
  if (lang === 'uk') text = text.replace('.', ',');
  return `${text} ${units[i]}`;
}

export function formatSpeed(bps?: number | null, lang: Lang = getLanguage()): string {
  return bps == null || !Number.isFinite(bps) ? '' : `${formatBytes(bps, lang)}/s`;
}

/** m:ss, or h:mm:ss from one hour up. Also used for durations. */
export function formatEta(secs?: number | null): string {
  if (secs == null || !Number.isFinite(secs)) return '';
  const total = Math.max(0, Math.floor(secs));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const pad = (x: number) => String(x).padStart(2, '0');
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
}

/** Whole percent, or null when the total is unknown (show an indeterminate bar). */
export function formatPercent(p: Progress): number | null {
  if (p.total == null || p.total <= 0) return null;
  return Math.min(100, Math.floor((p.downloaded / p.total) * 100));
}
