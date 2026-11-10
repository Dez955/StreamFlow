import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { setLanguage } from '../lib/i18n';
import UrlBar from './UrlBar.svelte';

beforeEach(() => setLanguage('en'));
const hint = 'Paste a link (for example, https://...)';

describe('UrlBar', () => {
  it('submits the typed link on Enter', async () => {
    const onsubmit = vi.fn();
    render(UrlBar, { props: { onsubmit } });
    await userEvent.type(screen.getByPlaceholderText(hint), 'https://a.b/c{Enter}');
    expect(onsubmit).toHaveBeenCalledWith('https://a.b/c');
  });

  it('submits with the search button and ignores empty input', async () => {
    const onsubmit = vi.fn();
    render(UrlBar, { props: { onsubmit } });
    await userEvent.click(screen.getByRole('button', { name: 'Analyze' }));
    expect(onsubmit).not.toHaveBeenCalled();
    await userEvent.type(screen.getByPlaceholderText(hint), 'https://a.b');
    await userEvent.click(screen.getByRole('button', { name: 'Analyze' }));
    expect(onsubmit).toHaveBeenCalledWith('https://a.b');
  });

  it('is disabled while busy', () => {
    render(UrlBar, { props: { onsubmit: vi.fn(), busy: true } });
    expect(screen.getByPlaceholderText(hint)).toBeDisabled();
  });
});
