<script lang="ts">
  import { onMount } from 'svelte';
  import * as api from '../lib/api';
  import { language, tr } from '../lib/i18n';
  import { formatPercent } from '../lib/format';
  import { toUserError } from '../lib/errors';
  import type { ToolId, ToolsProgress, ToolsStatus, UserError } from '../lib/types';

  interface Props {
    status: ToolsStatus;
    onupdated: (status: ToolsStatus) => void;
  }
  let { status, onupdated }: Props = $props();

  const NAMES: Record<ToolId, string> = { ytdlp: 'yt-dlp', ffmpeg: 'ffmpeg' };
  let busy = $state(false);
  let progress = $state<ToolsProgress | null>(null);
  let failure = $state<{ error: UserError; hint: string } | null>(null);

  const missing = $derived((['ytdlp', 'ffmpeg'] as ToolId[]).filter((id) => !status[id]));

  onMount(() => {
    let off: (() => void) | undefined;
    let alive = true;
    api.onToolsProgress((p) => (progress = p)).then((u) => (alive ? (off = u) : u()));
    return () => {
      alive = false;
      off?.();
    };
  });

  async function install() {
    busy = true;
    failure = null;
    let latest = status;
    for (const id of [...missing]) {
      try {
        latest = await api.toolsInstall(id);
      } catch (e) {
        failure = { error: toUserError(e), hint: await api.toolsHint(id).catch(() => '') };
        busy = false;
        progress = null;
        return;
      }
    }
    busy = false;
    progress = null;
    onupdated(latest);
  }

  const percent = $derived(progress ? formatPercent({ downloaded: progress.downloaded, total: progress.total, speedBps: null, etaSecs: null }) : null);
</script>

{#if missing.length > 0}
  <section class="banner">
    <p>{$tr('tools.banner', { tools: missing.map((id) => NAMES[id]).join(', ') })}</p>
    {#if busy && progress}
      <p class="status">{$tr('tools.installing', { tool: NAMES[progress.tool], percent: percent == null ? '' : `${percent}%` })}</p>
    {/if}
    {#if failure}
      <p class="error" role="alert">
        {$tr(`error.${failure.error.kind}`, { detail: failure.error.detail })}
        {#if failure.hint}{$tr('tools.manual', { hint: failure.hint })}{/if}
      </p>
    {/if}
    <button type="button" class="primary" disabled={busy} onclick={install}>{$tr('tools.install')}</button>
  </section>
{/if}
