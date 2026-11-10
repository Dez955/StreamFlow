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
    downloadStatus: 'Статус завантажень',
    noDownloads: 'Поки що немає завантажень',
    'state.queued': 'Очікує…',
    'state.downloading': 'Завантажується…',
    'state.processing': 'Обробка…',
    'state.paused': 'Пауза',
    'state.stopped': 'Зупинено',
    'state.done': 'Готово!',
    'state.failed': 'Помилка',
    'btn.pause': 'Пауза',
    'btn.resume': 'Продовжити',
    'btn.stop': 'Стоп',
    'btn.open': 'Відкрити',
    'btn.remove': 'Прибрати',
    'btn.retry': 'Повторити',
    settings: 'Налаштування',
    'settings.folder': 'Папка збереження',
    'settings.defaultFolder': 'Downloads (за замовчуванням)',
    'settings.chooseFolder': 'Обрати папку',
    'settings.resetFolder': 'Скинути',
    'settings.container': 'Контейнер відео',
    'settings.parallel': 'Одночасних завантажень',
    'settings.cookies': 'Cookies із браузера',
    'settings.cookiesNone': 'Не використовувати',
    'settings.language': 'Мова',
    'settings.theme': 'Тема',
    'settings.auto': 'Авто',
    'settings.light': 'Світла',
    'settings.dark': 'Темна',
    'settings.save': 'Зберегти',
    'settings.close': 'Закрити',
    'settings.tools': 'Інструменти',
    'settings.installed': 'встановлено',
    'settings.notInstalled': 'не встановлено',
    'settings.updateYtdlp': 'Оновити yt-dlp',
    'settings.updating': 'Оновлення…',
    'settings.fairUse': 'Завантажуйте лише те, що маєте право зберігати, і дотримуйтеся умов сервісів.',
    'tools.banner': 'Потрібно встановити: {tools}',
    'tools.install': 'Встановити автоматично',
    'tools.installing': 'Встановлення {tool}… {percent}',
    'tools.manual': 'Встановіть вручну: {hint}',
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
    downloadStatus: 'Download Status',
    noDownloads: 'No downloads yet',
    'state.queued': 'Waiting…',
    'state.downloading': 'Downloading…',
    'state.processing': 'Processing…',
    'state.paused': 'Paused',
    'state.stopped': 'Stopped',
    'state.done': 'Done!',
    'state.failed': 'Failed',
    'btn.pause': 'Pause',
    'btn.resume': 'Resume',
    'btn.stop': 'Stop',
    'btn.open': 'Open',
    'btn.remove': 'Remove',
    'btn.retry': 'Retry',
    settings: 'Settings',
    'settings.folder': 'Save folder',
    'settings.defaultFolder': 'Downloads (default)',
    'settings.chooseFolder': 'Choose folder',
    'settings.resetFolder': 'Reset',
    'settings.container': 'Video container',
    'settings.parallel': 'Parallel downloads',
    'settings.cookies': 'Cookies from browser',
    'settings.cookiesNone': 'Do not use',
    'settings.language': 'Language',
    'settings.theme': 'Theme',
    'settings.auto': 'Auto',
    'settings.light': 'Light',
    'settings.dark': 'Dark',
    'settings.save': 'Save',
    'settings.close': 'Close',
    'settings.tools': 'Tools',
    'settings.installed': 'installed',
    'settings.notInstalled': 'not installed',
    'settings.updateYtdlp': 'Update yt-dlp',
    'settings.updating': 'Updating…',
    'settings.fairUse': 'Only download what you have the right to keep, and respect the terms of the services you use.',
    'tools.banner': 'Needs to be installed: {tools}',
    'tools.install': 'Install automatically',
    'tools.installing': 'Installing {tool}… {percent}',
    'tools.manual': 'Install manually: {hint}',
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
