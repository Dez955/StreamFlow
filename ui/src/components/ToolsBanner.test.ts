import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { setLanguage } from '../lib/i18n';

vi.mock('../lib/api', () => ({
  toolsInstall: vi.fn(),
  toolsHint: vi.fn(async (tool: string) => `brew install ${tool === 'ytdlp' ? 'yt-dlp' : 'ffmpeg'}`),
  onToolsProgress: vi.fn(async () => () => {}),
}));

import * as api from '../lib/api';
import ToolsBanner from './ToolsBanner.svelte';

const install = api.toolsInstall as unknown as ReturnType<typeof vi.fn>;

beforeEach(() => {
  setLanguage('en');
  vi.clearAllMocks();
});

describe('ToolsBanner', () => {
  it('renders nothing when both tools are present', () => {
    const { container } = render(ToolsBanner, { props: { status: { ytdlp: '/a', ffmpeg: '/b' }, onupdated: vi.fn() } });
    expect(container.querySelector('.banner')).toBeNull();
  });

  it('names the missing tools and installs them automatically', async () => {
    install.mockResolvedValue({ ytdlp: '/a', ffmpeg: '/b' });
    const onupdated = vi.fn();
    render(ToolsBanner, { props: { status: { ytdlp: null, ffmpeg: '/b' }, onupdated } });
    expect(screen.getByText(/yt-dlp/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Install automatically' }));
    expect(install).toHaveBeenCalledTimes(1);
    expect(install).toHaveBeenCalledWith('ytdlp');
    await vi.waitFor(() => expect(onupdated).toHaveBeenCalledWith({ ytdlp: '/a', ffmpeg: '/b' }));
  });

  it('shows the manual command when automatic installation fails', async () => {
    install.mockRejectedValue({ kind: 'network', detail: 'offline' });
    render(ToolsBanner, { props: { status: { ytdlp: null, ffmpeg: null }, onupdated: vi.fn() } });
    await userEvent.click(screen.getByRole('button', { name: 'Install automatically' }));
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('brew install yt-dlp');
    expect(alert).toHaveTextContent('No network connection');
  });
});
