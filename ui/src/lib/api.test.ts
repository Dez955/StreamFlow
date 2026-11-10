import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(async () => undefined) }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }));

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import * as api from './api';

beforeEach(() => vi.clearAllMocks());

describe('api', () => {
  it('maps calls to Tauri commands with the argument names Rust expects', async () => {
    await api.analyze('https://a.b');
    expect(invoke).toHaveBeenCalledWith('analyze', { url: 'https://a.b' });
    await api.pause('job-1');
    expect(invoke).toHaveBeenCalledWith('pause', { id: 'job-1' });
    await api.openJob('job-1');
    expect(invoke).toHaveBeenCalledWith('open_job', { id: 'job-1' });
    await api.toolsInstall('ytdlp');
    expect(invoke).toHaveBeenCalledWith('tools_install', { tool: 'ytdlp' });
    await api.setSettings({ maxParallel: 3 } as never);
    expect(invoke).toHaveBeenCalledWith('set_settings', { settings: { maxParallel: 3 } });
  });

  it('forwards event payloads to the callback', async () => {
    const cb = vi.fn();
    await api.onJobChanged(cb);
    const handler = (listen as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(handler[0]).toBe('job-changed');
    handler[1]({ payload: { id: 'job-1' } });
    expect(cb).toHaveBeenCalledWith({ id: 'job-1' });
  });
});
