<script lang="ts">
  import { language, tr } from '../lib/i18n';
  import { formatBytes } from '../lib/format';
  import type { AudioChoice, AudioOnlyFormat, VideoChoice } from '../lib/types';

  interface Props {
    video: VideoChoice[];
    audio: AudioChoice[];
    estimateBest: number | null;
    videoValue: string;
    audioValue: string;
    audioOnlyFormat: AudioOnlyFormat;
  }
  let {
    video,
    audio,
    estimateBest,
    videoValue = $bindable(),
    audioValue = $bindable(),
    audioOnlyFormat = $bindable(),
  }: Props = $props();

  const uid = $props.id();
  const audioOnly = $derived(videoValue === 'none');

  function sizeLabel(size: number | null): string {
    return size == null ? '' : ` · ${formatBytes(size, $language)}`;
  }

  const estimate = $derived.by((): number | null => {
    if (videoValue === 'best' && audioValue === 'best') return estimateBest;
    const parts: (number | null)[] = [];
    if (videoValue !== 'none') {
      const v = videoValue === 'best' ? video[0] : video.find((c) => c.formatId === videoValue);
      if (v) parts.push(v.sizeBytes);
    }
    if (audio.length > 0) {
      const a = audioValue === 'best' ? audio[0] : audio.find((c) => c.formatId === audioValue);
      if (a) parts.push(a.sizeBytes);
    }
    if (parts.length === 0 || parts.some((p) => p == null)) return null;
    return (parts as number[]).reduce((x, y) => x + y, 0);
  });
</script>

<div class="picker">
  {#if video.length > 0}
    <div class="field">
      <label for="{uid}-video">{$tr('video')}</label>
      <select id="{uid}-video" bind:value={videoValue}>
        <option value="best">{$tr('best')} ({video[0].height}p)</option>
        {#each video as c (c.formatId)}
          <option value={c.formatId}>{c.height}p {c.codecLabel}{sizeLabel(c.sizeBytes)}</option>
        {/each}
        <option value="none">{$tr('noVideo')}</option>
      </select>
    </div>
  {/if}

  {#if audio.length > 0}
    <div class="field">
      <label for="{uid}-audio">{$tr('audio')}</label>
      <select id="{uid}-audio" bind:value={audioValue}>
        <option value="best">{$tr('best')} ({audio[0].codecLabel})</option>
        {#each audio as c (c.formatId)}
          <option value={c.formatId}>{c.codecLabel}{sizeLabel(c.sizeBytes)}</option>
        {/each}
      </select>
    </div>
  {/if}

  {#if audioOnly}
    <div class="field">
      <label for="{uid}-fmt">{$tr('audioFormat')}</label>
      <select id="{uid}-fmt" bind:value={audioOnlyFormat}>
        <option value="mp3">MP3</option>
        <option value="m4a">M4A</option>
        <option value="opus">Opus</option>
      </select>
    </div>
  {/if}

  <span class="estimate" data-testid="estimate">{estimate == null ? '—' : `~${formatBytes(estimate, $language)}`}</span>
</div>
