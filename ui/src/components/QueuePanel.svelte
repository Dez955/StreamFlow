<script lang="ts">
  import { tr } from '../lib/i18n';
  import JobCard from './JobCard.svelte';
  import type { JobSnapshot } from '../lib/types';

  interface Props {
    jobs: JobSnapshot[];
    onremove?: (id: string) => void;
  }
  let { jobs, onremove }: Props = $props();
</script>

<section class="card queue">
  <h2>{$tr('downloadStatus')}</h2>
  {#if jobs.length === 0}
    <p class="empty">{$tr('noDownloads')}</p>
  {:else}
    <div class="grid">
      {#each jobs as job (job.id)}
        <JobCard {job} {onremove} />
      {/each}
    </div>
  {/if}
</section>
