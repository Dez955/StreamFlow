import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  DownloadRequest,
  JobSnapshot,
  Settings,
  ToolId,
  ToolsProgress,
  ToolsStatus,
  VideoProbe,
} from './types';

export const analyze = (url: string) => invoke<VideoProbe>('analyze', { url });
export const start = (req: DownloadRequest) => invoke<string>('start', { req });
export const pause = (id: string) => invoke<void>('pause', { id });
export const resume = (id: string) => invoke<void>('resume', { id });
export const stop = (id: string) => invoke<void>('stop', { id });
export const remove = (id: string) => invoke<void>('remove', { id });
export const listJobs = () => invoke<JobSnapshot[]>('list_jobs');
export const openJob = (id: string) => invoke<void>('open_job', { id });
export const getSettings = () => invoke<Settings>('get_settings');
export const setSettings = (settings: Settings) => invoke<Settings>('set_settings', { settings });
export const toolsStatus = () => invoke<ToolsStatus>('tools_status');
export const toolsInstall = (tool: ToolId) => invoke<ToolsStatus>('tools_install', { tool });
export const toolsUpdate = () => invoke<ToolsStatus>('tools_update');

export const onJobChanged = (cb: (job: JobSnapshot) => void): Promise<UnlistenFn> =>
  listen<JobSnapshot>('job-changed', (e) => cb(e.payload));
export const onToolsProgress = (cb: (p: ToolsProgress) => void): Promise<UnlistenFn> =>
  listen<ToolsProgress>('tools-progress', (e) => cb(e.payload));

export const toolsHint = (tool: ToolId) => invoke<string>('tools_hint', { tool });
