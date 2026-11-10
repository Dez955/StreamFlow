import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { setLanguage } from '../lib/i18n';
import { job } from '../test-fixtures';
import type { JobState } from '../lib/types';

vi.mock('../lib/api', () => ({
  pause: vi.fn(async () => {}),
  resume: vi.fn(async () => {}),
  stop: vi.fn(async () => {}),
  remove: vi.fn(async () => {}),
  openJob: vi.fn(async () => {}),
}));

import * as api from '../lib/api';
import JobCard from './JobCard.svelte';

beforeEach(() => {
  setLanguage('en');
  vi.clearAllMocks();
});

const buttons = () => screen.queryAllByRole('button').map((b) => b.textContent?.trim());

describe('JobCard buttons per state', () => {
  const table: [JobState, string[]][] = [
    ['queued', ['Stop']],
    ['downloading', ['Pause', 'Stop']],
    ['processing', ['Pause', 'Stop']],
    ['paused', ['Resume', 'Stop']],
    ['stopped', ['Resume', 'Remove']],
    ['done', ['Open', 'Remove']],
    ['failed', ['Retry', 'Remove']],
  ];
  for (const [state, expected] of table) {
    it(`${state}: ${expected.join(', ')}`, () => {
      render(JobCard, { props: { job: job(state) } });
      expect(buttons()).toEqual(expected);
    });
  }
});

describe('JobCard progress', () => {
  const progress = { downloaded: 50_000_000, total: 200_000_000, speedBps: 1_000_000, etaSecs: 150 };

  it('shows percent, sizes, speed and ETA', () => {
    render(JobCard, { props: { job: job('downloading', { progress }) } });
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '25');
    expect(screen.getByText(/25%/)).toBeInTheDocument();
    expect(screen.getByText(/50 MB \/ 200 MB/)).toBeInTheDocument();
    expect(screen.getByText(/2:30/)).toBeInTheDocument();
  });

  it('is indeterminate when the total is unknown', () => {
    render(JobCard, { props: { job: job('downloading', { progress: { ...progress, total: null } }) } });
    expect(screen.getByRole('progressbar')).not.toHaveAttribute('aria-valuenow');
    expect(screen.queryByText(/NaN|Infinity/)).toBeNull();
  });

  it('is full when done', () => {
    render(JobCard, { props: { job: job('done', { filePath: '/x/y.mp4' }) } });
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '100');
    expect(screen.getByText('Done!')).toBeInTheDocument();
  });

  it('shows the title', () => {
    render(JobCard, { props: { job: job('queued') } });
    expect(screen.getByText('My video')).toBeInTheDocument();
    expect(screen.getByText('Waiting…')).toBeInTheDocument();
  });
});

describe('JobCard actions', () => {
  it('pause, stop calls the api', async () => {
    render(JobCard, { props: { job: job('downloading') } });
    await userEvent.click(screen.getByRole('button', { name: 'Pause' }));
    expect(api.pause).toHaveBeenCalledWith('job-1');
    await userEvent.click(screen.getByRole('button', { name: 'Stop' }));
    expect(api.stop).toHaveBeenCalledWith('job-1');
  });

  it('resume and retry call resume', async () => {
    const { unmount } = render(JobCard, { props: { job: job('paused') } });
    await userEvent.click(screen.getByRole('button', { name: 'Resume' }));
    unmount();
    render(JobCard, { props: { job: job('failed', { error: { kind: 'network', detail: 'x' } }) } });
    await userEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(api.resume).toHaveBeenCalledTimes(2);
  });

  it('open reveals the file; remove deletes and notifies the parent', async () => {
    const onremove = vi.fn();
    render(JobCard, { props: { job: job('done', { filePath: '/x/y.mp4' }), onremove } });
    await userEvent.click(screen.getByRole('button', { name: 'Open' }));
    expect(api.openJob).toHaveBeenCalledWith('job-1');
    await userEvent.click(screen.getByRole('button', { name: 'Remove' }));
    expect(api.remove).toHaveBeenCalledWith('job-1');
    expect(onremove).toHaveBeenCalledWith('job-1');
  });

  it('failed shows a localized error with the cookies hint for login errors', () => {
    render(JobCard, { props: { job: job('failed', { error: { kind: 'loginRequired', detail: 'Sign in' } }) } });
    expect(screen.getByRole('alert')).toHaveTextContent(/cookies/i);
  });
});
