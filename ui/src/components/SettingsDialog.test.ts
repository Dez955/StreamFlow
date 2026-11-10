import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { setLanguage } from '../lib/i18n';
import { defaultSettings } from '../test-fixtures';

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn(async () => '/Users/t/Videos') }));
vi.mock('../lib/api', () => ({
  toolsUpdate: vi.fn(async () => ({ ytdlp: '/x/yt-dlp', ffmpeg: '/x/ffmpeg' })),
}));

import * as api from '../lib/api';
import SettingsDialog from './SettingsDialog.svelte';

beforeEach(() => {
  setLanguage('en');
  vi.clearAllMocks();
});

const open = (props: Record<string, unknown> = {}) => {
  const onsave = vi.fn();
  const onclose = vi.fn();
  render(SettingsDialog, {
    props: { settings: defaultSettings, tools: { ytdlp: '/x/yt-dlp', ffmpeg: null }, onsave, onclose, ontools: vi.fn(), ...props },
  });
  return { onsave, onclose };
};

describe('SettingsDialog', () => {
  it('shows the default folder and the fair-use note', () => {
    open();
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText('Downloads (default)')).toBeInTheDocument();
    expect(screen.getByText(/only download what you have the right to keep/i)).toBeInTheDocument();
  });

  it('saves the chosen folder and container', async () => {
    const { onsave } = open();
    await userEvent.click(screen.getByRole('button', { name: 'Choose folder' }));
    expect(await screen.findByText('/Users/t/Videos')).toBeInTheDocument();
    await userEvent.selectOptions(screen.getByLabelText('Video container'), 'mkv');
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(onsave).toHaveBeenCalledWith({ ...defaultSettings, outputDir: '/Users/t/Videos', container: 'mkv' });
  });

  it('resets the folder to the default', async () => {
    const { onsave } = open({ settings: { ...defaultSettings, outputDir: '/a/b' } });
    await userEvent.click(screen.getByRole('button', { name: 'Reset' }));
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(onsave.mock.calls[0][0].outputDir).toBeNull();
  });

  it('lists tool status and updates yt-dlp', async () => {
    const ontools = vi.fn();
    open({ ontools });
    expect(screen.getByText(/yt-dlp: installed/)).toBeInTheDocument();
    expect(screen.getByText(/ffmpeg: not installed/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Update yt-dlp' }));
    expect(api.toolsUpdate).toHaveBeenCalled();
    await vi.waitFor(() => expect(ontools).toHaveBeenCalledWith({ ytdlp: '/x/yt-dlp', ffmpeg: '/x/ffmpeg' }));
  });

  it('closes without saving', async () => {
    const { onsave, onclose } = open();
    await userEvent.click(screen.getByRole('button', { name: 'Close' }));
    expect(onclose).toHaveBeenCalled();
    expect(onsave).not.toHaveBeenCalled();
  });
});
