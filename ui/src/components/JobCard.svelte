<script lang="ts">
  import * as api from '../lib/api';
  import { language, tr } from '../lib/i18n';
  import { formatBytes, formatEta, formatPercent, formatSpeed } from '../lib/format';
  import type { JobSnapshot } from '../lib/types';

  interface Props {
    job: JobSnapshot;
    onremove?: (id: string) => void;
  }
  let { job, onremove }: Props = $props();

  const active = $derived(job.state === 'downloading' || job.state === 'processing');
  const percent = $derived(job.state === 'done' ? 100 : job.progress ? formatPercent(job.progress) : null);
  const indeterminate = $derived(active && percent === null);
  const valueNow = $derived(indeterminate ? undefined : (percent ?? 0));

  const sizes = $derived.by(() => {
    const p = job.progress;
    if (!p) return '';
    return p.total != null ? `${formatBytes(p.downloaded, $language)} / ${formatBytes(p.total, $language)}` : formatBytes(p.downloaded, $language);
  });
  const speed = $derived(active ? formatSpeed(job.progress?.speedBps, $language) : '');
  const eta = $derived(active ? formatEta(job.progress?.etaSecs) : '');

  async function run(action: () => Promise<void>) {
    try {
      await action();
    } catch {
      /* the job snapshot carries the failure; a failed click needs no extra UI */
    }
  }

  async function remove() {
    await run(() => api.remove(job.id));
    onremove?.(job.id);
  }
</script>

<article class="job" data-state={job.state}>
  <div class="thumb">
    {#if job.request.thumbnail}
      <img src={job.request.thumbnail} alt="" referrerpolicy="no-referrer" />
    {/if}
  </div>
  <div class="body">
    <h3 title={job.request.title}>{job.request.title}</h3>
    <div class="bar" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={valueNow}>
      <div class="fill" class:indeterminate style:width={indeterminate ? undefined : `${valueNow}%`}></div>
    </div>
    <div class="details">
      {#if percent !== null}<span>{percent}%</span>{/if}
      {#if sizes}<span>{sizes}</span>{/if}
      {#if speed}<span>{speed}</span>{/if}
      {#if eta}<span>{eta}</span>{/if}
    </div>
  </div>
  {#if job.state === 'failed' && job.error}
    <p class="error" role="alert">{$tr(`error.${job.error.kind}`, { detail: job.error.detail })}</p>
  {/if}
  <div class="foot">
    <span class="state">{$tr(`state.${job.state}`)}</span>
    <div class="actions">
      {#if job.state === 'downloading' || job.state === 'processing'}
        <button type="button" onclick={() => run(() => api.pause(job.id))}>{$tr('btn.pause')}</button>
        <button type="button" onclick={() => run(() => api.stop(job.id))}>{$tr('btn.stop')}</button>
      {:else if job.state === 'queued'}
        <button type="button" onclick={() => run(() => api.stop(job.id))}>{$tr('btn.stop')}</button>
      {:else if job.state === 'paused'}
        <button type="button" onclick={() => run(() => api.resume(job.id))}>{$tr('btn.resume')}</button>
        <button type="button" onclick={() => run(() => api.stop(job.id))}>{$tr('btn.stop')}</button>
      {:else if job.state === 'stopped'}
        <button type="button" onclick={() => run(() => api.resume(job.id))}>{$tr('btn.resume')}</button>
        <button type="button" onclick={remove}>{$tr('btn.remove')}</button>
      {:else if job.state === 'done'}
        <button type="button" onclick={() => run(() => api.openJob(job.id))}>{$tr('btn.open')}</button>
        <button type="button" onclick={remove}>{$tr('btn.remove')}</button>
      {:else if job.state === 'failed'}
        <button type="button" onclick={() => run(() => api.resume(job.id))}>{$tr('btn.retry')}</button>
        <button type="button" onclick={remove}>{$tr('btn.remove')}</button>
      {/if}
    </div>
  </div>
</article>
