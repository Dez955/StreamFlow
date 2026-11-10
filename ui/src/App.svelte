<script lang="ts">
  import * as api from './lib/api';
  import { tr } from './lib/i18n';
  import { normalizeUrl } from './lib/url';
  import MediaCard from './components/MediaCard.svelte';
  import UrlBar from './components/UrlBar.svelte';
  import type { DownloadRequest, UserError, VideoProbe } from './lib/types';

  type Phase =
    | { kind: 'idle' }
    | { kind: 'analyzing' }
    | { kind: 'ready'; probe: VideoProbe; url: string }
    | { kind: 'error'; error: UserError };

  let phase = $state<Phase>({ kind: 'idle' });
  let input = $state('');

  function toUserError(e: unknown): UserError {
    if (e && typeof e === 'object' && 'kind' in e) return e as UserError;
    return { kind: 'other', detail: String(e) };
  }

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
</script>

<main class="app">
  <header class="brand">
    <span class="logo" aria-hidden="true">▶</span>
    <h1>StreamFlow</h1>
  </header>

  <UrlBar bind:value={input} onsubmit={analyze} busy={phase.kind === 'analyzing'} />

  {#if phase.kind === 'analyzing'}
    <p class="status">{$tr('analyzing')}</p>
  {:else if phase.kind === 'error'}
    <p class="error" role="alert">{$tr(`error.${phase.error.kind}`, { detail: phase.error.detail })}</p>
  {:else if phase.kind === 'ready'}
    {#key phase.url}
      <MediaCard probe={phase.probe} url={phase.url} ondownload={startDownload} />
    {/key}
  {/if}
</main>
