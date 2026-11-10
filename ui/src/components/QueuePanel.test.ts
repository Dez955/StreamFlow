import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import { setLanguage } from '../lib/i18n';
import { job } from '../test-fixtures';

vi.mock('../lib/api', () => ({
  pause: vi.fn(), resume: vi.fn(), stop: vi.fn(), remove: vi.fn(), openJob: vi.fn(),
}));

import QueuePanel from './QueuePanel.svelte';
import StatusBar from './StatusBar.svelte';

beforeEach(() => setLanguage('en'));

describe('QueuePanel', () => {
  it('shows an empty hint', () => {
    render(QueuePanel, { props: { jobs: [] } });
    expect(screen.getByText('Download Status')).toBeInTheDocument();
    expect(screen.getByText('No downloads yet')).toBeInTheDocument();
  });

  it('renders one card per job', () => {
    const jobs = [job('downloading'), { ...job('done'), id: 'job-2', request: { ...job('done').request, title: 'Second' } }];
    render(QueuePanel, { props: { jobs } });
    expect(screen.getByText('My video')).toBeInTheDocument();
    expect(screen.getByText('Second')).toBeInTheDocument();
  });
});

describe('StatusBar', () => {
  it('counts active, completed and total', () => {
    const jobs = [job('downloading'), { ...job('processing'), id: 'b' }, { ...job('queued'), id: 'c' }, { ...job('done'), id: 'd' }];
    render(StatusBar, { props: { jobs, onsettings: vi.fn() } });
    expect(screen.getByText('Active downloads: 3')).toBeInTheDocument();
    expect(screen.getByText('Completed: 1')).toBeInTheDocument();
    expect(screen.getByText('Total: 4')).toBeInTheDocument();
  });

  it('paused, stopped and failed jobs are not active', () => {
    const jobs = [job('paused'), { ...job('stopped'), id: 'b' }, { ...job('failed'), id: 'c' }];
    render(StatusBar, { props: { jobs, onsettings: vi.fn() } });
    expect(screen.getByText('Active downloads: 0')).toBeInTheDocument();
  });

  it('opens settings', async () => {
    const { default: userEvent } = await import('@testing-library/user-event');
    const onsettings = vi.fn();
    render(StatusBar, { props: { jobs: [], onsettings } });
    await userEvent.click(screen.getByRole('button', { name: /Settings/ }));
    expect(onsettings).toHaveBeenCalled();
  });
});
