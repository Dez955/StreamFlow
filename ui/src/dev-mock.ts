// Browser-only harness (open /mock.html with `npm run dev`): fakes the Tauri IPC so the UI can be
// inspected without the native shell. Not part of the production build.
type Handler = (e: { event: string; id: number; payload: unknown }) => void;
const callbacks = new Map<number, Handler>();
const listeners = new Map<string, number[]>();
let nextCallback = 1;

const svg = (a: string, b: string) =>
  `data:image/svg+xml;utf8,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="320" height="180"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${a}"/><stop offset="1" stop-color="${b}"/></linearGradient></defs><rect width="320" height="180" fill="url(#g)"/></svg>`)}`;

const mk = (id: string, title: string, state: string, downloaded: number, total: number | null, thumb: string | null) => ({
  id,
  request: { url: 'https://x.y/' + id, title, thumbnail: thumb, video: 'best', audio: 'best', audioOnlyFormat: 'mp3' },
  state,
  progress: downloaded ? { downloaded, total, speedBps: 4_200_000, etaSecs: 95 } : null,
  filePath: state === 'done' ? '/Users/me/Downloads/x.mp4' : null,
  error: null,
});

const jobs = [
  mk('job-1', 'Beautiful Landscapes of Europe', 'downloading', 1_200_000_000, 1_620_000_000, svg('#2e8b57', '#4f9de0')),
  mk('job-2', 'Beautiful Landscapes of Europe', 'done', 1_620_000_000, 1_620_000_000, svg('#2e8b57', '#4f9de0')),
  mk('job-3', 'WWDC 2024 Keynote - Highlights', 'downloading', 480_000_000, 1_600_000_000, svg('#222244', '#6a3f8f')),
  mk('job-4', 'Python Programming Tutorial', 'queued', 0, null, svg('#1f2a37', '#3a4a5a')),
];

const probe = {
  info: { id: 'abc', title: 'Beautiful Landscapes of Europe', uploader: 'Nature Channel', durationSecs: 212, thumbnail: svg('#2e8b57', '#4f9de0'), webpageUrl: 'https://x.y/abc', isPlaylist: false },
  video: [
    { formatId: '313', height: 2160, codecLabel: 'VP9', sizeBytes: 1_500_000_000 },
    { formatId: '137', height: 1080, codecLabel: 'H.264', sizeBytes: 300_000_000 },
    { formatId: '136', height: 720, codecLabel: 'H.264', sizeBytes: 150_000_000 },
  ],
  audio: [
    { formatId: '251', abr: 160, codecLabel: 'Opus 160k', sizeBytes: 5_000_000 },
    { formatId: '140', abr: 128, codecLabel: 'AAC 128k', sizeBytes: 4_000_000 },
  ],
  estimateBest: 1_505_000_000,
};

const emit = (event: string, payload: unknown) =>
  (listeners.get(event) ?? []).forEach((id) => callbacks.get(id)?.({ event, id, payload }));

(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
  transformCallback(cb: Handler) {
    const id = nextCallback++;
    callbacks.set(id, cb);
    return id;
  },
  unregisterCallback(id: number) {
    callbacks.delete(id);
  },
  convertFileSrc: (p: string) => p,
  async invoke(cmd: string, args: Record<string, unknown> = {}) {
    switch (cmd) {
      case 'plugin:event|listen': {
        const event = args.event as string;
        listeners.set(event, [...(listeners.get(event) ?? []), args.handler as number]);
        return args.handler;
      }
      case 'plugin:event|unlisten':
        return;
      case 'tools_status':
        return { ytdlp: '/mock/yt-dlp', ffmpeg: '/mock/ffmpeg' };
      case 'get_settings':
        return { outputDir: null, language: 'auto', theme: 'auto', maxParallel: 2, cookiesFrom: 'none', container: 'mp4' };
      case 'set_settings':
        return args.settings;
      case 'list_jobs':
        return jobs;
      case 'analyze':
        await new Promise((r) => setTimeout(r, 600));
        return probe;
      case 'start': {
        const j = mk('job-' + (jobs.length + 1), (args.req as { title: string }).title, 'queued', 0, null, svg('#2e8b57', '#4f9de0'));
        jobs.push(j);
        setTimeout(() => emit('job-changed', j), 50);
        return j.id;
      }
      default:
        return;
    }
  },
};
