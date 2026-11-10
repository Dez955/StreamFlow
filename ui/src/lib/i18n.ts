import { derived, get, writable } from 'svelte/store';

export type Lang = 'uk' | 'en';
export type LangSetting = 'auto' | Lang;

const dict: Record<Lang, Record<string, string>> = {
  uk: {
    pasteLink: 'Вставте посилання (наприклад, https://...)',
    'stats.active': 'Активні завантаження: {n}',
    'stats.done': 'Завершено: {n}',
    'stats.total': 'Всього: {n}',
  },
  en: {
    pasteLink: 'Paste a link (for example, https://...)',
    'stats.active': 'Active downloads: {n}',
    'stats.done': 'Completed: {n}',
    'stats.total': 'Total: {n}',
  },
};

export function detectLanguage(code?: string): Lang {
  return code?.toLowerCase().startsWith('uk') ? 'uk' : 'en';
}

const browserLanguage = () => (typeof navigator === 'undefined' ? undefined : navigator.language);

export const language = writable<Lang>(detectLanguage(browserLanguage()));

export function setLanguage(setting: LangSetting): void {
  if (setting === 'auto') language.set(detectLanguage(browserLanguage()));
  else language.set(setting === 'uk' ? 'uk' : 'en');
}

export function getLanguage(): Lang {
  return get(language);
}

function translate(lang: Lang, key: string, params?: Record<string, string | number>): string {
  const text = dict[lang][key] ?? dict.en[key] ?? key;
  return text.replace(/\{(\w+)\}/g, (_, name: string) => String(params?.[name] ?? `{${name}}`));
}

/** Plain translate with the current language (not reactive). */
export function t(key: string, params?: Record<string, string | number>): string {
  return translate(get(language), key, params);
}

/** Reactive translate for components: `{$tr('key')}`. */
export const tr = derived(language, (lang) => (key: string, params?: Record<string, string | number>) =>
  translate(lang, key, params),
);
