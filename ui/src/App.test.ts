import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { setLanguage } from './lib/i18n';
import { fullProbe, job } from './test-fixtures';

vi.mock('./lib/api', () => ({
  analyze: vi.fn(),
  start: vi.fn(async () => 'job-1'),
  listJobs: vi.fn(async () => []),
  getSettings: vi.fn(async () => ({ outputDir: null, language: 'auto', theme: 'auto', maxParallel: 2, cookiesFrom: 'none', container: 'mp4' })),
  setSettings: vi.fn(async (s: unknown) => s),
  toolsStatus: vi.fn(async () => ({ ytdlp: '/x/yt-dlp', ffmpeg: '/x/ffmpeg' })),
  toolsInstall: vi.fn(),
  toolsHint: vi.fn(async () => 'brew install yt-dlp'),
  toolsUpdate: vi.fn(),
  onJobChanged: vi.fn(async (cb: unknown) => {
    (globalThis as Record<string, unknown>).__emitJob = cb;
    return () => {};
  }),
  onToolsProgress: vi.fn(async () => () => {}),
  pause: vi.fn(), resume: vi.fn(), stop: vi.fn(), remove: vi.fn(), openJob: vi.fn(),
}));

import * as api from './lib/api';
import App from './App.svelte';

const analyze = api.analyze as unknown as ReturnType<typeof vi.fn>;
const start = api.start as unknown as ReturnType<typeof vi.fn>;
const hint = 'Paste a link (for example, https://...)';

beforeEach(() => {
  setLanguage('en');
  vi.clearAllMocks();
});

describe('App flow', () => {
  it('rejects non-links without calling the backend', async () => {
    render(App);
    await userEvent.type(screen.getByPlaceholderText(hint), 'привіт{Enter}');
    expect(analyze).not.toHaveBeenCalled();
    expect(await screen.findByRole('alert')).toHaveTextContent(/doesn't look like a link/i);
  });

  it('analyzes a trimmed link, shows the card, and starts the download', async () => {
    analyze.mockResolvedValue(fullProbe);
    render(App);
    await userEvent.type(screen.getByPlaceholderText(hint), '  https://site.example/watch?v=abc {Enter}');
    expect(analyze).toHaveBeenCalledWith('https://site.example/watch?v=abc');
    expect(await screen.findByText('Beautiful Landscapes of Europe')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Download' }));
    expect(start).toHaveBeenCalledWith(expect.objectContaining({ url: 'https://site.example/watch?v=abc', video: 'best' }));
    expect(screen.queryByText('Beautiful Landscapes of Europe')).toBeNull();
    expect(screen.getByPlaceholderText(hint)).toHaveValue('');
  });

  it('shows a localized message when the backend reports an error', async () => {
    analyze.mockRejectedValue({ kind: 'unsupported', detail: 'Unsupported URL' });
    render(App);
    await userEvent.type(screen.getByPlaceholderText(hint), 'https://nope.example/x{Enter}');
    expect(await screen.findByRole('alert')).toHaveTextContent(/not supported/i);
  });
});

describe('App queue and tools', () => {
  it('lists existing jobs and applies live updates', async () => {
    (api.listJobs as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce([job('downloading')]);
    render(App);
    expect(await screen.findByText('My video')).toBeInTheDocument();
    expect(await screen.findByText('Active downloads: 1')).toBeInTheDocument();
    const emit = (globalThis as Record<string, unknown>).__emitJob as (j: unknown) => void;
    emit(job('done', { filePath: '/x/y.mp4' }));
    expect(await screen.findByText('Completed: 1')).toBeInTheDocument();
  });

  it('offers installation when tools are missing', async () => {
    (api.toolsStatus as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce({ ytdlp: null, ffmpeg: null });
    render(App);
    expect(await screen.findByRole('button', { name: 'Install automatically' })).toBeInTheDocument();
  });

  it('does not flash the install banner before the tools status arrives', async () => {
    (api.toolsStatus as unknown as ReturnType<typeof vi.fn>).mockReturnValueOnce(new Promise(() => {}));
    render(App);
    await new Promise((r) => setTimeout(r, 30));
    expect(screen.queryByRole('button', { name: 'Install automatically' })).toBeNull();
  });

  it('opens and closes the settings dialog', async () => {
    render(App);
    await userEvent.click(await screen.findByRole('button', { name: /Settings/ }));
    expect(await screen.findByRole('dialog')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Close' }));
    expect(screen.queryByRole('dialog')).toBeNull();
  });
});
