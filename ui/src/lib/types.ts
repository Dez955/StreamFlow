// Mirrors the serde (camelCase) types of streamflow-core.
export type VideoSelect = 'best' | 'none' | { format: string };
export type AudioSelect = 'best' | { format: string };
export type AudioOnlyFormat = 'mp3' | 'm4a' | 'opus';

export interface DownloadRequest {
  url: string;
  title: string;
  thumbnail: string | null;
  video: VideoSelect;
  audio: AudioSelect;
  audioOnlyFormat: AudioOnlyFormat;
}

export interface VideoInfo {
  id: string;
  title: string;
  uploader: string | null;
  durationSecs: number | null;
  thumbnail: string | null;
  webpageUrl: string;
  isPlaylist: boolean;
}

export interface VideoChoice {
  formatId: string;
  height: number;
  codecLabel: string;
  sizeBytes: number | null;
}

export interface AudioChoice {
  formatId: string;
  abr: number;
  codecLabel: string;
  sizeBytes: number | null;
}

export interface VideoProbe {
  info: VideoInfo;
  video: VideoChoice[];
  audio: AudioChoice[];
  estimateBest: number | null;
}

export type JobState = 'queued' | 'downloading' | 'processing' | 'paused' | 'stopped' | 'done' | 'failed';

export interface Progress {
  downloaded: number;
  total: number | null;
  speedBps: number | null;
  etaSecs: number | null;
}

export type ErrorKind =
  | 'network'
  | 'unsupported'
  | 'loginRequired'
  | 'drm'
  | 'diskFull'
  | 'needsUpdate'
  | 'toolMissing'
  | 'invalidUrl'
  | 'other';

export interface UserError {
  kind: ErrorKind;
  detail: string;
}

export interface JobSnapshot {
  id: string;
  request: DownloadRequest;
  state: JobState;
  progress: Progress | null;
  filePath: string | null;
  error: UserError | null;
}

export type ToolId = 'ytdlp' | 'ffmpeg';

export interface ToolsStatus {
  ytdlp: string | null;
  ffmpeg: string | null;
}

export interface ToolsProgress {
  tool: ToolId;
  downloaded: number;
  total: number | null;
}

export interface Settings {
  outputDir: string | null;
  language: 'auto' | 'uk' | 'en';
  theme: 'auto' | 'light' | 'dark';
  maxParallel: number;
  cookiesFrom: 'none' | 'safari' | 'chrome' | 'firefox';
  container: 'mp4' | 'mkv';
}
