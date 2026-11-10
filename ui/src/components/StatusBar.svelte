<script lang="ts">
  import { tr } from '../lib/i18n';
  import type { JobSnapshot } from '../lib/types';

  interface Props {
    jobs: JobSnapshot[];
    onsettings: () => void;
  }
  let { jobs, onsettings }: Props = $props();

  const active = $derived(jobs.filter((j) => ['queued', 'downloading', 'processing'].includes(j.state)).length);
  const done = $derived(jobs.filter((j) => j.state === 'done').length);
</script>

<footer class="statusbar">
  <span>{$tr('stats.active', { n: active })}</span>
  <span>{$tr('stats.done', { n: done })}</span>
  <span>{$tr('stats.total', { n: jobs.length })}</span>
  <button type="button" class="link" onclick={onsettings}>
    {$tr('settings')}
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
      <circle cx="12" cy="12" r="3" />
      <path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1 7 17M17 7l2.1-2.1" />
    </svg>
  </button>
</footer>
