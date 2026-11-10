import { derived, get, writable } from 'svelte/store';

export type Lang = 'uk' | 'en';
export type LangSetting = 'auto' | Lang;

const dict: Record<Lang, Record<string, string>> = {
  uk: {
    pasteLink: 'Вставте посилання (наприклад, https://...)',
    'stats.active': 'Активні завантаження: {n}',
    'stats.done': 'Завершено: {n}',
    'stats.total': 'Всього: {n}',
    analyze: 'Аналізувати',
    analyzing: 'Аналізую посилання…',
    video: 'Відео',
    audio: 'Аудіо',
    audioFormat: 'Формат аудіо',
    best: 'Найкраща',
    noVideo: 'Без відео (лише аудіо)',
    download: 'Завантажити',
    playlistNotice: 'Це плейлист: буде завантажено лише перше відео.',
    'error.invalidUrl': 'Це не схоже на посилання. Вставте адресу, що починається з http:// або https://',
    'error.network': 'Немає з’єднання з мережею. Перевірте інтернет і спробуйте ще раз.',
    'error.unsupported': 'Цей сайт або посилання не підтримуються.',
    'error.loginRequired': 'Для цього відео потрібен вхід в обліковий запис. У Налаштуваннях можна взяти cookies із браузера.',
    'error.drm': 'Цей вміст захищено DRM, завантажити його неможливо.',
    'error.diskFull': 'Недостатньо місця на диску.',
    'error.needsUpdate': 'Сайт, схоже, змінився. Оновіть yt-dlp у Налаштуваннях і спробуйте ще раз.',
    'error.toolMissing': 'Потрібні інструменти (yt-dlp, ffmpeg) не встановлені.',
    'error.other': 'Не вдалося: {detail}',
  },
  en: {
    pasteLink: 'Paste a link (for example, https://...)',
    'stats.active': 'Active downloads: {n}',
    'stats.done': 'Completed: {n}',
    'stats.total': 'Total: {n}',
    analyze: 'Analyze',
    analyzing: 'Analyzing link…',
    video: 'Video',
    audio: 'Audio',
    audioFormat: 'Audio format',
    best: 'Best',
    noVideo: 'No video (audio only)',
    download: 'Download',
    playlistNotice: 'This is a playlist: only the first video will be downloaded.',
    'error.invalidUrl': "That doesn't look like a link. Paste an address that starts with http:// or https://",
    'error.network': 'No network connection. Check your internet and try again.',
    'error.unsupported': 'This site or link is not supported.',
    'error.loginRequired': 'This video requires you to be signed in. In Settings you can use cookies from your browser.',
    'error.drm': 'This content is DRM-protected and cannot be downloaded.',
    'error.diskFull': 'Not enough disk space.',
    'error.needsUpdate': 'The site seems to have changed. Update yt-dlp in Settings and try again.',
    'error.toolMissing': 'Required tools (yt-dlp, ffmpeg) are not installed.',
    'error.other': 'Something went wrong: {detail}',
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
