<script lang="ts">
  import { tr } from '../lib/i18n';

  interface Props {
    onsubmit: (url: string) => void;
    busy?: boolean;
    value?: string;
  }
  let { onsubmit, busy = false, value = $bindable('') }: Props = $props();

  function submit(event: Event) {
    event.preventDefault();
    if (value.trim()) onsubmit(value);
  }
</script>

<form class="urlbar" onsubmit={submit}>
  <input
    type="text"
    bind:value
    placeholder={$tr('pasteLink')}
    disabled={busy}
    autocomplete="off"
    autocapitalize="off"
    spellcheck="false"
  />
  <button type="submit" aria-label={$tr('analyze')} disabled={busy}>
    <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
      <circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" />
    </svg>
  </button>
</form>
