<script lang="ts">
  import { onMount } from 'svelte';
  import * as api from './lib/api';
  import { setLanguage, tr } from './lib/i18n';
  import { toUserError } from './lib/errors';
  import { normalizeUrl } from './lib/url';
  import MediaCard from './components/MediaCard.svelte';
  import QueuePanel from './components/QueuePanel.svelte';
  import SettingsDialog from './components/SettingsDialog.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import ToolsBanner from './components/ToolsBanner.svelte';
  import UrlBar from './components/UrlBar.svelte';
  import type { DownloadRequest, JobSnapshot, Settings, ToolsStatus, UserError, VideoProbe } from './lib/types';

  type Phase =
    | { kind: 'idle' }
    | { kind: 'analyzing' }
    | { kind: 'ready'; probe: VideoProbe; url: string }
    | { kind: 'error'; error: UserError };

  let phase = $state<Phase>({ kind: 'idle' });
  let input = $state('');
  let jobs = $state<JobSnapshot[]>([]);
  let settings = $state<Settings | null>(null);
  let tools = $state<ToolsStatus | null>(null); // unknown until the backend answers
  let showSettings = $state(false);

  function applySettings(s: Settings) {
    setLanguage(s.language);
    if (s.theme === 'auto') delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = s.theme;
  }

  function upsert(job: JobSnapshot) {
    jobs = jobs.some((j) => j.id === job.id) ? jobs.map((j) => (j.id === job.id ? job : j)) : [...jobs, job];
  }

  onMount(() => {
    let off: (() => void) | undefined;
    let alive = true;
    (async () => {
      const unlisten = await api.onJobChanged(upsert);
      if (alive) off = unlisten;
      else unlisten();
      const [s, existing, status] = await Promise.all([api.getSettings(), api.listJobs(), api.toolsStatus()]);
      settings = s;
      applySettings(s);
      for (const j of existing) upsert(j);
      tools = status;
    })().catch(() => {});
    return () => {
      alive = false;
      off?.();
    };
  });

  async function analyze(raw: string) {
    const url = normalizeUrl(raw);
    if (!url) {
      phase = { kind: 'error', error: { kind: 'invalidUrl', detail: '' } };
      return;
    }
    phase = { kind: 'analyzing' };
    try {
      phase = { kind: 'ready', probe: await api.analyze(url), url };
    } catch (e) {
      phase = { kind: 'error', error: toUserError(e) };
    }
  }

  async function startDownload(req: DownloadRequest) {
    try {
      await api.start(req);
      phase = { kind: 'idle' };
      input = '';
    } catch (e) {
      phase = { kind: 'error', error: toUserError(e) };
    }
  }

  async function saveSettings(next: Settings) {
    try {
      settings = await api.setSettings(next);
      applySettings(settings);
      showSettings = false;
    } catch (e) {
      phase = { kind: 'error', error: toUserError(e) };
      showSettings = false;
    }
  }
</script>

<main class="app">
  <header class="brand">
    <span class="logo" aria-hidden="true">▶</span>
    <h1>StreamFlow</h1>
  </header>

  <UrlBar bind:value={input} onsubmit={analyze} busy={phase.kind === 'analyzing'} />

  {#if tools}
    <ToolsBanner status={tools} onupdated={(s) => (tools = s)} />
  {/if}

  {#if phase.kind === 'analyzing'}
    <p class="status">{$tr('analyzing')}</p>
  {:else if phase.kind === 'error'}
    <p class="error" role="alert">{$tr(`error.${phase.error.kind}`, { detail: phase.error.detail })}</p>
  {:else if phase.kind === 'ready'}
    {#key phase.url}
      <MediaCard probe={phase.probe} url={phase.url} ondownload={startDownload} />
    {/key}
  {/if}

  <QueuePanel {jobs} onremove={(id) => (jobs = jobs.filter((j) => j.id !== id))} />
  <StatusBar {jobs} onsettings={() => (showSettings = true)} />
</main>

{#if showSettings && settings && tools}
  <SettingsDialog
    {settings}
    {tools}
    onsave={saveSettings}
    onclose={() => (showSettings = false)}
    ontools={(s) => (tools = s)}
  />
{/if}
