import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { setLanguage } from '../lib/i18n';
import { fullProbe } from '../test-fixtures';
import type { VideoProbe } from '../lib/types';
import MediaCard from './MediaCard.svelte';

beforeEach(() => setLanguage('en'));
const open = (probe: VideoProbe, ondownload = vi.fn()) => {
  render(MediaCard, { props: { probe, url: 'https://site.example/watch?v=abc', ondownload } });
  return ondownload;
};

describe('MediaCard quality pickers', () => {
  it('defaults both lists to Best and shows the estimated size', () => {
    open(fullProbe);
    expect(screen.getByLabelText('Video')).toHaveDisplayValue(/Best/);
    expect(screen.getByLabelText('Audio')).toHaveDisplayValue(/Best/);
    expect(screen.getByTestId('estimate')).toHaveTextContent('~1.5 GB');
  });

  it('shows a dash when the size is unknown', () => {
    open({ ...fullProbe, estimateBest: null, video: [{ ...fullProbe.video[0], sizeBytes: null }] });
    expect(screen.getByTestId('estimate')).toHaveTextContent('—');
    expect(screen.getByTestId('estimate')).not.toHaveTextContent('0 B');
  });

  it('lists every resolution plus an audio-only option', () => {
    open(fullProbe);
    const options = [...screen.getByLabelText('Video').querySelectorAll('option')].map((o) => o.value);
    expect(options).toEqual(['best', '313', '137', 'none']);
  });

  it('audio-only reveals the format select with mp3 as default', async () => {
    open(fullProbe);
    expect(screen.queryByLabelText('Audio format')).toBeNull();
    await userEvent.selectOptions(screen.getByLabelText('Video'), 'none');
    expect(screen.getByLabelText('Audio format')).toHaveDisplayValue('MP3');
  });

  it('hides the audio list when the source has no separate audio streams', () => {
    open({ ...fullProbe, audio: [] });
    expect(screen.queryByLabelText('Audio')).toBeNull();
  });

  it('audio-only sources hide the video list and download audio only', async () => {
    const ondownload = open({ ...fullProbe, video: [] });
    expect(screen.queryByLabelText('Video')).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Download' }));
    expect(ondownload.mock.calls[0][0]).toMatchObject({ video: 'none', audioOnlyFormat: 'mp3' });
  });

  it('shows a playlist notice', () => {
    open({ ...fullProbe, info: { ...fullProbe.info, isPlaylist: true } });
    expect(screen.getByText(/first video/i)).toBeInTheDocument();
  });
});

describe('MediaCard download request', () => {
  it('sends best/best by default with the typed url and title', async () => {
    const ondownload = open(fullProbe);
    await userEvent.click(screen.getByRole('button', { name: 'Download' }));
    expect(ondownload).toHaveBeenCalledWith({
      url: 'https://site.example/watch?v=abc',
      title: 'Beautiful Landscapes of Europe',
      thumbnail: 'https://img.example/t.jpg',
      video: 'best',
      audio: 'best',
      audioOnlyFormat: 'mp3',
    });
  });

  it('sends the chosen explicit formats', async () => {
    const ondownload = open(fullProbe);
    await userEvent.selectOptions(screen.getByLabelText('Video'), '137');
    await userEvent.selectOptions(screen.getByLabelText('Audio'), '251');
    await userEvent.click(screen.getByRole('button', { name: 'Download' }));
    expect(ondownload.mock.calls[0][0]).toMatchObject({ video: { format: '137' }, audio: { format: '251' } });
  });

  it('sends audio-only with the chosen container', async () => {
    const ondownload = open(fullProbe);
    await userEvent.selectOptions(screen.getByLabelText('Video'), 'none');
    await userEvent.selectOptions(screen.getByLabelText('Audio format'), 'm4a');
    await userEvent.click(screen.getByRole('button', { name: 'Download' }));
    expect(ondownload.mock.calls[0][0]).toMatchObject({ video: 'none', audioOnlyFormat: 'm4a' });
  });
});
