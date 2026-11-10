import type { VideoProbe } from './lib/types';

export const fullProbe: VideoProbe = {
  info: {
    id: 'abc',
    title: 'Beautiful Landscapes of Europe',
    uploader: 'Someone',
    durationSecs: 212,
    thumbnail: 'https://img.example/t.jpg',
    webpageUrl: 'https://site.example/watch?v=abc',
    isPlaylist: false,
  },
  video: [
    { formatId: '313', height: 2160, codecLabel: 'VP9', sizeBytes: 1_500_000_000 },
    { formatId: '137', height: 1080, codecLabel: 'H.264', sizeBytes: 300_000_000 },
  ],
  audio: [{ formatId: '251', abr: 160, codecLabel: 'Opus 160k', sizeBytes: 5_000_000 }],
  estimateBest: 1_505_000_000,
};

import type { JobSnapshot, JobState, Settings } from './lib/types';

export const job = (state: JobState, extra: Partial<JobSnapshot> = {}): JobSnapshot => ({
  id: 'job-1',
  request: { url: 'https://x.y/v', title: 'My video', thumbnail: null, video: 'best', audio: 'best', audioOnlyFormat: 'mp3' },
  state,
  progress: null,
  filePath: null,
  error: null,
  ...extra,
});

export const defaultSettings: Settings = {
  outputDir: null,
  language: 'auto',
  theme: 'auto',
  maxParallel: 2,
  cookiesFrom: 'none',
  container: 'mp4',
};
