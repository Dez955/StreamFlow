import { describe, it, expect } from 'vitest';
import { t, setLanguage, getLanguage, detectLanguage } from './i18n';

describe('i18n', () => {
  it('detects Ukrainian, falls back to English', () => {
    expect(detectLanguage('uk-UA')).toBe('uk');
    expect(detectLanguage('uk')).toBe('uk');
    expect(detectLanguage('en-US')).toBe('en');
    expect(detectLanguage('de-DE')).toBe('en');
    expect(detectLanguage(undefined)).toBe('en');
  });

  it('has the neutral paste hint without a site name', () => {
    setLanguage('uk');
    expect(t('pasteLink')).toBe('Вставте посилання (наприклад, https://...)');
    setLanguage('en');
    expect(t('pasteLink')).toBe('Paste a link (for example, https://...)');
    expect(t('pasteLink').toLowerCase()).not.toContain('youtube');
  });

  it('substitutes params and returns the key when missing', () => {
    setLanguage('uk');
    expect(t('stats.active', { n: 3 })).toBe('Активні завантаження: 3');
    expect(t('no.such.key')).toBe('no.such.key');
  });

  it('auto follows the browser language and unknown codes fall back to English', () => {
    setLanguage('auto');
    expect(getLanguage()).toBe(detectLanguage(navigator.language));
    setLanguage('xx' as never);
    expect(getLanguage()).toBe('en');
  });
});
