import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { setLanguage } from './lib/i18n';
import { fullProbe } from './test-fixtures';

vi.mock('./lib/api', () => ({
  analyze: vi.fn(),
  start: vi.fn(async () => 'job-1'),
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
