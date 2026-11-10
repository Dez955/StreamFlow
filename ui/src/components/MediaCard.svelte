<script lang="ts">
  import { untrack } from 'svelte';
  import { tr } from '../lib/i18n';
  import { formatEta } from '../lib/format';
  import QualityPicker from './QualityPicker.svelte';
  import type { AudioOnlyFormat, DownloadRequest, VideoProbe } from '../lib/types';

  interface Props {
    probe: VideoProbe;
    url: string;
    ondownload: (req: DownloadRequest) => void;
  }
  let { probe, url, ondownload }: Props = $props();

  // A source with no video streams (music sites) starts in audio-only mode.
  let videoValue = $state(untrack(() => (probe.video.length > 0 ? 'best' : 'none')));
  let audioValue = $state('best');
  let audioOnlyFormat = $state<AudioOnlyFormat>('mp3');

  function download() {
    ondownload({
      url,
      title: probe.info.title,
      thumbnail: probe.info.thumbnail,
      video: videoValue === 'best' ? 'best' : videoValue === 'none' ? 'none' : { format: videoValue },
      audio: audioValue === 'best' ? 'best' : { format: audioValue },
      audioOnlyFormat,
    });
  }

  const meta = $derived(
    [probe.info.uploader, formatEta(probe.info.durationSecs)].filter((x): x is string => !!x).join(' · '),
  );
</script>

<section class="card media">
  <div class="media-head">
    {#if probe.info.thumbnail}
      <img src={probe.info.thumbnail} alt="" referrerpolicy="no-referrer" />
    {/if}
    <div>
      <h2>{probe.info.title}</h2>
      {#if meta}<p class="meta">{meta}</p>{/if}
    </div>
  </div>
  {#if probe.info.isPlaylist}
    <p class="notice">{$tr('playlistNotice')}</p>
  {/if}
  <QualityPicker
    video={probe.video}
    audio={probe.audio}
    estimateBest={probe.estimateBest}
    bind:videoValue
    bind:audioValue
    bind:audioOnlyFormat
  />
  <button class="primary" type="button" onclick={download}>{$tr('download')}</button>
</section>
